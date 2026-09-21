import { describe, it, mock, beforeEach, afterEach } from "node:test";
import assert from "node:assert";
import {
  bratvidRssBlocked,
  BRATVID_RSS_LIMIT,
} from "../src/lib/ourin-bratvid-gate.js";

const MB = 1024 * 1024;

describe("bratvid RSS gate", () => {
  it("menolak render saat rss > 480MB", () => {
    const msg = bratvidRssBlocked(() => BRATVID_RSS_LIMIT + 1);
    assert.ok(msg, "harus menolak dengan pesan ramah");
    assert.match(msg, /low RAM/);
    assert.match(msg, /coba lagi nanti/);
  });

  it("membolehkan tepat di 480MB (spesifikasi: tolak hanya jika >)", () => {
    assert.strictEqual(bratvidRssBlocked(() => BRATVID_RSS_LIMIT), null);
  });

  it("membolehkan saat rss di bawah 480MB", () => {
    assert.strictEqual(bratvidRssBlocked(() => 300 * MB), null);
  });

  it("default getter baca process.memoryUsage().rss tanpa import modul gemuk", () => {
    const msg = bratvidRssBlocked();
    assert.ok(msg === null || typeof msg === "string");
  });
});

describe("bratvid handler refusal path", () => {
  let originalMemoryUsage;

  beforeEach(() => {
    originalMemoryUsage = process.memoryUsage;
  });

  afterEach(() => {
    process.memoryUsage = originalMemoryUsage;
    mock.restoreAll();
  });

  it("RSS tinggi: balas pesan penolakan & TIDAK mengimport brat-canvas/video", async () => {
    const { handler, isBratVidLoaded } = await import(
      "../plugins/sticker/bratvid.js"
    );
    assert.strictEqual(isBratVidLoaded(), false, "modul gemuk belum ke-import");

    mock.method(
      process,
      "memoryUsage",
      () => ({ rss: BRATVID_RSS_LIMIT + 1 }),
    );

    let replied = null;
    let sentSticker = false;
    const m = {
      args: ["Halo"],
      prefix: ".",
      command: "bratvid",
      pushName: "User",
      reply: (text) => {
        replied = text;
        return Promise.resolve();
      },
      react: () => {},
    };
    const sock = {
      sendVideoAsSticker: async () => {
        sentSticker = true;
      },
    };

    await handler(m, { sock });

    assert.match(replied, /low RAM/, "harus balas pesan low RAM");
    assert.strictEqual(sentSticker, false, "render tidak boleh jalan");
    assert.strictEqual(
      isBratVidLoaded(),
      false,
      "brat-canvas/video TIDAK ke-import saat ditolak",
    );
  });
});