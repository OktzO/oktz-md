import { test } from 'node:test';
import assert from 'node:assert';
import {
  AggregatorError,
  createAggregatorClient,
  envNameOf,
  missingEnvNameOf,
} from '../src/lib/aggregator.js';
import { CapabilityError, createResolver } from '../src/lib/resolve.js';
import { createBreaker } from '../src/lib/circuit-breaker.js';
import { createCapabilityCache } from '../src/lib/capability-cache.js';

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

test('kegagalan total melempar CapabilityError, tidak pernah dilayani dari cache', async () => {
  let jam = 1_700_000_000_000;
  const sekarang = () => jam;
  let hidup = true;

  // Jejak urutan akses. Aturan global "tidak ada stale-data fallback" berdiri
  // hanya di urutan baris: cache dibaca sebelum backend dijalankan, dan tidak
  // dibaca lagi setelahnya. Jejak ini mengunci urutannya supaya refactor yang
  // menambahkan cache.get di jalur kegagalan langsung ketahuan.
  const jejak = [];

  const cap = {
    stable: true,
    normalize: (r) => ({ v: r }),
    backends: [
      {
        name: 'a',
        kind: 'local',
        run: async () => {
          jejak.push('backend');
          if (!hidup) throw new Error('backend mati');
          return { raw: 1 };
        },
      },
    ],
  };

  const nyata = createCapabilityCache({ ttlMs: 1000, now: sekarang });
  const cache = {
    get: (capability, key) => {
      jejak.push('cache');
      return nyata.get(capability, key);
    },
    set: (capability, key, value) => nyata.set(capability, key, value),
    size: () => nyata.size(),
    clear: () => nyata.clear(),
  };

  const resolver = createResolver({ capabilities: caps({ rapuh: cap }), cache, now: sekarang });

  const pertama = await resolver.resolve('rapuh', { q: 1 });
  assert.equal(pertama.meta.cached, false);
  assert.equal(nyata.size(), 1, 'entri cache harus terisi setelah panggilan sukses');

  hidup = false;
  jam += 1001;

  await assert.rejects(
    () => resolver.resolve('rapuh', { q: 1 }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(
        error.tried,
        [{ name: 'a', reason: 'backend mati' }],
        'alasan kegagalan asli harus sampai ke pemanggil, bukan hasil lama',
      );
      return true;
    },
  );

  assert.deepEqual(
    jejak,
    ['cache', 'backend', 'cache', 'backend'],
    'cache hanya boleh dibaca di jalur cepat: satu kali sebelum backend, nol kali setelah backend gagal',
  );
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

test('backend menerima signal yang benar-benar di-abort saat budget habis', async () => {
  let signalDilihat = null;
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      {
        name: 'macet',
        kind: 'local',
        run: async (_args, ctx) => {
          signalDilihat = ctx?.signal;
          return new Promise(() => {});
        },
      },
      { name: 'cadangan', kind: 'api', run: async () => ({ raw: 'ok' }) },
    ],
  };
  const resolver = createResolver({
    capabilities: caps({ sinyal: cap }),
    budget: { localMs: 80, totalMs: 400 },
  });

  await resolver.resolve('sinyal');

  assert.ok(signalDilihat, 'backend harus menerima signal dari resolver');
  assert.ok(
    signalDilihat instanceof AbortSignal,
    `ctx.signal harus AbortSignal, dapat ${signalDilihat?.constructor?.name}`,
  );
  assert.equal(
    signalDilihat.aborted,
    true,
    'timeout harus memutus kerja lewat abort(), bukan hanya resolver berhenti menunggu',
  );
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

// ── applies: backend yang tidak berlaku untuk argumen ini ────────────────────
//
// Breaker menghitung backend yang gagal sebagai host yang salah. Backend yang
// "tidak berlaku" bukan host mati — dia tidak pernah dihubungi sama sekali —
// jadi harus dilewati tanpa satu pun failure tercatat, kalau tidak satu
// kapabilitas dengan backend bercabang bisa meng-evict backend lokalnya sendiri
// hanya karena lalu lintas memakai jalur yang tidak dilayaninya.

test('applies false: backend dilewati tanpa dijalankan dan tanpa failure', async () => {
  let dipanggil = [];
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'hanya-url', kind: 'local', applies: (a) => typeof a?.url === 'string', run: async () => { dipanggil.push('hanya-url'); return { raw: 1 }; } },
      { name: 'cadangan', kind: 'api', run: async () => { dipanggil.push('cadangan'); return { raw: 2 }; } },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ sel: cap }) });

  const out = await resolver.resolve('sel', { q: 'bukan url' });

  assert.equal(out.source, 'cadangan');
  assert.deepEqual(dipanggil, ['cadangan'], 'backend tidak berlaku tidak boleh dijalankan');
  assert.deepEqual(
    resolver.breaker.snapshot().filter((s) => s.name === 'hanya-url'),
    [],
    'backend yang dilewati tidak boleh punya slot breaker',
  );
  assert.deepEqual(
    resolver.breaker.snapshot().filter((s) => s.failures > 0),
    [],
    'tidak boleh ada kegagalan yang tercatat',
  );
});

test('applies: backend tidak berlaku tidak pernah OPEN meski lalu lintas tidak cocok', async () => {
  // Kalau applies diabaikan, 'hanya-url' dipanggil lima kali, gagal, dan
  // OPEN di ambang ketiga — padahal tidak pernah menerima satu pun request.
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'hanya-url', kind: 'local', applies: (a) => typeof a?.url === 'string', run: async () => { throw new Error('tidak boleh dipanggil'); } },
      { name: 'hanya-q', kind: 'local', applies: (a) => typeof a?.q === 'string', run: async () => { throw new Error('q mati'); } },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ noncocok: cap }) });

  for (let i = 0; i < 5; i += 1) {
    await resolver.resolve('noncocok', { q: 'x' }).catch(() => {});
  }

  assert.equal(
    resolver.breaker.isOpen('hanya-url'),
    false,
    'backend yang tidak berlaku tidak boleh OPEN',
  );
  const state = resolver.breaker.snapshot();
  assert.equal(
    state.filter((s) => s.name === 'hanya-url').length,
    0,
    'backend yang tidak berlaku tidak boleh punya slot breaker sama sekali',
  );
  assert.equal(state.find((s) => s.name === 'hanya-q')?.failures, 5, 'hanya yang benar-benar gagal yang dihitung');
});

test('applies: tanpa applies, backend dianggap selalu berlaku', async () => {
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [{ name: 'a', kind: 'local', run: async () => ({ raw: 1 }) }],
  };
  const resolver = createResolver({ capabilities: caps({ biasa: cap }) });
  const out = await resolver.resolve('biasa', { q: 'apa saja' });
  assert.equal(out.source, 'a', 'argumen tidak boleh membuat backend tanpa applies dilewati');
});

test('semua backend tidak berlaku → code no-applicable-backend, bukan tumpukan kegagalan', async () => {
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'a', kind: 'local', applies: () => false, run: async () => ({ raw: 1 }) },
      { name: 'b', kind: 'api', applies: () => false, run: async () => ({ raw: 2 }) },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ kosong: cap }) });

  await assert.rejects(
    () => resolver.resolve('kosong', { q: 'x' }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.code, 'no-applicable-backend');
      assert.deepEqual(error.tried, [], 'tidak ada backend yang dicoba, jadi tidak ada yang gagal');
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), [], 'breaker harus tetap kosong');
});

test('applies tidak merusak urutan tier: local tetap mencoba sebelum api', async () => {
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'api-first', kind: 'api', applies: () => true, run: async () => ({ raw: 'api' }) },
      { name: 'local-second', kind: 'local', applies: () => true, run: async () => ({ raw: 'local' }) },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ urut: cap }) });
  const out = await resolver.resolve('urut', {});
  assert.equal(
    out.source,
    'local-second',
    'baitingkat harus tetap(local sebelum api) walau backend punya applies',
  );
});

test('applies: local yang tidak berlaku tidak menghalangi jatuhnya ke api', async () => {
  const cap = {
    stable: false,
    normalize: (r) => ({ v: r }),
    backends: [
      { name: 'lokal-url', kind: 'local', applies: (a) => typeof a?.url === 'string', run: async () => ({ raw: 1 }) },
      { name: 'api-q', kind: 'api', run: async () => ({ raw: 2 }) },
    ],
  };
  const resolver = createResolver({ capabilities: caps({ jatuh: cap }) });
  const out = await resolver.resolve('jatuh', { q: 'x' });
  assert.equal(out.source, 'api-q');
  assert.deepEqual(
    resolver.breaker.snapshot().filter((s) => s.name === 'lokal-url'),
    [],
    'backend local yang tidak berlaku tidak boleh dicatat',
  );
});

// ── signal di aggregator ─────────────────────────────────────────────────────

test('hit meneruskan signal ke request, dan abort benar-benar mencabutnya', async () => {
  const controller = new AbortController();
  let optsTerlihat = null;
  let terputus = false;
  const http = {
    get: (_url, opts) =>
      new Promise((_resolve, reject) => {
        optsTerlihat = opts;
        opts.signal.addEventListener('abort', () => {
          terputus = true;
          reject(new Error('dibatalkan'));
        });
      }),
  };
  const client = createAggregatorClient({ http, keyOf: () => '' });

  const jalan = client.hit('nexray', '/x', { signal: controller.signal });
  controller.abort();

  await assert.rejects(() => jalan);
  assert.equal(
    optsTerlihat.signal,
    controller.signal,
    'signal pemanggil harus masuk ke config request aggregator',
  );
  assert.equal(terputus, true, 'abort harus benar-benar mencabut request aggregator');
});

test('hit tanpa signal tidak mengarang signal palsu di config', async () => {
  const http = httpPalsu(async () => ({ status: 200, data: { ok: true } }));
  const client = createAggregatorClient({ http, keyOf: () => '' });
  await client.hit('nexray', '/x');
  assert.equal(http.calls[0].opts.signal, undefined);
});
// ── peta host → env: satu-satunya tempat yang tahu keduanya ────────────────────
//
// Plugin tidak boleh menyebut host aggregator (semua akses aggregator lewat
// `hit`), tapi plugin tetap harus jujur saat key aggregator kosong. Jembatannya
// dua fungsi ini: `envNameOf` memetakan host ke nama env, dan
// `missingEnvNameOf` membacanya kembali dari kegagalan yang sudah tercatat.

test('envNameOf: host berkey dipetakan ke nama env .env', () => {
  assert.equal(envNameOf('neoxr'), 'APIKEY_NEOXR');
  assert.equal(envNameOf('cuki'), 'APIKEY_CUKI');
});

test('envNameOf: host tanpa key mengembalikan null, bukan nama env karangan', () => {
  // Host tanpa key tidak punya baris .env sama sekali; mengarang `APIKEY_NEXRAY`
  // akan mengarahkan owner ke baris yang tidak pernah dibaca.
  for (const host of ['nexray', 'izuka', 'siputzx', 'azbry', 'tidak-ada']) {
    assert.equal(envNameOf(host), null, `harus null: ${host}`);
  }
});

test('AggregatorError key kosong membawa envName', async () => {
  const client = createAggregatorClient({ http: httpPalsu(async () => ({ status: 200, data: {} })), keyOf: () => '' });
  await assert.rejects(
    () => client.hit('neoxr', '/api/sfile', { params: { url: 'u' } }),
    (error) => {
      assert.equal(error.envName, 'APIKEY_NEOXR');
      assert.match(error.message, /APIKEY_NEOXR/, 'pesan harus menyebut baris .env yang harus diisi');
      return true;
    },
  );
});

test('missingEnvNameOf membaca env dari tried resolver', () => {
  const error = new CapabilityError('semua backend sfile gagal', {
    capability: 'sfile',
    tried: [
      { name: 'sfile-mobi', reason: 'normalisasi gagal: tanpa URL' },
      { name: 'neoxr', reason: 'API key neoxr belum diisi, set APIKEY_NEOXR di .env; neoxr dilewati' },
    ],
  });
  assert.equal(missingEnvNameOf(error), 'APIKEY_NEOXR');
});

test('missingEnvNameOf: kegagalan tanpa masalah key mengembalikan null', () => {
  const error = new CapabilityError('semua backend sfile gagal', {
    capability: 'sfile',
    tried: [
      { name: 'sfile-mobi', reason: 'sfile.mobi timeout' },
      { name: 'neoxr', reason: 'neoxr menjawab 502' },
    ],
  });
  assert.equal(missingEnvNameOf(error), null, 'key kosong bukan penyebabnya');
  assert.equal(missingEnvNameOf(new Error('biasa')), null);
  assert.equal(missingEnvNameOf(undefined), null);
});
