// Task 6 regression tests: monitor limit, srt idle TTL, watcher cache cap.
// Run: node --test tests/memory-monitor-limit.test.mjs
import { describe, it, before, after } from "node:test";
import assert from "node:assert";

const RSS_LIMIT = 550 * 1024 * 1024;

describe("memory monitor sustained over-limit (M-j)", () => {
  let mod;

  before(async () => {
    mod = await import("../src/lib/ourin-memory-monitor.js");
  });

  it("flags after 3 consecutive over-limit checks, resets on drop", () => {
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), false, "1st");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), false, "2nd");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), true, "3rd consecutive");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), true, "stays flagged");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT - 1), false, "drop resets");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), false, "streak restarts");
  });
});

describe("srt session idle TTL 10 menit (plugins/owner/srt.js)", () => {
  let mod;

  before(async () => {
    global.srtSession = {};
    mod = await import("../plugins/owner/srt.js");
  });

  after(() => {
    delete global.srtSession;
  });

  it("expired session is dropped and chat treated as absent", () => {
    global.srtSession["chat1@g.us"] = {
      sender: "6281@s.whatsapp.net",
      count: 3,
      ts: Date.now() - 11 * 60 * 1000,
    };
    assert.equal(mod.getSrtSession("chat1@g.us"), null, "idle >10min expired");
    assert.equal(global.srtSession["chat1@g.us"], undefined, "expired removed");
  });

  it("recent session is returned and preserved", () => {
    global.srtSession["chat2@g.us"] = {
      sender: "6281@s.whatsapp.net",
      count: 1,
      ts: Date.now() - 60 * 1000,
    };
    const session = mod.getSrtSession("chat2@g.us");
    assert.ok(session, "active session returned");
    assert.ok(global.srtSession["chat2@g.us"], "still stored");
  });
});

describe("watcher fileStat cache cap 800 (FIFO)", () => {
  it("evicts oldest past 800", async () => {
    const { evictOldestOverCap } = await import("../src/lib/ourin-cache-cap.js");
    const map = new Map();
    for (let i = 0; i < 820; i++) map.set("plugin_" + i, { mtimeMs: i, size: 1 });
    evictOldestOverCap(map, 800);
    assert.equal(map.size, 800, "pinned to 800");
    assert.ok(!map.has("plugin_0"), "oldest evicted");
    assert.ok(map.has("plugin_819"), "newest kept");
    assert.ok(map.has("plugin_300"), "mid entries kept");
  });

  it("does nothing under cap", async () => {
    const { evictOldestOverCap } = await import("../src/lib/ourin-cache-cap.js");
    const map = new Map([["a", 1], ["b", 2]]);
    evictOldestOverCap(map, 800);
    assert.equal(map.size, 2);
  });
});