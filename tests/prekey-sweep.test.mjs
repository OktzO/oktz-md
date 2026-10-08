import { describe, it } from 'node:test';
import assert from 'node:assert';

const DAY = 86400000;

async function setupTurso() {
  const tursoModule = await import('../src/lib/turso.js');
  const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
  await tursoModule.initTursoTables(client);
  return { client, tursoModule };
}

// `base` bisa dipin supaya umur key tepat dan sweep bisa memakai `now` yang
// sama. Tanpa itu, satu milidetik antara seed dan sweep sudah menggeser key
// "tepat 30 hari" melewati batas — dan batas eksklusif ikut teruji, tapi
// hanya karena jam, bukan karena yang diuji.
async function seed(client, rows, base = Date.now()) {
  for (const [category, id, ageDays] of rows) {
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', category, id, JSON.stringify({ x: 1 }), base - ageDays * DAY],
    });
  }
}

async function countOf(client, category) {
  const r = await client.execute({
    sql: 'SELECT COUNT(*) n FROM session_keys WHERE scope=? AND category=?',
    args: ['main', category],
  });
  return r.rows[0].n;
}

describe('sweepPreKeys', () => {
  it('removes only pre-keys older than the threshold', async () => {
    const { client } = await setupTurso();
    const base = Date.now();
    await seed(client, [
      ['pre-key', 'old', 31],
      ['pre-key', 'fresh', 1],
      ['pre-key', 'edge', 30],
    ], base);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const res = await sweepPreKeys({ now: base });
    assert.strictEqual(res.ok, true);
    assert.strictEqual(res.deleted, 1, 'exactly one pre-key older than 30d');
    assert.strictEqual(await countOf(client, 'pre-key'), 2, 'fresh and edge survive');
  });

  it('treats exactly 30 days as inside the window', async () => {
    const { client } = await setupTurso();
    const base = Date.now();
    await seed(client, [['pre-key', 'edge', 30]], base);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const atBoundary = await sweepPreKeys({ now: base, maxAgeMs: 30 * DAY });
    assert.strictEqual(atBoundary.deleted, 0, 'tepat 30 hari belum basi');

    const pastBoundary = await sweepPreKeys({ now: base + 1, maxAgeMs: 30 * DAY });
    assert.strictEqual(pastBoundary.deleted, 1, 'satu milidetik lewat batas sudah basi');
  });

  it('never touches ratchet categories', async () => {
    const { client } = await setupTurso();
    await seed(client, [
      ['pre-key', 'old', 400],
      ['session', 's-old', 400],
      ['sender-key', 'sk-old', 400],
      ['identity-key', 'ik-old', 400],
      ['device-list', 'dl-old', 400],
      ['lid-mapping', 'lm-old', 400],
      ['tctoken', 'tc-old', 400],
    ]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    await sweepPreKeys();

    for (const cat of ['session', 'sender-key', 'identity-key', 'device-list', 'lid-mapping', 'tctoken']) {
      assert.strictEqual(await countOf(client, cat), 1, `${cat} must be untouched`);
    }
    assert.strictEqual(await countOf(client, 'pre-key'), 0);
  });

  it('does not sweep another scope', async () => {
    const { client } = await setupTurso();
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['jadibot:6285', 'pre-key', 'j-old', '{}', Date.now() - 400 * DAY],
    });
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    await sweepPreKeys();

    const r = await client.execute("SELECT COUNT(*) n FROM session_keys WHERE scope='jadibot:6285'");
    assert.strictEqual(r.rows[0].n, 1, 'jadibot pre-keys are live sessions, not garbage');
  });

  it('is idempotent when run twice', async () => {
    const { client } = await setupTurso();
    await seed(client, [['pre-key', 'old', 31]]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const first = await sweepPreKeys();
    const second = await sweepPreKeys();
    assert.strictEqual(first.deleted, 1);
    assert.strictEqual(second.deleted, 0, 'second pass has nothing left to remove');
    assert.strictEqual(second.ok, true);
  });

  it('honours an injected now and maxAgeMs', async () => {
    const { client } = await setupTurso();
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'pre-key', 'p', '{}', 1_000_000],
    });
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const early = await sweepPreKeys({ now: 1_000_000 + 10 * DAY, maxAgeMs: 30 * DAY });
    assert.strictEqual(early.deleted, 0, '10d old is inside the window');

    const late = await sweepPreKeys({ now: 1_000_000 + 31 * DAY, maxAgeMs: 30 * DAY });
    assert.strictEqual(late.deleted, 1);
  });

  it('reports zero deletions when there is nothing stale', async () => {
    const { client } = await setupTurso();
    await seed(client, [['pre-key', 'fresh', 0]]);
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, true);
    assert.strictEqual(res.deleted, 0);
  });
});