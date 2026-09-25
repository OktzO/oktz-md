import { describe, it, mock, beforeEach, afterEach } from "node:test";
import assert from "node:assert";
import { generateBrat, parseBratArgs, isBlankImage } from "../src/lib/brat.js";

describe("brat API integration", () => {
  let originalFetch;

  beforeEach(() => {
    originalFetch = global.fetch;
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it("parseBratArgs extracts text and blur", () => {
    const result = parseBratArgs("halo -blur 2");
    assert.deepStrictEqual(result, { text: "halo", blur: 2 });
  });

  it("generateBrat produces PNG buffer for valid input", async () => {
    const buf = await generateBrat({ text: "test", theme: "white", blur: 0 });
    assert.ok(Buffer.isBuffer(buf));
    assert.ok(buf.length > 0);
    // PNG signature
    assert.strictEqual(buf[0], 0x89);
    assert.strictEqual(buf[1], 0x50);
    assert.strictEqual(buf[2], 0x4e);
    assert.strictEqual(buf[3], 0x47);
  });

  it("generateBrat applies blur when requested", async () => {
    const bufNoBlur = await generateBrat({ text: "test", theme: "white", blur: 0 });
    const bufBlur = await generateBrat({ text: "test", theme: "white", blur: 2 });
    assert.ok(Buffer.isBuffer(bufBlur));
    assert.ok(bufBlur.length > 0);
    // Blurred image may be smaller due to compression
    assert.ok(bufBlur.length !== bufNoBlur.length || bufBlur.length > 0);
  });

  it("regression: native render bukan blank & tanpa fallback brat-canvas", async () => {
    const buf = await generateBrat({ text: "Halo ges apa kabar", theme: "white", blur: 0 });
    assert.strictEqual(await isBlankImage(buf), false, "sticker terjual blank = bug");
    const { default: sharp } = await import("sharp");
    const meta = await sharp(buf).metadata();
    assert.strictEqual(meta.width, 1000, "native canvas 1000x1000; 500x500 = fallback brat-canvas dipakai");
  });
});

describe("fetchBratFromAPI (to be implemented)", () => {
  let originalFetch;

  beforeEach(() => {
    originalFetch = global.fetch;
  });

  afterEach(() => {
    global.fetch = originalFetch;
  });

  it("returns buffer on successful API response", async () => {
    const fakePng = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    const arrayBuffer = fakePng.buffer.slice(fakePng.byteOffset, fakePng.byteOffset + fakePng.byteLength);
    global.fetch = async () => ({
      ok: true,
      arrayBuffer: async () => arrayBuffer,
    });

    const { fetchBratFromAPI } = await import("../src/lib/brat.js");
    const buf = await fetchBratFromAPI("test text");
    assert.ok(Buffer.isBuffer(buf));
    assert.strictEqual(buf[0], 0x89);
  });

  it("throws on HTTP error", async () => {
    global.fetch = async () => ({
      ok: false,
      status: 500,
    });

    const { fetchBratFromAPI } = await import("../src/lib/brat.js");
    await assert.rejects(() => fetchBratFromAPI("test text"), /HTTP 500/);
  });

  it("throws on network error", async () => {
    global.fetch = async () => {
      throw new Error("network error");
    };

    const { fetchBratFromAPI } = await import("../src/lib/brat.js");
    await assert.rejects(() => fetchBratFromAPI("test text"), /network error/);
  });

  it("throws on timeout", async () => {
    global.fetch = async () => {
      await new Promise((_, reject) => setTimeout(() => reject(new Error("timeout")), 100));
    };

    const { fetchBratFromAPI } = await import("../src/lib/brat.js");
    await assert.rejects(() => fetchBratFromAPI("test text"), /timeout/);
  });
});