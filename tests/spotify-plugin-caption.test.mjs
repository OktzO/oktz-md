import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// plugins/search/spotify.js diuji lewat handlernya, bukan lewat grep source.
// Dua jalur berbeda bisa mencapai branch tanpa tombol: cover gagal diambil
// (lalu lintas paling umum saat aggregator sehat) dan link hasil pencarian yang
// bukan track Spotify. Keduanya harus menjawab dengan caption yang tidak
// menjanjikan tombol atau perintah `.spdl`.

const REAKSI = [];
const BALASAN = [];
const PESAN_KIRIM = [];
let ISI_PESAN = null;
let hasilCari = [];
let coverGagal = false;

function mPalsu(teks) {
  return {
    chat: "chat@s.whatsapp.net",
    prefix: ".",
    command: "spotify",
    pushName: "Tester",
    text: teks,
    async react(emoji) { REAKSI.push(emoji); },
    async reply(text) { BALASAN.push(String(text)); return text; },
  };
}

mock.module("axios", {
  defaultExport: {
    async get(url) {
      if (coverGagal) throw new Error("cover gagal diambil");
      return { data: Buffer.from("gambar-{}", "utf8") };
    },
  },
});

// `sharp` dipanggil sebagai fungsi: sharp(buffer).resize().jpeg().toBuffer()
const sharpPalsu = {
  resize() { return this; },
  jpeg() { return this; },
  async toBuffer() { return Buffer.from("jpeg-1kb", "utf8"); },
};
mock.module("sharp", { defaultExport: () => sharpPalsu });

mock.module("onigis", {
  namedExports: {
    generateWAMessageFromContent(chat, content) {
      ISI_PESAN = content.buttonsMessage;
      return { message: { buttonsMessage: content.buttonsMessage }, key: { id: "id" } };
    },
  },
});

mock.module("../src/lib/resolve.js", {
  namedExports: {
    resolver: {
      resolve: async (capability, args) => {
        if (capability !== "spotify") throw new Error(`kapabilitas tak diminta: ${capability}`);
        return { ok: true, source: "nexray", data: { tracks: hasilCari }, meta: { tookMs: 1, cached: false } };
      },
    },
  },
});

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

// `resolve.js` di-mock di berkas ini dan hanya menarik `circuit-breaker` serta
// `capability-cache`, jadi rantai config.js → lid.js belum masuk ke proses test.
// Catatan: berkas ini hari ini belum menarik `lid.js` ke dalam prosesnya,
// jadi sentinel di sini guarding sesuatu yang belum terjadi. Murah sekarang,
// mahal besok kalau satu baris impor plugin berubah.
const spotifyPlugin = await impor("../plugins/search/spotify.js");
const handler = spotifyPlugin?.handler;

test("sentinel: plugin spotify benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof handler, "function");
});

function captionTerkirim() {
  if (ISI_PESAN) return ISI_PESAN.contentText;
  return BALASAN.at(-1) ?? "";
}

function tombolTerkirim() {
  return ISI_PESAN?.buttons ?? null;
}

const LAGU_SPOTIFY = {
  title: "Grateful",
  artist: "Neffex",
  url: "https://open.spotify.com/track/abc",
  cover: "https://i.scdn.co/image/a.jpg",
  duration: "3:45",
};

beforeEach(() => {
  REAKSI.length = 0;
  BALASAN.length = 0;
  PESAN_KIRIM.length = 0;
  ISI_PESAN = null;
  coverGagal = false;
  hasilCari = [{ ...LAGU_SPOTIFY }];
});

test("cover ada dan linknya track Spotify → tombol dipasang dan caption sesuai janji", async () => {
  const sock = {
    async relayMessage(chat, msg) { PESAN_KIRIM.push(msg); },
    async sendMessage() {},
  };
  await handler(mPalsu("neffex grateful"), { sock, text: "neffex grateful" });

  const caption = captionTerkirim();
  assert.ok(
    caption.includes("Atau tekan tombol di bawah ini untuk lagu pertama."),
    `kalimat tombol harus ada kalau tombolnya ada: ${caption.slice(-200)}`,
  );
  assert.equal(tombolTerkirim()?.length, 1, "harus tepat satu tombol");
  assert.equal(tombolTerkirim()[0].buttonId, ".spdl https://open.spotify.com/track/abc");
  assert.equal(PESAN_KIRIM.length, 1, "buttonsMessage harus benar-benar dikirim lewat relayMessage");
});

test("cover gagal diambil → caption tidak boleh menjanjikan tombol yang tidak ada", async () => {
  // Jalur paling umum: aggregator sehat, tapi fetch cover ditolak atau timeout.
  // branch tanpa tombol sudah ada di plugin, hanya captionnya yang bohong.
  coverGagal = true;
  const sock = { async relayMessage() {}, async sendMessage() {} };

  await handler(mPalsu("neffex grateful"), { sock, text: "neffex grateful" });

  assert.equal(tombolTerkirim(), null, "tidak boleh ada buttonsMessage tanpa tombol");
  assert.ok(BALASAN.length > 0, "harus jatuh ke m.reply(contentText)");
  const caption = captionTerkirim();
  assert.equal(
    caption.includes("Atau tekan tombol di bawah ini"),
    false,
    `caption tidak boleh menyebut tombol yang tidak dikirim: ${caption.slice(-200)}`,
  );
  // Saran `.spdl` boleh tetap ada di sini: link-nya memang track Spotify yang
  // bisa disalin dan diunduh, hanya tombolnya yang tidak terlampir.
  assert.ok(
    caption.includes("`.spdl <link>`"),
    "linknya track Spotify, jadi saran menyalin link masih benar",
  );
});

test("hasil pencarian bukan track Spotify → caption tidak menjanjikan .spdl", async () => {
  hasilCari = [{ ...LAGU_SPOTIFY, url: "https://youtube.com/watch?v=abc" }];
  const sock = { async relayMessage() {}, async sendMessage() {} };

  await handler(mPalsu("neffex grateful"), { sock, text: "neffex grateful" });

  const caption = captionTerkirim();
  assert.equal(tombolTerkirim(), null, "tidak boleh ada tombol untuk link yang ditolak .spdl");
  assert.equal(caption.includes("Atau tekan tombol di bawah ini"), false);
  assert.equal(caption.includes("`.spdl <link>`"), false);
});

test("judul dan daftar lagu tetap utuh di kedua cabang", async () => {
  coverGagal = true;
  const sock = { async relayMessage() {}, async sendMessage() {} };
  await handler(mPalsu("neffex grateful"), { sock, text: "neffex grateful" });

  const caption = captionTerkirim();
  assert.ok(caption.includes("*HASIL PENCARIAN SPOTIFY*"), "header harus tetap ada");
  assert.ok(caption.includes("*1. Grateful*"), "judul lagu harus tetap tercetak");
  assert.ok(caption.includes("🎤 Artis: Neffex"), "artis harus tetap tercetak");
  assert.ok(caption.includes("⏱️ Durasi: 3:45"), "durasi harus tetap tercetak");
  assert.ok(caption.includes("🔗 Link: https://open.spotify.com/track/abc"), "link harus tetap tercetak");
});
