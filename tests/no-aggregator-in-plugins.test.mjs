import { test } from "node:test";
import assert from "node:assert";
import { readFile, readdir } from "node:fs/promises";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

// ── ratchet, bukan gate kosong ───────────────────────────────────────────────
//
// Pada 2026-10-04 masih ada 92 plugin di luar 17 target Fase 1 yang memakai
// agregator. Itu backlog Fase 2-6 dan BOLEH ada, jadi test ini tidak boleh
// menuntut nol: assertion "tidak ada domain agregator di plugins/" akan gagal di
// 92 file dan tidak berarti apa-apa.
//
// Tapi juga tidak boleh hanya memeriksa 17 file Fase 1 — itu membuat 92 file lain
// bebas bocor tanpa pernah terdeteksi. Yang dipakai di sini adalah ratchet dua
// arah: satu himpunan path yang DIHARAPKAN masih memakai agregator, lalu
//   - memakai tapi tidak ada di himpunan  → gagal, ini kebocoran baru
//   - tidak memakai tapi ada di himpunan → gagal, supaya daftar ikut menyusut
// Pelanggaran dikumpulkan SEMUA lalu diasersikan sekali di akhir, supaya satu kali
// jalan menampilkan seluruh sisa dalam satu pesan — bukan hanya yang pertama.

// ── cara mencari usage ───────────────────────────────────────────────────────
//
// Dua sinyal, karena ada dua cara plugin menyentuh agregator di repo ini:
//
//   1. Host aggregator ditulis literal di sumber plugin. Daftar host di bawah
//      adalah isi AGGREGATORS di src/lib/aggregator.js (src/lib/aggregator.js:24-29)
//      ditambah satu alias: `api.nexray.web.id` ada di src/lib/apimanager.js:958
//      dan dipakai 10 plugin yang tidak menulis `api.nexray.eu.cc` sama sekali.
//      Kalau hanya mengikuti aggregator.js, 10 plugin itu lolos dari deteksi.
//
//   2. Plugin memanggil kelas provider di src/lib/apimanager.js yang baseURL-nya
//      menunjuk host agregator (`fotoApi.neoxr.*`, `fotoApi.nexray.*`). Dua plugin
//      memakai jalur ini tanpa menulis domain apa pun, jadi pencarian domain saja
//      tidak akan pernah melihatnya.
//
// Sinyal (2) sengaja sempit — hanya `\w+\.(neoxr|nexray)\s*\.`, bukan token
// `neoxr` bebas. Token bebas akan menangkap `config.APIkey?.neoxr` (nama env,
// bukan host) dan bahkan `"Mizukage Klon"` di plugins/game/kyubigame.js yang
// hanya kebetulan memuat huruf "izuka".
//
// HOST_AGREGATOR diverifikasi ulang terhadap src/lib/aggregator.js oleh test
// paling bawah di berkas ini, jadi host baru yang masuk ke aggregator.js tidak
// bisa lolos dari deteksi hanya karena daftar di sini lupa diperbarui.

const HOST_AGREGATOR = [
  "api.neoxr.eu",
  "api.nexray.eu.cc",
  "api.nexray.web.id",
  "my.izuka-api.xyz",
  "api.cuki.biz.id",
  "api.siputzx.my.id",
  "api.azbry.com",
];

// Dicocokkan per label host, bukan `includes` di seluruh string:
// `api.neoxr.eu.evil.example` adalah domain lain yang tetap memuat "api.neoxr.eu".
const POLA_HOST = HOST_AGREGATOR.map((h) =>
  new RegExp(`(?<![\\w.-])${h.replace(/\./g, "\\.")}(?![\\w-])`, "i"),
);

const POLA_PROVIDER = /\b\w+\.(neoxr|nexray)\s*\./;

function pakaiAggregator(isi) {
  return POLA_HOST.some((p) => p.test(isi)) || POLA_PROVIDER.test(isi);
}

// ── himpunan yang diharapkan ──────────────────────────────────────────────────
//
// CARA MEMBUAT ULANG (dijalankan 2026-10-04, 92 baris):
//
//   git grep -lE "api\.neoxr\.eu|api\.nexray\.|my\.izuka-api\.xyz|api\.cuki\.biz\.id|api\.siputzx\.my\.id|api\.azbry\.com" -- plugins/ | sort > /tmp/a
//   git grep -lE "\w+\.(neoxr|nexray)\s*\." -- plugins/ | sort > /tmp/b
//   cat /tmp/a /tmp/b | sort -u
//
// Jalankan ulang setiap kali aggregator.js mendapat host baru. Kalau jumlah
// barisnya berbeda dari 92, catat perintah mana yang menghasilkan apa di laporan
// perubahan — jangan hanya menambah atau menghapus baris supaya test hijau.
//
// Arah ratchet: setiap fase berikutnya yang memindahkan plugin ke kapabilitas
// harus MENGHAPUS path-nya dari daftar ini pada commit yang sama. Baris yang
// sudah tidak memakai agregator akan menggagalkan test, dan itulah mekanisme yang
// membuat daftar ini menyusut sendiri dan tidak pernah usang.
//
// Catatan asal-usul angka: plan Task 11 menyebut 93. Angka itu benar pada
// commit 156aae2, lalu turun ke 92 oleh ff397d2 (Task 9, applemusic) dan 90 oleh
// 50c2d69 (Task 10, hd). Dua plugin sisanya masuk lewat sinyal (2), jadi 92.

const MASIH_PAKAI_AGGREGATOR = new Set([
  "plugins/ai/anime-gen.js",
  "plugins/ai/characterai.js",
  "plugins/ai/matematika.js",
  "plugins/ai/musicmaker.js",
  "plugins/ai/quillbot.js",
  "plugins/ai/simi.js",
  "plugins/ai/sologo.js",
  "plugins/ai/txt2img.js",
  "plugins/anime/topanime.js",
  "plugins/asupan/asupantiktok.js",
  "plugins/canvas/applemusic.js",
  "plugins/canvas/balogo.js",
  "plugins/canvas/fakebankjago.js",
  "plugins/canvas/fakecall.js",
  "plugins/canvas/fakedana.js",
  "plugins/canvas/fakeml.js",
  "plugins/canvas/gura.js",
  "plugins/canvas/iqc2.js",
  "plugins/canvas/iqc.js",
  "plugins/canvas/musiccard.js",
  "plugins/canvas/pakustad.js",
  "plugins/canvas/starboy.js",
  "plugins/canvas/tiktokchat.js",
  "plugins/canvas/watercolortext.js",
  "plugins/download/pixeldraindl.js",
  "plugins/fun/fuckmylife.js",
  "plugins/fun/gachahusbu.js",
  "plugins/fun/gachawaifu.js",
  "plugins/fun/puisi.js",
  "plugins/fun/senja.js",
  "plugins/info/gag.js",
  "plugins/info/gagwatch.js",
  "plugins/info/harilibur.js",
  "plugins/info/jadwalbola.js",
  "plugins/primbon/artinama.js",
  "plugins/primbon/kecocokannamapasangan.js",
  "plugins/primbon/nomerhoki.js",
  "plugins/primbon/potensipenyakit.js",
  "plugins/primbon/ramalanjodoh.js",
  "plugins/primbon/sifatusahabisnis.js",
  "plugins/primbon/tafsirmimpi.js",
  "plugins/primbon/zodiak.js",
  "plugins/random/anime.js",
  "plugins/random/barandom.js",
  "plugins/random/cecanchina.js",
  "plugins/random/cecanindo.js",
  "plugins/random/cecanjepang.js",
  "plugins/random/cecankorea.js",
  "plugins/random/cecanthai.js",
  "plugins/random/cecanvietnam.js",
  "plugins/random/lahelu.js",
  "plugins/random/meme.js",
  "plugins/random/quotesimage.js",
  "plugins/religi/islami.js",
  "plugins/search/android1-get.js",
  "plugins/search/android1.js",
  "plugins/search/animeapaini.js",
  "plugins/search/apkmod-get.js",
  "plugins/search/apkmod.js",
  "plugins/search/carigrup.js",
  "plugins/search/chords.js",
  "plugins/search/comparation.js",
  "plugins/search/filmget.js",
  "plugins/search/film.js",
  "plugins/search/lyrics.js",
  "plugins/search/mangatoon.js",
  "plugins/search/mcpedl.js",
  "plugins/search/pixiv.js",
  "plugins/search/play.js",
  "plugins/search/resep.js",
  "plugins/search/tiktokfoto.js",
  "plugins/stalker/countrystalk.js",
  "plugins/stalker/discordstalk.js",
  "plugins/stalker/ffstalk.js",
  "plugins/stalker/genshinstalk.js",
  "plugins/stalker/pintereststalk.js",
  "plugins/stalker/robloxplayer.js",
  "plugins/sticker/attp.js",
  "plugins/sticker/emojimix.js",
  "plugins/sticker/linesticker.js",
  "plugins/sticker/pinpack.js",
  "plugins/sticker/smeme-animated.js",
  "plugins/sticker/stickerly.js",
  "plugins/tools/caribug.js",
  "plugins/tools/emojitoanimasi.js",
  "plugins/tools/emojitoimage.js",
  "plugins/tools/hitungwrmlbb.js",
  "plugins/tools/invoicemaker.js",
  "plugins/tools/musikapaini.js",
  "plugins/tools/phisingataubukan.js",
  "plugins/tools/spamngl.js",
  "plugins/tts/tts.js",
]);

// ── kumpulkan semua file plugin ──────────────────────────────────────────────

const AKAR = fileURLToPath(new URL("..", import.meta.url));
const DIREKTORI_PLUGIN = join(AKAR, "plugins");

async function semuaFilePlugin(dir = DIREKTORI_PLUGIN, terkumpul = []) {
  for (const entri of await readdir(dir, { withFileTypes: true })) {
    const penuh = join(dir, entri.name);
    if (entri.isDirectory()) await semuaFilePlugin(penuh, terkumpul);
    // Hanya `.js`: plugin di repo ini tidak pernah berekstensi lain, dan
    // menyertakan yang lain hanya menambah noise.
    else if (entri.isFile() && entri.name.endsWith(".js")) terkumpul.push(penuh);
  }
  return terkumpul;
}

const FILE_PLUGIN = await semuaFilePlugin();

// ── test 1: ratchet dua arah ─────────────────────────────────────────────────

test("ratchet: plugin yang memakai agregator harus terdaftar, dan yang terdaftar harus memakainya", async () => {
  // Assertion DI LUAR loop, disengaja. Kalau assert di dalam loop, file pertama
  // yang gagal menghentikan pemeriksaannya dan pesan error hanya menyebut satu
  // file — padahal yang diminta adalah menampilkan seluruh sisa dalam satu kali
  // jalan. Semua pelanggaran dikumpulkan dulu, lalu dilaporkan bersama.
  const bocorBaru = [];
  const usang = [];

  for (const penuh of FILE_PLUGIN) {
    const isi = await readFile(penuh, "utf8");
    const relatif = relative(AKAR, penuh).split("\\").join("/");
    if (pakaiAggregator(isi)) {
      if (!MASIH_PAKAI_AGGREGATOR.has(relatif)) bocorBaru.push(relatif);
    } else if (MASIH_PAKAI_AGGREGATOR.has(relatif)) {
      usang.push(relatif);
    }
  }

  // Dua assert terpisah, bukan satu: kebocoran baru dan daftar usang adalah dua
  // kegagalan yang berbeda dengan perbaikan yang berbeda, dan menyatukannya
  // membuat penyebabnya kabur.
  assert.deepEqual(
    bocorBaru,
    [],
    `plugin memakai agregator tapi tidak terdaftar di MASIH_PAKAI_AGGREGATOR — ` +
      `kebocoran baru, atau plugin Fase 1 yang belum dimigrasi (${bocorBaru.length} file):\n  - ` +
      bocorBaru.join("\n  - "),
  );
  assert.deepEqual(
    usang,
    [],
    `plugin terdaftar di MASIH_PAKAI_AGGREGATOR tapi sudah tidak memakai agregator — ` +
      `hapus dari daftar pada commit yang sama (${usang.length} file):\n  - ` +
      usang.join("\n  - "),
  );
});

// ── test 2: src/capabilities/ hanya boleh menyentuh agregator lewat aggregator ──
//
// Domain agregator BOLEH muncul di src/capabilities/ — itulah cara fallback-nya
// bekerja. Yang dilarang adalah memanggil transport itu secara langsung:
// `import axios` di kapabilitas berarti ada satu jalur yang tidak melewati
// `aggregator.hit`, jadi
// tidak ada breaker, tidak ada budget, tidak ada key management, dan tidak ada
// tempat satu titik untuk mematikan host itu.
//
// Arahnya dibalik dari test pertama: di `plugins/` host aggregator adalah
// larangan keras, sedangkan di `src/capabilities/` aggregator justru boleh
// dipanggil — tapi hanya lewat `aggregator.hit(...)`, tidak pernah langsung.

test("kapabilitas: tidak ada import axios atau fetch langsung, hanya lewat aggregator.hit", async () => {
  const DIREKTORI_KAPABILITAS = join(AKAR, "src", "capabilities");
  const file = (await readdir(DIREKTORI_KAPABILITAS, { withFileTypes: true }))
    .filter((e) => e.isFile() && e.name.endsWith(".js"))
    .map((e) => e.name);

  const axiosLangsung = [];
  const tanpaAggregator = [];

  for (const nama of file) {
    const isi = await readFile(join(DIREKTORI_KAPABILITAS, nama), "utf8");
    const label = `src/capabilities/${nama}`;

    // Pola import, bukan kemunculan kata: hd.js:156 menyebut "axios" di
    // komentarnya (menjelaskan bahwa aggregator.hit meneruskan FormData ke axios)
    // dan komentar itu benar serta berguna. Yang berbahaya adalah modul yang
    // benar-benar mengimpor atau memanggilnya.
    if (/^\s*import\s[^;]*["']axios["']/m.test(isi) || /\bfetch\s*\(/.test(isi)) {
      axiosLangsung.push(label);
    }

    // Kalau kapabilitas ini menyebut host agregator, ia WAJIB mengimpor
    // aggregator.js — tanpa itu request-nya keluar tanpa breaker dan tanpa
    // budget, dan tidak ada satu titik pun untuk mematikan host tersebut.
    const sebutHost = HOST_AGREGATOR.some((h) => isi.includes(h));
    const importAggregator = /from\s+["'][^"']*lib\/aggregator\.js["']/.test(isi);
    if (sebutHost && !importAggregator) tanpaAggregator.push(label);
  }

  assert.deepEqual(
    axiosLangsung,
    [],
    `kapabilitas memanggil transport langsung, bukan lewat aggregator.hit (${axiosLangsung.length} file):\n  - ` +
      axiosLangsung.join("\n  - "),
  );
  assert.deepEqual(
    tanpaAggregator,
    [],
    `kapabilitas menyebut host agregator tanpa mengimpor lib/aggregator.js — request-nya keluar tanpa breaker dan tanpa budget (${tanpaAggregator.length} file):\n  - ` +
      tanpaAggregator.join("\n  - "),
  );
});

// ── test 3: daftar host tidak boleh melenceng dari aggregator.js ────────────
//
// Tanpa test ini, menambahkan host baru ke src/lib/aggregator.js cukup untuk
// membuat ratchet di atas buta buta: plugin yang memakai host baru tidak akan
// terdeteksi, dan test tetap hijau karena host itu tidak ada di HOST_AGREGATOR.

test("ratchet: HOST_AGREGATOR mencakup semua host di src/lib/aggregator.js", async () => {
  const isi = await readFile(join(AKAR, "src", "lib", "aggregator.js"), "utf8");
  const dariAggregator = [...isi.matchAll(/base:\s*"(https?:\/\/[^"]+)"/g)].map((m) =>
    new URL(m[1]).hostname,
  );

  // aggregator.js boleh punya host yang TIDAK ada di sini (alias provider di
  // apimanager.js boleh menunjuk host lain), tapi tidak boleh ada host yang hilang:
  // itulah arah yang membuat deteksi bisa dilewati.
  const hilang = dariAggregator.filter((h) => !HOST_AGREGATOR.includes(h));
  assert.deepEqual(
    hilang,
    [],
    `host baru di src/lib/aggregator.js belum masuk HOST_AGREGATOR di test ini:\n  - ` +
      hilang.join("\n  - "),
  );
  assert.ok(dariAggregator.length > 0, "tidak ada host yang ter-parse dari aggregator.js");
});
