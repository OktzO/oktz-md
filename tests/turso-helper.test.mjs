import { describe, it } from 'node:test';
import assert from 'node:assert';

describe('turso helper', () => {
  it('returns null client when turso disabled', async () => {
    process.env.TURSO_ENABLED = 'false';
    const { createTursoClient } = await import('../src/lib/turso.js');
    const client = await createTursoClient({ enabled: false });
    assert.strictEqual(client, null);
    delete process.env.TURSO_ENABLED;
  });
});
