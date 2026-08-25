import { describe, it, mock } from 'node:test';
import assert from 'node:assert';

async function setupTurso() {
  const tursoModule = await import('../src/lib/ourin-turso.js');
  const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
  await tursoModule.initTursoTables(client);
  return { client, tursoModule };
}

describe('turso session keys.set atomic writes', () => {
  it('writes all keys via a single batch() call, not per-key execute()', async () => {
    const { client } = await setupTurso();
    const batchSpy = mock.method(client, 'batch');

    const { loadState } = await import('../src/lib/ourin-turso-session.js');
    const state = await loadState('main');

    await state.keys.set({
      'pre-key': {
        'id1': { keyPair: 'a' },
        'id2': { keyPair: 'b' },
      },
      'session': {
        'id3': { advSecretKey: 'c' },
      },
    });

    assert.strictEqual(batchSpy.mock.calls.length, 1, 'batch() should be called exactly once');
    const stmts = batchSpy.mock.calls[0].arguments[0];
    const mode = batchSpy.mock.calls[0].arguments[1];
    assert.strictEqual(stmts.length, 3, 'all 3 keys should be batched');
    assert.strictEqual(mode, 'write');
    for (const stmt of stmts) {
      assert.match(stmt.sql, /INSERT INTO session_keys/);
      assert.strictEqual(stmt.args[0], 'main', 'scope arg');
    }

    const rs = await client.execute({ sql: 'SELECT COUNT(*) AS c FROM session_keys', args: [] });
    assert.strictEqual(rs.rows[0].c, 3, 'all 3 rows persisted');

    batchSpy.mock.restore();
  });

  it('lands no rows when batch() fails (atomic all-or-nothing)', async () => {
    const { client } = await setupTurso();
    const originalBatch = client.batch;
    client.batch = async () => { throw new Error('turso hiccup'); };

    const { loadState } = await import('../src/lib/ourin-turso-session.js');
    const state = await loadState('main');

    await assert.rejects(
      state.keys.set({ 'pre-key': { 'id1': { keyPair: 'a' } } }),
      /turso hiccup/,
    );

    const rs = await client.execute({ sql: 'SELECT COUNT(*) AS c FROM session_keys', args: [] });
    assert.strictEqual(rs.rows[0].c, 0, 'no partial rows on failure');

    client.batch = originalBatch;
  });

  it('deleteTursoSession clears cached keys for every category of the scope', async () => {
    const { client } = await setupTurso();
    const { loadState, deleteTursoSession } = await import('../src/lib/ourin-turso-session.js');

    const scope = 'clear-cache-test';
    const state = await loadState(scope);
    await state.keys.set({
      'pre-key': { 'k1': { v: 1 } },
      'session': { 'k2': { v: 2 } },
      'sender-key': { 'k3': { v: 3 } },
    });

    const before = await state.keys.get('pre-key', ['k1']);
    assert.ok(before.k1, 'cache populated before delete');

    await deleteTursoSession(scope);

    const after = await state.keys.get('pre-key', ['k1']);
    assert.deepStrictEqual(after, {}, 'stale pre-key cache must not survive delete');

    const rs = await client.execute({
      sql: 'SELECT COUNT(*) AS c FROM session_keys WHERE scope = ?',
      args: [scope],
    });
    assert.strictEqual(rs.rows[0].c, 0, 'all DB rows for scope deleted');
  });
});
