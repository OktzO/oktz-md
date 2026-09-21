import { describe, it, before, beforeEach } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { fileURLToPath, pathToFileURL } from "node:url";

const execFileAsync = promisify(execFile);
const TTL_24H = 24 * 60 * 60 * 1000;
const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

describe("afk storage sweep (>5000 cap, drop since >24h)", () => {
  let mod;

  before(async () => {
    global.afkStorage = new Map();
    mod = await import("../plugins/group/afk.js");
  });

  beforeEach(() => {
    global.afkStorage.clear();
  });

  it("removes only stale entries, keeps active sub-24h AFKs", () => {
    const store = global.afkStorage;
    for (let i = 0; i < 5000; i++) {
      store.set(`stale${i}@s.whatsapp.net`, {
        reason: "lama",
        since: Date.now() - TTL_24H - 60 * 1000,
      });
    }
    for (let i = 0; i < 10; i++) {
      store.set(`fresh${i}@s.whatsapp.net`, {
        reason: "baru",
        since: Date.now() - 60 * 1000,
      });
    }
    assert.equal(store.size, 5010, "setup should exceed cap");

    mod.sweepAfkStorage();

    assert.equal(store.size, 10, "all stale (>24h) entries swept");
    assert.ok(store.has("fresh0@s.whatsapp.net"), "active AFK kept");
    assert.ok(!store.has("stale0@s.whatsapp.net"), "stale entry gone");
  });

  it("does not sweep when size is under cap", () => {
    const store = global.afkStorage;
    for (let i = 0; i < 100; i++) {
      store.set(`stale${i}@s.whatsapp.net`, {
        reason: "lama",
        since: Date.now() - TTL_24H - 60 * 1000,
      });
    }
    mod.sweepAfkStorage();
    assert.equal(store.size, 100, "under-cap map untouched even with stale data");
  });

  it("setAfkUser auto-triggers the sweep", () => {
    const store = global.afkStorage;
    for (let i = 0; i < 5000; i++) {
      store.set(`stale${i}@s.whatsapp.net`, { reason: "x", since: Date.now() - TTL_24H - 1000 });
    }
    mod.setAfkUser("6281111111111@s.whatsapp.net", "baru");
    assert.equal(store.size, 1, "stale backlog swept on new AFK insert");
    assert.ok(store.has("6281111111111@s.whatsapp.net"), "new AFK preserved");
  });
});

describe("lid cache eviction (10k cap via cacheLidJid)", () => {
  it("keeps lidCache at 10k, evicting oldest inserts (clean process)", async () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "lid-test-"));
    try {
      const script = `
import { pathToFileURL } from "node:url";
const mod = await import(pathToFileURL(process.env.LID_MOD).href);
const MAX = 10000;
const lids = [];
for (let i = 0; i < MAX + 2; i++) {
  const num = "6281" + String(i).padStart(8, "0");
  const lid = num + "@s.whatsapp.net";
  mod.cacheLidJid(lid, num);
  lids.push(lid);
}
process.stdout.write(JSON.stringify({
  base: 0,
  size: mod.getLidCacheSize(),
  oldestEvicted: !mod.getCachedJid(lids[0]),
  newestPresent: !!mod.getCachedJid(lids[lids.length - 1]),
}));
`;
      const { stdout } = await execFileAsync(
        process.execPath,
        ["--input-type=module", "-e", script],
        { env: { ...process.env, LID_MOD: path.join(ROOT, "src/lib/ourin-lid.js") }, cwd: tmp },
      );
      const out = JSON.parse(stdout.trim().split("\n").pop());
      assert.equal(out.size, 10000, "lid cache pinned at 10k");
      assert.equal(out.oldestEvicted, true, "oldest insert evicted past cap");
      assert.equal(out.newestPresent, true, "newest insert retained");
    } finally {
      fs.rmSync(tmp, { recursive: true, force: true });
    }
  });
});

describe("turso keys local cache cap 1000 (FIFO)", () => {
  it("trimLocalCache evicts oldest past the cap on load path", async () => {
    const mod = await import("../src/lib/ourin-turso-session.js");
    const local = new Map();
    for (let i = 0; i < 1200; i++) local.set("id_" + i, { v: i });
    mod.trimLocalCache(local, 1000);
    assert.equal(local.size, 1000, "trimmed to cap");
    assert.ok(!local.has("id_0"), "oldest evicted");
    assert.ok(local.has("id_1199"), "newest kept");
    assert.ok(local.has("id_200"), "middle entries kept");
  });

  it("does nothing under cap", async () => {
    const mod = await import("../src/lib/ourin-turso-session.js");
    const local = new Map([["a", 1], ["b", 2]]);
    mod.trimLocalCache(local, 1000);
    assert.equal(local.size, 2);
  });
});

describe("jadibot per-session group metadata cache cap 200", () => {
  it("evictOldestOverCap drops oldest beyond the 200 cap", async () => {
    const mod = await import("../src/lib/ourin-jadibot-manager.js");
    assert.equal(mod.GROUP_META_CACHE_CAP, 200, "cap value fixed at 200");
    const map = new Map();
    for (let i = 0; i < 210; i++) map.set("jid_" + i, { participants: [] });
    mod.evictOldestOverCap(map, mod.GROUP_META_CACHE_CAP);
    assert.equal(map.size, 200, "reduced to cap");
    assert.ok(!map.has("jid_0"), "oldest evicted");
    assert.ok(map.has("jid_209"), "newest kept");
    assert.ok(map.has("jid_100"), "mid entries kept");
  });
});

describe("auto-ai session prune (idle 24h + size cap 250)", () => {
  it("size cap 250 drops oldest when all sessions active", async () => {
    const mod = await import("../src/lib/ourin-auto-ai.js");
    const autoai = { sessions: {} };
    const now = Date.now();
    for (let i = 0; i < 260; i++) {
      autoai.sessions["user" + i] = { history: [], lastActive: now - 60 * 1000 };
    }
    mod.saveToHistory(autoai, "user260", "user", "hai");

    const keys = Object.keys(autoai.sessions);
    assert.equal(keys.length, 250, "size capped to 250 after idle prune found nothing");
    assert.ok(autoai.sessions.user260, "session being written kept");
    assert.ok(!autoai.sessions.user0, "oldest evicted by size cap");
    assert.ok(autoai.sessions.user259, "recent sessions kept");
  });

  it("idle 24h prune runs first and preserves active sessions", async () => {
    const mod = await import("../src/lib/ourin-auto-ai.js");
    const autoai = { sessions: {} };
    const now = Date.now();
    for (let i = 0; i < 260; i++) {
      autoai.sessions["user" + i] = {
        history: [],
        lastActive: i < 228 ? now - TTL_24H - 60 * 60 * 1000 : now - 60 * 1000,
      };
    }
    mod.saveToHistory(autoai, "user260", "user", "hai");

    assert.equal(Object.keys(autoai.sessions).length, 33, "228 idle + 1 new dropped down to active set");
    assert.ok(!autoai.sessions.user0, "idle >24h session removed by idle prune");
    assert.ok(autoai.sessions.user228, "active session kept");
    assert.ok(autoai.sessions.user260, "new session kept");
  });
});

describe("absen per-chat prune (>500 chats, drop createdAt >24h)", () => {
  let mod;

  before(async () => {
    global.absensi = {};
    mod = await import("../plugins/group/absen.js");
  });

  beforeEach(() => {
    global.absensi = {};
  });

  it("prunes stale chats only when map exceeds 500", () => {
    const now = Date.now();
    for (let i = 0; i < 500; i++) {
      global.absensi["chat_stale" + i] = {
        createdAt: new Date(now - TTL_24H - 60 * 1000).toISOString(),
        peserta: [],
      };
    }
    for (let i = 0; i < 10; i++) {
      global.absensi["chat_fresh" + i] = {
        createdAt: new Date(now - 60 * 1000).toISOString(),
        peserta: ["628" + i + "@s.whatsapp.net"],
      };
    }
    assert.equal(Object.keys(global.absensi).length, 510, "setup exceeds cap");

    mod.pruneAbsensi();

    assert.equal(Object.keys(global.absensi).length, 10, "stale chats pruned");
    assert.ok(global.absensi["chat_fresh0"], "fresh chat kept");
    assert.ok(!global.absensi["chat_stale499"], "stale chat gone");
  });

  it("handles numeric createdAt and keeps under-cap map intact", () => {
    const now = Date.now();
    global.absensi["num_chat"] = { createdAt: now - TTL_24H - 1000, peserta: [] };
    global.absensi["plain_chat"] = { createdAt: now - 1000, peserta: [] };
    mod.pruneAbsensi();
    assert.ok(global.absensi["num_chat"], "under-cap: no prune regardless of age");
    assert.ok(global.absensi["plain_chat"]);
  });
});