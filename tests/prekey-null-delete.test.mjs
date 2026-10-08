import { describe, it } from 'node:test';
import assert from 'node:assert';

async function setupTurso() {
  const tursoModule = await import('../src/lib/turso.js');
  const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
  await tursoModule.initTursoTables(client);
  return { client, tursoModule };
}

async function loadSeededState(scope) {
  const tursoModule = await import('../src/lib/turso.js');
  const client = tursoModule.getTursoClient();
  await client.execute({
    sql: 'INSERT INTO session_creds (scope, creds, updated_at) VALUES (?, ?, ?) ON CONFLICT(scope) DO UPDATE SET creds = excluded.creds',
    args: [scope, JSON.stringify({ registered: true }), 1],
  });
  const { loadState } = await import('../src/lib/turso-session.js');
  const state = await loadState(scope);
  assert.ok(state, `loadState(${scope}) should return a state once a row exists`);
  return state;
}

describe('keys.set delete path', () => {
  it('deletes the row when a pre-key value is null', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9001': { keyPair: 'a' } } });
    let rows = await client.execute("SELECT id FROM session_keys WHERE scope='main' AND category='pre-key'");
    assert.strictEqual(rows.rows.length, 1, 'pre-key should exist before delete');

    await state.keys.set({ 'pre-key': { '9001': null } });
    rows = await client.execute("SELECT id FROM session_keys WHERE scope='main' AND category='pre-key'");
    assert.strictEqual(rows.rows.length, 0, 'pre-key row must be gone, not rewritten');
  });

  it('does not leave a row whose data is the literal string null', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9002': { keyPair: 'a' } } });
    await state.keys.set({ 'pre-key': { '9002': null } });

    const rows = await client.execute("SELECT id, data FROM session_keys WHERE scope='main' AND category='pre-key'");
    assert.strictEqual(rows.rows.length, 0, 'no row may survive a delete');
  });

  it('still upserts when the value is not null', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9003': { keyPair: 'a' } } });
    await state.keys.set({ 'pre-key': { '9003': { keyPair: 'b' } } });

    const rows = await client.execute("SELECT data FROM session_keys WHERE scope='main' AND category='pre-key' AND id='9003'");
    assert.strictEqual(rows.rows.length, 1, 'upsert must stay a single row');
    assert.deepStrictEqual(JSON.parse(rows.rows[0].data), { keyPair: 'b' });
  });

  it('mixes inserts and deletes in one patch without dropping either', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { 'a': { k: 1 }, 'b': { k: 2 } } });
    await state.keys.set({
      'pre-key': { 'a': null, 'c': { k: 3 } },
      'session': { 's1': { advSecretKey: 'x' } },
    });

    const pre = await client.execute("SELECT id FROM session_keys WHERE category='pre-key' AND scope='main' ORDER BY id");
    assert.deepStrictEqual(pre.rows.map((r) => r.id), ['b', 'c'], 'a deleted, c inserted, b untouched');

    const sess = await client.execute("SELECT id FROM session_keys WHERE category='session' AND scope='main'");
    assert.strictEqual(sess.rows.length, 1, 'session insert must not be affected by the pre-key delete');
  });

  it('evicts the deleted key from the in-memory cache', async () => {
    const { client } = await setupTurso();
    const state = await loadSeededState('main');

    await state.keys.set({ 'pre-key': { '9004': { keyPair: 'a' } } });
    await state.keys.get('pre-key', ['9004']);
    await state.keys.set({ 'pre-key': { '9004': null } });

    const got = await state.keys.get('pre-key', ['9004']);
    assert.deepStrictEqual(got, {}, 'a deleted key must not be served from cache');
    assert.strictEqual(typeof got, 'object');
  });
});