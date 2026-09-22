import { describe, it } from "node:test";
import assert from "node:assert";
import { pickAlbumMedia } from "../plugins/sticker/sticker.js";

function entry({ id, fromMe = false, type = "imageMessage", ts }) {
  return {
    key: { fromMe, id },
    message: { [type]: {} },
    messageTimestamp: ts,
  };
}

function map(entries) {
  const m = new Map();
  for (const e of entries) m.set(e.key.id, e);
  return m;
}

const NOW = 1_000_000;

describe("pickAlbumMedia (anchor: pesan yg di-reply)", () => {
  it("ambil sibling media di sekitar anchor (dua arah), urut asc", () => {
    const r = pickAlbumMedia(
      map([
        entry({ id: "q", ts: NOW }),
        entry({ id: "b", ts: NOW + 2 }),
        entry({ id: "a", ts: NOW - 3 }),
        entry({ id: "far", ts: NOW + 60 }),
        entry({ id: "old", ts: NOW - 120 }),
      ]),
      { anchorTs: NOW, windowSec: 5, max: 10, mustInclude: ["q"] },
    );
    assert.deepStrictEqual(
      r.items.map((i) => i.id),
      ["a", "q", "b"],
    );
    assert.strictEqual(r.skipped, 0);
  });

  it("abaikan fromMe dan non-media", () => {
    const r = pickAlbumMedia(
      map([
        entry({ id: "own", fromMe: true, ts: NOW }),
        entry({ id: "text", type: "conversation", ts: NOW }),
        entry({ id: "audio", type: "audioMessage", ts: NOW }),
        entry({ id: "img", ts: NOW - 1 }),
      ]),
      { anchorTs: NOW, windowSec: 5, max: 10, mustInclude: [] },
    );
    assert.deepStrictEqual(
      r.items.map((i) => i.id),
      ["img"],
    );
  });

  it("cap max 10, hitung skipped", () => {
    const m = new Map();
    for (let i = 0; i < 13; i++) m.set(`id${i}`, entry({ id: `id${i}`, ts: NOW + i - 6 }));
    const r = pickAlbumMedia(m, { anchorTs: NOW, windowSec: 20, max: 10, mustInclude: [] });
    assert.strictEqual(r.items.length, 10);
    assert.strictEqual(r.skipped, 3);
  });

  it("quoted tua tetap masuk via mustInclude, sibling-nya tak", () => {
    const r = pickAlbumMedia(
      map([
        entry({ id: "oldq", ts: NOW - 3600 }),
        entry({ id: "old2", ts: NOW - 3599 }),
        entry({ id: "x", ts: NOW - 3600 }),
      ]),
      { anchorTs: NOW, windowSec: 5, max: 10, mustInclude: ["oldq"] },
    );
    assert.deepStrictEqual(r.items.map((i) => i.id), ["oldq"]);
  });

  it("kosong tanpa media", () => {
    const r = pickAlbumMedia(new Map(), { anchorTs: NOW, windowSec: 5, max: 10, mustInclude: [] });
    assert.deepStrictEqual(r.items, []);
    assert.strictEqual(r.skipped, 0);
  });
});