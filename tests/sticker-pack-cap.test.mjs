import { describe, it } from "node:test";
import assert from "node:assert";
import { readFile } from "node:fs/promises";
import {
  createStickerPackCache,
  packSizeBytes,
  STICKER_PACK_CAP_BYTES,
} from "../src/lib/socket.js";

const CONNECTION = new URL("../src/connection.js", import.meta.url).pathname;
const SOCKET = new URL("../src/lib/socket.js", import.meta.url).pathname;
const MB = 1024 * 1024;

const pack = (bytes) => ({
  name: "p",
  message: {
    stickerPackMessage: {
      stickers: [{ sticker: Buffer.alloc(bytes) }],
    },
  },
});

/* ------------------------------------------------------------------ */
/* C1a — cap harus sesuai plafon yang terdokumentasi                   */
/* ------------------------------------------------------------------ */

describe("STICKER_PACK_CAP_BYTES mengikuti plafon RSS yang terdokumentasi", () => {
  it("cap <= 32MB, bukan 200MB", () => {
    // src/lib/profiler.js dokumen: baseline RSS idle 290-375MB, kontainer
    // 1GB, --max-old-space-size=512. Buffer sticker itu ArrayBuffer di luar
    // heap, jadi --max-old-space-size tidak mengaturnya sama sekali.
    // 200MB di atas baseline 375MB = 575MB idle, dan cap 200MB persis
    // sebesar satu pack besar -> pack tunggal jadi tak terevict.
    assert.ok(
      STICKER_PACK_CAP_BYTES <= 32 * MB,
      `cap ${STICKER_PACK_CAP_BYTES / MB}MB jauh di atas plafon 32MB`,
    );
  });

  it("cap default pada createStickerPackCache() benar-benar yang baru", () => {
    const cache = createStickerPackCache();
    cache.set("a", pack(STICKER_PACK_CAP_BYTES + MB));
    assert.equal(
      cache.size,
      0,
      "pack yang melebihi cap harus ditolak, bukan disimpan",
    );
  });
});

/* ------------------------------------------------------------------ */
/* C1b — evict harus benar-benar mengembalikan ke bawah cap            */
/* ------------------------------------------------------------------ */

describe("evict mengembalikan total ke bawah cap, termasuk pack tunggal", () => {
  it("pack tunggal yang melebihi cap dibuang (dulu map.size > 1 mengunci)", () => {
    const cache = createStickerPackCache(200);
    cache.set("huge", pack(500));
    assert.equal(cache.size, 0, "pack tunggal oversized harus bisa dibuang");
    assert.equal(cache.has("huge"), false);
    assert.ok(cache.totalBytes <= 200);
  });

  it("cap tetap terjaga saat masuknya entri besar sendirian", () => {
    const cache = createStickerPackCache(1000);
    cache.set("big", pack(4000));
    assert.ok(cache.totalBytes <= 1000, `totalBytes=${cache.totalBytes}`);
  });

  it("entri yang masih muat dipertahankan, terlama yang dibuang", () => {
    const cache = createStickerPackCache(700);
    cache.set("a", pack(600));
    assert.ok(cache.get("a"), "600 <= 700, belum ada yang dibuang");
    cache.set("b", pack(300));
    assert.equal(cache.get("a"), undefined, "600+300 > 700, terlama dibuang");
    assert.ok(cache.get("b"), "entri yang muat dipertahankan");
    assert.ok(cache.totalBytes <= 700);
  });

  it("totalBytes tidak pernah negatif setelah evict beruntun", () => {
    const cache = createStickerPackCache(300);
    for (let i = 0; i < 50; i++) cache.set(`k-${i}`, pack(200));
    assert.ok(cache.totalBytes >= 0, `totalBytes=${cache.totalBytes}`);
    assert.ok(cache.totalBytes <= 300);
  });

  it("pack yang ukurannya PERSIS sama dengan cap tetap memaksa evict yang lain", () => {
    // regression: batas iterasi evict yang ikut menyusut (dibandingkan
    // dengan map.size) menghentikan loop di tengah jalan, jadi total
    // tertinggal jauh di atas cap.
    const cache = createStickerPackCache(200);
    for (let i = 0; i < 20; i++) cache.set(`p${i}`, pack(10));
    assert.equal(cache.size, 20);

    cache.set("huge", pack(200));
    assert.ok(
      cache.totalBytes <= 200,
      `total=${cache.totalBytes}, n=${cache.size} -- evict tidak tuntas`,
    );
    assert.equal(cache.size, 1, "hanya pack sebesar cap yang boleh tersisa");
    assert.ok(cache.get("huge"), "pack sebesar cap memang muat");
  });

  it("evict tuntas walau ada banyak entri (guard tidak ikut menyusut)", () => {
    const cache = createStickerPackCache(1000);
    for (let i = 0; i < 100; i++) cache.set(`k${i}`, pack(100));
    assert.equal(cache.size, 10, "1000/100 = 10 entri tepat");
    assert.equal(cache.totalBytes, 1000);
  });
});

/* ------------------------------------------------------------------ */
/* C1c — cache harus dibersihkan di disconnect dan logout             */
/* ------------------------------------------------------------------ */

describe("stickerPackCache dibersihkan saat disconnect dan logout", () => {
  it("disconnect (connection 'close') memanggil clearStickerPackCache()", async () => {
    const src = await readFile(CONNECTION, "utf8");
    const closeIdx = src.indexOf('if (c === S.C) {');
    assert.ok(closeIdx > -1, "blok connection close tidak ditemukan");
    // ambil isi blok S.C sampai S.O
    const openIdx = src.indexOf("if (c === S.O)", closeIdx);
    const block = src.slice(closeIdx, openIdx);
    assert.match(
      block,
      /clearStickerPackCache\(\)/,
      "blok close harus membersihkan cache sticker",
    );
  });

  it("logout() juga memanggil clearStickerPackCache()", async () => {
    const src = await readFile(CONNECTION, "utf8");
    const logoutIdx = src.indexOf("async function logout()");
    assert.ok(logoutIdx > -1, "logout() tidak ditemukan");
    const block = src.slice(logoutIdx, src.indexOf("\n}\n", logoutIdx));
    assert.match(block, /clearStickerPackCache\(\)/);
  });

  it("clearStickerPackCache() menolakkan cache global dan tidak throw", () => {
    return import("../src/lib/socket.js").then(({ clearStickerPackCache }) => {
      global.stickerPackCache = createStickerPackCache(1000);
      global.stickerPackCache.set("a", pack(100));
      assert.equal(global.stickerPackCache.size, 1);
      clearStickerPackCache();
      assert.equal(global.stickerPackCache?.size ?? 0, 0);
      // idempoten: aman dipanggil dua kali / saat cache belum ada
      assert.doesNotThrow(() => clearStickerPackCache());
      global.stickerPackCache = undefined;
    });
  });

  it("extendSocket mengekspos clearStickerPackCache di socket", async () => {
    const sock = await import("../src/lib/socket.js").then((m) =>
      m.extendSocket({
        sendMessage: async () => ({}),
        profilePictureUrl: async () => null,
        user: { id: "6281:3" },
      }),
    );
    assert.equal(typeof sock.clearStickerPackCache, "function");
    sock.saveStickerPack("p9", { stickerPackMessage: { stickers: [] } }, "x");
    assert.ok(global.stickerPackCache.has("p9"));
    sock.clearStickerPackCache();
    assert.equal(global.stickerPackCache.size, 0);
  });
});

/* ------------------------------------------------------------------ */
/* C1d — konstanta tidak boleh hidup di file yang tidak boleh diubah   */
/* ------------------------------------------------------------------ */

describe("cap tidak di-hardcode ulang di tempat lain", () => {
  it("socket.js hanya menyebut cap lewat konstantanya", async () => {
    const src = await readFile(SOCKET, "utf8");
    const occurrences = src.match(/200\s*\*\s*1024\s*\*\s*1024/g) || [];
    assert.equal(
      occurrences.length,
      0,
      "literal 200MB masih ada di socket.js",
    );
  });
});
