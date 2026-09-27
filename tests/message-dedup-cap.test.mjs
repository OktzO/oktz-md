import { describe, it } from "node:test";
import assert from "node:assert";
import { readFile } from "node:fs/promises";
import NodeCache from "node-cache";
import { createMessageDedup } from "../src/connection.js";

const CONNECTION = new URL("../src/connection.js", import.meta.url).pathname;

/* ------------------------------------------------------------------ */
/* C2 — node-cache maxKeys melempar, bukan evict                        */
/* ------------------------------------------------------------------ */

describe("node-cache v5 set() melempar ECACHEFULL, bukan evict", () => {
  it("membuktikan behavior library apa adanya (alasan fix dibutuhkan)", () => {
    const c = new NodeCache({ stdTTL: 30, useClones: false, maxKeys: 3 });
    c.set("a", true);
    c.set("b", true);
    c.set("c", true);
    assert.throws(() => c.set("d", true), /ECACHEFULL/);
    // bahkan set() ulang key yang sudah ada melempar saat cache penuh
    assert.throws(() => c.set("a", true), /ECACHEFULL/);
  });
});

describe("createMessageDedup — set() dijaga, evict oldest deterministik", () => {
  const make = (maxKeys = 5) =>
    createMessageDedup(
      new NodeCache({ stdTTL: 30, useClones: false, maxKeys }),
      maxKeys,
    );

  it("tidak melempar saat maxKeys terlampaui (bug asli: abort listener)", () => {
    const dedup = make(5);
    for (let i = 0; i < 500; i++) {
      assert.doesNotThrow(() => dedup.set(`msg-${i}`, true), `set #${i}`);
    }
  });

  it("tetap di bawah maxKeys setelah luap normal", () => {
    const dedup = make(5);
    for (let i = 0; i < 200; i++) dedup.set(`msg-${i}`, true);
    assert.ok(
      dedup.size <= 5,
      `cache tidak boleh melebihi cap (size=${dedup.size})`,
    );
  });

  it("tetap di bawah maxKeys walau set() dipanggil bersamaan (synchronous burst)", () => {
    const dedup = make(8);
    const results = [];
    for (let i = 0; i < 64; i++) {
      try {
        dedup.set(`b-${i}`, true);
        results.push(true);
      } catch (e) {
        results.push(e);
      }
    }
    assert.ok(
      results.every((r) => r === true),
      "tidak boleh ada set() yang melempar",
    );
    assert.ok(dedup.size <= 8, `size=${dedup.size}`);
  });

  it("masih menandai pesan yang baru sebagai duplikat (dedup berfungsi)", () => {
    const dedup = make(5);
    dedup.set("recent", true);
    assert.equal(dedup.has("recent"), true);
    assert.equal(dedup.has("never-seen"), false);
  });

  it("membuang entri terlama lebih dulu, bukan entri terbaru", () => {
    const dedup = make(3);
    dedup.set("oldest", true);
    dedup.set("mid", true);
    dedup.set("newest", true);
    dedup.set("overflow", true);

    assert.equal(dedup.has("newest"), true, "entri terbaru dipertahankan");
    assert.equal(dedup.has("overflow"), true, "entri yang baru masuk dipertahankan");
    assert.equal(dedup.has("oldest"), false, "entri terlama yang dibuang");
  });

  it("membersihkan cache + antrean FIFO sekaligus", () => {
    const dedup = make(3);
    for (let i = 0; i < 10; i++) dedup.set(`k-${i}`, true);
    dedup.clear();
    assert.equal(dedup.size, 0);
    assert.equal(dedup.has("k-9"), false);
  });
});

/* ------------------------------------------------------------------ */
/* titik pemanggilan wajib dijaga, bukan mengandalkan try/catch di luar  */
/* ------------------------------------------------------------------ */

describe("messages.upsert — processedMessages.set() tidak lagi telanjang", () => {
  it("call site memakai wrapper createMessageDedup, bukan cache mentah", async () => {
    const src = await readFile(CONNECTION, "utf8");

    // deklarasi cache mentah harus hilang
    assert.doesNotMatch(
      src,
      /processedMessages\.set\(/,
      "processedMessages.set() mentah masih ada — akan melempar ECACHEFULL",
    );

    // wrapper harus dipakai tepat di tempat dedup lama dipakai
    assert.match(
      src,
      /messageDedup\.has\(msgId\)[\s\S]{0,80}messageDedup\.set\(msgId/,
      "loop pesan harus memakai messageDedup.has/set",
    );
  });
});
