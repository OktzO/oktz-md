import { test } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Kedelapan modul kapabilitas diimpor LANGSUNG, bukan lewat `resolve()`.
// `resolve()` memang tidak menarik request apa pun sampai ada backend yang
// `run`, tapi ia juga memuat breaker, cache, dan budget — tiga hal yang tidak
// sedang diuji di sini. File yang hilang atau impor yang gagal harus
// menggagalkan test ini dengan pesan yang menyebut namanya, bukan diam.
//
// Modul diimpor satu per satu di dalam try/catch, bukan dengan import statis.
// Alasannya ada di blok sentinel di bawah: `src/capabilities/hd.js` mengimpor
// `config.js`, dan rantai itu config.js → src/lib/database.js → src/lib/lid.js,
// yang memasang `process.on("uncaughtException", …)` PENELAN di lid.js:61.
// Dengan import statis, satu impor yang gagal tidak akan menggagalkan berkas:
// proses keluar 0 dan node:test melaporkan "pass 1" dengan nol test di dalamnya.
// Persis mode kegagalan yang merusak review Task 4, saat blok proteksi RAM
// dihapus dan semua test tetap hijau. Bentuk try/catch + sentinel di bawah
// sudah dipakai tests/sfiledl-plugin.test.mjs dan di sana terbukti bekerja.

const SPESIFIKASI = {
  spotify: "../src/capabilities/spotify.js",
  pinterest: "../src/capabilities/pinterest.js",
  douyin: "../src/capabilities/douyin.js",
  sfile: "../src/capabilities/sfile.js",
  videy: "../src/capabilities/videy.js",
  youtube: "../src/capabilities/youtube.js",
  ytmusic: "../src/capabilities/ytmusic.js",
  hd: "../src/capabilities/hd.js",
};

const NAMA = Object.keys(SPESIFIKASI);
const MODUL = new Map();
const GALAT_IMPOR = new Map();
for (const nama of NAMA) {
  try {
    const mod = await import(SPESIFIKASI[nama]);
    MODUL.set(nama, mod?.default ?? mod);
  } catch (error) {
    GALAT_IMPOR.set(nama, error);
  }
}

// ── perilaku normalize pada input kosong ──────────────────────────────────────
//
// Empat input ini diuji sebagai satu kelompok karena semuanya satu kelas
// kegagalan: bentuk respons yang tidak berarti apa-apa. Kalau salah satu
// diteruskan apa adanya ke plugin, plugin menjalankan `sendMedia` atau
// `sendMessage` tanpa file dan tetap memberi centang hijau ke user yang tidak
// menerima apa-apa. Reviewer sudah dua kali menangkap output `normalize` yang
// terisi sebagian selama Phase 1, jadi ini yang dikunci.
//
// `null`, `undefined`, dan `{}` dilempar oleh KEDELAPAN modul. `[]` tidak
// seragam, dan itu bukan ketidaktelitian:
//
//   - Pencarian (spotify/pinterest/ytmusic) memakai array sebagai respons yang
//     sah. Daftar kosong berarti "tidak ada yang cocok", dan plugin harus bisa
//     mencetak itu. Melemparnya membuat plugin melaporkan "gagal" untuk
//     pencarian yang memang tidak menemukan apa pun — kesalahan yang lebih
//     buruk daripada pesan kosong yang dicegah. Yang diuji di sini bukan hanya
//     "tidak melempar", tapi juga bahwa bentuk yang lolos benar-benar BENTUK
//     LENGAP: daftar kosong, bukan objek dengan field yang tidak terisi.
//
//   - Unduhan (douyin/sfile/videy/youtube/hd) tidak punya bentuk daftar sama
//     sekali. `[]` di sana berarti backend mengembalikan amplop kosong, dan
//     meneruskannya menghasilkan `sendMedia` tanpa file. Harus jadi kegagalan.
//
// Nilai di `ARRAY_KOSONG` di bawah bukan tebakan: semuanya diambil dengan
// menjalankan `normalize([])` langsung ke delapan modul pada 2026-10-04, dan
// hasil itulah yang dikunci di sini. Kalau sebuah modul berubah bentuk, test ini
// gagal dan itu memang yang kita mau terjadi.
const ARRAY_KOSONG = {
  spotify: { tracks: [] },
  pinterest: { pins: [] },
  ytmusic: { tracks: [] },
};

const INPUT_KOSONG = [
  ["null", null],
  ["undefined", undefined],
  ["{}", {}],
];

// ── sentinel ─────────────────────────────────────────────────────────────────
//
// Test PERTAMA yang terdaftar. Kalau impor gagal, `MODUL` kosong dan semua test
// di bawah akan lulus dengan nol assertion — tanpa baris ini.

test("sentinel: kedelapan modul kapabilitas benar-benar terimpor", () => {
  const rusak = [...GALAT_IMPOR].map(
    ([nama, error]) => `${nama} (${SPESIFIKASI[nama]}): ${error?.message ?? error}`,
  );
  assert.deepEqual(
    [...GALAT_IMPOR.keys()],
    [],
    `impor kapabilitas gagal:\n  - ${rusak.join("\n  - ") || "tidak ada"}`,
  );
  assert.equal(MODUL.size, NAMA.length, `hanya ${MODUL.size}/${NAMA.length} modul terimpor`);
  for (const nama of NAMA) {
    assert.ok(MODUL.get(nama), `modul ${nama} tidak ada di peta`);
  }
});

// ── bentuk ekspor ────────────────────────────────────────────────────────────

test("kontrak: setiap kapabilitas mengekspor backends, normalize, dan stable", () => {
  // Semua pelanggaran dikumpulkan lalu diasersikan SEKALI, supaya satu kali jalan
  // menampilkan seluruh kontrak yang rusak, bukan hanya yang pertama.
  const rusak = [];
  for (const nama of NAMA) {
    const cap = MODUL.get(nama);
    if (!cap) continue; // sentinel sudah menggagalkan; di sini bukan penjaga.

    if (!("backends" in cap)) {
      rusak.push(`${nama}: tidak mengekspor \`backends\``);
    } else if (!Array.isArray(cap.backends)) {
      rusak.push(`${nama}: \`backends\` bukan array (${typeof cap.backends})`);
    } else if (cap.backends.length === 0) {
      // resolve.js:159 sudah melempar `no-backend` untuk array kosong, tapi itu
      // baru terlihat saat runtime. Di sini kegagalannya bisa dibaca saat review.
      rusak.push(`${nama}: \`backends\` kosong — kapabilitas tidak punya jalur apa pun`);
    }

    if (typeof cap.normalize !== "function") {
      rusak.push(`${nama}: \`normalize\` bukan function`);
    }

    // `stable` menentukan apakah hasil boleh keluar dari cache resolver, jadi
    // nilainya harus boolean murni. `undefined` diam-diam berarti "tidak
    // pernah di-cache" (lihat resolve.js:176), jadi typos seperti
    // `export const stable` tanpa `= false` akan lolos tanpa terlihat.
    if (typeof cap.stable !== "boolean") {
      rusak.push(`${nama}: \`stable\` bukan boolean (${JSON.stringify(cap.stable)})`);
    }
  }
  assert.deepEqual(
    rusak,
    [],
    `bentuk ekspor tidak sesuai kontrak:\n  - ${rusak.join("\n  - ") || "tidak ada"}`,
  );
});

// ── bentuk backend ───────────────────────────────────────────────────────────

test("kontrak: setiap backend punya name, kind, dan run yang bisa dipakai", () => {
  const rusak = [];
  for (const nama of NAMA) {
    const cap = MODUL.get(nama);
    if (!cap || !Array.isArray(cap.backends)) continue;

    cap.backends.forEach((b, i) => {
      const label = `capabilities/${nama}.js backends[${i}]${typeof b?.name === "string" ? ` (${b.name})` : ""}`;
      if (!b || typeof b !== "object") {
        rusak.push(`${label}: bukan objek`);
        return;
      }
      // `name` adalah kunci state circuit breaker (circuit-breaker.js menyimpan
      // state per nama), jadi string kosong berarti semua backend tanpa nama
      // berbagi satu state.
      if (typeof b.name !== "string" || b.name.trim() === "") {
        rusak.push(`${label}: \`name\` harus string tidak kosong`);
      }
      // `kind` bukan string bebas: resolve.js:98-101 menyusun antrean dengan
      // `b.kind === "local"` lalu semua sisanya. `kind: "lokal"` yang salah
      // ketik akan diam-diam menaruh aggregator DI DEPAN backend lokal — dan
      // tidak ada test lain yang akan menangkapnya.
      if (b.kind !== "local" && b.kind !== "api") {
        rusak.push(`${label}: \`kind\` harus "local" atau "api", bukan ${JSON.stringify(b.kind)}`);
      }
      if (typeof b.run !== "function") {
        rusak.push(`${label}: \`run\` bukan function`);
      }
    });
  }
  assert.deepEqual(
    rusak,
    [],
    `backend bermasalah:\n  - ${rusak.join("\n  - ") || "tidak ada"}`,
  );
});

// ── jaminan kemandirian ──────────────────────────────────────────────────────

test("mandiri: setiap kapabilitas punya minimal satu backend kind 'local'", () => {
  // Inilah yang dijanjikan Phase 1. Tanpa satu backend `local`, kapabilitas itu
  // secara struktural hanya bisa dijawab agregator, dan Phase 2-6 tidak akan
  // mengubahnya. Kegagalan di sini berarti seluruh Fase 1 tidakbertoleh.
  const tanpa = [];
  for (const nama of NAMA) {
    const cap = MODUL.get(nama);
    if (!cap || !Array.isArray(cap.backends)) continue;
    if (!cap.backends.some((b) => b?.kind === "local")) {
      tanpa.push(`${nama} (${cap.backends.map((b) => `${b?.name}:${b?.kind}`).join(", ")})`);
    }
  }
  assert.deepEqual(
    tanpa,
    [],
    `kapabilitas tanpa backend local:\n  - ${tanpa.join("\n  - ") || "tidak ada"}`,
  );
});

test("breaker: nama backend unik dalam satu kapabilitas", () => {
  // Circuit breaker menyimpan state per NAMA dan resolve.js:216 memanggil
  // `recordFailure(backend.name)`. Dua backend dengan nama sama berarti satu host
  // yang sehat bisa dipandang OPEN karena kegagalan backend lain, dan `snapshot()`
  // menghitung satu state untuk dua host — jumlah gagalnya salah tanpa ada error
  // sama sekali. Aturan "nama backend per host" ada di brief Task 2 dan test ini
  // adalah tempat ia ditegakkan.
  const bentrok = [];
  for (const nama of NAMA) {
    const cap = MODUL.get(nama);
    if (!cap || !Array.isArray(cap.backends)) continue;
    const terlihat = new Set();
    const dobel = new Set();
    for (const b of cap.backends) {
      const key = typeof b?.name === "string" ? b.name : JSON.stringify(b?.name);
      if (terlihat.has(key)) dobel.add(key);
      terlihat.add(key);
    }
    for (const key of dobel) bentrok.push(`${nama}: "${key}" dipakai lebih dari satu backend`);
  }
  assert.deepEqual(
    bentrok,
    [],
    `nama backend ganda:\n  - ${bentrok.join("\n  - ") || "tidak ada"}`,
  );
});

// ── normalize: menolak input yang tidak bermakna ─────────────────────────────

test("normalize: menolak null, undefined, dan {} di kedelapan kapabilitas", () => {
  const lolos = [];
  for (const nama of NAMA) {
    const cap = MODUL.get(nama);
    if (!cap || typeof cap.normalize !== "function") continue;
    for (const [label, nilai] of INPUT_KOSONG) {
      let melempar = false;
      try {
        cap.normalize(nilai);
      } catch {
        melempar = true;
      }
      if (!melempar) lolos.push(`${nama}: normalize(${label}) tidak melempar`);
    }
  }
  assert.deepEqual(
    lolos,
    [],
    `normalize menerima input kosong:\n  - ${lolos.join("\n  - ") || "tidak ada"}`,
  );
});

test("normalize: array kosong hanya lolos pada kapabilitas yang memang berbentuk daftar", () => {
  const salah = [];
  for (const nama of NAMA) {
    const cap = MODUL.get(nama);
    if (!cap || typeof cap.normalize !== "function") continue;
    const daftar = ARRAY_KOSONG[nama];

    // Jalur unduhan: `[]` tidak punya bentuk yang sah, jadi harus jadi kegagalan.
    if (daftar === undefined) {
      let melempar = false;
      try {
        cap.normalize([]);
      } catch {
        melempar = true;
      }
      if (!melempar) {
        salah.push(`${nama}: normalize([]) seharusnya melempar — jalur unduh tidak punya bentuk daftar`);
      }
      continue;
    }

    // Jalur pencarian: boleh lolos, tapi hanya sebagai daftar kosong yang utuh.
    let keluar;
    try {
      keluar = cap.normalize([]);
    } catch (error) {
      salah.push(`${nama}: normalize([]) melempar — "${error?.message ?? error}"`);
      continue;
    }
    try {
      assert.deepEqual(keluar, daftar);
    } catch {
      salah.push(
        `${nama}: normalize([]) menghasilkan ${JSON.stringify(keluar)}, bukan bentuk lengkap ${JSON.stringify(daftar)}`,
      );
    }
  }
  assert.deepEqual(
    salah,
    [],
    `perlakuan array kosong tidak konsisten:\n  - ${ salah.join("\n  - ") || "tidak ada"}`,
  );
});

// ── normalize: bentuk yang diterima harus menghasilkan URL yang bisa dikirim ──
//
// Test di atas membuktikan `normalize` MENOLAK yang tidak bermakna. Yang ini
// membuktikan sisi sebaliknya, dan kelas kegagalannya BERBEDA: bentuk respons
// yang memang sah harus tetap bisa dinormalisasi.
//
// Yang di-catching di sini adalah backend lokal yang berhenti bekerja. Kalau
// `kandidat()` di hd.js kehilangan satu nama field, atau `urlVideo` di videy.js
// menolak satu amplop, `normalize` tetap melempar untuk `{}` dan `[]` — dua
// assertion di atas tetap hijau — tapi backend lokal mulai selalu gagal untuk
// respons aslinya, dan setiap permintaan diam-diam naik ke tier aggregator.
// Itu kemunduran kemandirian tanpa satu pun error di log, jadi harus dikunci.
//
// Bentuk-bentuk di bawah adalah nama field yang benar-benar dibaca tiap modul,
// bukan tebakan: `downloadUrls` + `status` dari `get(code)` (imglarger),
// `res_url` dari `pollEnhanceTask` (hdvid), `result.output_url` dari izuka unblur
// (`git show ff397d2^:plugins/tools/hd3.js:63`), `data.url` dari neoxr videy,
// dan `video`/`download_url` sebagai string URL dari scraper douyin dan sfiledl.
const BENTUK_SAH = {
  videy: [{ url: "https://cdn.example/a.mp4" }, { data: { url: "https://cdn.example/a.mp4" } }],
  youtube: [{ status: true, url: "https://cdn.example/a.mp4" }, { result: { url: "https://cdn.example/a.mp4" } }],
  hd: [
    { url: "https://cdn.example/a.png" },
    { downloadUrls: ["https://cdn.example/a.png"], status: "success" },
    { res_url: "https://cdn.example/a.png" },
    { result: { output_url: ["https://cdn.example/a.png"] } },
  ],
  sfile: [
    { download_url: "https://d.example/a.apk", file_name: "a.apk" },
    { url: "https://d.example/a.apk", filename: "a.apk" },
  ],
  douyin: [{ status: true, video: "https://v.example/a.mp4", title: "x", platform: "douyin" }],
};

// Nama field yang dibaca plugin, bukan asumsi bahwa semuanya `url`. douyin
// menyodorkan `video` (src/scraper/douyin.js:45 `video: video ? video.url : null`)
// dan `audio` boleh kosong karena post tanpa trek audio itu nyata — jadi `video`
// yang wajib terisi, bukan `url` yang tidak ada di kontrak kapabilitas ini.
const FIELD_UNDAH = { videy: "url", youtube: "url", hd: "url", sfile: "url", douyin: "video" };

test("normalize: bentuk yang sah harus menghasilkan url http yang terisi", () => {
  const salah = [];
  for (const [nama, bentuk] of Object.entries(BENTUK_SAH)) {
    const cap = MODUL.get(nama);
    if (!cap || typeof cap.normalize !== "function") continue;
    for (const raw of bentuk) {
      let keluar;
      try {
        keluar = cap.normalize(raw);
      } catch (error) {
        salah.push(`${nama}: normalize(${JSON.stringify(raw)}) melempar — "${error?.message ?? error}"`);
        continue;
      }
      const field = FIELD_UNDAH[nama];
      const url = keluar?.[field];
      if (typeof url !== "string" || !/^https?:\/\/\S+$/i.test(url)) {
        salah.push(
          `${nama}: normalize(${JSON.stringify(raw)}) → ${JSON.stringify(keluar)} — \`${field}\` bukan URL http yang terisi`,
        );
      }
    }
  }
  assert.deepEqual(
    salah,
    [],
    `bentuk sah tidak menghasilkan url yang bisa dikirim:\n  - ${salah.join("\n  - ") || "tidak ada"}`,
  );
});
