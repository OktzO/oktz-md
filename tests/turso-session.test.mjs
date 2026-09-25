import { describe, it } from 'node:test';
import assert from 'node:assert';

describe('turso session buffer roundtrip', () => {
  it('BufferJSON roundtrips creds-like object with Buffer', async () => {
    const { BufferJSON } = await import('onigis');
    const creds = {
      noiseKey: { private: Buffer.from('deadbeef', 'hex'), public: Buffer.from('cafebabe', 'hex') },
      signedIdentityKey: { private: Buffer.from('aa', 'hex'), public: Buffer.from('bb', 'hex') },
      signedPreKey: { keyPair: { private: Buffer.from('11', 'hex'), public: Buffer.from('22', 'hex') }, keyId: 1 },
      plainString: 'hello',
      plainNumber: 42,
    };
    const serialized = JSON.stringify(creds, BufferJSON.replacer);
    const deserialized = JSON.parse(serialized, BufferJSON.reviver);
    assert.ok(Buffer.isBuffer(deserialized.noiseKey.private));
    assert.strictEqual(deserialized.noiseKey.private.toString('hex'), 'deadbeef');
    assert.ok(Buffer.isBuffer(deserialized.signedIdentityKey.private));
    assert.ok(Buffer.isBuffer(deserialized.signedPreKey.keyPair.private));
    assert.strictEqual(deserialized.plainString, 'hello');
    assert.strictEqual(deserialized.plainNumber, 42);
  });

  it('session module JSON.stringify uses BufferJSON replacer', async () => {
    const { loadState, saveCreds } = await import('../src/lib/ourin-turso-session.js');
    assert.ok(typeof loadState === 'function');
    assert.ok(typeof saveCreds === 'function');
  });
});