import { test } from 'node:test';
import assert from 'node:assert';
import {
  AggregatorError,
  createAggregatorClient,
} from '../src/lib/aggregator.js';
import { CapabilityError, createResolver } from '../src/lib/resolve.js';
import { createBreaker } from '../src/lib/circuit-breaker.js';

// ── aggregator ──────────────────────────────────────────────────────────────

// Transport palsu: aggregator menerima `http` lewat injeksi, jadi test tidak
// perlu mock modul axios sama sekali dan tidak pernah menyentuh jaringan.
function httpPalsu(handler) {
  const calls = [];
  return {
    calls,
    get(url, opts) {
      calls.push({ verb: 'get', url, opts });
      return handler(url, opts);
    },
    post(url, opts) {
      calls.push({ verb: 'post', url, opts });
      return handler(url, opts);
    },
  };
}

const kunci = {
  neoxr: 'k-neoxr',
  cuki: 'k-cuki',
};

test('hit neoxr menyusun URL dengan params dan mengirim header apikey', async () => {
  const http = httpPalsu(async () => ({ status: 200, data: { ok: true } }));
  const client = createAggregatorClient({
    http,
    keyOf: (name) => kunci[name] ?? '',
  });

  const out = await client.hit('neoxr', '/api/sfile', { params: { url: 'u' } });

  assert.equal(http.calls.length, 1);
  assert.equal(http.calls[0].url, 'https://api.neoxr.eu/api/sfile?url=u');
  assert.equal(http.calls[0].opts.headers.apikey, kunci.neoxr);
  assert.deepEqual(out, { ok: true });
});

test('aggregator tanpa key tidak mengirim header key sama sekali', async () => {
  const http = httpPalsu(async () => ({ status: 200, data: { ok: true } }));
  const client = createAggregatorClient({
    http,
    keyOf: (name) => kunci[name] ?? '',
  });

  await client.hit('nexray', '/api/stalker/github', { params: { username: 'torvalds' } });

  const headers = http.calls[0].opts.headers;
  assert.ok(
    !('apikey' in headers),
    `nexray tidak punya key, header tidak boleh ada. dapat: ${JSON.stringify(headers)}`,
  );
});

test('key dibutuhkan tapi kosong: gagal sebelum ada request, bukan 401 dari server', async () => {
  const http = httpPalsu(async () => ({ status: 200, data: { ok: true } }));
  const client = createAggregatorClient({ http, keyOf: () => '' });

  await assert.rejects(
    () => client.hit('neoxr', '/api/sfile'),
    (error) => {
      assert.ok(error instanceof AggregatorError);
      assert.equal(error.aggregator, 'neoxr');
      return true;
    },
  );
  assert.equal(http.calls.length, 0, 'tidak boleh=request kalau key sudah kosong');
});

test('respons 4xx/5xx jadi AggregatorError dengan status terisi', async () => {
  const http = httpPalsu(async () => ({ status: 503, data: {} }));
  const client = createAggregatorClient({ http, keyOf: () => 'k' });

  await assert.rejects(
    () => client.hit('nexray', '/x'),
    (error) => {
      assert.ok(error instanceof AggregatorError);
      assert.equal(error.status, 503, 'status upstream harus diteruskan apa adanya');
      return true;
    },
  );
});

test('kegagalan jaringan jadi AggregatorError dengan status 0', async () => {
  const http = httpPalsu(async () => {
    throw Object.assign(new Error('getaddrinfo ENOTFOUND'), { code: 'ENOTFOUND' });
  });
  const client = createAggregatorClient({ http, keyOf: () => 'k' });

  await assert.rejects(
    () => client.hit('nexray', '/x'),
    (error) => {
      assert.ok(error instanceof AggregatorError);
      assert.equal(error.status, 0, 'tidak ada respons HTTP, jadi bukan 500');
      return true;
    },
  );
});

test('nama aggregator asing jadi AggregatorError, bukan TypeError dari undefined', async () => {
  const http = httpPalsu(async () => ({ status: 200, data: {} }));
  const client = createAggregatorClient({ http, keyOf: () => '' });

  await assert.rejects(
    () => client.hit('nama-asing', '/x'),
    (error) => {
      assert.ok(
        error instanceof AggregatorError,
        `harus AggregatorError, dapat ${error?.constructor?.name}: ${error?.message}`,
      );
      assert.equal(error.aggregator, 'nama-asing');
      assert.equal(error.status, undefined, 'tidak ada respons sama sekali, status dibiarkan kosong');
      return true;
    },
  );
});

// ── facade ──────────────────────────────────────────────────────────────────

const okCap = {
  stable: false,
  normalize: (r) => ({ v: r }),
  backends: [{ name: 'a', kind: 'local', run: async () => ({ raw: 1 }) }],
};

const fallCap = {
  stable: false,
  normalize: (r) => ({ v: r }),
  backends: [
    { name: 'a', kind: 'local', run: async () => { throw new Error('mati'); } },
    { name: 'b', kind: 'api', run: async () => ({ raw: 2 }) },
  ],
};

const noDataCap = {
  stable: false,
  normalize: (r) => ({ v: r }),
  backends: [{ name: 'a', kind: 'local', run: async () => null }],
};

const stableCap = {
  stable: true,
  normalize: (r) => ({ v: r }),
  backends: [{ name: 'a', kind: 'local', run: async () => ({ raw: 3 }) }],
};

// RegistryCapabilities fake: fungsi panah yang mengembalikan objek kapabilitas
// langsung, tanpa file di disk, supaya test tidak bergantung pada modul yang
// belum dibuat.
const caps = (map) => Object.fromEntries(
  Object.entries(map).map(([name, cap]) => [name, () => cap]),
);

test('sukses lokal mengembalikan ok/source/data/meta', async () => {
  const resolver = createResolver({ capabilities: caps({ ok: okCap }) });
  const out = await resolver.resolve('ok');
  assert.equal(out.ok, true);
  assert.equal(out.source, 'a');
  assert.deepEqual(out.data, { v: { raw: 1 } });
  assert.equal(out.meta.cached, false);
  assert.equal(typeof out.meta.tookMs, 'number');
});

test('backend yang balas null diperlakukan gagal, bukan data kosong', async () => {
  const resolver = createResolver({ capabilities: caps({ nodata: noDataCap }) });
  await assert.rejects(
    () => resolver.resolve('nodata'),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.capability, 'nodata');
      assert.match(error.tried[0].reason, /tanpa data/);
      return true;
    },
  );
});

test('fallback: api dipakai setelah local gagal', async () => {
  const resolver = createResolver({ capabilities: caps({ fall: fallCap }) });
  const out = await resolver.resolve('fall');
  assert.equal(out.source, 'b');
  assert.deepEqual(out.data, { v: { raw: 2 } });
});

test('semua local dicoba sebelum api walau array menaruh api dulu', async () => {
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'api-first', kind: 'api', run: async () => ({ raw: 'api' }) },
      { name: 'local-second', kind: 'local', run: async () => ({ raw: 'local' }) },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ urut: cap }) });
  const out = await resolver.resolve('urut');
  assert.equal(
    out.source,
    'local-second',
    'local harus selalu lebih dulu: aggregator gratis mahal dan sering mati',
  );
});

test('semua backend gagal: CapabilityError dengan tried berisi 2 entri', async () => {
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'x', kind: 'local', run: async () => { throw new Error('x mati'); } },
      { name: 'y', kind: 'api', run: async () => { throw new Error('y mati'); } },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ mati: cap }) });
  await assert.rejects(
    () => resolver.resolve('mati'),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.capability, 'mati');
      assert.equal(error.tried.length, 2);
      assert.deepEqual(error.tried.map((t) => t.name).sort(), ['x', 'y']);
      return true;
    },
  );
});

test('nama kapabilitas tak dikenal ditolak dengan code unknown-capability', async () => {
  const resolver = createResolver({ capabilities: caps({}) });
  await assert.rejects(
    () => resolver.resolve('entah'),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.code, 'unknown-capability');
      assert.equal(error.capability, 'entah');
      return true;
    },
  );
});

test('anti-basi: kapabilitas tidak stable tidak pernah dilayani dari cache', async () => {
  let dipanggil = 0;
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [{ name: 'a', kind: 'local', run: async () => { dipanggil += 1; return { raw: dipanggil }; } }],
  };
  const resolver = createResolver({ capabilities: caps({ panas: cap }) });

  const pertama = await resolver.resolve('panas');
  const kedua = await resolver.resolve('panas');

  assert.equal(dipanggil, 2, 'backend harus jalan dua kali');
  assert.equal(pertama.meta.cached, false);
  assert.equal(kedua.meta.cached, false);
});

test('kapabilitas stable dilayani dari cache untuk key yang sama', async () => {
  let dipanggil = 0;
  const cap = {
    stable: true,
    normalize: (r) => ({ v: r }),
    backends: [{ name: 'a', kind: 'local', run: async () => { dipanggil += 1; return { raw: dipanggil }; } }],
  };
  const resolver = createResolver({ capabilities: caps({ tetap: cap }) });

  const pertama = await resolver.resolve('tetap', { q: 'halo' });
  const kedua = await resolver.resolve('tetap', { q: 'halo' });

  assert.equal(dipanggil, 1, 'panggilan kedua harus kena cache');
  assert.equal(kedua.meta.cached, true);
  assert.deepEqual(kedua.data, pertama.data);
  assert.equal(kedua.source, pertama.source);
});

test('kapabilitas stable dengan key berbeda tidak berbagi cache', async () => {
  let dipanggil = 0;
  const cap = {
    stable: true,
    normalize: (r) => ({ v: r }),
    backends: [{ name: 'a', kind: 'local', run: async () => { dipanggil += 1; return { raw: dipanggil }; } }],
  };
  const resolver = createResolver({ capabilities: caps({ tetap: cap }) });

  await resolver.resolve('tetap', { q: 'satu' });
  await resolver.resolve('tetap', { q: 'dua' });

  assert.equal(dipanggil, 2, 'key berbeda harus cache terpisah');
});

test('budget: backend local yang hang tidak menahan resolve selamanya', async () => {
  let apiDipanggil = 0;
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'macet', kind: 'local', run: () => new Promise(() => {}) },
      { name: 'cadangan', kind: 'api', run: async () => { apiDipanggil += 1; return { raw: 'ok' }; } },
    ],
  };
  const resolver = createResolver({
    capabilities: caps({ macet: cap }),
    budget: { localMs: 120, totalMs: 400 },
  });

  const mulai = Date.now();
  const out = await resolver.resolve('macet');
  const elapsed = Date.now() - mulai;

  assert.equal(out.source, 'cadangan');
  assert.ok(
    apiDipanggil > 0,
    'backend api tetap harus sempat dipanggil setelah local hang',
  );
  assert.ok(elapsed < 1500, `resolve harus selesai di bawah budget, habis ${elapsed}ms`);
});

test('breaker: backend yang OPEN tidak dipanggil lagi', async () => {
  let aDipanggil = 0;
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'a', kind: 'local', run: async () => { aDipanggil += 1; throw new Error('mati'); } },
      { name: 'b', kind: 'api', run: async () => ({ raw: 2 }) },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ breakeran: cap }) });

  for (let i = 0; i < 3; i += 1) await resolver.resolve('breakeran');
  assert.equal(aDipanggil, 3);

  const keempat = await resolver.resolve('breakeran');
  assert.equal(keempat.source, 'b');
  assert.equal(aDipanggil, 3, 'backend a sudah OPEN, tidak boleh dipanggil lagi');
});

test('bailout: saat semua backend OPEN resolver tetap mencoba satu', async () => {
  const dipanggil = [];
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'l1', kind: 'local', run: async () => { dipanggil.push('l1'); throw new Error('m1'); } },
      { name: 'l2', kind: 'local', run: async () => { dipanggil.push('l2'); throw new Error('m2'); } },
      { name: 'a1', kind: 'api', run: async () => { dipanggil.push('a1'); throw new Error('m3'); } },
      { name: 'a2', kind: 'api', run: async () => { dipanggil.push('a2'); throw new Error('m4'); } },
    ],
  };
  // Jam palsu supaya openedAt tiap backend berbeda pasti, bukan bergantung pada
  // presisi milidetik Date.now(): tanpa itu, mana yang "paling baru" bisa acak.
  let t = 1_700_000_000_000;
  const breaker = createBreaker({ now: () => (t += 1_000) });
  const resolver = createResolver({ capabilities: caps({ semuaMati: cap }), breaker });

  for (let i = 0; i < 3; i += 1) await resolver.resolve('semuaMati').catch(() => {});
  assert.equal(resolver.breaker.isOpen('l1'), true, 'semua backend harus OPEN');
  assert.equal(resolver.breaker.isOpen('l2'), true);
  assert.equal(resolver.breaker.isOpen('a1'), true);
  assert.equal(resolver.breaker.isOpen('a2'), true);

  dipanggil.length = 0;
  await resolver.resolve('semuaMati').catch(() => {});

  assert.equal(
    dipanggil.length,
    1,
    'harus mencoba tepat satu backend, bukan gagal seketika: kapabilitas tidak boleh mati permanen',
  );
  assert.equal(dipanggil[0], 'a2', 'yang dicoba adalah backend yang paling dekat ke pulih');
});