import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
import { CapabilityError } from "../src/lib/resolve.js";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini menguji `plugins/download/pindl.js` dengan cara memanggilnya,
// bukan dengan mencocokkan teks sumber. Uji lapis plugin dipisah dari
// tests/capabilities-phase1.test.mjs karena mengimpor plugin menuntut mock
// `src/lib/http.js` yang punya `f`, dan satu modul tidak boleh di-mock dua kali
// dalam satu berkas.
//
// Transport di bawahnya dimock, jadi tidak ada request yang keluar ke jaringan:
// scraper lokal memakai `httpAxios`, dan `f` (dipakai hanya di jalur GIF)
// sengaja dibuat melempar agar tidak pernah menembak CDN.

const PANGGILAN = [];
let balasIlovepin = () => {
  throw new Error("ilovepin tidak boleh dipanggil: upstream mati");
};
let balasProxy = () => {
  throw new Error("proxy ilovepin tidak boleh dipanggil: upstream mati");
};

const httpPalsu = {
  async get(url, opts) {
    PANGGILAN.push({ verb: "get", url, opts });
    return balasIlovepin(url, opts);
  },
  async post(url, body, config) {
    PANGGILAN.push({ verb: "post", url, config });
    return balasProxy(url, body, config);
  },
};

const fPalsu = async () => {
  throw new Error("f() tidak boleh dipanggil di test ini");
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu, f: fPalsu } });

const pindl = await import("../plugins/download/pindl.js");

const GAMBAR = "https://i.pinimg.com/originals/a.jpg";

/** Bentuk respons ilovepin yang membuat plugin benar-benar mengirim file. */
function pinGambar() {
  balasIlovepin = async () => ({ headers: {} });
  balasProxy = async () => ({
    data: {
      api: {
        status: "OK",
        title: "Judul",
        mediaItems: [
          { type: "Image", mediaExtension: "JPG", mediaFileSize: "1.2 MB", mediaUrl: GAMBAR },
        ],
      },
    },
  });
}

function pesanPengguna(text) {
  const balasan = [];
  const reaksi = [];
  return {
    balasan,
    reaksi,
    m: {
      text,
      prefix: ".",
      chat: "chat@s.whatsapp.net",
      args: [],
      reply: async (teks) => balasan.push(teks),
      react: async (emoji) => reaksi.push(emoji),
    },
  };
}

beforeEach(() => {
  PANGGILAN.length = 0;
  balasIlovepin = () => {
    throw new Error("ilovepin tidak boleh dipanggil: upstream mati");
  };
  balasProxy = () => {
    throw new Error("proxy ilovepin tidak boleh dipanggil: upstream mati");
  };
});

// ── jalur unduhan video: jangan pernah tarik file penuh tanpa batas ──────────

test("rencanaVideo: URL .mp4 aggregator jadi manifest HLS", () => {
  const rencana = pindl.rencanaVideo("https://v1.pinimg.com/videos/xx/720p.mp4");
  assert.equal(rencana.masterUrl, "https://v1.pinimg.com/videos/xx/hls.m3u8");
  assert.equal(
    rencana.hls,
    true,
    "manifest harus tetap diurai supaya trek audio ikut tersalin",
  );
});

test("rencanaVideo: URL /720p dari scraper lokal bukan manifest", () => {
  // src/scraper/pindl.js:134 contended URL video lokal berakhiran /720p dan
  // isinya mp4 progressive. Menganggapnya manifest berarti seluruh file ditarik
  // ke memori tanpa batas hanya supaya bisa di-`split("\n")`.
  const rencana = pindl.rencanaVideo("https://s15.mcontent.app/v3/videoProcess/1/720p");
  assert.equal(rencana.masterUrl, "https://s15.mcontent.app/v3/videoProcess/1/720p");
  assert.equal(rencana.hls, false, "file langsung: ffmpeg yang menariknya, bukan axios tanpa batas");
});

test("pindl: axios hanya boleh diambil di dalam cabang hls", () => {
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  const idx = src.indexOf("axios.get(masterUrl");
  assert.ok(idx > -1, "pengambilan manifest harus masih ada");
  assert.match(
    src.slice(Math.max(0, idx - 900), idx),
    /if \(hls\) \{/,
    "pengambilan manifest harus di dalam `if (hls)`, bukan dijalankan untuk semua URL video",
  );
});

test("pindl: unduhan video manual punya batas ukuran eksplisit", () => {
  // Jalur ini memakai instance axios biasa, yang maxContentLength-nya -1.
  // Di kotak 1GB dengan batas RSS 550MB, satu video besar sudah bisa membuat
  // proses dibunuh memory-monitor; gagal dengan pesan jauh lebih murah.
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  assert.match(
    src,
    /maxContentLength:\s*BATAS_BODY_VIDEO/,
    "unduhan video manual harus punya batas ukuran",
  );
});

// ── guard plugin: harus sekeras guard kapabilitas ────────────────────────────

test("guard plugin: notpinterest.com ditolak sebelum menyentuh resolve", async () => {
  // `notpinterest.com` lolos `url.includes("pinterest")`. Kalau guard plugin
  // hanya cek substring, guard kapabilitas yang menolaknya dan `cobaBackend`
  // mencatat kegagalan — tiga link seperti ini dari satu user sudah cukup
  // membuka breaker ilovepin 30 detik untuk semua orang, termasuk yang sedang
  // mengunduh pin yang sebenarnya bisa berjalan.
  const { balasan, reaksi, m } = pesanPengguna("https://notpinterest.com/pin/x");
  await pindl.handler(m, { sock: {} });

  assert.deepEqual(reaksi, [], "tidak boleh menyentuh resolve");
  assert.match(balasan.at(-1) ?? "", /tidak valid/i);
  assert.deepEqual(PANGGILAN, []);
});

test("guard plugin: halaman Pinterest yang bukan pin juga ditolak", async () => {
  const { balasan, reaksi, m } = pesanPengguna("https://id.pinterest.com/search/pins/?q=cewe");
  await pindl.handler(m, { sock: {} });

  assert.deepEqual(reaksi, [], "tidak boleh menyentuh resolve");
  assert.match(balasan.at(-1) ?? "", /tidak valid/i);
  assert.deepEqual(PANGGILAN, []);
});

test("guard plugin: link pin yang sah tetap diteruskan dan filenya dikirim", async () => {
  // Guard yang terlalu ketat mematikan command yang tadinya jalan, jadi arah
  // sebaliknya ikut dikunci: link sah harus mencapai scraper, mengirim file,
  // dan hanya lalu memberi centang hijau.
  pinGambar();
  const { reaksi, m } = pesanPengguna("https://id.pinterest.com/pin/87186942777228203/");
  const terkirim = [];

  await pindl.handler(m, {
    sock: {
      sendMedia: async (chat, media, _quoted, _m, options) =>
        terkirim.push({ chat, media, options }),
    },
  });

  assert.ok(reaksi.includes("🕕"), "link sah harus lanjut ke resolve");
  assert.equal(terkirim.length, 1);
  // Jalur gambar meneruskan URL-nya apa adanya ke sock (bukan objek media).
  assert.equal(terkirim[0].media, GAMBAR);
  assert.equal(terkirim[0].options.type, "image");
  assert.equal(reaksi.at(-1), "✅", "centang hijau hanya sah kalau ada file terkirim");
});

test("guard plugin: pin.it tetap diterima", async () => {
  pinGambar();
  const { balasan, m } = pesanPengguna("https://pin.it/3abcd");
  await pindl.handler(m, { sock: { sendMedia: async () => {} } });
  assert.equal(
    balasan.some((t) => /tidak valid/i.test(t)),
    false,
    "pin.it adalah pemendek resmi dan harus diterima",
  );
});
