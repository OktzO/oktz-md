// C1 + I10 regression (audit RAM safety 2026-09-21):
//  - emojiImageCache bratvid2 di-cap 128 + FIFO evict.
//  - emoji-apple.json parse sekali per proses (shared loader), objek yang sama
//    dibagi brat & bratvid2 (lama: dua objek 27MB module singleton).
import { test } from "node:test";
import assert from "node:assert/strict";
import { loadEmojiMap } from "../src/lib/emoji-map.js";
import { loadBratEmojiMap } from "../src/lib/brat.js";
import { loadEmojiMap as vidLoadEmojiMap, getEmojiImage, emojiImageCache } from "../plugins/sticker/bratvid2.js";

// Ambil satu emoji nyata dari map (key hex -> emoji) biar tak nembak key ajaib.
function emojiFromMapKey(map) {
  const key =
    Object.keys(map).find((k) => /^[0-9a-f]+(?:-[0-9a-f]+)+$/i.test(k)) ||
    Object.keys(map)[0];
  assert.ok(key, "map berisi key emoji");
  return key
    .split("-")
    .map((h) => String.fromCodePoint(parseInt(h, 16)))
    .join("");
}

test("emoji-apple.json: satu parse, satu objek dibagi brat & bratvid2", async () => {
  const shared = await loadEmojiMap();
  const viaBrat = await loadBratEmojiMap();
  const viaVid2 = await vidLoadEmojiMap();

  assert.strictEqual(viaBrat, shared, "brat harus dapat objek yang sama");
  assert.strictEqual(viaVid2, shared, "bratvid2 harus dapat objek yang sama");
  assert.ok(Object.keys(shared).length > 1000, "map berisi Apple emoji");
});

test("emoji-apple.json: JSON.parse jalan sekali per proses", async () => {
  const mod = await import("../src/lib/emoji-map.js");
  await loadEmojiMap();
  assert.strictEqual(
    mod.emojiMapParseCount,
    1,
    "JSON.parse emoji-apple.json harus sekali (shared loader)",
  );
});

test("bratvid2 emojiImageCache di-cap 128 dengan FIFO evict", async () => {
  emojiImageCache.clear();
  for (let i = 0; i < 128; i++) emojiImageCache.set(`fake-key-${i}`, { stub: true });
  assert.equal(emojiImageCache.size, 128);
  const oldest = emojiImageCache.keys().next().value;

  const map = await loadEmojiMap();
  const img = await getEmojiImage(emojiFromMapKey(map));
  assert.ok(img, "emoji nyata harus resolve jadi image");
  assert.equal(emojiImageCache.size, 128, "cache tidak boleh melebihi 128");
  assert.ok(emojiImageCache.has(emojiFromMapKey(map)), "entri terbaru ada di cache");
  assert.ok(!emojiImageCache.has(oldest), "FIFO: entri tertua harus ter-evict");
  emojiImageCache.clear();
});