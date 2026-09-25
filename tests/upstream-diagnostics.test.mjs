import { describe, it } from "node:test";
import assert from "node:assert";

function captureConsoleError() {
  const original = console.error;
  const lines = [];
  console.error = (...args) => {
    lines.push(args.map((a) => (a instanceof Error ? a.message : String(a))).join(" "));
  };
  return {
    lines,
    restore() {
      console.error = original;
    },
  };
}

describe("upstream failure diagnostics", () => {
  it("ytdl melaporkan kode error upstream, bukan menyembunyikannya", async () => {
    const mod = await import("../src/scraper/ytdl.js");
    assert.strictEqual(
      typeof mod.describeConvertFailure,
      "function",
      "describeConvertFailure harus di-export agar pesan diagnosable",
    );
    const msg = mod.describeConvertFailure({ status: false, error: 128 });
    assert.ok(
      /128/.test(msg),
      `pesan harus menyebut kode upstream 128, dapat: ${msg}`,
    );
  });

  it("ytdl tetap memberi pesan dasar saat upstream tidak kirim kode error", async () => {
    const mod = await import("../src/scraper/ytdl.js");
    assert.strictEqual(typeof mod.describeConvertFailure, "function");
    const msg = mod.describeConvertFailure({});
    assert.ok(msg.length > 0, "pesan dasar tidak boleh kosong");
    assert.ok(!/undefined|NaN/.test(msg), `pesan tidak boleh berisi NaN/undefined: ${msg}`);
    assert.strictEqual(mod.describeConvertFailure(null).length > 0, true);
  });

  it("ytmp3 mencatat kegagalan API izuka, bukan menelan error di catch kosong", async () => {
    const mod = await import("../plugins/download/ytmp3.js");
    assert.strictEqual(
      typeof mod.getAudioDownload,
      "function",
      "getAudioDownload harus di-export agar bisa diuji",
    );

    const cap = captureConsoleError();
    try {
      await mod.getAudioDownload("https://youtu.be/abc", {
        httpGet: async () => {
          throw new Error("Request failed with status code 500");
        },
        ytdlFn: async () => ({ status: false, mess: "Gagal konversi (upstream error 128)" }),
      });
    } catch {
      /* lemma: lempar error adalah perilaku yang diharapkan */
    } finally {
      cap.restore();
    }

    const joined = cap.lines.join("\n");
    assert.ok(
      joined.includes("500"),
      `kegagalan izuka harus tercatat di log, dapat: ${joined || "(kosong)"}`,
    );
  });

  it("permintaan http yang gagal dicatat, bukan diam-diam jadi null", async () => {
    const { f } = await import("../src/lib/http.js");
    const cap = captureConsoleError();
    let result;
    try {
      result = await f("http://nx-tidak-ada-domain-ini.invalid/probe");
    } finally {
      cap.restore();
    }
    assert.strictEqual(result, null, "f() tetap harus mengembalikan null agar caller tidak pecah");
    const joined = cap.lines.join("\n");
    assert.ok(
      joined.includes("nx-tidak-ada-domain-ini.invalid"),
      `URL yang gagal harus masuk log, dapat: ${joined || "(kosong)"}`,
    );
  });
});
