import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler lima plugin YouTube sungguhan. Yang dimock hanya
// dua hal: `src/lib/resolve.js` (supaya bisa diuji lewat jalur gagal tanpa
// menyentuh jaringan) dan `yt-search`/`axios` (sumber di luar kapabilitas ini).
// Plugin sendiri diimpor apa adanya supaya permukaannya ikut teruji.
//
// Dipisah dari tests/capabilities-phase1.test.mjs: `axios` dan `../src/lib/resolve.js`
// sudah dipakai mock lain di sana, dan satu modul tidak boleh di-mock dua kali
// dalam satu berkas.

const PANGGILAN_RESOLVE = [];
const MEDIA = [];
const PESAN = [];
const REAKSI = [];
const PREVIEW = [];

let hasilResolve = () => {
  throw new Error("resolver tidak boleh dipanggil: test ini belum menyesuaikan");
};
let hasilCari = () => ({ videos: [] });
let unduhanAudio = () => {
  throw new Error("audio tidak boleh diunduh di test ini");
};

const configUji = {
  async resolve(capability, args) {
    PANGGILAN_RESOLVE.push({ capability, args });
    return hasilResolve(capability, args);
  },
};
mock.module("../src/lib/resolve.js", { namedExports: { resolver: configUji } });

const CARI = () => hasilCari();
mock.module("yt-search", { defaultExport: CARI });

mock.module("axios", {
  defaultExport: {
    async get(url, opts) {
      return unduhanAudio(url, opts);
    },
  },
});

const ID = "dQw4w9WgXcQ";
const KANONIK = `https://www.youtube.com/watch?v=${ID}`;
const MP3 = "https://cdn.example/audio.mp3";
const MP4 = "https://cdn.example/video.mp4";
const THUMB = "https://i.ytimg.com/vi/x/hqdefault.jpg";

const VIDEO_CARI = {
  title: "Windah Basudara",
  url: KANONIK,
  thumbnail: THUMB,
  videoId: ID,
  description: "deskripsi video",
  ago: "2 tahun",
  views: { views: 1500000 },
  duration: { timestamp: "03:21" },
  timestamp: "03:21",
  seconds: 201,
  author: { name: "Ditzzy Official", username: "ditzzy" },
};

function mPalsu(text, { command = "yt", sender = "6281234567890@s.whatsapp.net" } = {}) {
  return {
    text,
    sender,
    chat: "chat@s.whatsapp.net",
    prefix: ".",
    command,
    pushName: "Tester",
    args: [],
    async reply(teks) { PESAN.push(String(teks)); return teks; },
    async react(emoji) { REAKSI.push(emoji); },
  };
}

function sockPalsu(ekstra = {}) {
  return {
    async sendMedia(chat, source, caption, quoted, options) {
      MEDIA.push({ chat, source, caption, options });
    },
    async sendMessage(chat, content) {
      PESAN.push({ kirim: chat, content });
    },
    async sendPreview(chat, payload) {
      PREVIEW.push({ chat, payload });
    },
    ...ekstra,
  };
}

async function muatPlugin(jalur) {
  try {
    return { modul: await import(jalur), galat: null };
  } catch (error) {
    return { modul: null, galat: error };
  }
}

const [ytmp3, ytmp4, playvid, playcall, playch] = await Promise.all([
  muatPlugin("../plugins/download/ytmp3.js"),
  muatPlugin("../plugins/download/ytmp4.js"),
  muatPlugin("../plugins/search/playvid.js"),
  muatPlugin("../plugins/search/playcall.js"),
  muatPlugin("../plugins/search/playch.js"),
]);

beforeEach(() => {
  PANGGILAN_RESOLVE.length = 0;
  MEDIA.length = 0;
  PESAN.length = 0;
  REAKSI.length = 0;
  PREVIEW.length = 0;
  hasilResolve = () => {
    throw new Error("resolver tidak boleh dipanggil: test ini belum menyesuaikan");
  };
  hasilCari = () => ({ videos: [{ ...VIDEO_CARI }] });
  unduhanAudio = () => {
    throw new Error("audio tidak boleh diunduh di test ini");
  };
});

/** Bentuk `CapabilityError` dari src/lib/resolve.js: alasan ada di `tried`. */
function gagalSemua(alasan = [{ name: "izuka", reason: "izuka menjawab 500" }]) {
  const e = new Error("semua backend youtube gagal");
  e.name = "CapabilityError";
  e.tried = alasan;
  return e;
}

function teksBalasan() {
  return PESAN.filter((p) => typeof p === "string").join("\n");
}

// ── sentinel ─────────────────────────────────────────────────────────────────

test("sentinel: kelima plugin benar-benar terimpor", () => {
  for (const [nama, hasil] of [
    ["ytmp3", ytmp3],
    ["ytmp4", ytmp4],
    ["playvid", playvid],
    ["playcall", playcall],
    ["playch", playch],
  ]) {
    assert.equal(hasil.galat, null, `impor ${nama} gagal: ${hasil.galat?.message ?? hasil.galat}`);
    assert.equal(typeof hasil.modul?.handler, "function", `${nama} tidak punya handler`);
    assert.equal(hasil.modul?.config?.name, nama, `config.name ${nama} berubah`);
  }
});

test("permukaan plugin tidak berubah: config, alias, cooldown, energi, dan status", () => {
  const diharapkan = [
    ["ytmp3", ytmp3.modul, ["youtubemp3", "ytaudio"], "download", 20, 2, true],
    ["ytmp4", ytmp4.modul, ["youtubemp4", "ytvideo"], "download", 20, 2, true],
    ["playvid", playvid.modul, ["playmp4"], "search", 15, 2, true],
    ["playcall", playcall.modul, ["telepon", "call", "playvn"], "search", 15, 2, false],
    ["playch", playch.modul, ["pch", "playsaluran"], "search", 15, 1, true],
  ];
  for (const [nama, modul, alias, category, cooldown, energi, aktif] of diharapkan) {
    assert.equal(modul.config.name, nama);
    assert.deepEqual(modul.config.alias, alias, `alias ${nama} berubah`);
    assert.equal(modul.config.category, category, `category ${nama} berubah`);
    assert.equal(modul.config.cooldown, cooldown, `cooldown ${nama} berubah`);
    assert.equal(modul.config.energi, energi, `energi ${nama} berubah`);
    assert.equal(modul.config.isEnabled, aktif, `isEnabled ${nama} berubah`);
  }
  // Atribut private/premium hanya ada di playcall dan tidak boleh hilang.
  assert.equal(playcall.modul.config.isOwner, false);
  assert.equal(playcall.modul.config.isPremium, false);
});

// ── download/ytmp3 ───────────────────────────────────────────────────────────

test("ytmp3: resolve mp3 lalu kirim medianya sendiri, tanpa aggregator dan tanpa unduhan penuh", async () => {
  hasilResolve = () => ({ ok: true, source: "ytdl-native", data: { title: "Lagu Uji", thumbnail: "", format: "mp3", url: MP3 }, meta: {} });

  await ytmp3.modul.handler(mPalsu(`https://youtu.be/${ID}`), { sock: sockPalsu() });

  assert.equal(PANGGILAN_RESOLVE.length, 1);
  assert.deepEqual(PANGGILAN_RESOLVE[0], {
    capability: "youtube",
    args: { url: `https://youtu.be/${ID}`, format: "mp3" },
  });
  assert.equal(MEDIA.length, 1, "harus tepat satu media");
  assert.equal(MEDIA[0].source, MP3, "media yang dikirim harus URL hasil resolve");
  assert.equal(MEDIA[0].options.type, "audio");
  assert.equal(MEDIA[0].options.mimetype, "audio/mpeg");
  assert.match(MEDIA[0].options.fileName, /Lagu Uji\.mp3$/);
  assert.deepEqual(REAKSI, ["🕕", "✅"]);
  assert.equal(teksBalasan(), "", "jalur sukses tidak boleh mengirim pesan error");
});

test("ytmp3: resolve gagal → tidak ada media dan tidak ada centang hijau", async () => {
  // Bentuk yang paling berbahaya: `normalize` lolos untuk respons yang tidak
  // berisi apa pun, jadi plugin mengirim tidak ada file lalu tetap Centang ✅.
  hasilResolve = () => {
    throw gagalSemua();
  };

  await ytmp3.modul.handler(mPalsu(KANONIK), { sock: sockPalsu() });

  assert.equal(MEDIA.length, 0, "tidak boleh ada yang dikirim");
  assert.equal(REAKSI.includes("✅"), false, "centang hijau tanpa file adalah kebohongan");
  assert.deepEqual(REAKSI, ["🕕", "❌"]);
  assert.match(teksBalasan(), /Gagal mengunduh audio\./);
});

test("ytmp3: link bukan YouTube ditolak di plugin, tanpa memanggil resolver", async () => {
  const sock = sockPalsu();
  for (const buruk of [
    "https://vimeo.com/12345",
    "https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
    "https://notyoutube.com/watch?v=dQw4w9WgXcQ",
  ]) {
    PESAN.length = 0;
    await ytmp3.modul.handler(mPalsu(buruk), { sock });
    assert.match(teksBalasan(), /❌ URL harus YouTube/, `harus ditolak: ${buruk}`);
  }
  assert.deepEqual(PANGGILAN_RESOLVE, [], "guard plugin harus mendahului resolver");
  assert.deepEqual(MEDIA, []);
});

test("ytmp3: tanpa argumen menampilkan contoh, dan short link tetap diterima", async () => {
  await ytmp3.modul.handler(mPalsu(""), { sock: sockPalsu() });
  assert.match(teksBalasan(), /ytmp3 https:\/\/youtube\.com\/watch\?v=xxx/);
  assert.deepEqual(PANGGILAN_RESOLVE, []);

  PESAN.length = 0;
  hasilResolve = () => ({ ok: true, source: "ytdl-native", data: { title: "Lagu", thumbnail: "", format: "mp3", url: MP3 } });
  await ytmp3.modul.handler(mPalsu(`https://youtu.be/${ID}`), { sock: sockPalsu() });
  assert.deepEqual(PANGGILAN_RESOLVE[0].args.url, `https://youtu.be/${ID}`, "guard tidak boleh menolak short link");
});

// ── download/ytmp4 ───────────────────────────────────────────────────────────

test("ytmp4: resolve mp4 lalu kirim sebagai video", async () => {
  hasilResolve = () => ({ ok: true, source: "youtube-fallback", data: { title: "Video Uji", thumbnail: THUMB, format: "mp4", url: MP4 } });

  await ytmp4.modul.handler(mPalsu(KANONIK), { sock: sockPalsu() });

  assert.deepEqual(PANGGILAN_RESOLVE[0], { capability: "youtube", args: { url: KANONIK, format: "mp4" } });
  assert.equal(MEDIA.length, 1);
  assert.equal(MEDIA[0].source, MP4);
  assert.equal(MEDIA[0].options.type, "video");
  assert.deepEqual(REAKSI, ["🕕", "✅"]);
});

test("ytmp4: resolve gagal → tidak ada media dan tidak ada centang hijau", async () => {
  hasilResolve = () => {
    throw gagalSemua([{ name: "youtube-fallback", reason: "SaveTube gagal: CDN tidak menjawab" }]);
  };

  await ytmp4.modul.handler(mPalsu(KANONIK), { sock: sockPalsu() });

  assert.equal(MEDIA.length, 0);
  assert.equal(REAKSI.includes("✅"), false);
  assert.match(teksBalasan(), /Gagal mengunduh video\./);
});

test("ytmp4: guard URL dipertahankan dan tidak memanggil resolver", async () => {
  await ytmp4.modul.handler(mPalsu("https://vimeo.com/12345"), { sock: sockPalsu() });
  assert.match(teksBalasan(), /❌ URL harus YouTube/);
  assert.deepEqual(PANGGILAN_RESOLVE, []);
  assert.deepEqual(MEDIA, []);
});

// ── search/playvid ───────────────────────────────────────────────────────────

test("playvid: preview dulu lalu video hasil resolve", async () => {
  hasilResolve = () => ({ ok: true, source: "youtube-fallback", data: { title: "Windah Basudara", thumbnail: THUMB, format: "mp4", url: MP4 } });

  await playvid.modul.handler(mPalsu("windah basudara"), { sock: sockPalsu(), _ : null });

  assert.equal(PREVIEW.length, 1, "preview harus terkirim");
  assert.equal(PREVIEW[0].payload.url, KANONIK);
  assert.deepEqual(PANGGILAN_RESOLVE[0], { capability: "youtube", args: { url: KANONIK, format: "mp4" } });
  assert.equal(MEDIA.length, 1);
  assert.equal(MEDIA[0].source, MP4);
  assert.deepEqual(REAKSI, ["🕕", "✅"]);
});

test("playvid: resolve gagal → preview tetap terkirim tapi tidak ada video dan tidak ada centang hijau", async () => {
  // Pesan error plugin menyebut "fitur putar videonya sedang ada kendala", jadi
  // branch ini harus tetap jujur: tidak ada video yang dikirim.
  hasilResolve = () => {
    throw gagalSemua();
  };

  await playvid.modul.handler(mPalsu("windah basudara"), { sock: sockPalsu(), _ : null });

  assert.equal(PREVIEW.length, 1, "preview berisi metadata pencarian bukan hasil unduhan");
  assert.equal(MEDIA.length, 0, "tidak boleh ada video yang dikirim");
  assert.equal(REAKSI.includes("✅"), false);
  assert.deepEqual(REAKSI, ["🕕", "❌"]);
  assert.match(teksBalasan(), /fitur putar videonya sedang ada kendala/);
});

test("playvid: tanpa query menampilkan panduan dan tidak menyentuh resolver", async () => {
  await playvid.modul.handler(mPalsu(""), { sock: sockPalsu(), _ : null });
  assert.match(teksBalasan(), /playvid <judul video>/);
  assert.deepEqual(PANGGILAN_RESOLVE, []);
  assert.deepEqual(MEDIA, []);
});

// ── search/playcall ──────────────────────────────────────────────────────────

test("playcall: audio diunduh ke berkas lalu panggilan dibuat", async (t) => {
  globalThis.voipClient = {
    async call(target, opsi) {
      PESAN.push({ panggil: target, opsi });
      return { on() {} };
    },
  };
  t.after(() => { delete globalThis.voipClient; });

  hasilResolve = () => ({ ok: true, source: "ytdl-native", data: { title: "Komang", thumbnail: "", format: "mp3", url: MP3 } });
  unduhanAudio = async () => ({ data: Buffer.alloc(2048, 7) });

  await playcall.modul.handler(mPalsu("komang"), { sock: sockPalsu(), text: "komang" });

  assert.deepEqual(PANGGILAN_RESOLVE[0], { capability: "youtube", args: { url: KANONIK, format: "mp3" } });
  assert.equal(REAKSI.includes("📞"), true, "panggilan harus dimulai");
  assert.ok(
    PESAN.some((p) => p?.panggil === "6281234567890"),
    "nomor tujuan harus dibersihkan dari format WhatsApp",
  );
});

test("playcall: resolve gagal → tidak ada panggilan dan tidak ada unduhan", async (t) => {
  globalThis.voipClient = { async call() { PESAN.push({ panggil: "dilewati" }); return { on() {} }; } };
  t.after(() => { delete globalThis.voipClient; });

  hasilResolve = () => {
    throw gagalSemua();
  };

  await playcall.modul.handler(mPalsu("komang"), { sock: sockPalsu(), text: "komang" });

  assert.equal(PESAN.some((p) => p?.panggil === "dilewati"), false, "tidak boleh ada panggilan");
  assert.deepEqual(REAKSI, ["🕕", "☢"]);
  assert.match(teksBalasan(), /lagi ada kendala/);
});

test("playcall: tanpa query menampilkan panduan dan tidak menyentuh resolver", async () => {
  await playcall.modul.handler(mPalsu(""), { sock: sockPalsu(), text: "" });
  assert.match(teksBalasan(), /PANGGILAN MUSIK \(PLAYCALL\)/);
  assert.deepEqual(PANGGILAN_RESOLVE, []);
});

// ── search/playch ────────────────────────────────────────────────────────────

test("playch: resolve gagal → tidak ada apa pun yang dikirim ke saluran", async () => {
  // Jalur convertersopus butuh ffmpeg dan unduhan penuh, jadi yang diuji di sini
  // adalah batasnya: kegagalan scraping tidak boleh sampai menyentuh channel.
  hasilResolve = () => {
    throw gagalSemua();
  };

  await playch.modul.handler(mPalsu("--idch 123456@newsletter komang"), { sock: sockPalsu() });

  assert.deepEqual(PANGGILAN_RESOLVE[0], {
    capability: "youtube",
    args: { url: KANONIK, format: "mp3" },
  });
  assert.equal(
    PESAN.some((p) => typeof p === "object" && p.kirim === "123456@newsletter"),
    false,
    "tidak boleh ada apa pun yang dikirim ke channel",
  );
  assert.equal(REAKSI.includes("✅"), false);
  assert.equal(REAKSI.at(-1), "☢");
  assert.match(teksBalasan(), /lagi ada kendala/);
});

test("playch: tanpa query menampilkan panduan dan berhenti sebelum resolver", async () => {
  await playch.modul.handler(mPalsu(""), { sock: sockPalsu() });
  assert.match(teksBalasan(), /PLAY SALURAN/);
  assert.deepEqual(PANGGILAN_RESOLVE, []);
  assert.deepEqual(MEDIA, [], "tanpa query tidak boleh ada media");
});

test("playch: --idch menimpa channel dari config dan query-nya dibersihkan", async () => {
  hasilResolve = () => {
    throw gagalSemua();
  };

  await playch.modul.handler(mPalsu("--idch 123456@newsletter komang"), { sock: sockPalsu() });

  assert.deepEqual(PANGGILAN_RESOLVE[0].args, { url: KANONIK, format: "mp3" });
});
