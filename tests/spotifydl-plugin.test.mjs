import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/download/spotifydl.js` sungguhan.
//
// Yang diuji adalah pemisahan dua kegagalan yang selama ini tertumpuk jadi satu.
// `normalize` (src/capabilities/spotify.js) sengaja tidak pernah mengembalikan record
// tanpa URL unduhan, jadi `if (!data?.url)` di dalam `try` tidak pernah bisa bernilai
// true, dan pesan "Server tidak merespon dengan tautan unduhan yang valid" yang
// menempel di cabang itu menjadi kode mati. Pemisahan sekarang dilakukan di `catch`
// lewat `adaNormalisasiGagal`: host yang menjawab tanpa tautan unduhan berarti
// masalahnya link-nya, sedangkan host yang tidak bisa dihubungi berarti masalahnya
// proses.
//
// Resolver di-mock supaya tidak ada request yang keluar ke jaringan dan supaya
// `tried` bisa dibentuk persis seperti yang dibentuk resolver sungguhan. Klasifikasi
// `tried` sendiri TIDAK ditiru di sini: helper aslinya diambil dari modul aslinya
// sebelum mock dipasang, supaya yang diuji di sini adalah kode yang benar-benar
// dipakai produksi.

const { adaNormalisasiGagal } = await import("../src/lib/resolve.js");

const PANGGILAN = [];
let hasilResolve = () => {
  throw new Error("resolver tidak boleh dipanggil di test ini");
};

mock.module("../src/lib/resolve.js", {
  namedExports: {
    resolver: {
      resolve: async (capability, args) => {
        PANGGILAN.push({ capability, args });
        return hasilResolve(capability, args);
      },
    },
    adaNormalisasiGagal,
  },
});

// ── sentinel import ─────────────────────────────────────────────────────────
//
// `spotifydl.js` hanya mengimpor `src/lib/resolve.js`, dan berkas ini me-mock-nya,
// jadi rantai `config.js` → `src/lib/lid.js` tidak masuk ke proses test. Sentinel
// tetap dipasang: kalau suatu saat plugin menambah satu baris impor, impor yang gagal
// tidak akan menggagalkan berkas ini tanpa dia — `src/lib/lid.js:61` memasang
// `process.on("uncaughtException", …)` yang menelan error, lalu berkas keluar 0 dengan
// `pass 1` dan nol test. Pola yang sama dipakai tests/applemusic-plugin.test.mjs,
// tests/sfiledl-plugin.test.mjs, dan tests/pap-search-outage.test.mjs.
const GALAT_IMPOR = [];

async function impor(specifier) {
  try {
    return await import(specifier);
  } catch (error) {
    GALAT_IMPOR.push(`${specifier} → ${error?.code ?? "?"}: ${error?.message ?? error}`);
    return null;
  }
}

const spotifydl = await impor("../plugins/download/spotifydl.js");

test("sentinel: plugin spotifydl benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  assert.deepEqual(GALAT_IMPOR, [], `impor gagal:\n${GALAT_IMPOR.join("\n")}`);
  assert.equal(typeof spotifydl?.handler, "function");
});

const LAGU = {
  title: "Grateful",
  artist: "Neffex",
  url: "https://cdn.example/grateful.mp3",
  mime: "audio/mpeg",
};
const LINK = "https://open.spotify.com/track/3RY0NyQQXxuAiyk5eAS4fC";

/** `CapabilityError` seperti yang benar dilempar resolver. */
function gagalDengan(tried) {
  const error = new Error("semua backend spotify gagal");
  error.name = "CapabilityError";
  error.capability = "spotify";
  error.code = "unresolved";
  error.tried = tried;
  return error;
}

// Dua kelas kegagalan nyata. Yang pertama: backend menjawab, lalu `normalize` menolak
// responsnya karena tidak ada URL unduhan — `reason`-nya diawali "normalisasi gagal:".
// Yang kedua: `run` sendiri melempar — timeout, DNS, atau backend mati.
const TANPA_URL = {
  name: "nexray",
  reason: "normalisasi gagal: spotify: respons tidak punya URL unduhan (field: title, artist)",
};
const MATI_TRANSPORT = { name: "nexray", reason: "batas waktu 5000ms habis" };
const MATI_LOCAL = { name: "spotyloader", reason: "spotyloader tidak terjangkau: ENOTFOUND" };

function pesanPengguna(text) {
  const balasan = [];
  const reaksi = [];
  const terkirim = [];
  return {
    balasan,
    reaksi,
    terkirim,
    m: {
      text,
      prefix: ".",
      command: "spdl",
      pushName: "Tester",
      chat: "chat@s.whatsapp.net",
      args: [],
      react: async (emoji) => reaksi.push(emoji),
      reply: async (teks) => balasan.push(String(teks)),
    },
  };
}

/** Jalankan handler dengan `console.error` ditahan, lalu kembalikan hasil + log. */
async function jalankan(text) {
  const asli = console.error;
  const log = [];
  console.error = (...args) => log.push(args);
  const { balasan, reaksi, terkirim, m } = pesanPengguna(text);
  try {
    await spotifydl.handler(m, {
      sock: {
        async sendMessage(chat, konten, opsi) {
          terkirim.push({ chat, konten, opsi });
        },
      },
    });
  } finally {
    console.error = asli;
  }
  return { balasan, reaksi, terkirim, log };
}

beforeEach(() => {
  PANGGILAN.length = 0;
  hasilResolve = () => ({
    ok: true,
    source: "spotyloader",
    data: LAGU,
    meta: { tookMs: 1, cached: false },
  });
});

// ── jalur sehat ──────────────────────────────────────────────────────────────

test("lagu valid → audio terkirim dan centang hijau", async () => {
  const { balasan, reaksi, terkirim } = await jalankan(LINK);

  assert.equal(terkirim.length, 1);
  assert.equal(terkirim[0].konten.audio.url, LAGU.url);
  assert.equal(terkirim[0].konten.mimetype, "audio/mpeg");
  assert.equal(reaksi.at(-1), "✅");
  assert.deepEqual(balasan, []);
});

// ── backend menjawab tanpa URL unduhan ───────────────────────────────────────

test("backend menjawab tanpa URL → pesan tautan, bukan 'kesalahan sistem'", async () => {
  // Inilah regresi yang diperbaiki. `normalize` menutup jalan `data.url` kosong, jadi
  // sebelum ini kasus ini jatuh ke pesan "kesalahan sistem": user mengira bot-nya
  // yang bermasalah, padahal tautannya memang tidak ada di sana.
  hasilResolve = () => {
    throw gagalDengan([MATI_LOCAL, TANPA_URL]);
  };

  const { balasan, reaksi, terkirim } = await jalankan(LINK);

  assert.deepEqual(terkirim, [], "tidak boleh ada file yang diklaim terkirim");
  assert.match(balasan.at(-1) ?? "", /Gagal mengambil lagu/);
  assert.match(balasan.at(-1) ?? "", /tautan unduhan yang valid/);
  assert.doesNotMatch(balasan.join("\n"), /kesalahan sistem/);
  assert.equal(reaksi.at(-1), "❌");
  assert.equal(reaksi.includes("✅"), false, "centang hijau hanya sah kalau file benar-benar terkirim");
});

test("backend menjawab tanpa URL → rincian alasan tetap sampai ke operator", async () => {
  hasilResolve = () => {
    throw gagalDengan([TANPA_URL]);
  };

  const { log } = await jalankan(LINK);

  assert.equal(log.length, 1, `harus tepat satu pencatatan, dapat ${log.length}`);
  const [label, error] = log[0];
  assert.match(String(label), /Spotify DL Error/);
  assert.equal(error?.name, "CapabilityError");
  assert.deepEqual(error?.tried?.map((t) => t.name), ["nexray"]);
});

// ── kegagalan proses: harus tetap messaging operator ────────────────────────

test("semua backend mati di transport → tetap pesan kesalahan sistem", async () => {
  // Penjaga arah sebaliknya. `adaNormalisasiGagal` hanya berguna kalau kegagalan
  // transport TIDAK ikut terhitung: kalau iya, setiap outage akan menyamar jadi
  // "link-nya salah" dan operator kehilangan satu-satunya alarmnya.
  hasilResolve = () => {
    throw gagalDengan([MATI_LOCAL, MATI_TRANSPORT]);
  };

  const { balasan, reaksi, terkirim } = await jalankan(LINK);

  assert.deepEqual(terkirim, []);
  assert.match(balasan.at(-1) ?? "", /kesalahan sistem/);
  assert.doesNotMatch(balasan.join("\n"), /Gagal mengambil lagu/);
  assert.equal(reaksi.at(-1), "❌");
});

test("satu backend mati di transport, satu menjawab tanpa URL → tetap pesan tautan", async () => {
  // Host yang menjawab tanpa URL adalah bukti langsung bahwa tautan tidak ada.
  // Host lain yang tidak bisa dihubungi tidak membatalkan bukti itu, jadi gabungan
  // keduanya tidak boleh mengembalikan user ke pesan "kesalahan sistem".
  hasilResolve = () => {
    throw gagalDengan([MATI_LOCAL, TANPA_URL]);
  };

  const { balasan } = await jalankan(LINK);

  assert.match(balasan.at(-1) ?? "", /Gagal mengambil lagu/);
});

// ── guard yang tidak boleh berubah ───────────────────────────────────────────

test("link yang bukan track Spotify ditolak sebelum menyentuh resolve", async () => {
  const { balasan, reaksi, terkirim } = await jalankan("https://youtube.com/watch?v=abc");

  assert.deepEqual(PANGGILAN, [], "tidak boleh menyentuh resolver");
  assert.deepEqual(terkirim, []);
  assert.match(balasan.at(-1) ?? "", /link Spotify-nya mana/);
  assert.deepEqual(reaksi, []);
});

// ── pin pesan ────────────────────────────────────────────────────────────────

test("teks pesan tautan tidak berubah, dan sekarang hidup di dalam `catch`", async () => {
  // Pin ini dibaca dari sumber dengan sengaja. Tujuan perubahan ini bukan mengganti
  // kalimat pesannya, melainkan membuatnya bisa dijangkau: sebelum ini teksnya ada
  // di dead branch yang tidak pernah dieksekusi. Kalau ada yang mengedit kalimatnya,
  // test ini harus ikut merah — dan kalau ada yang memindahkannya kembali ke dalam
  // `try`, test reachability di atas harus ikut merah juga.
  const src = fs.readFileSync(path.join(process.cwd(), "plugins/download/spotifydl.js"), "utf8");
  assert.match(src, /⚠️ \*Gagal mengambil lagu!\*/);
  assert.match(src, /Server tidak merespon dengan tautan unduhan yang valid\./);
  const posisiTangkap = src.indexOf("} catch (error) {");
  const posisiPesan = src.indexOf("Server tidak merespon");
  assert.ok(posisiTangkap > -1, "blok catch harus ada");
  assert.ok(
    posisiPesan > posisiTangkap,
    "pesan hanya hidup di dalam catch; di dalam try ia tidak pernah terjangkau",
  );
});