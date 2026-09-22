import { describe, it, before } from "node:test";
import assert from "node:assert";

describe("AsyncPool queue cap (64, drop-newest)", () => {
  let m;

  before(async () => {
    m = await import("../src/lib/ourin-async-pool.js");
  });

  describe("shouldDropNewest decision", () => {
    it("default (maxQueued null) never drops", () => {
      assert.equal(m.shouldDropNewest(1000, null), false);
      assert.equal(m.shouldDropNewest(0, null), false);
    });

    it("drops only at/over cap", () => {
      assert.equal(m.shouldDropNewest(63, 64), false, "below cap enqueue ok");
      assert.equal(m.shouldDropNewest(64, 64), true, "at cap drops");
      assert.equal(m.shouldDropNewest(100, 64), true, "over cap drops");
    });
  });

  describe("pool behavior under flood", () => {
    it("rejects newest past cap, warns, does not abort running handler", async () => {
      const dropped = [];
      const pool = new m.AsyncPool(1, {
        maxQueued: 2,
        onDrop: (queued, max) => dropped.push({ queued, max }),
      });

      let releaseRunning;
      const runningPromise = new Promise((r) => (releaseRunning = r));
      let runningResolved = false;

      const p0 = pool.add(async () => {
        await runningPromise;
        runningResolved = true;
      });
      const queued = [pool.add(async () => {}), pool.add(async () => {})];

      assert.equal(pool.size, 2, "queue holds 2 below cap");

      let dropErr = null;
      await pool.add(() => {}).catch((e) => (dropErr = e));

      assert.ok(dropErr instanceof Error, "add past cap rejects");
      assert.match(dropErr.message, /queue full/);
      assert.match(dropErr.message, new RegExp(`\\b${2}\\b`), "message carries actual queued depth");
      assert.equal(dropped.length, 1, "onDrop warn fired once");
      assert.equal(dropped[0].queued, 2);
      assert.equal(dropped[0].max, 2);
      assert.equal(pool.size, 2, "queue still bounded at cap");
      assert.equal(runningResolved, false, "running handler untouched");

      releaseRunning();
      await p0;
      await Promise.allSettled(queued);
      assert.equal(runningResolved, true, "running handler finished normally");
      assert.equal(pool.size, 0, "queue drained after release");
    });

    it("pool without cap never drops (existing behavior)", async () => {
      const pool = new m.AsyncPool(1);

      let releaseRunning;
      const runningPromise = new Promise((r) => (releaseRunning = r));
      const p0 = pool.add(async () => { await runningPromise; });
      const queued = [];
      for (let i = 0; i < 5; i++) queued.push(pool.add(async () => {}));

      assert.equal(pool.size, 5, "queue accepts more than capped size");
      releaseRunning();
      await p0;
      await Promise.allSettled(queued);
      assert.equal(pool.size, 0);
    });
  });
});