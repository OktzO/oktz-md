import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/download/sfiledl.js` sungguhan. Uji
// lapis plugin dipisah dari tests/capabilities-phase1.test.mjs karena mengimpor
// plugin ini menuntut mock `src/lib/http.js`, dan satu modul tidak boleh
// di-mock dua kali dalam satu berkas.

const PANGGILAN = [];
const HAL = [
  '<html><head><meta property="og:url" content="https://sfile.mobi/abc123" />' +
    '<meta property="og:description" content="uploaded by planetvdn on 22 January 2026" /></head>' +
    "<body><h1>Spotify PREMIUM_9.1.14.864</h1><p>100.42 MB</p>" +
    "<span>1.2k downloads</span></body></html>",
  '<html><body><a id="download" href="https://sfile.mobi/gate/abc123">Download</a></body></html>',
  '<html><body><script>var u="https:\\/\\/download0426.sfile.co\\/downloadfile\\/2200881\\/724157\\/' +
    '99f1f1d69f8fd931e1447af03be9cec0\\/spotify-premium_9.1.14.864.apk?k=8a782a007c623b62889412b2f5a49424";' +
    "</script></body></html>",
];

const UNDUHAN =
  "https://download0426.sfile.co/downloadfile/2200881/724157/99f1f1d69f8fd931e1447af03be9cec0/" +
  "spotify-premium_9.1.14.864.apk?k=8a782a007c623b62889412b2f5a49424";

let halamanSfile = () => {
  throw new Error("sfile.mobi tidak boleh dipanggil: upstream mati");
};
let keyNeoxr = "k-neoxr-untuk-test";

const httpPalsu = {
  async get(url, opts) {
    PANGGILAN.push({ verb: "get", url, opts });
    const host = new URL(String(url)).hostname;
    // Dicocokkan ke host: URL aggregator untuk kapabilitas ini memuat
    // "sfile.mobi" di query string-nya, jadi pencocokan substring salah arah.
    if (host.endsWith("sfile.mobi") || host.endsWith("sfile.co")) {
      return halamanSfile(url, opts);
    }
    if (host === "api.neoxr.eu") {
      return {
        status: 200,
        data: { data: { url: UNDUHAN, filename: "a.apk", mime: "application/vnd.android.package-archive" } },
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
let sfiledl = null;
let galatImporPlugin = null;
try {
  sfiledl = await import("../plugins/download/sfiledl.js");
} catch (error) {
  galatImporPlugin = error;
}

test("sentinel: plugin benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.equal(
    galatImporPlugin,
    null,
    `impor plugin gagal: ${galatImporPlugin?.message ?? galatImporPlugin}`,
  );
  assert.equal(typeof sfiledl?.handler, "function");
  assert.equal(typeof sfiledl?.config?.name, "string");
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
      command: "sfiledl",
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

function sfileLokalSukses(hal = HAL) {
  let i = 0;
  halamanSfile = async () => ({ data: hal[i++] ?? hal[hal.length - 1], headers: {} });
}

beforeEach(() => {
  PANGGILAN.length = 0;
  keyNeoxr = "k-neoxr-untuk-test";
  config.APIkey.neoxr = keyNeoxr;
  halamanSfile = () => {
    throw new Error("sfile.mobi tidak boleh dipanggil: upstream mati");
  };
});

// ── guard plugin ─────────────────────────────────────────────────────────────

test("guard plugin: host palsu ditolak sebelum menyentuh resolve", async () => {
  // `sfile.mobi.evil.example` lolos `includes("sfile.mobi")`. Kalau guard plugin
  // hanya cek substring, guard kapabilitas yang menolak dan `cobaBackend`
  // mencatat kegagalan — tiga link seperti ini dari satu user sudah cukup
  // membuka breaker sfile yang sehat.
  for (const url of ["https://sfile.mobi.evil.example/x", "https://notsfile.mobi/x", "halo"]) {
    sfileLokalSukses();
    const { balasan, reaksi, m } = pesanPengguna(url);
    await sfiledl.handler(m, { sock: sockPalsu() });

    assert.deepEqual(reaksi, [], `tidak boleh menyentuh resolve: ${url}`);
    assert.match(balasan.at(-1) ?? "", /sfile\.mobi/i);
    assert.deepEqual(PANGGILAN, [], `tidak boleh ada request: ${url}`);
  }
});

test("guard plugin: link sah tetap meneruskan dan dokumen terkirim", async () => {
  sfileLokalSukses();
  const { reaksi, m } = pesanPengguna("https://sfile.mobi/abc123");
  const sock = sockPalsu();

  await sfiledl.handler(m, { sock });

  assert.ok(reaksi.includes("🕕"));
  assert.equal(sock.terkirim.length, 1);
  assert.equal(sock.terkirim[0].media, UNDUHAN);
  assert.equal(sock.terkirim[0].options.type, "document");
  assert.equal(sock.terkirim[0].options.fileName, "Spotify PREMIUM_9.1.14.864");
  // Scraper lokal tidak mengirim mime; normalize menjadikannya string kosong,
  // bukan `undefined` yang akan menjadi nama file "undefined".
  assert.equal(sock.terkirim[0].options.mimetype, "");
  assert.equal(reaksi.at(-1), "✅", "centang hijau hanya sah kalau file benar-benar terkirim");
});

test("scraper lokal gagal → aggregator neoxr dipakai dan file tetap terkirim", async () => {
  // `#download` hilang: scraper mengembalikan `download_url: null`, normalize
  // melempar, aggregator mengambil alih.
  sfileLokalSukses([HAL[0], HAL[2]]);
  const { m } = pesanPengguna("https://sfile.mobi/abc123");
  const sock = sockPalsu();

  await sfiledl.handler(m, { sock });

  assert.ok(
    PANGGILAN.some((c) => new URL(String(c.url)).hostname === "api.neoxr.eu"),
    "aggregator harus dihubungi setelah scraper lokal gagal",
  );
  assert.equal(sock.terkirim[0].media, UNDUHAN);
  assert.equal(sock.terkirim[0].options.mimetype, "application/vnd.android.package-archive");
});

// ── key aggregator: jujur, tapi hanya kalau memang dibutuhkan ──────────────

test("key aggregator kosong dan scraper lokal berhasil → tetap jalan, tanpa pesan key", async () => {
  // Ini perubahan perilaku yang disengaja: sebelum Phase 1 plugin berhenti
  // sebelum mencoba apa pun kalau key aggregator kosong, padahal scraper lokal
  // tidak butuh key. Menolak di depan akan mematikan command yang bisa jalan.
  config.APIkey.neoxr = "";
  sfileLokalSukses();
  const { balasan, reaksi, m } = pesanPengguna("https://sfile.mobi/abc123");
  const sock = sockPalsu();

  await sfiledl.handler(m, { sock });

  assert.equal(sock.terkirim.length, 1, "scraper lokal tidak butuh key");
  assert.equal(reaksi.at(-1), "✅");
  assert.equal(
    balasan.some((t) => /APIKEY_/.test(t)),
    false,
    "tidak ada alasan menyebut key kalau key tidak masalahnya",
  );
});

test("key aggregator kosong dan scraper lokal gagal → sebut APIKEY_NEOXR", async () => {
  config.APIkey.neoxr = "";
  sfileLokalSukses([HAL[0], HAL[2]]);
  const { balasan, reaksi, m } = pesanPengguna("https://sfile.mobi/abc123");
  const sock = sockPalsu();

  await sfiledl.handler(m, { sock });

  assert.deepEqual(sock.terkirim, []);
  assert.equal(reaksi.at(-1), "❌");
  assert.match(balasan.at(-1) ?? "", /APIKEY_NEOXR/, "user harus tahu baris .env yang mana");
});

test("semua backend menjawab tanpa URL → pesan link, dan tetap bukan pesan key", async () => {
  // Dua backend menjawab, tapi tidak ada yang memberi URL unduhan: scraper lokal
  // mengembalikan `download_url: null` dan aggregator menjawab `{ data: {} }`.
  // Sebidaknya satu dari mereka menjawab, dan jawabannya "tidak ada file di sini" —
  // jadi `adaNormalisasiGagal` benar dan pesan yang dipakai adalah pesan link.
  //
  // Ekspektasi ini BERUBAH dari sebelum Task 11: dulu `!data?.url` dianggap mungkin
  // terjadi dan pesan link dianggap mungkin tampil, padahal `normalize` menutupnya.
  // Yang berubah cuma JALUR pesannya, bukan teksnya: `☢` + template operator
  // digantikan `❌` + pesan yang memberitahu user apa yang harus diperbaiki.
  sfileLokalSukses([HAL[0], HAL[2]]);
  const { balasan, reaksi, m } = pesanPengguna("https://sfile.mobi/abc123");
  const asli = httpPalsu.get;
  httpPalsu.get = async (url, opts) => {
    PANGGILAN.push({ verb: "get", url, opts });
    if (new URL(String(url)).hostname === "api.neoxr.eu") return { status: 200, data: { data: {} } };
    return asli(url, opts);
  };
  const sock = sockPalsu();
  try {
    await sfiledl.handler(m, { sock });
  } finally {
    httpPalsu.get = asli;
  }

  assert.deepEqual(sock.terkirim, [], "tidak boleh ada file yang diklaim terkirim");
  assert.equal(reaksi.at(-1), "❌");
  assert.match(balasan.at(-1) ?? "", /File mungkin tidak tersedia/);
  assert.equal(reaksi.includes("✅"), false, "centang hijau hanya sah kalau file benar-benar terkirim");
  assert.equal(balasan.some((t) => /APIKEY_/.test(t)), false, "key bukan penyebabnya");
});

test("scraper lokal dan aggregator mati → pesan error umum", async () => {
  halamanSfile = async () => {
    throw new Error("sfile.mobi timeout");
  };
  const asli = httpPalsu.get;
  httpPalsu.get = async (url, opts) => {
    PANGGILAN.push({ verb: "get", url, opts });
    if (new URL(String(url)).hostname === "api.neoxr.eu") throw new Error("neoxr 502");
    return asli(url, opts);
  };
  const { balasan, reaksi, m } = pesanPengguna("https://sfile.mobi/abc123");
  try {
    await sfiledl.handler(m, { sock: sockPalsu() });
  } finally {
    httpPalsu.get = asli;
  }

  assert.equal(reaksi.at(-1), "☢");
  // Template error operator (config.errorTemplate) — isinya berubah, perilakunya
  // tidak: ini jalur gagalnya proses, bukan jalur key kosong.
  assert.match(balasan.at(-1) ?? "", /`\.sfiledl`/);
});

// Key asli dipin untuk test lain di repo yang membaca .env operator.
test("key aggregator dikembalikan apa adanya", () => {
  config.APIkey.neoxr = neoxrAsli;
  assert.equal(config.APIkey.neoxr, neoxrAsli);
});