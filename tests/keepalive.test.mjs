import { describe, it } from "node:test";
import assert from "node:assert";
import config from "../config.js";
import { KEEPALIVE_FLOOR_MS, resolveKeepAlive } from "../src/lib/keepalive.js";

describe("keepalive must leave enough stall tolerance for an idle bot", () => {
  it("config declares an explicit keepAliveIntervalMs at or above the floor", () => {
    const v = config.session?.keepAliveIntervalMs;
    assert.notEqual(
      v,
      undefined,
    );
    assert.ok(
      v >= KEEPALIVE_FLOOR_MS,
      `keepAliveIntervalMs=${v} memberi toleransi ${v + 5000}ms — di bawah floor ${KEEPALIVE_FLOOR_MS}`,
    );
  });

  it("tolerance is at least 30s of event-loop stall", () => {
    const { intervalMs } = resolveKeepAlive({ configured: config.session?.keepAliveIntervalMs });
    assert.ok(
      intervalMs + 5000 >= 30_000,
      `toleransi ${intervalMs + 5000}ms terlalu kecil untuk bot yang idle`,
    );
  });

  it("never returns an interval below the floor, even if configured lower", () => {
    assert.equal(resolveKeepAlive({ configured: 5000 }).intervalMs, KEEPALIVE_FLOOR_MS);
    assert.equal(resolveKeepAlive({ configured: 0 }).intervalMs, KEEPALIVE_FLOOR_MS);
    assert.equal(resolveKeepAlive({ configured: -1 }).intervalMs, KEEPALIVE_FLOOR_MS);
    assert.equal(resolveKeepAlive({}).intervalMs, KEEPALIVE_FLOOR_MS);
    assert.equal(resolveKeepAlive({ configured: null }).intervalMs, KEEPALIVE_FLOOR_MS);
  });

  it("respects a configured value above the floor", () => {
    assert.equal(resolveKeepAlive({ configured: 45000 }).intervalMs, 45000);
  });
});
