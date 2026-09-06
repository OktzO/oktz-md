import { describe, it } from 'node:test';
import assert from 'node:assert';
import { addExpWithLevelCheck } from '../src/lib/ourin-level.js';

const SENDER = '6281234567890@s.whatsapp.net';

function fakeDb() {
  const store = {};
  const calls = { setUser: 0, save: 0 };
  const db = {
    calls,
    store,
    getUser(jid) {
      return store[jid.replace(/@.+/g, '')] || null;
    },
    setUser(jid, data = {}) {
      calls.setUser++;
      const clean = jid.replace(/@.+/g, '');
      store[clean] = { ...(store[clean] || {}), ...data };
      return store[clean];
    },
    updateExp(jid, amount) {
      const user = db.getUser(jid);
      if (!user) return 0;
      if (user.exp === -1) return -1;
      user.exp = Math.max(0, Math.min(9000000000, (user.exp ?? 0) + amount));
      db.setUser(jid, user);
      return user.exp;
    },
    save() {
      calls.save++;
    },
  };
  return db;
}

function fakeM() {
  return { sender: SENDER, pushName: 'Tester', prefix: '.', chat: SENDER };
}

describe('addExpWithLevelCheck single-write', () => {
  it('per body message triggers exactly one setUser call', async () => {
    const db = fakeDb();
    const user = db.setUser(SENDER, { exp: 100 });
    user.rpg = {};
    user.settings = { levelupNotif: false };
    db.calls.setUser = 0;

    const res = await addExpWithLevelCheck(null, fakeM(), db, user, 15);

    assert.strictEqual(db.calls.setUser, 1, `expected 1 setUser, got ${db.calls.setUser}`);
    assert.strictEqual(res.leveledUp, false);
    assert.strictEqual(user.exp, 115);
  });

  it('persists exp and rpg in one payload', async () => {
    const db = fakeDb();
    const user = db.setUser(SENDER, { exp: 100, rpg: { level: 1 } });
    user.settings = { levelupNotif: false };
    db.calls.setUser = 0;

    await addExpWithLevelCheck(null, fakeM(), db, user, 15);

    assert.strictEqual(db.store['6281234567890'].exp, 115);
    assert.ok(db.store['6281234567890'].rpg, 'rpg must be persisted');
  });

  it('level-up runs after data write: rpg stats updated, result returned', async () => {
    const db = fakeDb();
    const user = db.setUser(SENDER, { exp: 9990 });
    user.rpg = {};
    user.settings = { levelupNotif: false };
    db.calls.setUser = 0;

    const res = await addExpWithLevelCheck(null, fakeM(), db, user, 15);

    assert.strictEqual(res.leveledUp, true);
    assert.strictEqual(res.newLevel, 2);
    assert.strictEqual(user.rpg.level, 2);
    assert.strictEqual(user.rpg.maxHealth, 110);
    assert.strictEqual(user.exp, 10005);
  });

  it('exp -1 (frozen exp) stays untouched', async () => {
    const db = fakeDb();
    const user = db.setUser(SENDER, { exp: -1 });
    user.rpg = {};

    const res = await addExpWithLevelCheck(null, fakeM(), db, user, 15);

    assert.strictEqual(user.exp, -1);
    assert.strictEqual(res.leveledUp, false);
  });

  it('null user returns zero result without db writes', async () => {
    const db = fakeDb();
    const res = await addExpWithLevelCheck(null, fakeM(), db, null, 15);
    assert.deepStrictEqual(res, {
      leveledUp: false,
      notified: false,
      oldLevel: 1,
      newLevel: 1,
    });
    assert.strictEqual(db.calls.setUser, 0);
  });
});
