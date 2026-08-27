import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

const SENDER = '6281234567890@s.whatsapp.net';
const AFK_USER = '6289876543210@s.whatsapp.net';
const GROUP = '120363000000000000@g.us';

async function loadAfk() {
  return await import('../plugins/group/afk.js');
}

function fakeDb() {
  const users = {};
  return {
    users,
    getUser(jid) { return users[jid.replace(/@.+/g, '')] || null; },
    setUser(jid, data = {}) {
      const clean = jid.replace(/@.+/g, '');
      users[clean] = { ...(users[clean] || {}), ...data };
      return users[clean];
    },
  };
}

function mockReply() {
  const sent = [];
  return {
    sent,
    reply(text, opts) { sent.push({ text, opts }); return Promise.resolve(); },
  };
}

describe('afk persistence', () => {
  let mod;

  beforeEach(async () => {
    mod = await loadAfk();
  });

  it('setAfkUser stores afk record in db', async () => {
    const db = fakeDb();
    mod.setAfkUser(SENDER, 'lagi makan', db);
    assert.deepStrictEqual(db.users['6281234567890'].afk, {
      reason: 'lagi makan',
      since: mod.getAfkUser(SENDER, db).since,
    });
  });

  it('getAfkUser reads from db (survives restart)', async () => {
    const db = fakeDb();
    db.users['6289876543210'] = { afk: { reason: 'kerja', since: Date.now() - 60000 } };
    const afk = mod.getAfkUser(AFK_USER, db);
    assert.ok(afk, 'afk record should come from db');
    assert.strictEqual(afk.reason, 'kerja');
  });

  it('removeAfkUser clears afk from db', async () => {
    const db = fakeDb();
    db.users['6289876543210'] = { afk: { reason: 'x', since: Date.now() } };
    mod.removeAfkUser(AFK_USER, db);
    const user = db.users['6289876543210'];
    assert.ok(!user.afk, `afk should be gone in db, got ${JSON.stringify(user.afk)}`);
    assert.ok(!mod.getAfkUser(AFK_USER, db));
  });

  it('falls back to memory when db rejects jid (group jid)', async () => {
    const db = fakeDb();
    mod.setAfkUser(GROUP, 'test', db);
    assert.ok(mod.getAfkUser(GROUP), 'memory fallback should work for rejected jid');
  });
});

describe('afk alias guard', () => {
  it('typing .away while afk does not end-and-restart afk twice', async () => {
    const mod = await loadAfk();
    mod.setAfkUser(SENDER, 'test');
    let replied = false;
    const m = {
      sender: SENDER,
      isCommand: true,
      command: 'away',
      mentionedJid: [],
      isGroup: false,
      reply: async () => { replied = true; },
    };
    await mod.checkAfk(m, {});
    assert.strictEqual(replied, false, '.away alias must not trigger afk-end reply');
    mod.removeAfkUser(SENDER);
  });
});

describe('afk mention throttle', () => {
  it('notifies only once per mentioner-pair per session', async () => {
    const mod = await loadAfk();
    mod.setAfkUser(AFK_USER, 'sibuk');
    const msg = () => ({
      sender: SENDER,
      isCommand: false,
      command: null,
      mentionedJid: [AFK_USER],
      isGroup: true,
      ...mockReply(),
    });

    const first = msg();
    await mod.checkAfk(first, {});
    assert.strictEqual(first.sent.length, 1, 'first mention should notify');

    const second = msg();
    await mod.checkAfk(second, {});
    assert.strictEqual(second.sent.length, 0, 'repeat mention must be throttled');

    mod.removeAfkUser(AFK_USER);
  });
});
