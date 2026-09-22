import { describe, it, before } from "node:test";
import assert from "node:assert";

describe("scheduler repeat expiresAt", () => {
  let m;

  before(async () => {
    m = await import("../src/lib/ourin-scheduler.js");
  });

  const now = Date.now();

  it("repeat + expiresAt in past stops (returns true)", () => {
    assert.equal(m.isRepeatExpired(true, now - 1000, now), true);
  });

  it("repeat + expiresAt equal to now stops (>=)", () => {
    assert.equal(m.isRepeatExpired(true, now, now), true);
  });

  it("repeat + expiresAt in future continues (returns false)", () => {
    assert.equal(m.isRepeatExpired(true, now + 60_000, now), false);
  });

  it("repeat without expiresAt continues forever", () => {
    assert.equal(m.isRepeatExpired(true, null, now), false);
    assert.equal(m.isRepeatExpired(true, undefined, now), false);
  });

  it("non-repeat task never expires via this path", () => {
    assert.equal(m.isRepeatExpired(false, now - 1000, now), false);
  });
});

describe("scheduler reload predicate (shouldReloadTask)", () => {
  let m;

  before(async () => {
    m = await import("../src/lib/ourin-scheduler.js");
  });

  const now = Date.now();
  const future = new Date(now + 30 * 60 * 1000).toISOString();
  const past = new Date(now - 30 * 60 * 1000).toISOString();

  it("expired repeat with FUTURE nextRun is skipped on load", () => {
    const task = { repeat: true, expiresAt: now - 1000, nextRun: future };
    assert.equal(m.shouldReloadTask(task, now), false, "expired repeat must not re-arm even with future nextRun");
  });

  it("active repeat (future expiresAt) still loads", () => {
    const task = { repeat: true, expiresAt: now + 60 * 60 * 1000, nextRun: future };
    assert.equal(m.shouldReloadTask(task, now), true);
  });

  it("repeat without expiresAt loads forever (old path)", () => {
    const task = { repeat: true, expiresAt: null, nextRun: past };
    assert.equal(m.shouldReloadTask(task, now), true, "expiresAt null => never expires, nextRun irrelevant");
    const taskUndefined = { repeat: true, nextRun: undefined };
    assert.equal(m.shouldReloadTask(taskUndefined, now), true);
  });

  it("one-shot loads only if nextRun in future (old path)", () => {
    const futureTask = { repeat: false, nextRun: future };
    const pastTask = { repeat: false, nextRun: past };
    assert.equal(m.shouldReloadTask(futureTask, now), true);
    assert.equal(m.shouldReloadTask(pastTask, now), false);
  });
});

describe("notifiedGroups age sweep (>60min)", () => {
  let m;

  before(async () => {
    m = await import("../src/lib/ourin-scheduler.js");
  });

  const now = Date.now();

  it("removes entries older than 60 min, keeps fresh ones", () => {
    const map = new Map([
      ["stale_123@x", now - 61 * 60 * 1000],
      ["boundary_456@x", now - 60 * 60 * 1000],
      ["fresh_789@x", now - 60 * 1000],
    ]);

    m.sweepNotifiedGroups(map, now);

    assert.equal(map.has("stale_123@x"), false, "stale >60min swept");
    assert.equal(map.has("boundary_456@x"), true, "exactly 60min kept (< not <=)");
    assert.equal(map.has("fresh_789@x"), true, "fresh kept");
  });

  it("sweep is a no-op on empty map and under-cap maps", () => {
    const empty = new Map();
    m.sweepNotifiedGroups(empty, now);
    assert.equal(empty.size, 0);

    const allFresh = new Map([["a@x", now - 1000], ["b@x", now - 59 * 60 * 1000]]);
    m.sweepNotifiedGroups(allFresh, now);
    assert.equal(allFresh.size, 2, "nothing older than 60min stays");
  });
});