import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini memanggil handler `plugins/tools/hd2.js` dan `plugins/tools/hd3.js`
// sungguhan dengan resolver palsu, supaya yang diuji adalah batas plugin: pesan
// bantuan, reaksi, nama file yang dikirim, dan pembersihan berkas sementara.
//
// Tiga hal dimock: resolver (sudah dipakai berkas lain, jadi mock-nya di sini),
// `sharp` untuk thumbnail, dan `axios` sebagai jaring pengaman — plugin tidak
// boleh membutuhkannya lagi, dan kalau ada sisa `axios` import, test ini gagal
// dengan pesan jelas.

const PANGGILAN = [];
const REAKSI = [];
const BALASAN = [];
const TERKIRIM = [];

let hasilResolve = () => {
  throw new Error("resolver tidak boleh dipanggil");
};
let unduhanGagal = false;
let unduhanKosong = false;

mock.module("../src/lib/resolve.js", {
  namedExports: {
    resolver: {
      resolve: async (kapabilitas, args) => {
        PANGGILAN.push({ kapabilitas, args, adaBerkas: fs.existsSync(args?.media ?? "") });
        return hasilResolve(kapabilitas, args);
      },
    },
  },
});

const sharpPalsu = {
  resize() { return this; },
  jpeg() { return this; },
  async toBuffer() { return Buffer.from("thumb", "utf8"); },
};
mock.module("sharp", { defaultExport: () => sharpPalsu });

mock.module("axios", {
  defaultExport: {
    get: async () => {
      throw new Error("axios tidak boleh dipakai plugin hd");
    },
    post: async () => {
      throw new Error("axios tidak boleh dipakai plugin hd");
    },
  },
});

// ── sentinel import ───────────────────────────────────────────────────────────
//
// Kedua plugin mengimpor `../../config.js` secara langsung, dan rantai impor itu
// `config.js` → database.js → logger.js → lid.js. `src/lib/lid.js:61` memasang
// `process.on("uncaughtException", …)` yang menelan semua error, termasuk
// `ERR_MODULE_NOT_FOUND`. Akibatnya impor plugin yang gagal tidak menggagalkan
// berkas: proses keluar 0 dan node:test melaporkan `pass 1` dengan nol test.
//
// Karena itu impor plugin tidak boleh dibiarkan gagal telanjang di top-level.
// Kalau `await import` di situ melempar, modul berkas ini tidak pernah selesai
// dievaluasi dan `test()` di bawah tidak pernah terdaftar — sentinel yang
// diletakkan setelah impor justru tidak pernah ikut jalan. Kesalahannya
// ditangkap, lalu assertion-nya yang menggagalkan. Pola yang sama dipakai
// tests/sfiledl-plugin.test.mjs.
const plugin = {};
for (const nama of ["hd2", "hd3"]) {
  plugin[nama] = null;
  plugin[`${nama}Galat`] = undefined;
  try {
    plugin[nama] = await import(`../plugins/tools/${nama}.js`);
  } catch (error) {
    plugin[`${nama}Galat`] = error;
  }
}

test("sentinel: kedua plugin benar-benar terimpor (bukan berkas kosong yang hijau)", () => {
  for (const nama of ["hd2", "hd3"]) {
    assert.equal(
      plugin[`${nama}Galat`],
      undefined,
      `impor ${nama} gagal: ${plugin[`${nama}Galat`]?.message ?? plugin[`${nama}Galat`]}`,
    );
    assert.equal(typeof plugin[nama]?.handler, "function", nama);
    assert.equal(typeof plugin[nama]?.config?.name, "string", nama);
  }
});

function pesanPengguna({ teks = "", quoted = null, media = true } = {}) {
  REAKSI.length = 0;
  BALASAN.length = 0;
  return {
    text: teks,
    prefix: ".",
    command: "hd2",
    pushName: "Tester",
    chat: "chat@s.whatsapp.net",
    args: [],
    isImage: media,
    isMedia: media,
    quoted,
    async download() {
      if (unduhanGagal) throw new Error("unduh gagal");
      if (unduhanKosong) return null;
      return Buffer.from("gambar-pengguna", "utf8");
    },
    async react(emoji) { REAKSI.push(emoji); },
    async reply(teks) { BALASAN.push(String(teks)); return teks; },
  };
}

function sockPalsu() {
  TERKIRIM.length = 0;
  return {
    terkirim: TERKIRIM,
    async sendMessage(chat, isi, opsi) {
      TERKIRIM.push({ chat, isi, opsi });
    },
  };
}

const JAWAB = { ok: true, source: "photoai-imglarger", data: { url: "https://cdn.example/naik.jpg" }, meta: { tookMs: 42000, cached: false } };

beforeEach(() => {
  PANGGILAN.length = 0;
  REAKSI.length = 0;
  BALASAN.length = 0;
  TERKIRIM.length = 0;
  hasilResolve = () => {
    throw new Error("resolver tidak boleh dipanggil");
  };
  unduhanGagal = false;
  unduhanKosong = false;
});

// ── guard dan pesan bantuan ───────────────────────────────────────────────────

test("tanpa gambar: pesan bantuan utuh dan resolver tidak disentuh", async () => {
  for (const nama of ["hd2", "hd3"]) {
    const m = pesanPengguna({ media: false });
    m.command = nama;
    await plugin[nama].handler(m, { sock: sockPalsu() });

    assert.equal(PANGGILAN.length, 0, `${nama} tidak boleh menghubungi resolve tanpa gambar`);
    assert.equal(BALASAN.length, 1, nama);
    assert.ok(BALASAN[0].includes(`*${m.prefix}${nama}*`), `${nama} nama perintah di bantuan: ${BALASAN[0]}`);
    assert.ok(BALASAN[0].includes("*Cara Penggunaan:*"), nama);
    assert.ok(BALASAN[0].includes("gambar"), nama);
  }
});

test("pesan bantuan hd3 masih menjanjikan unblur, bukan enhance", async () => {
  const m = pesanPengguna({ media: false });
  await plugin.hd3.handler(m, { sock: sockPalsu() });
  const teks = BALASAN[0];
  assert.ok(teks.includes("blur"), `hd3 harus menyebut blur: ${teks}`);
  assert.ok(teks.includes("_Proses rendering mungkin memerlukan waktu beberapa saat._"), `kalimat proses hilang: ${teks}`);
});

// ── jalur sukses ──────────────────────────────────────────────────────────────

test("hd2 mengirim media dari resolver, dengan reaksi dan nama file yang sama", async () => {
  hasilResolve = async () => JAWAB;
  const m = pesanPengguna();
  const sock = sockPalsu();

  await plugin.hd2.handler(m, { sock });

  assert.deepEqual(
    PANGGILAN.map((c) => c.kapabilitas),
    ["hd"],
  );
  assert.equal(PANGGILAN[0].args.kind, "imglarger");
  assert.equal(PANGGILAN[0].adaBerkas, true, "media harus berupa file yang benar-benar ada saat resolve dipanggil");
  assert.deepEqual(REAKSI, ["🕕", "✅"]);
  assert.equal(TERKIRIM.length, 1);
  assert.equal(TERKIRIM[0].isi.document.url, "https://cdn.example/naik.jpg");
  assert.equal(TERKIRIM[0].isi.mimetype, "image/jpeg");
  assert.match(TERKIRIM[0].isi.fileName, /^HD_BY_.+\.jpg$/);
  assert.ok(TERKIRIM[0].isi.jpegThumbnail, "thumbnail harus tetap dikirim");
});

test("hd3 mengirim media dari resolver dengan nama file UNBLUR", async () => {
  hasilResolve = async () => ({ ok: true, source: "fgsi-enchantvideo", data: { url: "https://cdn.example/jelas.jpg" }, meta: { tookMs: 9, cached: false } });
  const m = pesanPengguna();
  m.command = "hd3";
  const sock = sockPalsu();

  await plugin.hd3.handler(m, { sock });

  assert.equal(PANGGILAN[0].args.kind, "unblur");
  assert.match(TERKIRIM[0].isi.fileName, /^UNBLUR_BY_.+\.jpg$/);
  assert.equal(TERKIRIM[0].isi.document.url, "https://cdn.example/jelas.jpg");
});

test("gambar dari quoted message dipakai, dan pilihan unduh yang sama seperti sebelumnya", async () => {
  hasilResolve = async () => JAWAB;
  let dipanggil = 0;
  const m = pesanPengguna({ quoted: { isMedia: true, type: "imageMessage", download: async () => { dipanggil += 1; return Buffer.from("dari-quoted", "utf8"); } } });
  const sock = sockPalsu();

  await plugin.hd2.handler(m, { sock });

  assert.equal(dipanggil, 1, "quoted harus diunduh");
  assert.equal(TERKIRIM.length, 1);
});

test("berkas sementara dihapus setelah sukses", async () => {
  hasilResolve = async () => JAWAB;
  const m = pesanPengguna();
  const sock = sockPalsu();

  await plugin.hd2.handler(m, { sock });

  const media = PANGGILAN[0].args.media;
  assert.equal(fs.existsSync(media), false, `berkas sementara masih ada: ${media}`);
});

// ── jalur gagal ───────────────────────────────────────────────────────────────

test("unduhan kosong: pesan yang sama, tanpa resolve dan tanpa terkirim", async () => {
  // Dua kelas kegagalan unduh punya reaksi berbeda SEBELUM Phase 1 dan itu
  // sengaja dipertahankan: buffer kosong berarti pesan khusus, sedangkan
  // `download()` yang melempar tetap masuk jalur error umum.
  unduhanKosong = true;
  // Kata "berikan" muncul di kedua plugin sejak sebelum Phase 1
  // (`git show ff397d2^:plugins/tools/hd3.js`), jadi keduanya dikunci sama.
  const pesanPerPlugin = {
    hd2: "Maaf, sistem gagal mengunduh gambar yang kamu berikan. Silakan coba kirim ulang gambarnya!",
    hd3: "Maaf, sistem gagal mengunduh gambar yang kamu berikan. Silakan coba kirim ulang gambarnya!",
  };
  for (const nama of ["hd2", "hd3"]) {
    const m = pesanPengguna();
    m.command = nama;
    const sock = sockPalsu();
    await plugin[nama].handler(m, { sock });

    assert.equal(PANGGILAN.length, 0, nama);
    assert.deepEqual(REAKSI, ["🕕", "❌"], nama);
    assert.equal(BALASAN.length, 1, nama);
    assert.equal(BALASAN[0], pesanPerPlugin[nama], nama);
    assert.equal(TERKIRIM.length, 0, nama);
  }
});

test("download yang melempar tetap memakai jalur error umum, seperti sebelumnya", async () => {
  // Jangan "memperbaiki" kelas ini jadi pesan khusus: yang berubah hanyalah
  // isi `catch`, dan reaksinya tetap tanda error.
  unduhanGagal = true;
  for (const nama of ["hd2", "hd3"]) {
    const m = pesanPengguna();
    m.command = nama;
    const sock = sockPalsu();
    await plugin[nama].handler(m, { sock });

    assert.equal(PANGGILAN.length, 0, nama);
    assert.deepEqual(REAKSI, ["🕕", "☢"], nama);
    assert.equal(TERKIRIM.length, 0, nama);
    assert.ok(BALASAN[0].includes("Kayaknya command"), nama);
  }
});

test("unduhan kosong diperlakukan sama dengan gagal", async () => {
  unduhanKosong = true;
  const m = pesanPengguna();
  await plugin.hd2.handler(m, { sock: sockPalsu() });

  assert.equal(PANGGILAN.length, 0);
  assert.deepEqual(REAKSI, ["🕕", "❌"]);
});

test("resolver gagal: tanda error, template pesan, dan tidak ada file terkirim", async () => {
  // Ini bentuk yang paling penting: kegagalan SETELAH enhancement berhasil
  // tidak boleh pernah berubah menjadi centang hijau atau file terkirim.
  for (const nama of ["hd2", "hd3"]) {
    hasilResolve = async () => {
      throw new Error("semua backend hd gagal");
    };
    const m = pesanPengguna();
    m.command = nama;
    const sock = sockPalsu();
    await plugin[nama].handler(m, { sock });

    assert.deepEqual(REAKSI, ["🕕", "☢"], nama);
    assert.equal(TERKIRIM.length, 0, `${nama} tidak boleh mengirim apa-apa`);
    assert.equal(BALASAN.length, 1, nama);
    assert.ok(BALASAN[0].includes("Kayaknya command"), nama);
    assert.ok(BALASAN[0].includes(nama), `${nama} harus muncul di template: ${BALASAN[0]}`);
    assert.doesNotMatch(BALASAN[0], /semua backend hd gagal/, `${nama} membocorkan error internal`);
  }
});

test("berkas sementara tetap dihapus ketika resolve gagal", async () => {
  hasilResolve = async () => {
    throw new Error("semua backend hd gagal");
  };
  const m = pesanPengguna();
  await plugin.hd2.handler(m, { sock: sockPalsu() });

  const media = PANGGILAN[0].args.media;
  assert.equal(fs.existsSync(media), false, `sisa gambar user tertinggal: ${media}`);
});

test("berkas sementara dihapus ketika pengiriman gagal", async () => {
  // `sendMessage` melempar setelah resolve sukses, dan `catch` plugin menelan
  // error itu. Kalau pembersihan hanya di jalur sukses, setiap file yang gagal
  // dikirim akan menumpuk di disk.
  hasilResolve = async () => JAWAB;
  const sock = {
    async sendMessage() {
      throw new Error("kirim gagal");
    },
  };
  const m = pesanPengguna();
  await plugin.hd2.handler(m, { sock });

  const media = PANGGILAN[0].args.media;
  assert.equal(fs.existsSync(media), false, `sisa gambar user tertinggal: ${media}`);
  assert.deepEqual(REAKSI, ["🕕", "✅", "☢"]);
});

test("berkas sementara tidak menumpuk di tmp/ setelah banyak permintaan", async () => {
  hasilResolve = async () => JAWAB;
  const sebelum = fs.readdirSync(os.tmpdir()).filter((n) => n.startsWith("hd-")).length;

  for (let i = 0; i < 3; i++) {
    const m = pesanPengguna();
    await plugin.hd2.handler(m, { sock: sockPalsu() });
  }

  const sesudah = fs.readdirSync(os.tmpdir()).filter((n) => n.startsWith("hd-")).length;
  assert.equal(sesudah, sebelum, "nama berkas sementara harus unik per permintaan");
});

// ── permukaan plugin ──────────────────────────────────────────────────────────

test("config kedua plugin tidak berubah", () => {
  assert.deepEqual(plugin.hd2.config, {
    name: "hd2",
    alias: ["enhance2", "upscale2", "aienhancer"],
    category: "tools",
    description: "Enhance gambar menjadi HD dengan AI (V3)",
    usage: ".hd2 (reply gambar)",
    example: ".hd2",
    isOwner: false,
    isPremium: false,
    isGroup: false,
    isPrivate: false,
    cooldown: 30,
    energi: 2,
    isEnabled: true,
  });
  assert.deepEqual(plugin.hd3.config, {
    name: "hd3",
    alias: ["enhance3", "upscale3", "unblur"],
    category: "tools",
    description: "Memperjelas gambar blur menjadi tajam dengan AI (Unblur)",
    usage: ".hd3 (reply gambar)",
    example: ".hd3",
    cooldown: 20,
    energi: 2,
    isEnabled: true,
  });
});