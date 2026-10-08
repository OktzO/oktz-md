import { describe, it } from 'node:test';
import assert from 'node:assert';

describe('sweep survives a dead Turso', () => {
  it('returns ok:false instead of throwing when no client is configured', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    await tursoModule.closeTurso();
    const { sweepPreKeys } = await import('../src/lib/turso-session.js');

    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, false, 'a dead mirror must not look like success');
    assert.strictEqual(res.deleted, 0);
    assert.ok(typeof res.error === 'string' && res.error.length > 0, 'the reason must reach the log');
  });

  it('returns ok:false when the query itself fails', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
    await tursoModule.initTursoTables(client);
    await client.execute('DROP TABLE session_keys');

    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, false);
    assert.strictEqual(res.deleted, 0);
  });

  it('never throws, whatever the client does', async () => {
    const tursoModule = await import('../src/lib/turso.js');
    const client = await tursoModule.createTursoClient({ enabled: true, url: 'file::memory:' });
    client.execute = async () => { throw new Error('boom'); };

    const { sweepPreKeys } = await import('../src/lib/turso-session.js');
    const res = await sweepPreKeys();
    assert.strictEqual(res.ok, false);
    assert.match(res.error, /boom/);
  });
});