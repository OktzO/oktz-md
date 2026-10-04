import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini menjalankan cabang GIF `plugins/download/pindl.js` sungguhan.
//
// Cabang itu satu-satunya pembacaan tanpa batas yang tersisa di branch ini:
// `f(media.url, "buffer")` memakai undici, yang tidak punya `maxContentLength`, jadi
// satu GIF raksasa bisa masuk penuh ke memori di kotak 1GB yang plafon RSS-nya
// 550MB. Batas 64MB milik jalur video tidak pernah berlaku di sini. Sekarang
// cabang itu menarik lewat `httpAxios`, yang batasnya 25MB.
//
// Uji dua arah: GIF benar-benar masuk sebagai Buffer utuh lewat transport berbatas,
// dan transport tanpa batas itu benar-benar tidak terpakai.
//
// Uji lapis plugin dipisah dari tests/pindl-plugin.test.mjs karena berkas itu sudah
// `mock.module` `src/lib/http.js` dengan stub-nya sendiri, dan satu modul tidak
// boleh di-mock dua kali dalam satu berkas.

const PANGGILAN = [];
const UNDUHAN_AXIOS = [];
const PERINTAH_FFMPEG = [];
// Byte GIF yang benar-benar ada di disk saat ffmpeg dipanggil. Plugin menghapus
// file itu di `finally`, jadi pembacaannya harus terjadi di dalam stub, bukan
// sesudah handler selesai.
const GIF_DIBACA_FFMPEG = [];
let dataGif = null;

// `httpAxios` dipalsukan supaya berkas ini menguji PEMASANGAN batasnya di pindl,
// bukan transport sungguhan. Batas yang diwarisi instance aslinya diuji terpisah
// dari sumber `src/lib/http.js` di bawah.
const httpPalsu = {
  async get(url, opts) {
    PANGGILAN.push({ verb: "get", url, opts });
    UNDUHAN_AXIOS.push({ url, opts });
    if (new URL(String(url)).hostname !== "i.pinimg.com") {
      throw new Error(`host tak terduga di test ini: ${url}`);
    }
    return { status: 200, data: dataGif, headers: {} };
  },
  async post() {
    throw new Error("POST tidak boleh dipakai di test ini");
  },
};

// Tripwire. `f()` tidak punya batas ukuran, jadi kalau cabang GIF masih memakainya
// pesan galat ini yang akan muncul.
const fPalsu = async (url) => {
  throw new Error(`f() tidak boleh dipakai — tidak punya batas ukuran: ${url}`);
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu, f: fPalsu } });

// Cabang GIF memanggil ffmpeg sungguhan kalau tidak dipalsukan. Stub ini menulis
// file keluaran yang diminta perintah, supaya handler bisa membaca dan mengirimnya
// tanpa menjalankan ffmpeg di dalam test.
mock.module("../src/lib/ffmpeg.js", {
  namedExports: {
    queueFFmpeg: async (perintah) => {
      PERINTAH_FFMPEG.push(perintah);
      const masuk = /-i "([^"]+)"/.exec(perintah)?.[1];
      if (masuk && fs.existsSync(masuk)) GIF_DIBACA_FFMPEG.push(fs.readFileSync(masuk));
      const keluar = /"([^"]+)"\s*$/.exec(perintah)?.[1];
      if (keluar) fs.writeFileSync(keluar, Buffer.from("mp4-palsu", "utf8"));
    },
  },
});

// Resolve selalu berhasil dengan satu item gambar `.gif`, jadi yang diuji benar-benar
// cabang GIF dan bukan jalur gambar biasa yang melempar URL mentah ke sock.
const GIF_URL = "https://i.pinimg.com/originals/a.gif";
mock.module("../src/lib/resolve.js", {
  namedExports: {
    resolver: {
      resolve: async () => ({
        ok: true,
        source: "ilovepin",
        data: { media: [{ type: "image", url: GIF_URL }] },
        meta: { tookMs: 1, cached: false },
      }),
    },
    // pindl tidak memakainya, tapi modul aslinya mengekspornya dan bentuk impor
    // plugin harus tetap sama dengan modul sungguhan.
    adaNormalisasiGagal: () => false,
  },
});

// ── sentinel import ─────────────────────────────────────────────────────────
//
// Rantai impor di berkas ini melewati `src/lib/error.js` → `config.js` →
// `src/lib/database.js` → `src/lib/lid.js`, dan `src/lib/lid.js:61` memasang
// `process.on("uncaughtException", …)` yang menelan semua error — termasuk yang
// dilempar modul yang gagal diimpor. Akibatnya impor yang gagal TIDAK menggagalkan
// berkas: proses keluar 0 dan node:test melaporkan `pass 1` dengan nol test.
//
// Pola yang sama dipakai tests/applemusic-plugin.test.mjs,
// tests/sfiledl-plugin.test.mjs, dan tests/pap-search-outage.test.mjs.
// `src/lib/lid.js` sendiri tidak diubah di sini: itu penanganan crash produksi dan
// di luar cakupan berkas ini.
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
      command: "pindl",
      pushName: "Tester",
      chat: "chat@s.whatsapp.net",
      args: [],
      reply: async (teks) => balasan.push(String(teks)),
      react: async (emoji) => reaksi.push(emoji),
    },
  };
}

/** GIF 1KB: cukup besar supaya tidak disangka placeholder, kecil supaya aman. */
function gifPalsu() {
  const buf = Buffer.alloc(1024);
  buf.write("GIF89a", "ascii");
  for (let i = 6; i < buf.length; i += 1) buf[i] = i % 251;
  return buf;
}

beforeEach(() => {
  PANGGILAN.length = 0;
  UNDUHAN_AXIOS.length = 0;
  PERINTAH_FFMPEG.length = 0;
  GIF_DIBACA_FFMPEG.length = 0;
  dataGif = gifPalsu();
});

async function jalankanGif() {
  const terkirim = [];
  const { balasan, reaksi, m } = pesanPengguna("https://pin.it/a");
  await pindl.handler(m, {
    sock: {
      sendMedia: async (chat, media, _quoted, _m, options) =>
        terkirim.push({ chat, media, options }),
    },
  });
  return { terkirim, balasan, reaksi };
}

// ── lewat: GIF masuk sebagai Buffer utuh lewat transport berbatas ───────────

test("GIF diambil lewat httpAxios yang berbatas, bukan lewat f()", async () => {
  await jalankanGif();

  assert.deepEqual(
    UNDUHAN_AXIOS.map((c) => c.url),
    [GIF_URL],
    "harus tepat satu pengambilan lewat httpAxios",
  );
  // `arraybuffer` adalah yang membuat adapter axios Node mengembalikan Buffer.
  // Tanpa itu `res.data` bisa string dan file GIF rusak di disk.
  assert.equal(UNDUHAN_AXIOS[0].opts?.responseType, "arraybuffer");
});

test("GIF tidak meng-override batas yang diwarisi httpAxios", async () => {
  // Batas 25MB milik `httpAxios` (src/lib/http.js:20) dipakai bersama semua
  // pembacaan HTTP di repo ini. Menyalin angkanya ke plugin berarti ada dua
  // konstanta yang bisa melenceng; membiarkan angka itu tidak tertulis di plugin
  // berarti perubahan batas di satu tempat langsung mengubah semua pemakai —
  // dan itu memang yang diinginkan.
  await jalankanGif();

  assert.equal(
    UNDUHAN_AXIOS[0].opts?.maxContentLength,
    undefined,
    "batas harus diwarisi dari instance httpAxios, bukan ditulis ulang di plugin",
  );
});

test("byte GIF masuk ke ffmpeg utuh, tanpa jadi string di tengah", async () => {
  const asli = dataGif;
  const { terkirim, balasan, reaksi } = await jalankanGif();

  assert.equal(PERINTAH_FFMPEG.length, 1, "GIF harus diteruskan ke ffmpeg");
  // Yang dibaca plugin setelah ffmpeg adalah mp4 hasil, jadi keutuhan byte GIF
  // dibuktikan dari file yang dibaca stub: kalau `raw` sempat jadi string, isi file
  // itu tidak akan sama dengan buffer aslinya.
  assert.deepEqual(
    GIF_DIBACA_FFMPEG[0],
    asli,
    "GIF harus sampai ke ffmpeg tanpa kehilangan satu byte pun",
  );
  assert.equal(terkirim.length, 1, "cabang GIF tidak boleh jatuh ke fallback URL mentah");
  assert.equal(terkirim[0].options.gifPlayback, true);
  assert.deepEqual(reaksi, ["🕕", "✅"], "centang hijau hanya sah kalau file benar-benar terkirim");
  assert.deepEqual(balasan, []);
});

// ── tidak boleh lewat: f() dan deklarasi batas ─────────────────────────────

test("pindl tidak mengimpor f() dan tidak memanggilnya", () => {
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  const baris = src.split("\n").find((b) => b.includes("lib/http.js"));
  const impor = /^import \{([^}]+)\} from "\.\.\/\.\.\/src\/lib\/http\.js";$/.exec(baris?.trim() ?? "");
  assert.ok(impor, `baris impor http.js tidak dikenali: ${baris}`);
  assert.deepEqual(
    impor[1].split(",").map((n) => n.trim()),
    ["httpAxios"],
    "plugin ini hanya boleh mengimpor httpAxios yang berbatas",
  );
  // Baris komentar dibuang dulu: nama `f()` sengaja disebut di komentar yang
  // menjelaskan kenapa ia tidak dipakai, dan itu bukan pemanggilan.
  const kode = src
    .split("\n")
    .filter((b) => !/^\s*(\/\/|\/\*|\*)/.test(b))
    .join("\n");
  assert.doesNotMatch(kode, /[^.\w]f\(/, "f() tidak boleh dipanggil dari plugin ini");
});

test("httpAxios mendeklarasikan maxContentLength eksplisit dan berhingga", () => {
  // Batas yang diwarisi cabang GIF diuji di atas; di sini yang diuji adalah bahwa
  // batas itu benar-benar ADA di `src/lib/http.js`. Kalau instance ini punya
  // `maxContentLength: Infinity` — atau kehilangan deklarasinya — semua pembacaan
  // HTTP di repo ini kembali tanpa batas dan tidak ada test lain yang berkedip.
  const sumber = fs.readFileSync(path.join(process.cwd(), "src/lib/http.js"), "utf8");
  const blok = /export const httpAxios = axios\.create\(\{([\s\S]*?)\n\}\)/.exec(sumber);
  assert.ok(blok, "httpAxios harus tetap axios.create({ ... })");

  const batas = /maxContentLength:\s*(\d+)\s*\*\s*(\d+)\s*\*\s*(\d+)/.exec(blok[1]);
  assert.ok(batas, `maxContentLength harus ditulis eksplisit: ${blok[1]}`);
  const nilai = batas.slice(1).reduce((total, angka) => total * Number(angka), 1);

  assert.ok(Number.isFinite(nilai) && nilai > 0, `batas harus berhingga dan positif: ${nilai}`);
  assert.ok(
    nilai <= 64 * 1024 * 1024,
    `batas ${Math.round(nilai / 1024 / 1024)}MB terlalu longgar untuk plafon RSS 550MB`,
  );
});