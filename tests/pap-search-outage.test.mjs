import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/search/pap.js` sungguhan, karena yang
// diuji adalah PERILAKU yang dilihat user: kapabilitas mati harus berbeda dari
// hasil kosong. `fetchImageUrls` dulu menelan setiap throw dan mengembalikan `[]`,
// jadi aggregator yang mati dilaporkan sebagai "pap kosong" — persis pesan yang
// menyuruh mereka menunggu retry ke host yang baru saja gagal.
//
// Uji lapis plugin dipisah dari tests/capabilities-phase1.test.mjs karena berkas itu
// sudah `mock.module` untuk `src/lib/http.js` dan `axios`, dan satu modul tidak
// boleh di-mock dua kali dalam satu berkas.
//
// Resolver di-mock, jadi tidak ada satu pun request yang keluar ke jaringan.

const PANGGILAN = [];
let jawabPinterest = () => {
  throw new Error("pinterest tidak boleh dipanggil di test ini");
};

mock.module("../src/lib/resolve.js", {
  namedExports: {
    resolver: {
      resolve: async (capability, args) => {
        PANGGILAN.push({ capability, args });
        return jawabPinterest(capability, args);
      },
    },
  },
});

// Record-then-throw supaya test bisa membuktikan bahwa kapabilitas yang hidup
// benar-benar dipakai, bukan hanya tidak melempar.
const UNDUHAN = [];
mock.module("axios", {
  defaultExport: {
    async get(url) {
      UNDUHAN.push(url);
      throw new Error(`axios tidak boleh jatuh di test ini: ${url}`);
    },
  },
});

// `onigis` sengaja TIDAK di-mock. Rantai impor yang dilalui `src/lib/error.js` →
// `config.js` menarik modul yang butuh banyak export onigis sekaligus, dan mock
// parsial akan menggagalkan impor itu (sentinel di bawah yang menangkapnya). Handler
// yang diuji juga tidak pernah memanggil onigis: pada jalur hasil kosong maupun
// padam, eksekusi berhenti sebelum `prepareWAMessageMedia`.

// ── sentinel import ─────────────────────────────────────────────────────────
//
// Rantai impor di berkas ini melewati `src/lib/error.js` → `config.js` →
// `src/lib/database.js` → `src/lib/lid.js`, dan `src/lib/lid.js:61` memasang
// `process.on("uncaughtException", …)` yang menelan semua error — termasuk yang
// dilempar modul yang gagal diimpor. Akibatnya impor yang gagal TIDAK menggagalkan
// berkas: proses keluar 0 dan node:test melaporkan `pass 1` dengan nol test.
//
// Pola yang sama dipakai tests/applemusic-plugin.test.mjs,
// tests/sfiledl-plugin.test.mjs, dan tests/pindl-plugin.test.mjs. `src/lib/lid.js`
// sendiri tidak diubah di sini: itu penanganan crash produksi dan di luar cakupan.
const GALAT_IMPOR = [];

async function impor(specifier) {
  try {
    return await import(specifier);
  } catch (error) {
    GALAT_IMPOR.push(`${specifier} → ${error?.code ?? "?"}: ${error?.message ?? error}`);
    return null;
  }
}

const pap = await impor("../plugins/search/pap.js");

test("sentinel: plugin pap benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof pap?.handler, "function");
});

// ── fake user + error ────────────────────────────────────────────────────────

/** `CapabilityError` seperti yang benar dilempar resolver. */
function gagalSemua(tried) {
  const error = new Error("semua backend pinterest gagal");
  error.name = "CapabilityError";
  error.capability = "pinterest";
  error.code = "unresolved";
  error.tried = tried;
  return error;
}

/** Pesan aggregator/kapabilitas yang mati — batas waktu, bukan "tidak ada pin". */
const MATI = { name: "ilovepin", reason: "batas waktu 1500ms habis" };

function pesanPengguna(tipe) {
  const balasan = [];
  const reaksi = [];
  return {
    balasan,
    reaksi,
    m: {
      args: [tipe],
      prefix: ".",
      command: "pap",
      pushName: "Tester",
      chat: "chat@s.whatsapp.net",
      react: async (emoji) => reaksi.push(emoji),
      reply: async (teks) => balasan.push(String(teks)),
    },
  };
}

/** Tangkap `console.error` supaya bisa diperiksa dan tidak mencemari output test. */
async function tangkapLog(jalankan) {
  const asli = console.error;
  const baris = [];
  console.error = (...args) => baris.push(args);
  try {
    await jalankan();
  } finally {
    console.error = asli;
  }
  return baris;
}

const SEBUAH = { ok: true, source: "ilovepin", meta: { tookMs: 1, cached: false } };
const TIDAK_ADA_PIN = { ...SEBUAH, data: { pins: [] } };

beforeEach(() => {
  PANGGILAN.length = 0;
  UNDUHAN.length = 0;
  jawabPinterest = () => TIDAK_ADA_PIN;
});

// ── hasil kosong yang sah ────────────────────────────────────────────────────

test("kapabilitas menjawab dan tidak punya pin → pesan 'kosong', bukan error", async () => {
  const { balasan, reaksi, m } = pesanPengguna("cewe");
  const log = await tangkapLog(() => pap.handler(m, { sock: {} }));

  assert.ok(PANGGILAN.length >= 1, "harus tetap mencoba resolve");
  assert.equal(reaksi.at(-1), "❌");
  assert.match(balasan.at(-1) ?? "", /pap cewe lagi kosong/);
  assert.deepEqual(log, [], "tidak ada kegagalan untuk dicatat: kapabilitas menjawab");
});

test("satu query gagal lalu query lain menjawab kosong → tetap 'kosong'", async () => {
  // Ini yang membedakan "kosong" dari "mati": kalau SATU query sempat dijawab,
  // kapabilitasnya hidup, dan jawaban kosongnya sah. Hanya waktu TIDAK ADA query
  // yang menjawab, penyebabnya di luar user.
  jawabPinterest = (_capability, args) => {
    if (args.q === "cewe cantik indonesia") throw gagalSemua([MATI]);
    return TIDAK_ADA_PIN;
  };
  const { balasan, reaksi, m } = pesanPengguna("cewe");
  const log = await tangkapLog(() => pap.handler(m, { sock: {} }));

  assert.equal(reaksi.at(-1), "❌");
  assert.match(balasan.at(-1) ?? "", /lagi kosong/);
  assert.equal(log.length, 0, "satu query berhasil dijawab, jadi ini bukan padam");
});

// ── kapabilitas mati ─────────────────────────────────────────────────────────

test("semua query gagal → JANGAN bilang kosong; laporkan kegagalan ke operator", async () => {
  // Inilah regresi yang diperbaiki. Sebelumnya setiap throw ditelan dan hasilnya
  // `[]`, jadi user membaca "kosong" sementara tidak ada satu pun backend yang
  // sempat menjawab.
  jawabPinterest = () => {
    throw gagalSemua([
      MATI,
      { name: "nexray", reason: "nexray menjawab 503" },
    ]);
  };
  const { balasan, reaksi, m } = pesanPengguna("cewe");
  const log = await tangkapLog(() => pap.handler(m, { sock: {} }));

  assert.equal(reaksi.at(-1), "☢", "padam bukan hasil kosong");
  assert.doesNotMatch(
    balasan.join("\n"),
    /lagi kosong/,
    "pesan 'kosong' untuk host yang mati menyuruh user menunggu retry ke host yang sama",
  );
  // Rincian per host harus sampai ke operator. Ini satu-satunya tempat nama host
  // bisa dibaca, jadi meringkasnya jadi pesan user berarti menghilangkan bukti.
  assert.equal(log.length, 1, `harus tepat satu pencatatan, dapat ${log.length}`);
  const [label, error] = log[0];
  assert.match(String(label), /PAP Search/);
  assert.equal(error?.name, "CapabilityError");
  assert.deepEqual(error?.tried?.map((t) => t.name), ["ilovepin", "nexray"]);
});

test("semua query gagal → user dapat template error, bukan teks internal", async () => {
  jawabPinterest = () => {
    throw gagalSemua([MATI]);
  };
  const { balasan, m } = pesanPengguna("femboy");
  await tangkapLog(() => pap.handler(m, { sock: {} }));

  assert.match(
    balasan.at(-1) ?? "",
    /`\.femboy`|\.pap/,
    "pesan harus menunjuk command-nya, bukan nama host",
  );
  assert.doesNotMatch(
    balasan.join("\n"),
    /ilovepin|azbry|neoxr|nexray/i,
    "nama host aggregator tidak boleh bocor ke user",
  );
});

test("semua query gagal → ketiga query tetap dicoba sebelum menyerah", async () => {
  // Kegagalan tidak boleh menghentikan loop lebih awal: tiap query harus tetap
  // punya peluang yang sama seperti sebelum Phase 1.
  jawabPinterest = () => {
    throw gagalSemua([MATI]);
  };
  const { m } = pesanPengguna("femboy");
  await tangkapLog(() => pap.handler(m, { sock: {} }));

  assert.deepEqual(
    PANGGILAN.map((c) => c.args.q),
    ["femboy", "femboy anime", "femboy aesthetic"],
  );
});

// ── pin pin ──────────────────────────────────────────────────────────────────

test("satu query mati lalu query lain menjawab → URL-nya tetap dipakai untuk unduhan", async () => {
  // Penjaga arah sebaliknya: memperbaiki pemisahan "mati" vs "kosong" tidak boleh
  // mematikan jalur sukses. Kalau `fetchImageUrls` jadi melempar begitu saja pada
  // query pertama yang gagal, pin yang berhasil ditemukan tidak akan pernah
  // diunduh dan test ini tidak akan pernah melihat `UNDUHAN`.
  jawabPinterest = (_capability, args) => {
    if (args.q !== "cowo ganteng indonesia") throw gagalSemua([MATI]);
    return { ...SEBUAH, data: { pins: [{ image: "https://i.pinimg.com/originals/a.jpg" }] } };
  };
  const { balasan, m } = pesanPengguna("cowo");
  const log = await tangkapLog(() => pap.handler(m, { sock: {} }));

  assert.deepEqual(UNDUHAN, ["https://i.pinimg.com/originals/a.jpg"], "pin yang berhasil harus diunduh");
  // `axios` sengaja melempar, jadi handler berhenti di "Gambar tidak tersedia" —
  // bukti bahwa itu berhenti di jalur unduh, bukan di pemisahan kosong/padam.
  assert.match(balasan.at(-1) ?? "", /Gambar tidak tersedia/);
  assert.deepEqual(log, [], "tidak ada kegagalan yang perlu dicatat");
});