import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/search/applemusic.js` sungguhan dengan
// resolver palsu, supaya yang diuji adalah batas plugin: pesan, pemotongan lima
// hasil, dan teks yang dirender dari `data.tracks` — bukan transport di bawahnya.
//
// Specifier `../src/lib/resolve.js` dipakai berkas lain, jadi mock-nya harus
// berada di berkas ini.

const PANGGILAN = [];

let hasilResolve = () => {
  throw new Error("resolver tidak boleh dipanggil");
};

mock.module("../src/lib/resolve.js", {
  namedExports: {
    resolver: {
      resolve: async (kapabilitas, args) => {
        PANGGILAN.push({ kapabilitas, args });
        return hasilResolve(kapabilitas, args);
      },
    },
  },
});

// ── sentinel import ───────────────────────────────────────────────────────────
//
// Berkas ini mengimpor `../src/lib/resolve.js` (mock) sebelum plugin, dan rantai
// impor plugin itu `../../src/lib/error.js` → `../../config.js` → … → `src/lib/lid.js`.
// `src/lib/lid.js:61` memasang `process.on("uncaughtException", …)` yang menelan
// semua error, termasuk `ERR_MODULE_NOT_FOUND`. Akibatnya impor plugin yang
// gagal tidak menggagalkan berkas: proses keluar 0 dan node:test melaporkan
// `pass 1` dengan nol test di dalamnya.
//
// Karena itu impor plugin tidak boleh dibiarkan gagal telanjang di top-level.
// Kalau `await import` di situ melempar, modul berkas ini tidak pernah selesai
// dievaluasi dan `test()` di bawah tidak pernah terdaftar — sentinel yang
// diletakkan setelah impor justru tidak pernah ikut jalan. Kesalahannya
// ditangkap, lalu assertion-nya yang menggagalkan. Pola yang sama dipakai
// tests/sfiledl-plugin.test.mjs.
let applemusic = null;
let galatImporPlugin = null;
try {
  applemusic = await import("../plugins/search/applemusic.js");
} catch (error) {
  galatImporPlugin = error;
}

test("sentinel: plugin benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.equal(
    galatImporPlugin,
    null,
    `impor plugin gagal: ${galatImporPlugin?.message ?? galatImporPlugin}`,
  );
  assert.equal(typeof applemusic?.handler, "function");
  assert.equal(typeof applemusic?.config?.name, "string");
});

function pesanPengguna(text) {
  const balasan = [];
  return {
    balasan,
    m: {
      text,
      prefix: ".",
      command: "applemusic",
      pushName: "Tester",
      chat: "chat@s.whatsapp.net",
      args: [],
      reply: async (teks) => balasan.push(teks),
    },
  };
}

const JAWAB = {
  ok: true,
  source: "ytmusic",
  data: {
    tracks: [
      {
        title: "Best Friend",
        artist: "Rex Orange County",
        durationSec: 225,
        cover: "https://lh3.example/a.jpg",
        url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
      },
      {
        title: "Sunriser",
        artist: "Lee Brice",
        durationSec: 190,
        cover: "https://lh3.example/b.jpg",
        url: "https://www.youtube.com/watch?v=aaaaaaaaaaa",
      },
    ],
  },
  meta: { tookMs: 12, cached: false },
};

beforeEach(() => {
  PANGGILAN.length = 0;
  hasilResolve = () => {
    throw new Error("resolver tidak boleh dipanggil");
  };
});

test("kueri diteruskan ke resolver sebagai { q } dan hasilnya dirender", async () => {
  hasilResolve = async () => JAWAB;
  const { balasan, m } = pesanPengguna("best friend");

  await applemusic.handler(m, { sock: {} });

  assert.deepEqual(PANGGILAN, [{ kapabilitas: "ytmusic", args: { q: "best friend" } }]);
  assert.equal(balasan.length, 1);
  const teks = balasan[0];
  assert.ok(teks.includes("🍎 *ᴀᴘᴘʟᴇ ᴍᴜsɪᴄ sᴇᴀʀᴄʜ*"), `judul blok hilang: ${teks}`);
  assert.ok(teks.includes("> Query: *best friend*"), `baris query hilang: ${teks}`);
  assert.ok(teks.includes("*1.* ```Best Friend```"), `nomor atau judul hilang: ${teks}`);
  assert.ok(teks.includes("Rex Orange County"), `artist tidak tampil: ${teks}`);
  assert.ok(teks.includes("https://www.youtube.com/watch?v=dQw4w9WgXcQ"), `tautan tidak tampil: ${teks}`);
  assert.ok(teks.includes("*2.* ```Sunriser```"));
});

test("kueri kosong dijawab cara pakai tanpa menyentuh resolver", async () => {
  const { balasan, m } = pesanPengguna("   ");

  await applemusic.handler(m, { sock: {} });

  assert.deepEqual(PANGGILAN, [], "argumen kosong tidak boleh sampai ke resolver");
  assert.equal(balasan.length, 1);
  const teks = balasan[0];
  assert.ok(teks.includes("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*"), `judul cara pakai hilang: ${teks}`);
  assert.ok(teks.includes("applemusic <query>"), `contoh perintah hilang: ${teks}`);
  assert.ok(teks.includes("Contoh:"), `label contoh hilang: ${teks}`);
  assert.ok(teks.includes(".applemusic Best Friend"), `contoh kedua hilang: ${teks}`);
});

test("hasil kosong dari host dijawab pesan tidak ditemukan, bukan daftar kosong", async () => {
  // Daftar kosong itu jawaban yang sah dari kapabilitas; yang memutuskan
  // kalimatnya adalah plugin, seperti sebelum Phase 1.
  hasilResolve = async () => ({ ok: true, source: "ytmusic", data: { tracks: [] }, meta: { tookMs: 3, cached: false } });
  const { balasan, m } = pesanPengguna("tidak ada lagu ini");

  await applemusic.handler(m, { sock: {} });

  assert.equal(balasan.length, 1);
  assert.equal(balasan[0], "❌ Tidak ditemukan hasil untuk: tidak ada lagu ini");
});

test("track tanpa artist tampil sebagai Unknown, bukan undefined", async () => {
  // Bentuk agregator nexray tidak selalu punya `subtitle`, dan sebelum Phase 1
  // plugin menulis `t.subtitle || 'Unknown'`, jadi fallback itu harus tetap ada.
  hasilResolve = async () => ({
    ok: true,
    source: "nexray",
    data: {
      tracks: [{ title: "Tanpa Artis", artist: "", durationSec: 0, cover: "", url: "https://music.apple.com/x" }],
    },
    meta: { tookMs: 3, cached: false },
  });
  const { balasan, m } = pesanPengguna("tanpa artis");

  await applemusic.handler(m, { sock: {} });

  assert.ok(balasan[0].includes("📀 `Unknown`"), `dapat: ${balasan[0]}`);
  assert.doesNotMatch(balasan[0], /undefined/);
});

test("track dengan artist null tidak pernah tercetak sebagai null", async () => {
  hasilResolve = async () => ({
    ok: true,
    source: "ytmusic",
    data: {
      tracks: [{ title: "X", artist: null, durationSec: 0, cover: "", url: "https://x.example" }],
    },
    meta: { tookMs: 3, cached: false },
  });
  const { balasan, m } = pesanPengguna("x");

  await applemusic.handler(m, { sock: {} });

  assert.doesNotMatch(balasan[0], /undefined|null/);
});

test("kegagalan resolver dijawab template error yang sama seperti sebelumnya", async () => {
  hasilResolve = async () => {
    throw new Error("semua backend ytmusic gagal");
  };
  const { balasan, m } = pesanPengguna("best friend");

  await applemusic.handler(m, { sock: {} });

  assert.equal(balasan.length, 1);
  // Template berasal dari `config.errorTemplate`, jadi yang diuji adalah bahwa
  // jalur gagalnya tetap jatuh ke template itu — bukan kalimat yang ditulis ulang.
  assert.ok(balasan[0].includes("Silahkan coba lagi nanti"), `dapat: ${balasan[0]}`);
  assert.ok(balasan[0].includes("Tester"), `pushName tidak masuk template: ${balasan[0]}`);
  assert.ok(balasan[0].includes("`.applemusic`"), `nama command tidak masuk template: ${balasan[0]}`);
  assert.doesNotMatch(balasan[0], /semua backend ytmusic gagal/, `error internal bocor ke user: ${balasan[0]}`);
});

test("lima hasil pertama saja yang ditampilkan", async () => {
  const tracks = Array.from({ length: 8 }, (_, i) => ({
    title: `Lagu ${i + 1}`,
    artist: `Artis ${i + 1}`,
    durationSec: 100,
    cover: "",
    url: `https://music.apple.com/${i + 1}`,
  }));
  hasilResolve = async () => ({ ok: true, source: "ytmusic", data: { tracks }, meta: { tookMs: 3, cached: false } });
  const { balasan, m } = pesanPengguna("best friend");

  await applemusic.handler(m, { sock: {} });

  const teks = balasan[0];
  assert.ok(teks.includes("*5.* ```Lagu 5```"), `lima pertama harus tampil: ${teks}`);
  assert.ok(!teks.includes("Lagu 6"), "hasil keenam dan seterusnya harus dipotong");
});

test("resolusi gagal tidak pernah menampilkan track dari hasil sebelumnya", async () => {
  // Plugin tidak boleh menyimpan hasil sendiri. Kegagalan kedua harus mengganti
  // tampilan, bukan menampilkan sisa jawaban yang sudah basi.
  hasilResolve = async () => JAWAB;
  const { balasan, m } = pesanPengguna("best friend");
  await applemusic.handler(m, { sock: {} });
  assert.ok(balasan[0].includes("Best Friend"));

  hasilResolve = async () => {
    throw new Error("semua backend ytmusic gagal");
  };
  await applemusic.handler(m, { sock: {} });
  assert.equal(balasan.length, 2);
  assert.ok(!balasan[1].includes("Best Friend"), `jawaban basi tampil lagi: ${balasan[1]}`);
});

test("permukaan plugin tidak berubah: config dan nama command", () => {
  assert.deepEqual(applemusic.config, {
    name: "applemusic",
    alias: ["amusic", "am"],
    category: "search",
    description: "Cari lagu di Apple Music",
    usage: ".applemusic <query>",
    example: ".applemusic Best Friend",
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 5,
    energi: 0,
    isEnabled: true,
  });
});