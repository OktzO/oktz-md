// Task 6 regression tests: monitor limit + latch, srt idle TTL, watcher cache
// cap, swgc/swgcv2 timeout unlink, smemevid error-path unlink.
// Run: node --test tests/memory-monitor-limit.test.mjs
import { describe, it, before, after } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const RSS_LIMIT = 550 * 1024 * 1024;

describe("memory monitor sustained over-limit + warn latch (M-j)", () => {
  let mod;

  before(async () => {
    mod = await import("../src/lib/memory-monitor.js");
  });

  it("evaluateOverLimit flags after 3 consecutive checks, resets on drop", () => {
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), false, "1st");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), false, "2nd");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), true, "3rd consecutive");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), true, "stays flagged");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT - 1), false, "drop resets");
    assert.equal(mod.evaluateOverLimit(RSS_LIMIT + 1), false, "streak restarts");
  });

  it("shouldWarnOverLimit fires ONCE per sustained streak (no re-warn)", () => {
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), false, "pre-3 streak");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), true, "3rd → warn fires");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), false, "still over → NO re-warn");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), false, "still over → NO re-warn (2)");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT - 1), false, "drop clears latch");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), false, "re-streak 1st");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), false, "re-streak 2nd");
    assert.equal(mod.shouldWarnOverLimit(RSS_LIMIT + 1), true, "re-streak 3rd → warn again");
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
  before(async () => {
    await import("../src/lib/cache-cap.js");
  });

  it("evicts oldest past 800", async () => {
    const { evictOldestOverCap } = await import("../src/lib/cache-cap.js");
    const map = new Map();
    for (let i = 0; i < 820; i++) map.set("plugin_" + i, { mtimeMs: i, size: 1 });
    evictOldestOverCap(map, 800);
    assert.equal(map.size, 800, "pinned to 800");
    assert.ok(!map.has("plugin_0"), "oldest evicted");
    assert.ok(map.has("plugin_819"), "newest kept");
    assert.ok(map.has("plugin_300"), "mid entries kept");
  });

  it("does nothing under cap", async () => {
    const { evictOldestOverCap } = await import("../src/lib/cache-cap.js");
    const map = new Map([["a", 1], ["b", 2]]);
    evictOldestOverCap(map, 800);
    assert.equal(map.size, 2);
  });
});

describe("swgc/swgcv2 pending timeout unlinks tempFile (seam: pending map + 10-min timer)", () => {
  it("swgc timeout deletes tempFile and clears pending entry", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "swgc-"));
    const tempFile = path.join(tmp, "swgc.bin");
    fs.writeFileSync(tempFile, "x");
    const mod = await import("../plugins/owner/swgc.js");
    const sender = "6281111111111@s.whatsapp.net";
    const m = {
      sender,
      chat: "2222222222222@g.us",
      prefix: ".",
      text: "hai",
      args: [],
      reply: async () => {},
    };
    const sock = { groupFetchAllParticipating: async () => ({}) };
    await mod.handler(m, { sock, db: null });

    const entry = mod.pendingSwgc.get(sender);
    assert.ok(entry, "pending seeded by handler");
    mod.pendingSwgc.set(sender, { rawContent: entry.rawContent, tempFile, timestamp: entry.timestamp });

    t.mock.timers.tick(10 * 60 * 1000 + 1);
    assert.equal(fs.existsSync(tempFile), false, "temp unlinked on timeout");
    assert.ok(!mod.pendingSwgc.has(sender), "pending cleared, no double-unlink possible");
  });

  it("swgcv2 timeout deletes tempFile and clears pending entry", async (t) => {
    t.mock.timers.enable({ apis: ["setTimeout"] });
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "swgcv2-"));
    const tempFile = path.join(tmp, "swgcv2.bin");
    fs.writeFileSync(tempFile, "x");
    const mod = await import("../plugins/owner/swgcv2.js");
    const sender = "6281222222222@s.whatsapp.net";
    const m = {
      sender,
      chat: "2222222222222@g.us",
      prefix: ".",
      text: "hai",
      args: [],
      reply: async () => {},
    };
    const sock = { groupFetchAllParticipating: async () => ({}) };
    await mod.handler(m, { sock, db: null });

    const entry = mod.pendingSwgcV2.get(sender);
    assert.ok(entry, "pending seeded by handler");
    mod.pendingSwgcV2.set(sender, { rawContent: entry.rawContent, tempFile, timestamp: entry.timestamp });

    t.mock.timers.tick(10 * 60 * 1000 + 1);
    assert.equal(fs.existsSync(tempFile), false, "temp unlinked on timeout");
    assert.ok(!mod.pendingSwgcV2.has(sender), "pending cleared, no double-unlink possible");
  });
});

describe("smemevid error path unlinks os.tmpdir temp files", () => {
  it("no new vid-/overlay- temp files left behind when processing errors", async () => {
    const mod = await import("../plugins/sticker/smemevid.js");
    const before = new Set(
      fs.readdirSync(os.tmpdir()).filter((f) => /^(vid-|vid-out-|overlay-)/.test(f)),
    );
    const m = {
      isVideo: false,
      quoted: { isVideo: true, download: async () => Buffer.from("garbage-not-video") },
      args: ["TOP|BOTTOM"],
      prefix: ".",
      react: async () => {},
      reply: async () => {},
    };
    const sock = { sendVideoAsSticker: async () => { throw new Error("must not reach"); } };
    await mod.handler(m, { sock });

    const after = fs
      .readdirSync(os.tmpdir())
      .filter((f) => /^(vid-|vid-out-|overlay-)/.test(f) && !before.has(f));
    assert.equal(after.length, 0, `leftover temp files: ${after.join(", ")}`);
  });
});