import { test, mock } from "node:test";
import assert from "node:assert";

mock.module("../src/lib/error.js", {
  defaultExport: (prefix, command, pushName) =>
    `error ${prefix}${command} ${pushName}`,
});

const btchResult = {
  developer: "BOTCAHX",
  status: true,
  result: [
    {
      thumbnail: "https://example.com/thumb.jpg",
      url: "https://d.rapidcdn.app/v2?token=h." +
        Buffer.from(
          JSON.stringify({ url: "https://scontent.example.com/x.mp4?st=1" }),
        ).toString("base64") +
        ".s",
    },
  ],
};

let btchImpl = async () => btchResult;

mock.module("btch-downloader", {
  namedExports: {
    igdl: async (url) => btchImpl(url),
  },
});

let txt2qr = null;
let qrcustom = null;
let ig = null;
let galatImpor = null;
try {
  txt2qr = await import("../plugins/tools/txt2qr.js");
  qrcustom = await import("../plugins/tools/qrcustom.js");
  ig = await import("../src/scraper/ig.js");
} catch (error) {
  galatImpor = error;
}

test("sentinel: modul terimpor", () => {
  assert.equal(galatImpor, null, `impor gagal: ${galatImpor?.message ?? galatImpor}`);
  assert.equal(typeof txt2qr?.handler, "function");
  assert.equal(typeof qrcustom?.handler, "function");
  assert.equal(typeof ig?.default, "function");
});

function konteks() {
  const balasan = [];
  const reaksi = [];
  const terkirim = [];
  const m = {
    text: "hello",
    args: ["hello"],
    prefix: ".",
    command: "txt2qr",
    pushName: "Tester",
    chat: "chat@s.whatsapp.net",
    reply: async (t) => balasan.push(t),
    react: async (e) => reaksi.push(e),
  };
  const sock = {
    sendMessage: async (chat, payload, opts) => terkirim.push({ chat, payload, opts }),
  };
  return { balasan, reaksi, terkirim, m, sock };
}

test("txt2qr menghasilkan PNG lokal tanpa panggil API", async () => {
  const { reaksi, terkirim, m, sock } = konteks();
  await txt2qr.handler(m, { sock });
  assert.equal(terkirim.length, 1);
  const img = terkirim[0].payload.image;
  assert.ok(Buffer.isBuffer(img));
  assert.deepEqual([...img.subarray(0, 4)], [0x89, 0x50, 0x4e, 0x47], "harus PNG");
  assert.equal(reaksi.at(-1), "✅");
});

test("qrcustom menghasilkan PNG dan logo menyatu", async () => {
  const { reaksi, terkirim, m, sock } = konteks();
  await qrcustom.handler(m, { sock });
  assert.equal(terkirim.length, 1);
  const img = terkirim[0].payload.image;
  assert.deepEqual([...img.subarray(0, 4)], [0x89, 0x50, 0x4e, 0x47], "harus PNG");
  assert.equal(reaksi.at(-1), "📱");
});

test("ig: media terpetakan, video terdeteksi dari token", async () => {
  const r = await ig.default("https://www.instagram.com/reel/x");
  assert.equal(r.status, true);
  assert.equal(r.media.length, 1);
  assert.equal(r.media[0].type, "video");
  assert.ok(r.media[0].url.startsWith("https://"));
});

test("ig: upstream gagal melempar error, bukan media kosong", async () => {
  btchImpl = async () => ({ status: false, result: [] });
  try {
    await ig.default("https://www.instagram.com/reel/x");
    assert.fail("harus melempar");
  } catch (e) {
    assert.match(e.message, /Gagal/);
  } finally {
    btchImpl = async () => btchResult;
  }
});
