import { describe, it } from "node:test";
import assert from "node:assert";

import { resolveFromSock, LID_RESOLVE_TIMEOUT_MS } from "../src/lib/lid.js";

describe("lid resolveFromSock — latency guard", () => {
  it("negative cache: LID tanpa mapping tidak memanggil repo berulang", async () => {
    const LID = "999000111222@lid";
    let calls = 0;
    const sock = {
      signalRepository: { lidMapping: { getPNForLID: async () => { calls++; return null; } } },
      store: { contacts: {} },
    };
    const r1 = await resolveFromSock(LID, sock);
    assert.equal(r1, LID);
    assert.equal(calls, 1);
    const r2 = await resolveFromSock(LID, sock);
    assert.equal(r2, LID);
    assert.equal(calls, 1, "panggilan kedua harus dari negative cache");
  });

  it("inflight dedup: dua panggilan bersamaan hanya memanggil repo sekali", async () => {
    const LID = "999000111333@lid";
    let calls = 0;
    let resolveRepo;
    const repoPromise = new Promise((r) => { resolveRepo = r; });
    const sock = {
      signalRepository: { lidMapping: { getPNForLID: async () => { calls++; await repoPromise; return null; } } },
      store: { contacts: {} },
    };
    const p1 = resolveFromSock(LID, sock);
    const p2 = resolveFromSock(LID, sock);
    resolveRepo();
    const [a, b] = await Promise.all([p1, p2]);
    assert.equal(a, LID);
    assert.equal(b, LID);
    assert.equal(calls, 1, "repo hanya dipanggil sekali");
  });

  it("bounded: repo yang hang mengembalikan jid dalam ~timeout, bukan menunggu", async () => {
    const LID = "999000111444@lid";
    const sock = {
      signalRepository: { lidMapping: { getPNForLID: () => new Promise((r) => setTimeout(() => r(null), 2000)) } },
      store: { contacts: {} },
    };
    const start = Date.now();
    const r = await resolveFromSock(LID, sock);
    const elapsed = Date.now() - start;
    assert.equal(r, LID);
    assert.ok(elapsed < LID_RESOLVE_TIMEOUT_MS + 200, `elapsed ${elapsed}ms harus <= timeout`);
  });
});
