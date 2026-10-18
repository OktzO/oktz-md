import { describe, it } from "node:test";
import assert from "node:assert";

import { getGroupMetadata, GROUP_META_WAIT_MS } from "../src/lib/group-meta.js";

describe("group-meta getGroupMetadata — bounded wait", () => {
  it("mengembalikan metadata bila fetch cepat", async () => {
    const meta = { id: "g@g.us", participants: [] };
    const sock = { groupMetadata: async () => meta };
    const cache = new Map();
    global.groupMetadataCache = cache;
    const r = await getGroupMetadata(sock, "g@g.us", { waitMs: 1000 });
    assert.deepEqual(r, meta);
    assert.ok(cache.get("g@g.us")?.data, "cache harus terisi");
    delete global.groupMetadataCache;
  });

  it("mengembalikan null bila fetch hang, dalam ~waitMs", async () => {
    const sock = { groupMetadata: () => new Promise((r) => setTimeout(() => r(null), 200)) };
    global.groupMetadataCache = new Map();
    const start = Date.now();
    const r = await getGroupMetadata(sock, "hang@g.us", { waitMs: 80 });
    const elapsed = Date.now() - start;
    assert.equal(r, null);
    assert.ok(elapsed < 80 + 100, `elapsed ${elapsed}ms`);
    delete global.groupMetadataCache;
  });

  it("inflight dedup: dua panggilan bersamaan hanya fetch sekali", async () => {
    let calls = 0;
    let resolveFetch;
    const fetchPromise = new Promise((r) => { resolveFetch = r; });
    const sock = { groupMetadata: async () => { calls++; await fetchPromise; return { id: "x@g.us" }; } };
    global.groupMetadataCache = new Map();
    const p1 = getGroupMetadata(sock, "x@g.us", { waitMs: 1000 });
    const p2 = getGroupMetadata(sock, "x@g.us", { waitMs: 1000 });
    resolveFetch();
    const [a, b] = await Promise.all([p1, p2]);
    assert.equal(calls, 1);
    assert.deepEqual(a, { id: "x@g.us" });
    assert.deepEqual(b, { id: "x@g.us" });
    delete global.groupMetadataCache;
  });

  it("cache hit tidak memanggil fetch", async () => {
    const cache = new Map();
    cache.set("c@g.us", { data: { id: "c@g.us", cached: true }, timestamp: Date.now() });
    global.groupMetadataCache = cache;
    let calls = 0;
    const sock = { groupMetadata: async () => { calls++; return null; } };
    const r = await getGroupMetadata(sock, "c@g.us", { waitMs: 100 });
    assert.deepEqual(r, { id: "c@g.us", cached: true });
    assert.equal(calls, 0);
    delete global.groupMetadataCache;
  });
});
