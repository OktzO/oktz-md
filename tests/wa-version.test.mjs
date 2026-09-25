import { describe, it } from "node:test";
import assert from "node:assert";

describe("wa version resolution", () => {
  const ok = (v) => ({ version: v, isLatest: true });
  const fallback = (v, error) => ({ version: v, isLatest: false, error });

  it("returns the live revision when the first source succeeds", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => ok([2, 3000, 111]), async () => ok([2, 3000, 222])],
      cache: null,
    });
    assert.deepEqual(version, [2, 3000, 111]);
  });

  it("tries the second source when the first returns a fallback", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => fallback([9, 9, 9], new Error("blocked")), async () => ok([2, 3000, 222])],
      cache: null,
    });
    assert.deepEqual(version, [2, 3000, 222]);
  });

  it("falls back to the library default when every source is blocked", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => fallback([9, 9, 9], new Error("a")), async () => fallback([9, 9, 9], new Error("b"))],
      cache: null,
      defaultVersion: [2, 3000, 1043857760],
    });
    assert.deepEqual(version, [2, 3000, 1043857760]);
  });

  it("never returns undefined when every source throws", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => { throw new Error("boom"); }, async () => { throw new Error("boom"); }],
      cache: null,
      defaultVersion: [2, 3000, 1043857760],
    });
    assert.deepEqual(version, [2, 3000, 1043857760]);
  });

  it("never returns undefined when there are no sources at all", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [],
      cache: null,
      defaultVersion: [2, 3000, 1043857760],
    });
    assert.deepEqual(version, [2, 3000, 1043857760]);
  });

  it("prefers a cached live version without calling any source", async () => {
    const m = await import("../src/lib/wa-version.js");
    let called = false;
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => { called = true; return ok([2, 3000, 999]); }],
      cache: { v: [2, 3000, 111], t: Date.now() },
    });
    assert.deepEqual(version, [2, 3000, 111]);
    assert.equal(called, false);
  });

  it("refetches once the cache has expired", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => ok([2, 3000, 222])],
      cache: { v: [2, 3000, 111], t: Date.now() - 7 * 3600e3 },
    });
    assert.deepEqual(version, [2, 3000, 222]);
  });

  it("falls back to a stale cached version when every source is blocked", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [async () => { throw new Error("boom"); }],
      cache: { v: [2, 3000, 111], t: Date.now() - 7 * 3600e3 },
      defaultVersion: [9, 9, 9],
    });
    assert.deepEqual(version, [2, 3000, 111]);
  });

  it("gives up on a source that hangs past the timeout", async () => {
    const m = await import("../src/lib/wa-version.js");
    const { version } = await m.resolveWaVersion({
      fetchers: [() => new Promise(() => {}), async () => ok([2, 3000, 222])],
      cache: null,
      timeoutMs: 50,
      defaultVersion: [9, 9, 9],
    });
    assert.deepEqual(version, [2, 3000, 222]);
  });

  it("reports whether the resolved version should be cached", async () => {
    const m = await import("../src/lib/wa-version.js");
    const fresh = await m.resolveWaVersion({
      fetchers: [async () => ok([2, 3000, 111])],
      cache: null,
    });
    assert.equal(fresh.isLatest, true);

    const stale = await m.resolveWaVersion({
      fetchers: [async () => fallback([9, 9, 9], new Error("x"))],
      cache: null,
      defaultVersion: [9, 9, 9],
    });
    assert.equal(stale.isLatest, false);
  });
});
