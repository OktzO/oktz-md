import { describe, it } from 'node:test';
import assert from 'node:assert';

const LID = '131010810181018@lid';
const LID_SW = '131010810181018@s.whatsapp.net';
const PN = '6285608953677:0@s.whatsapp.net';
const PN_CLEAN = '6285608953677@s.whatsapp.net';

let moduleCache = null;
async function loadLid() {
  if (!moduleCache) moduleCache = await import('../src/lib/ourin-lid.js');
  return moduleCache;
}

describe('ourin-lid resolveFromSock', () => {
  it('returns PN from signalRepository for @lid jid', async () => {
    const mod = await loadLid();
    const sock = {
      signalRepository: {
        lidMapping: {
          getPNForLID: async (lid) => {
            assert.strictEqual(lid, LID);
            return PN;
          },
        },
      },
    };
    const result = await mod.resolveFromSock(LID, sock);
    assert.strictEqual(result, PN_CLEAN);
  });

  it('normalizes LID-converted @s.whatsapp.net to @lid before getPNForLID', async () => {
    const mod = await loadLid();
    const sock = {
      signalRepository: {
        lidMapping: {
          getPNForLID: async (lid) => {
            assert.strictEqual(lid, LID);
            return PN;
          },
        },
      },
    };
    const result = await mod.resolveFromSock(LID_SW, sock);
    assert.strictEqual(result, PN_CLEAN);
  });

  it('falls back to store.contacts loop when signal repo misses', async () => {
    const mod = await loadLid();
    const sock = {
      signalRepository: {
        lidMapping: {
          getPNForLID: async () => null,
        },
      },
      store: {
        contacts: {
          [PN_CLEAN]: { id: PN_CLEAN, lid: LID },
        },
      },
    };
    const result = await mod.resolveFromSock(LID, sock);
    assert.strictEqual(result, PN_CLEAN);
  });

  it('returns jid unchanged when nothing resolves (unique LID, no cache)', async () => {
    const mod = await loadLid();
    const uniqueLid = '120363123456789@lid';
    const sock = {
      signalRepository: {
        lidMapping: {
          getPNForLID: async () => null,
        },
      },
      store: { contacts: {} },
    };
    const result = await mod.resolveFromSock(uniqueLid, sock);
    assert.strictEqual(result, uniqueLid);
  });
});
