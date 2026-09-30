import { test } from 'node:test';
import assert from 'node:assert';
import { createBreaker } from '../src/lib/circuit-breaker.js';

// Jam palsu yang dikontrol manual: breaker ditentukan `now()` yang di-inject,
// jadi pakai Date.now() langsung membuat hasil test bergantung pada kecepatan
// mesin (cooldown bisa terlewati atau belum setengah jalan).
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

const COOLDOWN = 30_000;

test('isOpen false di awal untuk host yang belum pernah gagal', () => {
  const breaker = createBreaker({ now: jam().now });
  assert.equal(breaker.isOpen('neoxr'), false);
});

test('dua kegagalan belum membuka breaker (threshold 3)', () => {
  const breaker = createBreaker({ now: jam().now });
  breaker.recordFailure('neoxr');
  breaker.recordFailure('neoxr');
  assert.equal(breaker.isOpen('neoxr'), false);
  assert.equal(
    breaker.snapshot().find((s) => s.name === 'neoxr').failures,
    2,
    'dua kegagalan harus tercatat, bukan diabaikan',
  );
});

test('kegagalan ketiga membuka breaker', () => {
  const breaker = createBreaker({ now: jam().now });
  breaker.recordFailure('neoxr');
  breaker.recordFailure('neoxr');
  breaker.recordFailure('neoxr');
  assert.equal(breaker.isOpen('neoxr'), true);
});

test('setelah cooldown lewat breaker tutup lagi (HALF_OPEN lolos sebagai percobaan)', () => {
  const clock = jam();
  const breaker = createBreaker({ cooldownMs: COOLDOWN, now: clock.now });
  for (let i = 0; i < 3; i += 1) breaker.recordFailure('neoxr');
  assert.equal(breaker.isOpen('neoxr'), true);
  clock.advance(COOLDOWN);
  assert.equal(
    breaker.isOpen('neoxr'),
    false,
    'tepat di cooldown breaker harus CLOSED, tidak masih OPEN',
  );
});

test('recordSuccess saat OPEN me-reset state sepenuhnya', () => {
  const clock = jam();
  const breaker = createBreaker({ cooldownMs: COOLDOWN, now: clock.now });
  for (let i = 0; i < 3; i += 1) breaker.recordFailure('neoxr');
  assert.equal(breaker.isOpen('neoxr'), true, 'breaker harus OPEN dulu');
  breaker.recordSuccess('neoxr');
  assert.equal(breaker.isOpen('neoxr'), false);
  const state = breaker.snapshot().find((s) => s.name === 'neoxr');
  assert.equal(state.failures, 0);
  assert.equal(state.openedAt, null);
});

test('snapshot memuat host yang punya state', () => {
  const breaker = createBreaker({ now: jam().now });
  breaker.recordFailure('neoxr');
  const snapshot = breaker.snapshot();
  assert.ok(
    snapshot.some((s) => s.name === 'neoxr'),
    `snapshot harus memuat neoxr, dapat: ${JSON.stringify(snapshot)}`,
  );
  assert.deepEqual(
    Object.keys(snapshot[0]).sort(),
    ['failures', 'name', 'openedAt'],
    'bentuk entri snapshot harus stabil',
  );
});

test('bounded: 100 nama berbeda tetap 64 slot, yang paling lama tidak dipakai ter-evict', () => {
  const breaker = createBreaker({ max: 64, now: jam().now });
  for (let i = 0; i < 100; i += 1) breaker.recordFailure(`host-${i}`);
  const snapshot = breaker.snapshot();
  assert.equal(snapshot.length, 64, 'slot harus dibatasi 64');
  assert.ok(
    !snapshot.some((s) => s.name === 'host-0'),
    'nama paling lama tidak dipakai harus ter-evict duluan',
  );
  assert.ok(
    snapshot.some((s) => s.name === 'host-99'),
    'nama terbaru harus masih ada',
  );
});

test('nama berbeda punya state terpisah', () => {
  const breaker = createBreaker({ now: jam().now });
  for (let i = 0; i < 3; i += 1) breaker.recordFailure('a');
  assert.equal(breaker.isOpen('a'), true);
  assert.equal(
    breaker.isOpen('b'),
    false,
    'kegagalan host a tidak boleh menutup host b',
  );
});