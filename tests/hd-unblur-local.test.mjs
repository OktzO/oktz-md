import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini menguji DISPATCH kapabilitas `hd`: `kind` menentukan scraper lokal
// mana yang dipanggil. `src/scraper/hd.js` dan `src/scraper/hdvid.js` di-mock
// karena keduanya menembak host yang butuh kunci atau sudah tidak hidup.
// Bentuk yang masing-masing kembalikan diturunkan dari kode scraper itu sendiri
// dan dijelaskan di blok Task 10 pada tests/capabilities-phase1.test.mjs.
//
// Yang dipalsukan hanya scrapernya. Modul kapabilitas dan resolver sungguhan,
// sehingga `applies`, urutan tier, dan budget ikut teruji.
//
// `aggregator` juga di-mock dan default-nya melempar: kalau sebuah test salah
// menghitung urutan tier, tanpa stub ini ia akan menembak host sungguhan.
// Test yang memang memeriksa tier api sebagai jawaban memasang `balasAggregator`
// sendiri secara eksplisit.

const PANGGILAN = [];

let hasilUpload = { code: "lgIhxcRf", imageId: "1", type: 13 };
let hasilCheck = { downloadUrls: ["https://photoai.imglarger.com/upscaler/lgIhxcRf.jpg"], status: "success" };
let polGagal = false;
// 0 = polling instan. Diisi >0 untuk menyimulasikan render yang butuh waktu.
let polLambatMs = 0;
let hasilUnblur = {
  taskId: "t-1",
  createdAt: "2026-10-03T00:00:00Z",
  pollUrl: "https://fgsi.example/poll/t-1",
  res_url: "https://cdn.example/jelas.jpg",
};

mock.module("../src/scraper/hd.js", {
  namedExports: {
    async upload(filePath) {
      PANGGILAN.push({ fungsi: "upload", filePath });
      return hasilUpload;
    },
    async get(code) {
      PANGGILAN.push({ fungsi: "get", code });
      return hasilCheck;
    },
  },
});

mock.module("../src/scraper/hdvid.js", {
  namedExports: {
    async createEnhanceTask(filePath, apiKey) {
      PANGGILAN.push({ fungsi: "createEnhanceTask", filePath, apiKey });
      return { taskId: "t-1", createdAt: "2026-10-03T00:00:00Z", pollUrl: "https://fgsi.example/poll/t-1" };
    },
    async pollEnhanceTask(pollUrl, opsi) {
      PANGGILAN.push({ fungsi: "pollEnhanceTask", pollUrl, opsi });
      // Polling host sungguhan memang berulang beberapa detik; di sini satu
      // putaran supaya test tidak ikut menunggu — kecuali `polLambatMs` diisi,
      // untuk membuktikan resolver menyerah, bukan host-nya yang menyerah.
      if (polLambatMs > 0) {
        await new Promise((r) => setTimeout(r, polLambatMs));
      }
      if (polGagal) throw new Error("Polling HD video gagal");
      return hasilUnblur;
    },
  },
});

// Default-nya melempar supaya test yang salah menghitung urutan tier tidak
// menembak host sungguhan diam-diam. `balasAggregator` dipakai hanya oleh test
// yang memang sedang memeriksa tier api sebagai jawaban.
let balasAggregator = () => {
  throw new Error("aggregator tidak boleh dipanggil: urutan tier salah di test ini");
};

mock.module("../src/lib/aggregator.js", {
  namedExports: {
    aggregator: {
      hit: async (name, path) => {
        PANGGILAN.push({ fungsi: "aggregator", name, path });
        return balasAggregator(name, path);
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

const kap = await impor("../src/capabilities/hd.js");
const { createResolver } = await impor("../src/lib/resolve.js");

// `src/capabilities/hd.js` membaca `config.APIkey.fgsi` saat runtime, jadi test
// key harus memakai instance config yang sama — mengimpor ulang berkas lain hanya
// menghasilkan salinan lain.
const configUji = (await impor("../config.js"))?.default;

const unblur = kap?.backends.find((b) => b.name === "fgsi-enchantvideo");
const imglarger = kap?.backends.find((b) => b.name === "photoai-imglarger");

test("sentinel: modul hd, resolver, dan config benar-benar terimpor", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof kap?.normalize, "function");
  assert.equal(typeof unblur?.run, "function", "backend fgsi-enchantvideo tidak terbaca");
  assert.equal(typeof imglarger?.run, "function", "backend photoai-imglarger tidak terbaca");
  assert.equal(typeof createResolver, "function", "resolver tidak terimpor");
  assert.ok(configUji?.APIkey, "config.js tidak terimpor");
});

function resolverUji(opsi = {}) {
  return createResolver({ capabilities: { hd: () => kap }, ...opsi });
}

const MEDIA = path.join(os.tmpdir(), "hd-unblur-uji.jpg");

// Backend menolak file yang tidak ada sebelum scraperMana pun dipanggil, jadi
// berkas benar-benar harus ada di disk — bukan hanya string.
fs.writeFileSync(MEDIA, Buffer.from("bukan jpeg sungguhan, tapi cukup untuk di-path-kan", "utf8"));
process.on("exit", () => {
  try {
    fs.unlinkSync(MEDIA);
  } catch {}
});

// Test key di bawah mengubah `config.APIkey.fgsi`, jadi nilainya dipulihkan
// setelah setiap test — kalau tidak, test lain di berkas ini ikut terpengaruh
// lewat urutan, dan itu akan terlihat sebagai kegagalan yang intermittent.
const keyFgsiAsli = configUji.APIkey.fgsi;
beforeEach(() => {
  PANGGILAN.length = 0;
  configUji.APIkey.fgsi = keyFgsiAsli;
  hasilUpload = { code: "lgIhxcRf", imageId: "1", type: 13 };
  hasilCheck = { downloadUrls: ["https://photoai.imglarger.com/upscaler/lgIhxcRf.jpg"], status: "success" };
  polGagal = false;
  polLambatMs = 0;
  balasAggregator = () => {
    throw new Error("aggregator tidak boleh dipanggil: urutan tier salah di test ini");
  };
  hasilUnblur = {
    taskId: "t-1",
    createdAt: "2026-10-03T00:00:00Z",
    pollUrl: "https://fgsi.example/poll/t-1",
    res_url: "https://cdn.example/jelas.jpg",
  };
});

test("kind imglarger memakai upload lalu get, dan tidak menyentuh scraper video", async () => {
  const keluar = await resolverUji().resolve("hd", { kind: "imglarger", media: MEDIA });

  assert.equal(keluar.source, "photoai-imglarger");
  assert.deepEqual(keluar.data, { url: "https://photoai.imglarger.com/upscaler/lgIhxcRf.jpg" });
  assert.deepEqual(
    PANGGILAN.map((c) => c.fungsi),
    ["upload", "get"],
    "imglarger hanya boleh memakai upload + get",
  );
  assert.equal(PANGGILAN[0].filePath, MEDIA, "path file harus diteruskan apa adanya");
  // `code` dari `upload` adalah satu-satunya hal yang membuat `get(code)` berguna.
  assert.equal(PANGGILAN[1].code, "lgIhxcRf");
});

test("kind unblur memakai createEnhanceTask lalu pollEnhanceTask", async () => {
  const keluar = await resolverUji().resolve("hd", { kind: "unblur", media: MEDIA });

  assert.equal(keluar.source, "fgsi-enchantvideo");
  assert.deepEqual(keluar.data, { url: "https://cdn.example/jelas.jpg" });
  assert.deepEqual(
    PANGGILAN.map((c) => c.fungsi),
    ["createEnhanceTask", "pollEnhanceTask"],
    "unblur hanya boleh memakai createEnhanceTask + pollEnhanceTask",
  );
  assert.equal(PANGGILAN[1].pollUrl, "https://fgsi.example/poll/t-1", "pollUrl harus diteruskan");
});

test("polling memakai key yang sama dengan pembuatan task, dan tidak memakai default 10 menit", async () => {
  // `createEnhanceTask` menerima key dari `config.APIkey.fgsi` (`APIKEY_FGSI`),
  // sedangkan default `pollEnhanceTask` membaca alias lama `FGSI_API_KEY`.
  // Kalau key tidak ikut diteruskan, task berhasil dibuat lalu polling ditolak —
  //-gejala yang jauh lebih sulit dibaca daripada "backend mati".
  //
  // `timeoutMs` juga dipin: default scraper 10 menit dipakai plugin `hdvid`
  // untuk video, dan polling yang sudah kalah race dari budget resolver tidak
  // boleh menggantung sockétibeberapa menit di kotak 1GB.
  const opts = { apiKey: "k-fgsi-untuk-test", timeoutMs: 60_000 };
  configUji.APIkey.fgsi = "k-fgsi-untuk-test";

  await unblur.run({ kind: "unblur", media: MEDIA }, {});

  const buat = PANGGILAN.find((c) => c.fungsi === "createEnhanceTask");
  const poll = PANGGILAN.find((c) => c.fungsi === "pollEnhanceTask");
  assert.equal(buat.apiKey, opts.apiKey, "pembuatan task memakai key dari config");
  assert.equal(poll.opsi?.apiKey, opts.apiKey, "polling harus memakai key yang sama");
  assert.equal(poll.opsi?.timeoutMs, opts.timeoutMs, "polling yatim tidak boleh menunggu 10 menit");
  assert.notEqual(poll.opsi?.timeoutMs, 10 * 60 * 1000, "default 10 menit tidak boleh dipakai di sini");
});

test("key kosong tidak dikirim sebagai string kosong ke scraper", async () => {
  // `createEnhanceTask(filePath, apiKey = DEFAULT_API_KEY)` memakai nilai
  // default HANYA kalau argumennya `undefined`. Mengirim `""` menimpanya dengan
  // string kosong dan membuat header `apikey` berisi nol byte — berbeda dari
  // "tidak ada key", yang membiarkan scraper memakai default env-nya sendiri.
  // Polling punya bentuk masalah sama lewat opsi: `{ apiKey: "" }` menimpa
  // default `pollEnhanceTask`, jadi opsi key harus DIHILANGKAN, bukan dikosongkan.
  configUji.APIkey.fgsi = "   ";

  await unblur.run({ kind: "unblur", media: MEDIA }, {});

  const poll = PANGGILAN.find((c) => c.fungsi === "pollEnhanceTask");
  assert.equal("apiKey" in (poll.opsi ?? {}), false, "opsi key harus dihilangkan agar default scraper berlaku");
  assert.equal(
    PANGGILAN.find((c) => c.fungsi === "createEnhanceTask").apiKey,
    undefined,
    "argumen key harus `undefined`, bukan string kosong",
  );
});

test("upload tanpa code tidak memanggil get — tidak ada yang bisa diprobe", async () => {
  hasilUpload = {};
  await assert.rejects(() => resolverUji().resolve("hd", { kind: "imglarger", media: MEDIA }), /semua backend hd gagal/);
  assert.deepEqual(
    PANGGILAN.filter((c) => c.fungsi !== "aggregator").map((c) => c.fungsi),
    ["upload"],
    "get(code) tanpa code hanya membuang satu request",
  );
});

test("hasil upload yang hanya punya `data` tidak dibaca sebagai code", async () => {
  // `upload()` di src/scraper/hd.js sudah membuka satu amplop `data.data`.
  // Membaca `data.code` di sini akan menghasilkan null, lalu diam-diam
  // melewati polling.
  hasilUpload = { data: { code: "lgIhxcRf" } };
  await assert.rejects(() => resolverUji().resolve("hd", { kind: "imglarger", media: MEDIA }), /semua backend hd gagal/);
  assert.equal(PANGGILAN.some((c) => c.fungsi === "get"), false, "code harus dibaca dari field yang benar");
});

test("polling gagal menjadi kegagalan backend, lalu tier api tetap dicoba", async () => {
  polGagal = true;
  await assert.rejects(
    () => resolverUji().resolve("hd", { kind: "unblur", media: MEDIA }),
    (error) => {
      assert.deepEqual(error.tried.map((t) => t.name), ["fgsi-enchantvideo", "izuka"]);
      return true;
    },
  );
  assert.equal(PANGGILAN.at(-1).fungsi, "aggregator", "cadangan aggregator harus tetap dicoba");
});

test("polling TIDAK diikat ke signal resolver: abort pada ctx tidak menghentikan poll", async () => {
  // Ruling 3: yang dibudget `resolve()` hanya pembuatan task. Kalau `ctx.signal`
  // ikut masuk ke polling, seluruh budget habis menunggu render dan backend
  // aggregator tidak pernah sempat dicoba — persis kebalikan dari tujuan Fase 1.
  //
  // Test ini memanggil backend-nya langsung dengan signal yang SUDAH di-abort.
  // Backend yang mengikat polling ke signal akan gagal di sini; yang benar
  // tetap menyelesaikan polling dan mengembalikan URL.
  const controller = new AbortController();
  controller.abort();

  const keluar = await unblur.run({ kind: "unblur", media: MEDIA }, { signal: controller.signal });

  assert.deepEqual(keluar, { res_url: "https://cdn.example/jelas.jpg", taskId: "t-1", createdAt: "2026-10-03T00:00:00Z", pollUrl: "https://fgsi.example/poll/t-1" });
  assert.equal(
    PANGGILAN.find((c) => c.fungsi === "pollEnhanceTask").opsi?.signal,
    undefined,
    "polling sengaja tidak menerima signal apa pun",
  );
});

test("abort pada ctx juga tidak boleh mengikat upload", async () => {
  // `upload(filePath)` di src/scraper/hd.js tidak menerima opsi sama sekali,
  // jadi tidak ada jalur untuk mengikatnya. Test ini mengunci kenyataan itu
  // supaya dokumentasinya tidak basi: kalau scraper suatu saat menerima signal,
  // test ini wajib ditulis ulang.
  const controller = new AbortController();
  await imglarger.run({ kind: "imglarger", media: MEDIA }, { signal: controller.signal });
  assert.deepEqual(
    PANGGILAN.map((c) => c.fungsi),
    ["upload", "get"],
  );
});

test("budget yang habis di tengah polling tetap membuka jalan ke tier api", async () => {
  // Konsekuensi nyata dari "polling di luar budget": `resolve()` menyerah sesuai
  // budgetnya lalu pindah backend. Yang dijamin di sini adalah hal yang penting:
  // aggregator DIDAPAT giliran, sehingga cadangan aggregator tidak pernah hilang
  // hanya karena render lokal berjalan lama.
  //
  // Polling di sini memakai backend UNBLUR YANG SUNGGUHNYA, bukan kapabilitas
  // tiruan. Versi pertama test ini memakai objek backend buatan yang menghormati
  // signal — itu hanya membuktikan resolver bisa membatalkan, bukan bahwa
  // `pollEnhanceTask` yang tidak menerima signal tetap menyisakan jalan ke tier
  // api. Yang diuji di sini justru kebalikannya: polling yang TIDAK bisa
  // dibatalkan harus tetap melepaskan resolver sesuai budgetnya.
  polLambatMs = 120;
  balasAggregator = async () => ({ status: true, result: { output_url: ["https://cdn.example/naik.jpg"] } });
  const mulai = Date.now();
  const keluar = await resolverUji({ budget: { localMs: 40, totalMs: 600 } }).resolve("hd", {
    kind: "unblur",
    media: MEDIA,
  });

  assert.equal(keluar.source, "izuka", "polling lambat tidak boleh menahan tier aggregator");
  assert.deepEqual(keluar.data, { url: "https://cdn.example/naik.jpg" });
  assert.ok(
    Date.now() - mulai < 1000,
    "resolve() harus menyerah sesuai budget, bukan menunggu polling selesai",
  );
  assert.ok(
    PANGGILAN.some((c) => c.fungsi === "pollEnhanceTask"),
    "polling memang sempat dimulai — ini buktinya jalan, bukan sekadar tidak dibaca",
  );
  assert.equal(PANGGILAN.at(-1).fungsi, "aggregator", "aggregator harus dipanggil paling akhir");
});