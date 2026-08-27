// Regression tests for confirmed memory leaks.
// Run: node --test tests/memory-leaks.test.mjs
import { describe, it, before, after } from "node:test";
import assert from "node:assert";

describe("Scheduler CronJob leak (ourin-scheduler.js initScheduler)", () => {
  let mod, config;

  before(async () => {
    config = (await import("../config.js")).default;
    mod = await import("../src/lib/ourin-scheduler.js");
  });

  after(() => {
    for (const [, job] of mod.activeCronJobs) {
      try {
        job.stop();
      } catch {}
    }
    mod.activeCronJobs.clear();
  });

  it("initScheduler registers the 5-min messageSaver job under a key", () => {
    mod.initScheduler(config, null);
    assert.ok(
      mod.activeCronJobs.has("messageSaverTick"),
      "messageSaverTick should be tracked in activeCronJobs",
    );
  });

  it("calling initScheduler twice does not stack duplicate jobs", () => {
    const beforeSize = mod.activeCronJobs.size;
    const job1 = mod.activeCronJobs.get("messageSaverTick");
    mod.initScheduler(config, null);
    const job2 = mod.activeCronJobs.get("messageSaverTick");
    assert.strictEqual(
      mod.activeCronJobs.size,
      beforeSize,
      "tracked job count must not grow",
    );
    assert.notStrictEqual(job1, job2, "old job should be stopped and replaced");
  });
});

describe("antispam spamTracker cap (plugins/group/antispam.js)", () => {
  let mod;

  before(async () => {
    mod = await import("../plugins/group/antispam.js");
  });

  after(() => {
    mod.spamTracker.clear();
  });

  it("checkSpam sweeps stale entries once the tracker exceeds the cap", () => {
    const fakeDb = {
      getGroup: () => ({ antispam: true, antispamDelay: 2000 }),
    };
    const fakeM = {
      isGroup: true,
      isAdmin: false,
      isOwner: false,
      fromMe: false,
      sender: "111111111111@s.whatsapp.net",
      chat: "222222222222@g.us",
    };
    const sock = { groupParticipantsUpdate: async () => {} };

    // Seed >5000 stale entries (older than the 10-min sweep cutoff).
    const stale = Date.now() - 60 * 60 * 1000;
    for (let i = 0; i < 5500; i++) {
      mod.spamTracker.set(`${fakeM.chat}_5${String(i).padStart(12, "0")}@s.whatsapp.net`, {
        count: 0,
        lastMessage: stale,
        warnings: 0,
      });
    }
    assert.ok(mod.spamTracker.size > 5000, "seed should exceed the cap");

    mod.checkSpam(fakeM, sock, fakeDb);
    assert.ok(
      mod.spamTracker.size < 5000,
      `expected sweep after cap, got ${mod.spamTracker.size}`,
    );
  });
});

describe("waifupool pages cap (plugins/fun/waifupool.js)", () => {
  let mod;

  before(async () => {
    mod = await import("../plugins/fun/waifupool.js");
  });

  after(() => {
    mod.pages.clear();
  });

  it("handler trims stale page entries once the map exceeds the cap", async () => {
    const stale = Date.now() - 60 * 60 * 1000;
    for (let i = 0; i < 600; i++) {
      mod.pages.set(`user${i}@s.whatsapp.net:q${i}`, { page: 0, ts: stale });
    }
    assert.ok(mod.pages.size > 500, "seed should exceed the cap");

    const m = {
      sender: "333333333333@s.whatsapp.net",
      chat: "444444444444@g.us",
      prefix: ".",
      args: ["rare"],
      reply: async () => ({ key: { id: "x" } }),
    };
    await mod.handler(m, { sock: { sendButton: async () => ({ key: { id: "x" } }) } });

    assert.ok(
      mod.pages.size < 600,
      `expected trim after cap, got ${mod.pages.size}`,
    );
  });
});
