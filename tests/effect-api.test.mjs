import { describe, it } from "node:test";
import assert from "node:assert";

// Tujuh plugin to*.js butuh API efek gambar (ganti background) yang menerima
// URL. dippedrotekan 2026-09-27: api-faa.my.id/faa/* membalas 403 dari mesin
// ini karena Cloudflare WAF memblokir per IP klien. Karena tiap efek cuma punya
// SATU host, satu WAF = 7 command mati tanpappekbadian.
//
// Perhatikan: WAF memblokir per IP, jadi dari VPS owner host ini bisa jadi
// tetap jalan. Karena itu daftar provider harus bisa dikonfigurasi, bukan
// dipindah permanen.
describe("applyImageEffect", () => {
  const IMG = "https://litter.catbox.moe/x.jpg";
// header PNG asli + padding, guard butuh >= 12 byte
const PNG_BYTES = Buffer.from([0x89,0x50,0x4e,0x47,0x0d,0x0a,0x1a,0x0a,0,0,0,0x0d]);

  it("punya injeksi provider supaya bisa diuji tanpa network", async () => {
    const { applyImageEffect } = await import("../src/lib/effect-api.js");
    assert.equal(typeof applyImageEffect, "function");
  });

  it("pakai provider pertama yang berhasil", async () => {
    const { applyImageEffect } = await import("../src/lib/effect-api.js");
    const png = PNG_BYTES;
    const url = await applyImageEffect("tomekah", IMG, {
      providers: [
        { name: "primary", base: "https://a.test", run: async () => png },
      ],
    });
    assert.ok(Buffer.isBuffer(url));
    assert.equal(url.length, PNG_BYTES.length);
  });

  it("pindah ke provider berikutnya saat yang pertama gagal", async () => {
    const { applyImageEffect } = await import("../src/lib/effect-api.js");
    const png = PNG_BYTES;
    const tried = [];
    const buf = await applyImageEffect("tomekah", IMG, {
      providers: [
        { name: " waf", base: "https://a.test", run: async () => { tried.push("waf"); throw Object.assign(new Error("forbidden"), { status: 403 }); } },
        { name: "cadangan", base: "https://b.test", run: async () => { tried.push("cadangan"); return png; } },
      ],
    });
    assert.equal(buf.length, PNG_BYTES.length);
    assert.deepEqual(tried, ["waf", "cadangan"]);
  });

  it("HTML dianggap gagal dan provider berikutnya yang dipakai", async () => {
    const { applyImageEffect } = await import("../src/lib/effect-api.js");
    const seen = [];
    const out = await applyImageEffect("tomekah", IMG, {
      providers: [
        { name: "html", base: "https://a.test", run: async () => { seen.push("html"); return Buffer.from("<!DOCTYPE html><html>"); } },
        { name: "png", base: "https://b.test", run: async () => { seen.push("png"); return PNG_BYTES; } },
      ],
    });
    assert.deepEqual(seen, ["html", "png"]);
    assert.equal(out.length, PNG_BYTES.length, "hasil harus PNG, bukan HTML");
  });

  it("error akhir menyebut tiap provider dan statusnya", async () => {
    const { applyImageEffect } = await import("../src/lib/effect-api.js");
    await assert.rejects(
      () => applyImageEffect("tomekah", IMG, {
        providers: [
          { name: "faa", base: "https://a.test", run: async () => { throw Object.assign(new Error("forbidden"), { status: 403 }); } },
          { name: "vynaa", base: "https://b.test", run: async () => { throw Object.assign(new Error("not found"), { status: 404 }); } },
        ],
      }),
      (err) => {
        assert.match(err.message, /faa/, "harus menyebut faa");
        assert.match(err.message, /403/, "harus menyebut status 403");
        assert.match(err.message, /vynaa/);
        assert.match(err.message, /404/);
        return true;
      },
    );
  });

  it("menandai 403 sebagai blokir WAF, bukan host mati", async () => {
    const { describeProviderFailure } = await import("../src/lib/effect-api.js");
    assert.match(describeProviderFailure({ status: 403 }), /WAF|blokir/i);
    assert.match(describeProviderFailure({ status: 404 }), /tidak ada|endpoint/i);
    assert.match(describeProviderFailure({ status: 429 }), /limit/i);
  });

  // f() di src/lib/http.js me-return null untuk SEMUA error dan membuang
  // status code, sehingga WAF 403 pernah dilaporkan sebagai "502 server error".
  // Diagnosis jadi mustahil. runEffectWA harus membaca status sebenarnya.
  it("menyreporting status HTTP yang sebenarnya, bukan menebak", async () => {
    const { runEffectWA } = await import("../src/lib/effect-api.js");

    await assert.rejects(
      () => runEffectWA(
        { base: "https://a.test" },
        "tomekah",
        "https://img.test/x.jpg",
        { fetchImpl: async () => ({ ok: false, status: 403, arrayBuffer: async () => new ArrayBuffer(0) }) },
      ),
      (e) => {
        assert.equal(e.status, 403, "status asli harus diteruskan");
        return true;
      },
    );

    await assert.rejects(
      () => runEffectWA(
        { base: "https://a.test" },
        "tomekah",
        "https://img.test/x.jpg",
        { fetchImpl: async () => ({ ok: false, status: 404, arrayBuffer: async () => new ArrayBuffer(0) }) },
      ),
      (e) => {
        assert.equal(e.status, 404);
        return true;
      },
    );
  });

  it("menolak body HTML walau status 200", async () => {
    const { runEffectWA } = await import("../src/lib/effect-api.js");
    const html = Buffer.from("<!DOCTYPE html><html><body>halo</body></html>".padEnd(64, " "));
    await assert.rejects(
      () => runEffectWA(
        { base: "https://a.test" },
        "tomekah",
        "https://img.test/x.jpg",
        { fetchImpl: async () => ({ ok: true, status: 200, arrayBuffer: async () => html }) },
      ),
      /HTML|bukan gambar/i,
    );
  });

  it("mengembalikan buffer gambar saat sukses", async () => {
    const { runEffectWA } = await import("../src/lib/effect-api.js");
    const png = PNG_BYTES;
    const out = await runEffectWA(
      { base: "https://a.test" },
      "tomekah",
      "https://img.test/x.jpg",
      { fetchImpl: async () => ({ ok: true, status: 200, arrayBuffer: async () => png.buffer.slice(png.byteOffset, png.byteOffset + png.byteLength) }) },
    );
    assert.equal(out.length, PNG_BYTES.length);
  });

  it("daftar default tidak memuat host yang DNS-nya mati", async () => {
    const { DEFAULT_EFFECT_PROVIDERS } = await import("../src/lib/effect-api.js");
    for (const p of DEFAULT_EFFECT_PROVIDERS) {
      assert.ok(
        !/vynaa\.web\.id/.test(p.base),
        "vynaa.web.id ENOTFOUND (DNS mati) jangan jadi default",
      );
    }
  });

  it("memakai daftar provider default yang bisa dikonfigurasi", async () => {
    const { DEFAULT_EFFECT_PROVIDERS } = await import("../src/lib/effect-api.js");
    assert.ok(Array.isArray(DEFAULT_EFFECT_PROVIDERS));
    assert.ok(DEFAULT_EFFECT_PROVIDERS.length >= 1);
    for (const p of DEFAULT_EFFECT_PROVIDERS) {
      assert.ok(p.name && p.base, "provider harus punya name + base");
    }
  });
});
