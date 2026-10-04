import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";

const calls = [];
let SPOTYLOADER_OK;
let galat = null;

async function postPalsu(url, body, config) {
  calls.push({ url, body, config });
  if (galat) throw galat;
  return { data: SPOTYLOADER_OK };
}

mock.module("../src/lib/http.js", {
  namedExports: { httpAxios: { post: postPalsu } },
});

// Versi spotify.js SEBELUM task ini meng-import axios mentah, jadi mock di
// ../src/lib/http.js saja tidak menyintercept request di module scope — test
// jadi menembak spotyloader.com sungguhan. Mock axios juga menjaga test ini
// offline dan membuat efek samping import benar-benar terlihat di `calls`.
mock.module("axios", { defaultExport: { post: postPalsu } });

const { downloadSpotify } = await import("../src/scraper/spotify.js");

// Di-snapshot sebelum test apa pun jalan. Kalau blok CLI belum di-guard,
// pemanggilan di module scope sudah tercatat di sini — dan reset `calls` di
// beforeEach tidak boleh bisa menutupi itu.
const callsSaatImport = calls.length;

beforeEach(() => {
  calls.length = 0;
  galat = null;
  SPOTYLOADER_OK = {
    downloadLink: "https://cdn/a.mp3",
    post: { name: "Judul", artist: "Artis", mime: "audio/mpeg" },
  };
});

test("import modul tidak menembak request", () => {
  assert.equal(
    callsSaatImport,
    0,
    `import harus bebas efek samping, dapat ${callsSaatImport} request`,
  );
});

test("downloadSpotify mengembalikan judul, artis, url, dan mime", async () => {
  const out = await downloadSpotify("https://open.spotify.com/track/x");
  assert.deepEqual(out, {
    title: "Judul",
    artist: "Artis",
    url: "https://cdn/a.mp3",
    mime: "audio/mpeg",
  });
});

test("POST dikirim ke endpoint spotyloader dengan body { url } dan header terverifikasi", async () => {
  await downloadSpotify("https://open.spotify.com/track/x");
  assert.equal(calls.length, 1);
  const call = calls[0];
  assert.equal(call.url, "https://spotyloader.com/api/spotify/track");
  assert.deepEqual(call.body, { url: "https://open.spotify.com/track/x" });
  const h = call.config.headers;
  assert.equal(h["Content-Type"], "application/json");
  assert.equal(h.Referer, "https://spotyloader.com/");
  assert.equal(h.Origin, "https://spotyloader.com");
  assert.match(h["User-Agent"], /^Mozilla\/5\.0/);
});

test("signal diteruskan ke httpAxios, supaya budget resolver bisa memutus request", async () => {
  // Tanpa ini, AbortController milik resolve.js hanya menghentikan resolver
  // menunggu: POST-nya tetap jalan sampai timeout 15 detik milik httpAxios.
  const controller = new AbortController();
  await downloadSpotify("https://open.spotify.com/track/x", { signal: controller.signal });
  assert.equal(
    calls[0].config.signal,
    controller.signal,
    "AbortSignal harus masuk ke config post",
  );
});

test("panggilan tanpa signal tetap jalan — signature lama tidak boleh pecah", async () => {
  await downloadSpotify("https://open.spotify.com/track/x");
  assert.equal(calls.length, 1);
  assert.equal(calls[0].config.signal, undefined, "tanpa signal, axios tidak boleh dapat signal palsu");
});

test("respons tanpa downloadLink ditolak, bukan diloloskan sebagai data kosong", async () => {
  SPOTYLOADER_OK = { post: { name: "Judul" }, error: "track tidak ditemukan" };
  await assert.rejects(
    () => downloadSpotify("https://open.spotify.com/track/x"),
    (error) => {
      assert.ok(error instanceof Error);
      // Pesan harus menyebut responsnya, kalau tidak user cuma lihat "gagal".
      assert.match(error.message, /downloadLink/);
      assert.match(error.message, /track tidak ditemukan/);
      return true;
    },
  );
});

test("error dari post dilempar, bukan ditelan ke console.error", async () => {
  galat = Object.assign(new Error("ECONNREFUSED spotyloader.com"), {
    code: "ECONNREFUSED",
  });
  const original = console.error;
  const ditelan = [];
  console.error = (...args) => {
    ditelan.push(args.map(String).join(" "));
  };
  try {
    await assert.rejects(
      () => downloadSpotify("https://open.spotify.com/track/x"),
      (error) => {
        assert.match(error.message, /ECONNREFUSED spotyloader\.com/);
        // Tidak ada respons HTTP ⇒ tidak ada status. Kehilangan status inilah
        // yang membuat resolver tidak bisa membedakan DNS gagal dari 500.
        assert.equal(error.status, undefined);
        return true;
      },
    );
  } finally {
    console.error = original;
  }
  assert.deepEqual(ditelan, [], `error tidak boleh ditelan lewat console.error: ${ditelan}`);
});

test("error upstream menyimpan cause asli dan status HTTP", async () => {
  const asli = Object.assign(new Error("Request failed with status code 403"), {
    code: "ERR_BAD_REQUEST",
    // Body kosong: persis yang Cloudflare balas saat memblokir. Kalau `??`
    // dipakai di sini, user dapat pesan "spotyloader gagal: " tanpa isi.
    response: { status: 403, data: "" },
  });
  galat = asli;

  await assert.rejects(
    () => downloadSpotify("https://open.spotify.com/track/x"),
    (error) => {
      // Status HTTP harus terbaca langsung supaya consumer bisa memilih
      // backend fallback berdasarkan bentuk kegagalan, bukan tebakan.
      assert.equal(error.status, 403);
      assert.equal(error.cause, asli, "error asli harus utuh di cause");
      assert.equal(error.cause.code, "ERR_BAD_REQUEST", "code asli harus terjangkau");
      // Body kosong harus jatuh ke error.message, bukan jadi pesan kosong.
      assert.match(error.message, /spotyloader gagal: Request failed with status code 403/);
      return true;
    },
  );
});
