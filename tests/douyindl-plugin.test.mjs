import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/download/douyindl.js` sungguhan, bukan
// mencocokkan teksnya. Uji lapis plugin dipisah dari
// tests/capabilities-phase1.test.mjs karena mengimpor plugin ini menuntut mock
// `src/lib/http.js`, dan satu modul tidak boleh di-mock dua kali dalam satu
// berkas.
//
// Transport di bawahnya dimock, jadi tidak ada request yang keluar ke jaringan.

const PANGGILAN = [];
let balasSnap = () => {
  throw new Error("snapvideotools tidak boleh dipanggil: upstream mati");
};
let balasAggregator = () => {
  throw new Error("aggregator tidak boleh dipanggil: upstream mati");
};

const httpPalsu = {
  async get(url, opts) {
    PANGGILAN.push({ verb: "get", url, opts });
    return balasAggregator(url, opts);
  },
  async post(url, body, config) {
    PANGGILAN.push({ verb: "post", url, config });
    return balasSnap(url, body, config);
  },
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu } });

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

// Catatan: berkas ini hari ini belum menarik `lid.js` ke dalam prosesnya,
// jadi sentinel di sini guarding sesuatu yang belum terjadi. Murah sekarang,
// mahal besok kalau satu baris impor plugin berubah.
const douyindl = await impor("../plugins/download/douyindl.js");

test("sentinel: plugin douyindl benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof douyindl?.handler, "function");
});

const VIDEO = "https://v3.douyinvod.com/abc/video.mp4";
const AUDIO = "https://v3.douyinvod.com/abc/audio.mp3";

/** Bentuk dua-lapis yang benar-benar dibaca `DouyinDL` (`response.data.data`). */
function snapSukses({ video = true, audio = true } = {}) {
  balasSnap = async () => ({
    data: {
      data: {
        title: "Judul Douyin",
        platformName: "Douyin",
        mediaUrls: [
          ...(video ? [{ type: "video", url: VIDEO }] : []),
          ...(audio ? [{ type: "audio", url: AUDIO }] : []),
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
      command: "douyindl",
      pushName: "Tester",
      chat: "chat@s.whatsapp.net",
      args: [],
      reply: async (teks) => balasan.push(teks),
      react: async (emoji) => reaksi.push(emoji),
    },
  };
}

function sockPalsu() {
  const terkirim = [];
  return {
    terkirim,
    sendMedia: async (chat, media, quoted, _m, options) => {
      terkirim.push({ chat, media, quoted, options });
    },
  };
}

beforeEach(() => {
  PANGGILAN.length = 0;
  balasSnap = () => {
    throw new Error("snapvideotools tidak boleh dipanggil: upstream mati");
  };
  balasAggregator = () => {
    throw new Error("aggregator tidak boleh dipanggil: upstream mati");
  };
});

// ── guard plugin: harus sekeras guard kapabilitas ────────────────────────────

test("guard plugin: host palsu ditolak sebelum menyentuh resolve", async () => {
  // `douyin.com.evil.example` dan `notdouyin.com` lolos `includes("douyin")`.
  // Kalau guard plugin hanya cek substring, guard kapabilitas yang menolak dan
  // `cobaBackend` mencatat kegagalan — tiga link seperti ini dari satu user
  // sudah cukup membuka breaker snapvideotools 30 detik untuk semua orang.
  for (const url of [
    "https://douyin.com.evil.example/video/1",
    "https://notdouyin.com/video/1",
    "https://example.com/?u=douyin.com",
    "halo",
  ]) {
    snapSukses();
    const { balasan, reaksi, m } = pesanPengguna(url);
    await douyindl.handler(m, { sock: sockPalsu() });

    assert.deepEqual(reaksi, [], `tidak boleh menyentuh resolve: ${url}`);
    assert.match(balasan.at(-1) ?? "", /tidak valid/i);
    assert.deepEqual(PANGGILAN, [], `tidak boleh ada request: ${url}`);
  }
});

test("guard plugin: link sah tetap meneruskan sampai file dikirim", async () => {
  // Arah lain dari guard juga dikunci: guard yang terlalu ketat mematikan
  // command yang tadinya jalan.
  snapSukses();
  const { reaksi, m } = pesanPengguna("https://v.douyin.com/abc123/");
  const sock = sockPalsu();

  await douyindl.handler(m, { sock });

  assert.ok(reaksi.includes("🕕"), "link sah harus lanjut ke resolve");
  assert.equal(sock.terkirim.length, 2, "video dan audio harus tetap dikirim");
  assert.equal(sock.terkirim[0].media, VIDEO);
  assert.equal(sock.terkirim[0].options.type, "video");
  assert.match(sock.terkirim[0].quoted, /Judul Douyin/, "caption lama harus tetap dipakai");
  assert.equal(sock.terkirim[1].media, AUDIO);
  assert.equal(sock.terkirim[1].options.type, "audio");
  assert.equal(sock.terkirim[1].quoted, null, "audio dikirim tanpa caption seperti sebelumnya");
  assert.equal(reaksi.at(-1), "✅", "centang hijau hanya sah kalau file benar-benar terkirim");
});

test("guard plugin: tanpa teks, bantuan tetap muncul tanpa request", async () => {
  const { balasan, m } = pesanPengguna("");
  await douyindl.handler(m, { sock: sockPalsu() });

  assert.match(balasan.at(-1) ?? "", /PENGGUNAAN/);
  assert.deepEqual(PANGGILAN, []);
});

test("scraper lokal gagal → aggregator neoxr membakar penuh dan file tetap terkirim", async () => {
  // Fallback harus benar-benar dipakai, bukan hanya ada di kapabilitas: backend
  // lokal hidup tapi menjawab tanpa video, aggregator yang menyediakan video.
  snapSukses({ video: false, audio: false });
  balasAggregator = async () => ({
    status: 200,
    data: { status: true, result: { platform: "Douyin", title: "Dari aggregator", video: VIDEO, audio: AUDIO } },
  });
  const { m } = pesanPengguna("https://v.douyin.com/abc123/");
  const sock = sockPalsu();

  await douyindl.handler(m, { sock });

  assert.ok(
    PANGGILAN.some((c) => String(c.url).includes("azbry.com")),
    "aggregator harus dihubungi setelah scraper lokal tidak memberi video",
  );
  assert.equal(sock.terkirim[0].media, VIDEO);
  assert.match(sock.terkirim[0].quoted, /Dari aggregator/);
});

test("semua backend gagal → pesan jujur, bukan centang hijau", async () => {
  snapSukses({ video: false, audio: false });
  balasAggregator = async () => ({ status: 200, data: { status: true, result: { platform: "Douyin" } } });
  const { balasan, reaksi, m } = pesanPengguna("https://v.douyin.com/abc123/");
  const sock = sockPalsu();

  await douyindl.handler(m, { sock });

  assert.deepEqual(sock.terkirim, [], "tidak boleh ada file yang diklaim terkirim");
  assert.equal(reaksi.at(-1), "☢");
  assert.match(balasan.at(-1) ?? "", /Gagal mengambil data Douyin/);
});

test("aggregator status false tapi result-nya penuh → tidak ada video yang dikirim", async () => {
  // azbry menjawab 200 dengan `status: false` sambil tetap mengisi `result`.
  // Plugin sebelum Phase 1 menolak bentuk ini (res.data?.status &&
  // res.data?.result); kalau pemeriksaan itu hilang, `result` yang penuh
  // diteruskan apa adanya dan user menerima video dari scraping yang gagal.
  snapSukses({ video: false, audio: false });
  balasAggregator = async () => ({
    status: 200,
    data: {
      status: false,
      msg: "gagal",
      result: { platform: "Douyin", title: "Dari aggregator", video: VIDEO, audio: AUDIO },
    },
  });
  const { balasan, reaksi, m } = pesanPengguna("https://v.douyin.com/abc123/");
  const sock = sockPalsu();

  await douyindl.handler(m, { sock });

  assert.deepEqual(sock.terkirim, [], "status false bukan scraping yang sukses");
  assert.equal(reaksi.includes("✅"), false, "centang hijau hanya sah kalau video benar-benar terkirim");
  assert.equal(reaksi.at(-1), "☢");
  assert.equal(balasan.length > 0, true, "user harus tetap diberi jawaban");
});
