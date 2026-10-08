import { describe, it } from 'node:test';
import assert from 'node:assert';

describe('sessionKeyStats', () => {
  it('groups keys and bytes by category', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
    await tursoModule.initTursoTables(client);

    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'pre-key', 'a', '{"pad":"xxxxxxxx"}', 1],
    });
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'pre-key', 'b', '{"pad":"yyyy"}', 1],
    });
    await client.execute({
      sql: 'INSERT INTO session_keys (scope, category, id, data, updated_at) VALUES (?, ?, ?, ?, ?)',
      args: ['main', 'session', 's', '{"pad":"zz"}', 1],
    });

    const { sessionKeyStats } = await import('../src/lib/turso-session.js');
    const stats = await sessionKeyStats();
    assert.strictEqual(stats.total, 3);
    const pre = stats.byCategory.find((c) => c.category === 'pre-key');
    assert.strictEqual(pre.keys, 2);
    assert.ok(pre.bytes > 0, 'bytes must be measured, not zero');
    assert.ok(stats.bytes >= pre.bytes);
  });

  it('degrades to zeros when Turso is absent', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    await tursoModule.closeTurso();
    const { sessionKeyStats } = await import('../src/lib/turso-session.js');
    const stats = await sessionKeyStats();
    assert.deepStrictEqual(stats, { total: 0, bytes: 0, byCategory: [] });
  });
});