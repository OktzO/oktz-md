// Test logika versi rilis launcher (node:test, tanpa dependensi).
// Import launcher.js aman: efek samping (unduh binary, spawn bot) hanya
// jalan kalau file itu jadi entry point.
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  ambilAset,
  ambilVersiTerbaru,
  tagDariUrl,
  urlAset,
  versiCocok,
} from "./launcher.js";

const AKAR = path.dirname(fileURLToPath(import.meta.url));

/** fetch palsu: rekam URL dan balas sesuai `respons` (objek atau fungsi). */
function fetchPalsu(respons) {
  const panggil = [];
  const fn = async (url) => {
    panggil.push(url);
    const r = typeof respons === "function" ? respons(url, panggil.length) : respons;
    return {
      ok: r.ok ?? true,
      status: r.status ?? 200,
      url: r.url ?? url,
      arrayBuffer: async () => r.data ?? new Uint8Array([1, 2, 3]).buffer,
    };
  };
  fn.panggil = panggil;
  return fn;
}

test("tagDariUrl ambil tag dari redirect releases/latest", () => {
  assert.equal(
    tagDariUrl("https://github.com/OktzO/oktz-rs/releases/tag/v0.1.8"),
    "v0.1.8"
  );
});

test("tagDariUrl ambil tag dari URL download aset", () => {
  assert.equal(
    tagDariUrl("https://github.com/OktzO/oktz-rs/releases/download/v0.1.8/oktz-linux-x64"),
    "v0.1.8"
  );
});

test("tagDariUrl null kalau URL tanpa tag", () => {
  assert.equal(tagDariUrl("https://github.com/OktzO/oktz-rs/releases/latest"), null);
  assert.equal(tagDariUrl(""), null);
});

test("urlAset pakai URL per-tag", () => {
  assert.equal(
    urlAset("v0.1.8", "oktz-linux-x64"),
    "https://github.com/OktzO/oktz-rs/releases/download/v0.1.8/oktz-linux-x64"
  );
});

test("urlAset tanpa tag pakai URL rilis terbaru", () => {
  assert.equal(
    urlAset(null, "oktz-linux-x64"),
    "https://github.com/OktzO/oktz-rs/releases/latest/download/oktz-linux-x64"
  );
});

test("ambilVersiTerbaru baca tag dari redirect", async () => {
  const f = fetchPalsu({ url: "https://github.com/OktzO/oktz-rs/releases/tag/v0.1.8" });
  assert.equal(await ambilVersiTerbaru(f), "v0.1.8");
  assert.equal(f.panggil[0], "https://github.com/OktzO/oktz-rs/releases/latest");
});

test("ambilVersiTerbaru null kalau jaringan gagal", async () => {
  const f = async () => {
    throw new Error("offline");
  };
  assert.equal(await ambilVersiTerbaru(f), null);
});

test("ambilVersiTerbaru null kalau repo belum punya rilis", async () => {
  const f = fetchPalsu({ ok: false, status: 404 });
  assert.equal(await ambilVersiTerbaru(f), null);
});

test("ambilAset unduh aset per-tag dan laporkan tag", async () => {
  const f = fetchPalsu({ data: new Uint8Array([7, 8, 9]).buffer });
  const r = await ambilAset("oktz-linux-x64-static", "v0.1.8", f);
  assert.equal(r.ok, true);
  assert.equal(r.tag, "v0.1.8");
  assert.deepEqual([...r.data], [7, 8, 9]);
  assert.equal(
    f.panggil[0],
    "https://github.com/OktzO/oktz-rs/releases/download/v0.1.8/oktz-linux-x64-static"
  );
});

test("ambilAset laporkan gagal tanpa exception saat 404", async () => {
  const f = fetchPalsu({ ok: false, status: 404 });
  const r = await ambilAset("oktz-linux-x64-static", "v0.1.8", f);
  assert.equal(r.ok, false);
  assert.equal(r.data, null);
});

test("ambilAset fallback ke aset glibc kalau aset statis hilang", async () => {
  const f = fetchPalsu((url) =>
    url.includes("-static")
      ? { ok: false, status: 404 }
      : { data: new Uint8Array([1]).buffer }
  );
  const r = await ambilAset("oktz-linux-x64-static", "v0.1.8", f);
  assert.equal(r.ok, true);
  assert.equal(r.nama, "oktz-linux-x64");
  assert.equal(
    f.panggil[1],
    "https://github.com/OktzO/oktz-rs/releases/download/v0.1.8/oktz-linux-x64"
  );
});

test("versiCocok bandingkan penanda versi", () => {
  assert.equal(versiCocok("v0.1.8\n", "v0.1.8"), true);
  assert.equal(versiCocok("v0.1.7", "v0.1.8"), false);
});

test("fallback rilis launcher menunjuk v0.1.9", async () => {
  const modul = await import("./launcher.js");
  assert.equal(modul.VERSI_BIN, "v0.1.9");
});

test("import launcher tanpa entry point tidak menjalankan bot", () => {
  const launcher = pathToFileURL(path.join(AKAR, "launcher.js")).href;
  const kode =
    `await import(${JSON.stringify(launcher)}); ` +
    `console.log(JSON.stringify({sigint: process.listenerCount("SIGINT"), ` +
    `sigterm: process.listenerCount("SIGTERM")}))`;
  const hasil = spawnSync(process.execPath, ["--input-type=module", "-e", kode], {
    cwd: AKAR,
    encoding: "utf8",
    timeout: 5_000,
  });
  assert.equal(hasil.status, 0, hasil.stderr);
  assert.deepEqual(JSON.parse(hasil.stdout), { sigint: 0, sigterm: 0 });
  assert.doesNotMatch(hasil.stderr, /\[launcher\]/);
});
