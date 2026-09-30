import { test } from 'node:test';
import assert from 'node:assert';
import { createCapabilityCache } from '../src/lib/capability-cache.js';

// Jam palsu manual: cache menerima `now` yang di-inject supaya batas umur bisa
// diuji tanpa menunggu detik sungguhan.
function jam(start = 1_700_000_000_000) {
  let t = start;
  return {
    now: () => t,
    advance(ms) {
      t += ms;
      return t;
    },
  };
}

test('set lalu get mengembalikan nilai yang disimpan', () => {
  const cache = createCapabilityCache({ now: jam().now });
  cache.set('bola', 'a', 'v');
  assert.equal(cache.get('bola', 'a'), 'v');
});

test('key berbeda tidak bocor ke key lain', () => {
  const cache = createCapabilityCache({ now: jam().now });
  cache.set('bola', 'a', 'v');
  assert.equal(cache.get('bola', 'b'), undefined);
});

test('kapabilitas berbeda tidak saling bocor walau key sama', () => {
  const cache = createCapabilityCache({ now: jam().now });
  cache.set('bola', 'a', 'v');
  assert.equal(cache.get('teman', 'a'), undefined);
});

test('setelah ttlMs + 1 nilai sudah kedaluwarsa', () => {
  const clock = jam();
  const cache = createCapabilityCache({ ttlMs: 1000, now: clock.now });
  cache.set('bola', 'a', 'v');
  clock.advance(1001);
  assert.equal(cache.get('bola', 'a'), undefined);
});

test('tepat di ttlMs nilai masih hidup (batas inklusif)', () => {
  const clock = jam();
  const cache = createCapabilityCache({ ttlMs: 1000, now: clock.now });
  cache.set('bola', 'a', 'v');
  clock.advance(1000);
  assert.equal(cache.get('bola', 'a'), 'v', 'umur == ttlMs belum boleh dianggap basi');
});

test('bounded: 250 key ditahan, tidak lebih dari 200', () => {
  const cache = createCapabilityCache({ max: 200, ttlMs: 60_000, now: jam().now });
  for (let i = 0; i < 250; i += 1) cache.set('bola', `k${i}`, i);
  assert.equal(cache.size(), 200);
});

test('evict menyasar yang paling lama tidak dipakai, bukan paling lama disimpan', () => {
  const cache = createCapabilityCache({ max: 200, ttlMs: 60_000, now: jam().now });
  for (let i = 0; i < 200; i += 1) cache.set('bola', `k${i}`, `v${i}`);
  // Sentuh k0 supaya jadi paling baru dipakai walau paling lama disimpan.
  assert.equal(cache.get('bola', 'k0'), 'v0');
  cache.set('bola', 'k200', 'v200');
  assert.equal(
    cache.get('bola', 'k0'),
    'v0',
    'entri yang baru dipakai tidak boleh ter-evict',
  );
  assert.equal(
    cache.get('bola', 'k1'),
    undefined,
    'k1 sekarang jadi yang paling lama tidak dipakai, jadi yang victim',
  );
  assert.equal(cache.get('bola', 'k200'), 'v200');
});