import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";

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

// ── sentinel import ─────────────────────────────────────────────────────────
//
// Rantai impor di berkas ini melewati modul yang memanggil `src/lib/aggregator.js`
// → `config.js` → `src/lib/database.js` → `src/lib/lid.js`, dan `src/lib/lid.js:61`
// memasang `process.on("uncaughtException", …)` yang menelan semua error — termasuk
// yang dilempar modul yang gagal diimpor. Akibatnya impor yang gagal TIDAK
// menggagalkan berkas: proses keluar 0 dan node:test melaporkan `pass 1` dengan nol
// test di dalamnya.
//
// Impor jadi tidak boleh dibiarkan gagal telanjang. Kalau `await import` melempar,
// badan modul ini tidak pernah selesai dievaluasi dan tidak satu pun `test()`
// terdaftar — sentinel yang diletakkan setelah impor pun tidak akan ikut jalan.
// Kesalahannya ditangkap, lalu assertion sentinel yang menggagalkan.
//
// Pola yang sama dipakai tests/applemusic-plugin.test.mjs dan
// tests/sfiledl-plugin.test.mjs. `src/lib/lid.js` sendiri tidak diubah di sini: itu
// penanganan crash produksi dan di luar cakupan tugas ini.
const GALAT_IMPOR = [];

async function impor(specifier) {
  try {
    return await import(specifier);
  } catch (error) {
    GALAT_IMPOR.push(`${specifier} → ${error?.code ?? "?"}: ${error?.message ?? error}`);
    return null;
  }
}

const pindl = await impor("../plugins/download/pindl.js");

test("sentinel: plugin pindl benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof pindl?.handler, "function");
  assert.equal(typeof pindl?.rencanaVideo, "function");
});

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

// ── Lindungan RAM: setiap lubang punya coverage sendiri ──────────────────────
//
// Batas 64MB adalah perubahan perilaku yang disetujui karena plafon RSS 550MB,
// jadi ambangnya diuji langsung (fungsi murni), sedangkan *pemasangannya* diuji
// lewat sumber: jalur video tidak bisa dijalankan tanpa ffmpeg sungguhan, jadi
// tidak ada cara lain mengujinya tanpa memalsukan lebih banyak daripada yang
// hilang.

const BATAS = 64 * 1024 * 1024;

test("terlaluBesar: tepat di batas masih boleh, satu byte di atasnya ditolak", () => {
  assert.equal(pindl.terlaluBesar(BATAS), false, "batas harus inklusif, bukan eksklusif");
  assert.equal(pindl.terlaluBesar(BATAS + 1), true);
  assert.equal(pindl.terlaluBesar(0), false);
});

test("pindl: kedua pengambilan video lewat axios dibatasi ukurannya", () => {
  // Pemeriksaan hanya pada satu call site tidak cukup: menghapus batas dari
  // pengambilan manifest akan lolos kalau yang diuji cuma fallback.
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  const bagian = src.split("axios.get(media.url");
  assert.equal(bagian.length, 2, "harus ada tepat dua pengambilan video lewat axios");

  assert.match(
    bagian[0],
    /axios\.get\(masterUrl[\s\S]*?maxContentLength: BATAS_BODY_VIDEO/,
    "pengambilan manifest HLS juga harus dibatasi ukurannya",
  );
  assert.match(
    bagian[1],
    /maxContentLength: BATAS_BODY_VIDEO/,
    "unduhan manual harus dibatasi ukurannya",
  );
});

test("pindl: file hasil ffmpeg diperiksa ukurannya sebelum dibaca ke memori", () => {
  // `sendMedia` menerima Buffer, jadi file yang sudah diproses ffmpeg dibaca utuh.
  // Pemeriksaan harus ada DAN harus mendahului pembacaan itu; salah satu saja
  // tidak berguna.
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  // Titik panggil, bukan definisi: `terlaluBesar` dideklarasikan di atas handler.
  const cek = src.indexOf("if (terlaluBesar(ukuran))");
  const baca = src.indexOf("fs.readFileSync(outputFile)");

  const ukur = src.indexOf("fs.statSync(outputFile).size");

  assert.ok(ukur > -1, "ukuran file hasil ffmpeg harus dibaca dari statSync");
  assert.ok(cek > -1, "pemeriksaan ukuran harus ada");
  assert.ok(baca > -1, "pembacaan file hasil ffmpeg harus ada");
  assert.ok(
    ukur < cek,
    "yang diperiksa harus ukuran file hasil ffmpeg, bukan angka lain",
  );
  assert.ok(cek < baca, "pemeriksaan harus mendahului fs.readFileSync, bukan sesudahnya");
});

test("pindl: galat ukuran tidak diulang lewat fallback yang pasti gagal lagi", () => {
  // Fallback mengunduh ulang URL yang sama dengan batas yang sama, jadi user
  // akan menunggu satu pull 64MB yang pasti gagal lalu membaca pesan axios
  // bahasa Inggris. Galat ukuran harus langsung jadi alasan gagal.
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  const idx = src.indexOf("} catch (err) {", src.indexOf("await queueFFmpeg"));
  assert.ok(idx > -1, "blok catch jalur video harus ada");

  const blok = src.slice(idx, idx + 700);
  assert.match(
    blok,
    /terlaluBesar/,
    "fallback harus dilewati kalau galatnya pemeriksaan ukuran",
  );
  assert.match(
    src,
    /terlaluBesar\?: true|\.terlaluBesar = true/,
    "galat ukuran harus ditandai supaya bisa dibedakan dari galat jaringan",
  );
});
