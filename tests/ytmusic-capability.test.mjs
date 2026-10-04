import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini menguji backend LOKAL kapabilitas `ytmusic`, jadi ia harus
// mengimpor `ytmusic-api` dengan implementasi palsu. `tests/capabilities-phase1.test.mjs`
// sudah mockspecifier yang sama — Node menolak `mock.module` kedua untuk
// specifier yang sama — jadi pemisahan ini bukan pilihan gaya, tapi syarat.
//
// Yang dipalsukan hanya libnya. Modul kapabilitas diimpor sungguhan, sehingga
// file yang hilang atau salah nama export tetap menggagalkan test ini. Resolvernya
// juga sungguhan, sehingga urutan tier dan budget ikut teruji, bukan hanya
// bentuk `normalize`.
//
// `aggregator` ikut dipalsukan dan SELALU melempar. Backend ini tidak punya
// stub agregator, jadi tanpa itu satu test yang/isinya salah bentuk akan
// menembak jaringan sungguhan lewat tier api dan tidak gagal.

const PANGGILAN = [];

let PANGGILAN_AGGREGATOR = 0;

class YTMusicPalsu {
  constructor() {
    PANGGILAN.push({ event: "new" });
    this.siap = false;
  }

  async initialize() {
    PANGGILAN.push({ event: "initialize" });
    if (inisialisasiGagal) throw new Error("initialize gagal: jaringan mati");
    this.siap = true;
    return this;
  }

  async search(kunci) {
    PANGGILAN.push({ event: "search", kunci, siap: this.siap });
    if (!this.siap) throw new Error("search sebelum initialize: instance belum siap");
    return hasilSearch(kunci);
  }
}

let hasilSearch = () => {
  throw new Error("ytmusic tidak boleh dipanggil: upstream mati");
};
let inisialisasiGagal = false;

mock.module("ytmusic-api", { defaultExport: YTMusicPalsu });
mock.module("../src/lib/aggregator.js", {
  namedExports: {
    aggregator: {
      hit: async (name, path) => {
        PANGGILAN_AGGREGATOR += 1;
        throw new Error(`aggregator tidak boleh dipanggil di berkas ini: ${name} ${path}`);
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

// Catatan: `aggregator` di-mock di berkas ini, jadi rantai config.js → lid.js
// belum masuk ke proses test. Sentinel tetap dipasang karena mock itu bisa
// dihapus kapan saja.
const kap = await impor("../src/capabilities/ytmusic.js");
const { createResolver } = await impor("../src/lib/resolve.js");

const lokal = kap?.backends.find((b) => b.kind === "local");

test("sentinel: modul ytmusic dan resolver benar-benar terimpor", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof kap?.normalize, "function");
  assert.equal(typeof lokal?.run, "function", "backend lokal ytmusic tidak terbaca");
  assert.equal(typeof createResolver, "function", "resolver tidak terimpor");
});

/**
 * Modul kapabilitas yang sama, tapi instance modul BARU.
 *
 * Singleton di dalam modul sengaja bertahan seumur proses — itu justru
 * sifat yang diuji. Untuk menguji awal siklus hidup singleton (instance belum
 * dibuat, inisialisasi gagal lalu diulang) test butuh modul yang belum pernah
 * dimuat; query string membuat Node memuat ulang berkas yang sama dengan
 * `let instance` yang baru. Nama host, `backends`, dan `normalize` tetap identik
 * karena keduanya berasal dari berkas yang sama.
 */
let penghitungModul = 0;
function kapBaru() {
  penghitungModul += 1;
  return import(`../src/capabilities/ytmusic.js?modul=${penghitungModul}`);
}

function resolverUji(kemampuan = kap, opsi = {}) {
  return createResolver({ capabilities: { ytmusic: () => kemampuan }, ...opsi });
}

const SONG = {
  type: "SONG",
  videoId: "dQw4w9WgXcQ",
  name: "Best Friend",
  artists: [{ name: "Rex Orange County" }],
  duration: 225,
  thumbnails: [{ url: "https://lh3.example/kecil.jpg" }, { url: "https://lh3.example/besar.jpg" }],
};

beforeEach(() => {
  PANGGILAN.length = 0;
  PANGGILAN_AGGREGATOR = 0;
  hasilSearch = () => {
    throw new Error("ytmusic tidak boleh dipanggil: upstream mati");
  };
  inisialisasiGagal = false;
});

test("instance dibuat malas: mengimpor kapabilitas tidak menyentuh jaringan", () => {
  // `initialize()` menembak jaringan. Kalau ia jalan saat import, satu
  // `resolve()` kapabilitas lain pun akan ikut membayar koneksi yang tidak pernah
  // dipakai. Test ini harus yang pertama: setelahnya singleton memang sudah hidup.
  assert.deepEqual(PANGGILAN, [], "tidak boleh ada instance YTMusic sebelum backend dipanggil");
});

test("initialize() dipanggil setelah new, lalu dipakai untuk search", async () => {
  hasilSearch = async () => [SONG];

  const keluar = await resolverUji().resolve("ytmusic", { q: "best friend" });

  assert.deepEqual(
    PANGGILAN.map((c) => c.event),
    ["new", "initialize", "search"],
    "urutan wajib new → initialize → search",
  );
  assert.equal(keluar.source, "ytmusic");
  assert.deepEqual(keluar.data, {
    tracks: [
      {
        title: "Best Friend",
        artist: "Rex Orange County",
        durationSec: 225,
        cover: "https://lh3.example/besar.jpg",
        url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
      },
    ],
  });
  assert.equal(keluar.meta.cached, false, "stable false: tidak boleh keluar dari cache");
});

test("panggilan berikutnya memakai instance yang sama, initialize tidak diulang", async () => {
  // Singleton harus bertahan seumur proses: `initialize()` menyentuh jaringan,
  // jadi mengulangnya per permintaan membuat setiap `.applemusic` membayar satu
  // koneksi tambahan.
  hasilSearch = async () => [SONG];
  const resolver = resolverUji();

  await resolver.resolve("ytmusic", { q: "satu" });
  PANGGILAN.length = 0;
  await resolver.resolve("ytmusic", { q: "dua" });

  assert.deepEqual(
    PANGGILAN.map((c) => c.event),
    ["search"],
    "panggilan kedua hanya boleh search",
  );
  assert.equal(PANGGILAN[0].kunci, "dua");
});

test("panggilan bersamaan berbagi satu initialize, tidak dua instance", async () => {
  // `play2.js:221-229` menaruh instance sebelum `await initialize()`, sehingga
  // pemanggil kedua yang tiba saat inisialisasi berjalan menerima instance yang
  // belum siap dan gagal. Di sini keduanya harus menunggu inisialisasi yang sama.
  const kapBaruDenganAwalKosong = await kapBaru();
  hasilSearch = async () => [SONG];
  const resolver = resolverUji(kapBaruDenganAwalKosong);

  const [a, b] = await Promise.all([
    resolver.resolve("ytmusic", { q: "sama" }),
    resolver.resolve("ytmusic", { q: "sama" }),
  ]);

  assert.equal(PANGGILAN.filter((c) => c.event === "new").length, 1, "tidak boleh ada instance kedua");
  assert.equal(PANGGILAN.filter((c) => c.event === "initialize").length, 1, "initialize harus tepat sekali");
  assert.equal(PANGGILAN.filter((c) => c.event === "search").length, 2);
  assert.ok(
    PANGGILAN.filter((c) => c.event === "search").every((c) => c.siap === true),
    "tidak boleh ada search sebelum initialize selesai",
  );
  for (const keluar of [a, b]) assert.equal(keluar.source, "ytmusic");
});

test("initialize yang gagal tidak disimpan, dan panggilan berikutnya mencoba lagi", async () => {
  const modul = await kapBaru();
  inisialisasiGagal = true;
  hasilSearch = async () => [SONG];

  await assert.rejects(
    () => resolverUji(modul).resolve("ytmusic", { q: "a" }),
    /semua backend ytmusic gagal/,
  );
  // Tier api memang tetap diberi giliran setelah lokal gagal — itu urutan
  // resolver, dan di sini aggregator palsu yang membuat resolve() berhenti.
  assert.equal(PANGGILAN_AGGREGATOR, 1, "tier api harus tetap mencoba setelah lokal gagal");

  inisialisasiGagal = false;
  const keluar = await resolverUji(modul).resolve("ytmusic", { q: "b" });

  assert.equal(keluar.source, "ytmusic", "instance yang gagal harus di-reset, bukan disimpan setengah siap");
  assert.equal(PANGGILAN.filter((c) => c.event === "new").length, 2, "percobaan kedua harus membuat instance baru");
});

test("kueri dipangkas dan argumen kosong berhenti sebelum new", async () => {
  await assert.rejects(
    () => resolverUji().resolve("ytmusic", { q: "   " }),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      return true;
    },
  );
  assert.deepEqual(PANGGILAN, [], "backend tidak boleh dijalankan untuk argumen yang sudah ditolak");

  hasilSearch = async () => [SONG];
  await resolverUji().resolve("ytmusic", { q: "  best friend  " });
  assert.equal(PANGGILAN.find((c) => c.event === "search").kunci, "best friend");
});

test("backend lokal menolak argumen tanpa q walau dipanggil langsung", async () => {
  // `applies` sudah menyaring ini, tapi backend ini juga dipanggil langsung oleh
  // pemanggil lain; guard di dalam backend membuat kegagalan input berhenti
  // sebelum jaringan.
  await assert.rejects(() => lokal.run({}, {}), /butuh \{ q \}/);
  await assert.rejects(() => lokal.run({ q: "" }, {}), /butuh \{ q \}/);
  assert.deepEqual(PANGGILAN, []);
});

test("hasil pencarian nullish dihitung kegagalan, bukan data kosong", async () => {
  // Resolver memperlakukan null/undefined sebagai "tanpa data" supaya
  // scraper yang targetnya tidak ada tidak disajikan sebagai jawaban kosong.
  // Di sini tiainya yang dikunci: entri tanpa hasil harus tetap gagal supaya
  // tier api sempat dicoba.
  for (const nilai of [undefined, null]) {
    PANGGILAN.length = 0;
    hasilSearch = async () => nilai;
    await assert.rejects(
      () => resolverUji().resolve("ytmusic", { q: "best friend" }),
      (error) => {
        // Lokal "tanpa data" baru setelah itu tier api tetap dicoba: jawaban
        // kosong tidak boleh menutup jalan ke host lain.
        assert.deepEqual(error.tried.map((t) => t.name), ["ytmusic", "nexray"]);
        assert.equal(error.tried[0].reason, "tanpa data");
        return true;
      },
    );
  }
});

test("daftar campuran: ARTIST dan PLAYLIST yang lebih dulu tidak jadi hasil", async () => {
  // Ini jebakan yang paling merusak kalau lolos: `search()` menaruh entri
  // ARTIST/PLAYLIST di depan, jadi backend yang memakai `hasil[0]` akan
  // mengembalikan nama artist, bukan lagu.
  hasilSearch = async () => [
    { type: "ARTIST", name: "Rex Orange County", subscribers: "1.2M" },
    { type: "PLAYLIST", title: "Throwback 2019", browseId: "VLCA" },
    SONG,
  ];

  const keluar = await resolverUji().resolve("ytmusic", { q: "best friend" });
  assert.equal(keluar.data.tracks.length, 1);
  assert.equal(keluar.data.tracks[0].title, "Best Friend");
});

test("daftar kosong tetap sukses: plugin yang memutuskan cara menampilkannya", async () => {
  hasilSearch = async () => [];
  const keluar = await resolverUji().resolve("ytmusic", { q: "tidak ada lagu ini" });
  assert.equal(keluar.source, "ytmusic");
  assert.deepEqual(keluar.data, { tracks: [] });
});