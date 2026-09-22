import { describe, it, before, beforeEach } from "node:test";
import assert from "node:assert";
import { sweepGroupMetadataCache } from "../src/lib/ourin-lid.js";
import {
  createStickerPackCache,
  packSizeBytes,
  extendSocket,
} from "../src/lib/ourin-socket.js";

const AGE_30M = 30 * 60 * 1000;

function pack(name, stickerBytes) {
  return {
    name,
    message: {
      stickerPackMessage: {
        stickerPackId: `${name}-id`,
        stickers: [{ sticker: Buffer.alloc(stickerBytes), emojis: ["😀"] }],
      },
    },
  };
}

describe("groupMetadataCache age-sweep (>800 cap, drop since >30min)", () => {
  let cache;

  beforeEach(() => {
    cache = new Map();
  });

  it("drops only stale (>30min) entries when size >800", () => {
    const now = Date.now();
    for (let i = 0; i < 800; i++) {
      cache.set(`stale${i}@g.us`, {
        data: { subject: `s${i}` },
        timestamp: now - AGE_30M - 60 * 1000,
      });
    }
    for (let i = 0; i < 10; i++) {
      cache.set(`fresh${i}@g.us`, {
        data: { subject: `f${i}` },
        timestamp: now - 60 * 1000,
      });
    }
    assert.equal(cache.size, 810);

    const dropped = sweepGroupMetadataCache(cache, now);

    assert.equal(dropped, 800);
    assert.equal(cache.size, 10);
    assert.ok(cache.has("fresh0@g.us"), "fresh entry kept");
    assert.ok(!cache.has("stale0@g.us"), "stale entry dropped");
  });

  it("keeps full cached metadata for surviving lookups", () => {
    const now = Date.now();
    cache.set("keep@g.us", {
      data: { subject: "K", participants: [{ id: "p", admin: null }] },
      timestamp: now - 1000,
    });
    for (let i = 0; i < 801; i++) {
      cache.set(`stale${i}@g.us`, {
        data: { subject: "x" },
        timestamp: now - AGE_30M - 1000,
      });
    }
    sweepGroupMetadataCache(cache, now);
    assert.equal(cache.get("keep@g.us").data.subject, "K");
    assert.equal(cache.get("keep@g.us").data.participants.length, 1);
  });

  it("does not sweep when size is at or under 800", () => {
    const now = Date.now();
    for (let i = 0; i < 100; i++) {
      cache.set(`stale${i}@g.us`, {
        data: { subject: "x" },
        timestamp: now - AGE_30M - 1000,
      });
    }
    assert.equal(sweepGroupMetadataCache(cache, now), 0);
    assert.equal(cache.size, 100, "under-cap map untouched even with stale data");
  });
});

describe("stickerPackCache byte-cap (200MB default, evict oldest)", () => {
  it("computes pack size from sticker buffers", () => {
    const data = pack("p", 1024);
    assert.equal(packSizeBytes(data), 1024);
  });

  it("falls back to non-zero estimate when no buffers found", () => {
    assert.ok(packSizeBytes({ message: { stickerPackMessage: { stickerPackId: "x", stickers: [] } } }) >= 1);
  });

  it("evicts oldest pack until total under cap", () => {
    const cache = createStickerPackCache(1000);
    cache.set("a", pack("a", 600));
    cache.set("b", pack("b", 600));

    assert.equal(cache.get("a"), undefined, "oldest evicted");
    assert.ok(cache.get("b"), "newest kept");
    assert.ok(cache.totalBytes <= 1000);
  });

  it("evicts multiple packs until under cap", () => {
    const cache = createStickerPackCache(1000);
    cache.set("a", pack("a", 400));
    cache.set("b", pack("b", 400));
    cache.set("c", pack("c", 400));
    cache.set("d", pack("d", 400));

    assert.equal(cache.get("a"), undefined);
    assert.equal(cache.get("b"), undefined);
    assert.ok(cache.get("c"), "kept");
    assert.ok(cache.get("d"), "kept");
    assert.ok(cache.totalBytes <= 1000);
  });

  it("keeps a single oversized pack (never evicts last entry)", () => {
    const cache = createStickerPackCache(200);
    cache.set("huge", pack("huge", 500));
    assert.ok(cache.get("huge"), "single pack retained even over cap");
  });

  it("tracks re-set of existing key without double counting", () => {
    const cache = createStickerPackCache(900);
    cache.set("a", pack("a", 300));
    cache.set("b", pack("b", 300));
    cache.set("a", pack("a", 300));
    cache.set("c", pack("c", 300));
    assert.equal(cache.size, 3, "no double count (900 <= cap), nothing evicted");
    assert.equal(cache.totalBytes, 900);
  });

  it("enumerates entries for getSavedPacks", () => {
    const cache = createStickerPackCache(1000);
    cache.set("a", pack("a", 100));
    const ids = [...cache.entries()].map(([id]) => id);
    assert.deepEqual(ids, ["a"]);
  });
});

describe("stickerPack save + contacts cap via extendSocket", () => {
  let sock;

  before(async () => {
    global.stickerPackCache = undefined;
    const fakeSock = {
      sendMessage: async () => ({}),
      profilePictureUrl: async () => null,
      user: { id: "6281:3" },
    };
    sock = await extendSocket(fakeSock);
    assert.ok(sock.saveStickerPack, "extendSocket wires saveStickerPack");
  });

  beforeEach(() => {
    if (global.stickerPackCache) global.stickerPackCache.clear();
    else global.stickerPackCache = createStickerPackCache();
  });

  it("saveStickerPack only touches the in-memory cache (no per-insert disk persist)", () => {
    let sets = 0;
    const spy = {
      set: (k, v) => {
        sets++;
        realCache.set(k, v);
        return spy;
      },
      get: (k) => realCache.get(k),
      entries: () => realCache.entries(),
      has: (k) => realCache.has(k),
      clear: () => realCache.clear(),
    };
    const realCache = createStickerPackCache();
    global.stickerPackCache = spy;

    sock.saveStickerPack("p1", {
      stickerPackMessage: { stickerPackId: "p1", stickers: [] },
    }, "Pack 1");

    assert.equal(sets, 1, "exactly one cache set per incoming pack");
    assert.ok(global.stickerPackCache.get("p1"), "pack retained for on-demand rebuild");
  });

  it("getSavedPacks lists saved packs", () => {
    sock.saveStickerPack("p2", { stickerPackMessage: { stickerPackId: "p2" } }, "Pack 2");
    const packs = sock.getSavedPacks();
    assert.equal(packs.length, 1);
    assert.equal(packs[0].id, "p2");
    assert.equal(packs[0].name, "Pack 2");
  });

  it("getName skips store.contacts write at 500-entry cap", async () => {
    const contacts = {};
    for (let i = 0; i < 500; i++) {
      contacts[`628${String(100000000 + i)}@s.whatsapp.net`] = { name: `c${i}` };
    }
    const target = "62899999999@s.whatsapp.net";
    sock.store = { contacts };
    sock.getBusinessProfile = async () => ({
      wid: { user: "62899999999" },
      name: "BizName",
    });
    sock.onWhatsApp = async () => [];

    const name = await sock.getName(target);

    assert.equal(name, "BizName", "name still returned");
    assert.equal(Object.keys(contacts).length, 500);
    assert.ok(!contacts[target], "write skipped when store already at cap");
  });

  it("getName writes store.contacts when under cap", async () => {
    const contacts = {};
    for (let i = 0; i < 499; i++) {
      contacts[`628${String(100000000 + i)}@s.whatsapp.net`] = { name: `c${i}` };
    }
    const target = "62810000007@s.whatsapp.net";
    sock.store = { contacts };
    sock.getBusinessProfile = async () => ({
      wid: { user: "62810000007" },
      name: "BizName",
    });
    sock.onWhatsApp = async () => [];

    await sock.getName(target);

    assert.equal(Object.keys(contacts).length, 500);
    assert.equal(contacts[target].name, "BizName");
  });
});