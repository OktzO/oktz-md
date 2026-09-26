import { describe, it } from 'node:test';
import assert from 'node:assert';

import { runUploadFanout } from '../src/lib/upload-fanout.js';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function makeItems(specs) {
  return specs.map((s, i) => ({
    name: s.name || `host${i}`,
    run: async () => {
      await sleep(s.delay || 0);
      if (s.fail) throw new Error(`${s.name} gagal`);
      return s.value;
    },
  }));
}

describe('runUploadFanout', () => {
  it('menjalankan seluruh item bareng-barang, bukan satu-satu', async () => {
    let inflight = 0;
    let peak = 0;

    const items = Array.from({ length: 9 }, (_, i) => ({
      name: `h${i}`,
      run: async () => {
        inflight += 1;
        if (inflight > peak) peak = inflight;
        try {
          await sleep(100);
          return { url: `https://x/${i}` };
        } finally {
          inflight -= 1;
        }
      },
    }));

    const t0 = Date.now();
    const res = await runUploadFanout(items, { deadlineMs: 5000 });
    const elapsed = Date.now() - t0;

    assert.ok(peak >= 4, `peak concurrency cuma ${peak} — masih sequential`);
    assert.ok(elapsed < 600, `terlalu lama (${elapsed}ms) untuk 9 item paralel`);
    assert.equal(res.done.length, 9);
    assert.equal(res.failed.length, 0);
  });

  it('potong setelah deadline dan tidak menunggu host yang menggantung', async () => {
    const items = makeItems(
      Array.from({ length: 5 }, (_, i) => ({ name: `slow${i}`, delay: 5000 })),
    );

    const t0 = Date.now();
    const res = await runUploadFanout(items, { deadlineMs: 300 });
    const elapsed = Date.now() - t0;

    assert.ok(elapsed < 1200, `deadline tidak dipatuhi: ${elapsed}ms`);
    assert.equal(res.done.length, 0);
    assert.equal(res.timedOut.length, 5, 'host yang menggantung harus dilaporkan');
  });

  it('kirim hasil yang sudah jadi walau satu host lambat', async () => {
    // 6 host cepat + 1 host lambat 4 detik. Timeout per-host 200ms, jadi
    // host lambatKepotong sendiri tanpa menahan yang lain.
    const items = makeItems([
      ...Array.from({ length: 6 }, (_, i) => ({
        name: `fast${i}`,
        value: { url: `https://fast/${i}` },
      })),
      { name: 'slow', delay: 4000, value: { url: 'https://slow/1' } },
    ]);

    const t0 = Date.now();
    const res = await runUploadFanout(items, {
      deadlineMs: 10000,
      perHostMs: 200,
    });
    const elapsed = Date.now() - t0;

    assert.ok(elapsed < 1500, `host lambat menahan semua: ${elapsed}ms`);
    assert.equal(res.done.length, 6, 'enam host cepat harus tetap masuk');
    assert.ok(
      res.done.some((d) => d.url === 'https://fast/0'),
      'hasil host cepat harus dikembalikan',
    );
    assert.ok(
      res.timedOut.includes('slow'),
      `host lambat harus masuk timedOut. Got: ${JSON.stringify(res)}`,
    );
  });

  it('host yang gagal dilaporkan terpisah dari host timeout', async () => {
    const items = makeItems([
      { name: 'ok', value: { url: 'https://ok/1' } },
      { name: 'broken', fail: true },
      { name: 'hang', delay: 4000, value: { url: 'https://hang/1' } },
    ]);

    const res = await runUploadFanout(items, {
      deadlineMs: 5000,
      perHostMs: 200,
    });

    assert.deepEqual(res.done.map((d) => d.url), ['https://ok/1']);
    assert.deepEqual(res.failed, ['broken']);
    assert.deepEqual(res.timedOut, ['hang']);
  });

  it('item yang return-nya bukan object url dianggap gagal', async () => {
    const items = makeItems([
      { name: 'nourl', value: { html: '<b>x</b>' } },
      { name: 'nullval', value: null },
      { name: 'ok', value: { url: 'https://ok/1' } },
    ]);

    const res = await runUploadFanout(items, { deadlineMs: 2000 });

    assert.deepEqual(res.done.map((d) => d.url), ['https://ok/1']);
    assert.deepEqual(
      res.failed.sort(),
      ['nourl', 'nullval'],
      'harus dapat url, bukan cuma tidak throw',
    );
  });

  it('daftar item kosong tidak melempar', async () => {
    const res = await runUploadFanout([], { deadlineMs: 1000 });
    assert.deepEqual(res, { done: [], failed: [], timedOut: [] });
  });

  it('semua gagal → done kosong, bukan error', async () => {
    const items = makeItems([
      { name: 'a', fail: true },
      { name: 'b', fail: true },
    ]);

    const res = await runUploadFanout(items, { deadlineMs: 2000 });
    assert.equal(res.done.length, 0);
    assert.deepEqual(res.failed.sort(), ['a', 'b']);
  });
});
