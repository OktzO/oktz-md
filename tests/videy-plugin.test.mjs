import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/download/videy.js` sungguhan. Uji
// lapis plugin dipisah dari tests/capabilities-phase1.test.mjs karena mengimpor
// plugin ini menuntut mock `src/lib/http.js`, dan satu modul tidak boleh
// di-mock dua kali dalam satu berkas.

const PANGGILAN = [];
const VIDEO = "https://cdn.videy.co/7ZH1ZRIF.mp4";
const LINK = "https://videy.co/v?id=7ZH1ZRIF";

let cdnVidey = () => {
  throw new Error("cdn.videy.co tidak boleh dipanggil: upstream mati");
};
let keyNeoxr = "k-neoxr-untuk-test";

const httpPalsu = {
  async get(url, opts) {
    PANGGILAN.push({ verb: "get", url, opts });
    // Dicocokkan ke host: URL aggregator untuk kapabilitas ini memuat "videy.co"
    // di query string-nya, jadi pencocokan substring salah arah.
    const host = new URL(String(url)).hostname;
    if (host === "cdn.videy.co") {
      return cdnVidey(url, opts);
    }
    if (host === "api.neoxr.eu") {
      // Bentuk yang dibaca plugin sebelum Phase 1 (`res.data.status` lalu
      // `res.data.data.url`): satu lapis `data` saja di dalam body neoxr.
      return {
        status: 200,
        data: { status: true, data: { url: VIDEO } },
      };
    }
    throw new Error(`host tak terduga di test ini: ${host}`);
  },
  async post(url, body, config) {
    PANGGILAN.push({ verb: "post", url, config });
    throw new Error("POST tidak boleh dipakai di test ini");
  },
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu } });

const config = (await import("../config.js")).default;
const neoxrAsli = config.APIkey.neoxr;
config.APIkey.neoxr = keyNeoxr;

// ── sentinel import ───────────────────────────────────────────────────────────
//
// Blok ini yang membuat berkas ini tidak bisa hijau tanpa isi.
//
// Berkas ini mengimpor `../config.js` (untuk menyematkan key neoxr) SEBELUM
// mengimpor plugin, dan rantai impor itu config.js → database.js → logger.js →
// lid.js. `src/lib/lid.js:61` memasang `process.on("uncaughtException", …)` yang
// menelan semua error, termasuk `ERR_MODULE_NOT_FOUND`. Akibatnya plugin yang
// gagal diimpor tidak menggagalkan berkas: proses keluar 0 dan node:test
// melaporkan `pass 1` dengan nol test di dalamnya.
//
// Karena itu impor plugin TIDAK boleh dibiarkan gagal telanjang di top-level.
// Kalau `await import` di situ melempar, modul berkas ini tidak pernah selesai
// dievaluasi dan `test()` di bawah tidak pernah terdaftar — sentinel yang
// diletakkan setelah impor (bentuk yang paling wajar) justru tidak pernah ikut
// jalan. Kesalahannya ditangkap, lalu assertion-nya yang menggagalkan.
//
// PERBAIKAN YANG BENAR ADA DI PRODUKSI, BUKAN DI SINI: modul pustaka tidak
// boleh memasang penelan `uncaughtException` saat diimpor. `index.js:411` sudah
// punya `setupAntiCrash` yang menghitung kegagalan, jadi handler di lid.js
// perlu ditinjau di sana — lihat task triase terpisah. Yang di sini hanya
// memastikan berkas ini tidak bisa hijau tanpa isi.
//
// tests/douyindl-plugin.test.mjs tidak kena karena tidak pernah mengimpor
// `config.js`, jadi ia gagal keras dan blok ini tidak dibutuhkannya.
let videy = null;
let galatImporPlugin = null;
try {
  videy = await import("../plugins/download/videy.js");
} catch (error) {
  galatImporPlugin = error;
}

test("sentinel: plugin benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.equal(
    galatImporPlugin,
    null,
    `impor plugin gagal: ${galatImporPlugin?.message ?? galatImporPlugin}`,
  );
  assert.equal(typeof videy?.handler, "function");
  assert.equal(typeof videy?.config?.name, "string");
});

function pesanPengguna(text) {
  const balasan = [];
  const reaksi = [];
  return {
    balasan,
    reaksi,
    m: {
      text,
      prefix: ".",
      command: "videy",
      pushName: "Tester",
      chat: "chat@s.whatsapp.net",
      args: [],
      reply: async (teks) => balasan.push(teks),
      react: async (emoji) => reaksi.push(emoji),
    },
  };
}

function sockPalsu() {
  const terkirim = [];
  return {
    terkirim,
    sendMedia: async (chat, media, quoted, _m, options) => {
      terkirim.push({ chat, media, quoted, options });
    },
  };
}

/** Bentuk CDN videy: 206 satu byte karena probe meminta `Range: bytes=0-0`. */
function cdnHidup() {
  cdnVidey = async () => ({
    status: 206,
    headers: { "content-type": "video/mp4" },
    data: Buffer.from([0x00]),
  });
}

function cdnMati() {
  cdnVidey = async () => {
    const e = new Error("Request failed with status code 404");
    e.response = { status: 404 };
    throw e;
  };
}

beforeEach(() => {
  PANGGILAN.length = 0;
  keyNeoxr = "k-neoxr-untuk-test";
  config.APIkey.neoxr = keyNeoxr;
  cdnVidey = () => {
    throw new Error("cdn.videy.co tidak boleh dipanggil: upstream mati");
  };
});

// ── guard plugin ─────────────────────────────────────────────────────────────

test("guard plugin: tanpa argumen menjawab usage, tanpa menyentuh resolve", async () => {
  const { balasan, reaksi, m } = pesanPengguna("");
  await videy.handler(m, { sock: sockPalsu() });

  assert.match(balasan.at(-1) ?? "", /Masukkan URL videy\.co/);
  assert.deepEqual(PANGGILAN, [], "tidak boleh ada request");
});

test("guard plugin: host palsu ditolak sebelum menyentuh resolve", async () => {
  cdnHidup();
  for (const url of [
    "https://example.com/video.mp4",
    "halo",
    // Label host, bukan substring: ketiganya lolos `url.match(/videy\.co/i)` yang
    // lama, dan tanpa guard per label guard kapabilitas yang menolak — yang
    // berarti satu `run` lokal dan satu kegagalan di breaker untuk semua orang.
    "https://videy.co.evil.example/v?id=7ZH1ZRIF",
    "https://notvidey.co/v?id=7ZH1ZRIF",
    "https://example.com/videy.co",
  ]) {
    const { balasan, reaksi, m } = pesanPengguna(url);
    await videy.handler(m, { sock: sockPalsu() });

    assert.match(balasan.at(-1) ?? "", /videy\.co/i);
    assert.deepEqual(reaksi, [], `tidak boleh menyentuh resolve: ${url}`);
    assert.deepEqual(PANGGILAN, [], `tidak boleh ada request: ${url}`);
  }
});

test("guard plugin: sub-domain videy tetap diterima", async () => {
  // Halaman videy dilayani dari `www.videy.co`, jadi versi ketat jangan
  // memotong pemakaian nyata.
  cdnHidup();
  const { balasan, m } = pesanPengguna("https://www.videy.co/v?id=7ZH1ZRIF");
  await videy.handler(m, { sock: sockPalsu() });

  assert.doesNotMatch(balasan.at(-1) ?? "", /URL tidak valid/, "sub-domain sah tidak boleh ditolak guard");
});

test("link sah meneruskan dan video terkirim dari CDN", async () => {
  cdnHidup();
  const { reaksi, m } = pesanPengguna(LINK);
  const sock = sockPalsu();

  await videy.handler(m, { sock });

  assert.ok(reaksi.includes("🕕"));
  assert.equal(sock.terkirim.length, 1);
  assert.equal(sock.terkirim[0].media, VIDEO);
  assert.equal(sock.terkirim[0].options.type, "video");
  assert.equal(sock.terkirim[0].options.contextInfo.forwardingScore, 99);
  assert.equal(reaksi.at(-1), "✅", "centang hijau hanya sah kalau video benar-benar terkirim");
  assert.equal(
    PANGGILAN.some((c) => new URL(String(c.url)).hostname === "api.neoxr.eu"),
    false,
    "aggregator tidak boleh dihubungi selama CDN hidup",
  );
});

// ── fallback ─────────────────────────────────────────────────────────────────

test("link mati di CDN → aggregator neoxr dipakai dan video tetap terkirim", async () => {
  cdnMati();
  const { m } = pesanPengguna(LINK);
  const sock = sockPalsu();

  await videy.handler(m, { sock });

  assert.ok(
    PANGGILAN.some((c) => new URL(String(c.url)).hostname === "api.neoxr.eu"),
    "aggregator harus dihubungi setelah CDN gagal",
  );
  assert.equal(sock.terkirim[0].media, VIDEO);
  assert.equal(sock.terkirim[0].options.type, "video");
});

// ── key aggregator: jujur, tapi hanya kalau memang dibutuhkan ──────────────

test("key aggregator kosong dan CDN hidup → tetap jalan, tanpa pesan key", async () => {
  // Ini perubahan perilaku yang disengaja: sebelum Phase 1 plugin berhenti
  // sebelum mencoba apa pun kalau key aggregator kosong, padahal resolve lokal
  // tidak butuh key. Menolak di depan akan mematikan command yang bisa jalan.
  config.APIkey.neoxr = "";
  cdnHidup();
  const { balasan, reaksi, m } = pesanPengguna(LINK);
  const sock = sockPalsu();

  await videy.handler(m, { sock });

  assert.equal(sock.terkirim.length, 1, "CDN lokal tidak butuh key");
  assert.equal(reaksi.at(-1), "✅");
  assert.equal(
    balasan.some((t) => /APIKEY_/.test(t)),
    false,
    "tidak ada alasan menyebut key kalau key tidak masalahnya",
  );
});

test("key aggregator kosong dan CDN gagal → sebut APIKEY_NEOXR", async () => {
  config.APIkey.neoxr = "";
  cdnMati();
  const { balasan, reaksi, m } = pesanPengguna(LINK);
  const sock = sockPalsu();

  await videy.handler(m, { sock });

  assert.deepEqual(sock.terkirim, []);
  assert.equal(reaksi.at(-1), "❌");
  assert.match(balasan.at(-1) ?? "", /APIKEY_NEOXR/, "user harus tahu baris .env yang mana");
});

test("semua backend menjawab tanpa URL → pesan link, dan tetap bukan pesan key", async () => {
  // Bentuk di bawah adalah tautan yang benar-benar kedaluwarsa: CDN membalas 404
  // dan aggregator hidup menjawab 200 sambil menandai dirinya gagal dengan
  // `error: "expired"` — DAN tetap membawa URL. Plugin sebelum Phase 1 menolak
  // `!data.status` tepat untuk bentuk ini, jadi memuat `url` di sini adalah bagian
  // dari yang diuji: tanpa penolakan status, video terkirim dan reaksi berakhir `✅`.
  //
  // Ekspektasi ini BERUBAH dari sebelum Task 11: dulu pesan link dianggap mungkin
  // tampil padahal `normalize` menutup jalan `!data?.url`. Yang berubah cuma JALUR
  // pesannya, bukan teksnya.
  cdnMati();
  const asli = httpPalsu.get;
  httpPalsu.get = async (url, opts) => {
    PANGGILAN.push({ verb: "get", url, opts });
    if (new URL(String(url)).hostname === "api.neoxr.eu") {
      return { status: 200, data: { status: false, error: "expired", data: { url: VIDEO } } };
    }
    return asli(url, opts);
  };
  const { balasan, reaksi, m } = pesanPengguna(LINK);
  const sock = sockPalsu();
  try {
    await videy.handler(m, { sock });
  } finally {
    httpPalsu.get = asli;
  }

  assert.deepEqual(sock.terkirim, [], "tidak boleh ada file yang diklaim terkirim");
  assert.equal(reaksi.at(-1), "❌");
  assert.match(balasan.at(-1) ?? "", /sudah expired/);
  assert.equal(reaksi.includes("✅"), false, "centang hijau hanya sah kalau video benar-benar terkirim");
  assert.equal(balasan.some((t) => /APIKEY_/.test(t)), false, "key bukan penyebabnya");
});

test("CDN dan aggregator mati → pesan error umum", async () => {
  cdnMati();
  const asli = httpPalsu.get;
  httpPalsu.get = async (url, opts) => {
    PANGGILAN.push({ verb: "get", url, opts });
    if (new URL(String(url)).hostname === "api.neoxr.eu") throw new Error("neoxr 502");
    return asli(url, opts);
  };
  const { balasan, reaksi, m } = pesanPengguna(LINK);
  try {
    await videy.handler(m, { sock: sockPalsu() });
  } finally {
    httpPalsu.get = asli;
  }

  assert.equal(reaksi.at(-1), "☢");
  // Template error operator (config.errorTemplate) — isinya berubah, perilakunya
  // tidak: ini jalur gagalnya proses, bukan jalur key kosong.
  assert.match(balasan.at(-1) ?? "", /`\.videy`/);
});

// Key asli dipin untuk test lain di repo yang membaca .env operator.
test("key aggregator dikembalikan apa adanya", () => {
  config.APIkey.neoxr = neoxrAsli;
  assert.equal(config.APIkey.neoxr, neoxrAsli);
});
