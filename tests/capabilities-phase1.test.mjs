import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
import FormData from "form-data";
import { CapabilityError, createResolver } from "../src/lib/resolve.js";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Modul kapabilitas diimpor sungguhan (tidak dimock), jadi import-nya ikut
// diuji: file yang hilang harus menggagalkan test ini, bukan lolos diam-diam.
// Yang dimock adalah transport di bawahnya — `httpAxios` yang dipakai
// spotyloader dan oleh `aggregator.hit` — sehingga tidak ada satu pun request
// yang keluar ke jaringan dan upstream yang sedang mati tidak bisa membuat
// test berkedip merah.

const PANGGILAN_HTTP = [];
const PANGGILAN_SEARCH = [];

let balasSpotyloader = () => {
  throw new Error("http tidak boleh dipanggil: upstream mati");
};
let hasilAggregator = () => {
  throw new Error("aggregator tidak boleh dipanggil: upstream mati");
};

const httpPalsu = {
  async get(url, opts) {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    // GET hanya lewat `aggregator.hit`, jadi sama-sama harus bisa dikendalikan
    // test. `status` wajib ikut: aggregator menolak respons tanpa status 2xx.
    if (url.includes("nexray.eu.cc")) return hasilAggregator(url, opts);
    throw new Error("GET ke sumber daya luar tidak diizinkan di test ini");
  },
  async post(url, body, config) {
    PANGGILAN_HTTP.push({ verb: "post", url, config });
    return balasSpotyloader(url, body, config);
  },
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu } });

// Tripwire, bukan harness: backend `ytmusic` sudah dicabut dari kapabilitas ini
// karena YT Music tidak pernah menghasilkan ID track Spotify. Mock ini sengaja
// dibiarkan supaya kalau `ytmusic-api` sampai diimpor lagi, test gagal dengan
// pesan yang jelas — bukan diam-diam menembak jaringan.
class YTMusicPalsu {
  async initialize() {}
  async search(q) {
    PANGGILAN_SEARCH.push(q);
    throw new Error("ytmusic-api tidak boleh dipakai untuk kapabilitas spotify");
  }
}
mock.module("ytmusic-api", { defaultExport: YTMusicPalsu });

const kapspotify = await import("../src/capabilities/spotify.js");

const backendLokal = kapspotify.backends.find((b) => b.kind === "local");
const backendApi = kapspotify.backends.find((b) => b.kind === "api");

function resolverUji() {
  // Registry diganti agar test ini tidak ikut bergantung pada tujuh modul
  // kapabilitas lain yang belum dibuat.
  return createResolver({ capabilities: { spotify: () => kapspotify } });
}

beforeEach(() => {
  PANGGILAN_HTTP.length = 0;
  PANGGILAN_SEARCH.length = 0;
  balasSpotyloader = () => {
    throw new Error("http tidak boleh dipanggil: upstream mati");
  };
  hasilAggregator = () => {
    throw new Error("aggregator tidak boleh dipanggil: upstream mati");
  };
});

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: bentuk unduhan satu lagu jadi empat field", () => {
  const keluar = kapspotify.normalize({
    title: "Grateful",
    artist: "Neffex",
    url: "https://cdn.example/a.mp3",
    mime: "audio/mpeg",
  });
  assert.deepEqual(keluar, {
    title: "Grateful",
    artist: "Neffex",
    url: "https://cdn.example/a.mp3",
    mime: "audio/mpeg",
  });
});

test("normalize: bentuk pencarian jadi { tracks } dengan lima field per track", () => {
  // `thumbnail` adalah nama field yang benar-benar dikirim agregator; `cover`
  // hanya nama keluar dari normalize, bukan nama masuk.
  const keluar = kapspotify.normalize({
    tracks: [
      {
        title: "Grateful",
        artist: "Neffex",
        url: "https://open.spotify.com/track/abc",
        thumbnail: "https://cdn.example/c.jpg",
        duration: "3:45",
      },
    ],
  });
  assert.ok(Array.isArray(keluar.tracks));
  assert.deepEqual(keluar.tracks, [
    {
      title: "Grateful",
      artist: "Neffex",
      url: "https://open.spotify.com/track/abc",
      cover: "https://cdn.example/c.jpg",
      duration: "3:45",
    },
  ]);
  assert.equal(keluar.title, undefined, "bentuk pencarian tidak boleh punya field lagu");
});

test("normalize: durasi dari aggregator (teks) diteruskan apa adanya", () => {
  const keluar = kapspotify.normalize({
    tracks: [{ title: "Grateful", url: "https://open.spotify.com/track/a", duration: "3:45" }],
  });
  assert.equal(keluar.tracks[0].duration, "3:45");
});

test("normalize: entri aggregator tanpa judul dibuang, bukan jadi baris kosong", () => {
  // Daftar hasil yang dicetak ke user: entri bertitle kosong jadi baris "*1. *"
  // yang tidak bisa diidentifikasi. Ditolak di normalize supaya backend lain
  // masih sempat mencoba sebelum hasilnya tampil.
  const keluar = kapspotify.normalize({
    tracks: [
      { url: "https://open.spotify.com/track/a" },
      { title: "Grateful", url: "https://open.spotify.com/track/b" },
    ],
  });
  assert.equal(keluar.tracks.length, 1);
  assert.equal(keluar.tracks[0].title, "Grateful");
});

test("normalize: entri tanpa URL apa pun dibuang, daftar boleh kosong", () => {
  assert.deepEqual(kapspotify.normalize({ tracks: [{ title: "Tanpa link" }] }), { tracks: [] });
});

test("normalize(null) melempar, bukan mengembalikan bentuk kosong", () => {
  assert.throws(() => kapspotify.normalize(null), /tidak dikenali/);
});

test("normalize: array kosong tetap bentuk yang dikenali, bukan kegagalan", () => {
  assert.deepEqual(kapspotify.normalize({ tracks: [] }), { tracks: [] });
  assert.deepEqual(kapspotify.normalize([]), { tracks: [] });
});

test("normalize: nilai yang bukan objek ditolak", () => {
  for (const nilai of ["teks", 7, true, undefined]) {
    assert.throws(() => kapspotify.normalize(nilai), /tidak dikenali/, `nilai: ${String(nilai)}`);
  }
});

test("normalize: respons tanpa URL unduhan ditolak, bukan url kosong", () => {
  assert.throws(
    () => kapspotify.normalize({ title: "Grateful", artist: "Neffex" }),
    /URL unduhan/,
  );
});

test("normalize: nama field aggregator lama tetap terbaca", () => {
  // Peta agregator memakai `download_url` dan `thumbnail`, scraper lokal
  // memakai `url` dan `mime`. Kalau alias ini hilang, fallback aggregator
  // hanya bisa gagal diam-diam.
  const keluar = kapspotify.normalize({
    title: "Grateful",
    artist: "Neffex",
    download_url: "https://cdn.example/a.mp3",
    mimetype: "audio/mpeg",
  });
  assert.equal(keluar.url, "https://cdn.example/a.mp3");
  assert.equal(keluar.mime, "audio/mpeg");
});

test("normalize: entri aggregator tanpa URL unduhan ditolak", () => {
  assert.throws(
    () => kapspotify.normalize({ title: "Grateful", artist: "Neffex" }),
    /URL unduhan/,
  );
});

// ── Review Focus #1: guard URL ────────────────────────────────────────────────

test("guard: link Spotify salah format ditolak tanpa satu pun request HTTP", async () => {
  await assert.rejects(
    () => backendLokal.run({ url: "spotify:track:abc" }),
    (error) => {
      assert.ok(error instanceof Error);
      // Pesan wajib menyebut format yang diterima: tanpa itu user hanya
      // melihat "gagal" dan tidak bisa tahu apa yang harus diketik.
      assert.match(error.message, /open\.spotify\.com\/track/);
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_HTTP, [], "guard harus jalan sebelum scraper, bukan sesudahnya");
});

test("guard: resolve() juga melempar CapabilityError dan tetap tanpa request", async () => {
  const resolver = resolverUji();
  await assert.rejects(
    () => resolver.resolve("spotify", { url: "spotify:track:abc" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.capability, "spotify");
      assert.ok(error.tried.length > 0, "backend yang dicoba harus tercatat");
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_HTTP, []);
  assert.deepEqual(PANGGILAN_SEARCH, []);
});

test("guard: album, playlist, dan huruf besar diterima (semua bentuk punya track)", async () => {
  for (const url of [
    "https://open.spotify.com/album/abc",
    "https://open.spotify.com/playlist/abc",
    "HTTPS://OPEN.SPOTIFY.COM/TRACK/abc",
  ]) {
    balasSpotyloader = async () => ({
      data: {
        downloadLink: "https://cdn.example/a.mp3",
        post: { name: "Judul", artist: "Artis", mime: "audio/mpeg" },
      },
    });
    const keluar = await backendLokal.run({ url });
    assert.equal(keluar.url, "https://cdn.example/a.mp3", `harus diterima: ${url}`);
  }
});

test("guard: link open.spotify.com yang bukan track/album/playlist ditolak", async () => {
  await assert.rejects(
    () => backendLokal.run({ url: "https://open.spotify.com/artist/abc" }),
    /open\.spotify\.com\/track/,
  );
  assert.deepEqual(PANGGILAN_HTTP, []);
});

// ── kelas kegagalan scraper lokal ─────────────────────────────────────────────

test("upstream menjawab status → kelas 'ditolak upstream', status diteruskan", async () => {
  balasSpotyloader = async () => {
    throw Object.assign(new Error("Request failed with status code 403"), {
      response: { status: 403, data: "" },
    });
  };
  await assert.rejects(
    () => backendLokal.run({ url: "https://open.spotify.com/track/abc" }),
    (error) => {
      assert.match(error.message, /ditolak upstream/);
      assert.equal(error.status, 403, "status asli harus tetap terjangkau");
      assert.match(error.message, /403/);
      return true;
    },
  );
});

test("tidak ada respons sama sekali → kelas 'tidak terjangkau', status kosong", async () => {
  balasSpotyloader = async () => {
    throw Object.assign(new Error("getaddrinfo ENOTFOUND spotyloader.com"), {
      code: "ENOTFOUND",
    });
  };
  await assert.rejects(
    () => backendLokal.run({ url: "https://open.spotify.com/track/abc" }),
    (error) => {
      // Tidak boleh dipalsukan jadi 500: status kosong itulah yang membedakan
      // host mati dari host yang menjawab, dan itu yang menentukan backend
      // cadangan mana yang masih layak dicoba (konvensi serialize.js:560).
      assert.match(error.message, /tidak terjangkau/);
      assert.equal(error.status, undefined);
      return true;
    },
  );
});

test("respons 200 tanpa downloadLink → kelas 'track tidak ada di sana'", async () => {
  balasSpotyloader = async () => ({
    data: { post: { name: "Judul", artist: "Artis" }, error: "track tidak ditemukan" },
  });
  await assert.rejects(
    () => backendLokal.run({ url: "https://open.spotify.com/track/abc" }),
    (error) => {
      // Host hidup dan menjawab; hanya isinya yang kosong. Kelas ini justru
      // yang paling mungkin punya jalan keluar di aggregator lain.
      assert.match(error.message, /tidak ada di sana/);
      assert.equal(error.status, undefined);
      return true;
    },
  );
});

// ── wiring resolver ──────────────────────────────────────────────────────────

test("stable false: URL unduhan kedaluwarsa dan metadata bisa berubah", () => {
  assert.equal(
    kapspotify.stable,
    false,
    "dari cache tidak boleh ada: download URL spotyloader kedaluwarsa",
  );
});

test("nama backend per-host, bukan per-URL atau per-kueri", async () => {
  const resolver = resolverUji();
  const nama = new Set(kapspotify.backends.map((b) => b.name));
  assert.equal(
    nama.size,
    kapspotify.backends.length,
    "nama backend wajib unik per host, kalau tidak breaker salah hitung",
  );
  for (const n of nama) {
    assert.match(n, /^[a-z0-9][a-z0-9-]*$/, `nama backend bukan label host yang wajar: ${n}`);
  }
  assert.ok(
    nama.has("spotyloader"),
    `backend unduhan lokal harus bernama spotyloader, dapat ${[...nama].join(", ")}`,
  );

  // Argumen berbeda total, termasuk yang tidak dilayani backend mana pun, tapi
  // nama tetap sama. Kalau nama diturunkan dari argumen, breaker akan
  // allocate slot baru tiap permintaan dan host yang OPEN akan ter-evict lalu
  // terlihat sehat lagi.
  const varian = [
    { q: "satu" },
    { q: "dua" },
    { url: "https://open.spotify.com/track/abc" },
    { url: "https://open.spotify.com/album/zzz" },
    { q: "tiga" },
  ];
  for (const args of varian) {
    await resolver.resolve("spotify", args).catch(() => {});
  }
  const slot = new Set(resolver.breaker.snapshot().map((s) => s.name));
  assert.equal(
    [...slot].every((n) => nama.has(n)),
    true,
    `slot breaker hanya boleh berisi nama host yang dideklarasikan: ${[...slot].join(", ")}`,
  );
  assert.ok(
    slot.size <= nama.size,
    `jumlah slot tidak boleh melebihi jumlah nama backend: ${slot.size} > ${nama.size}`,
  );
});

test("minimal satu backend local dan satu api sebagai cadangan", () => {
  const lokal = kapspotify.backends.filter((b) => b.kind === "local");
  const api = kapspotify.backends.filter((b) => b.kind !== "local");
  assert.ok(lokal.length >= 1, `backend local minimal satu, dapat ${lokal.length}`);
  assert.ok(api.length >= 1, "harus ada cadangan aggregator, spotyloader bisa mati");
  assert.equal(api[0].name, "nexray", "host cadangan harus nexray (tanpa API key)");
});

test("tidak ada backend ytmusic: YT Music tidak pernah menghasilkan ID track Spotify", () => {
  // plugins/search/spotplay.js hanya memakai hasil yang berawalan
  // open.spotify.com/track/, dan YT Music hanya bisa mengembalikan tautan
  // YouTube. Satu-satunya sumber data Spotify di kapabilitas ini adalah
  // aggregator, jadi backend YT Music selalu mengambil giliran pertama pada
  // pencarian lalu tidak pernah bisa dipakai — dan tiringkan breaker dengan
  // kegagalan yang bukan kegagalan host.
  const nama = kapspotify.backends.map((b) => b.name);
  assert.equal(
    nama.includes("ytmusic"),
    false,
    `backend ytmusic harus dicabut, dapat: ${nama.join(", ")}`,
  );
  assert.deepEqual(
    kapspotify.backends.map((b) => b.kind),
    ["local", "api"],
    `harus tepat satu local dan satu api, dapat: ${nama.join(", ")}`,
  );
});

test("unduhan: spotyloader dulu, aggregator hanya jadi cadangan", async () => {
  const dipanggil = [];
  balasSpotyloader = async () => {
    dipanggil.push("spotyloader");
    return {
      data: { downloadLink: "https://cdn/a.mp3", post: { name: "J", artist: "A", mime: "audio/mpeg" } },
    };
  };
  hasilAggregator = async () => {
    dipanggil.push("nexray");
    return { data: { status: true, result: { url: "https://cdn/b.mp3" } } };
  };
  const resolver = resolverUji();
  const keluar = await resolver.resolve("spotify", { url: "https://open.spotify.com/track/abc" });
  assert.deepEqual(dipanggil, ["spotyloader"], "aggregator tidak boleh diakses sebelum host lokal");
  assert.equal(keluar.source, "spotyloader");
  assert.equal(keluar.data.url, "https://cdn/a.mp3");
});

test("pencarian: { q } tidak pernah menyentuh spotyloader, hasilnya tracks dari aggregator", async () => {
  hasilAggregator = async () => ({
    status: 200,
    data: {
      status: true,
      result: [
        {
          title: "Grateful",
          artist: "Neffex",
          url: "https://open.spotify.com/track/abc",
          thumbnail: "https://i.scdn.co/image/a.jpg",
          duration: "3:45",
        },
      ],
    },
  });
  const resolver = resolverUji();
  const keluar = await resolver.resolve("spotify", { q: "neffex grateful" });

  assert.equal(keluar.source, "nexray");
  assert.equal(keluar.data.tracks.length, 1);
  assert.deepEqual(keluar.data.tracks[0], {
    title: "Grateful",
    artist: "Neffex",
    url: "https://open.spotify.com/track/abc",
    cover: "https://i.scdn.co/image/a.jpg",
    duration: "3:45",
  });
  // Tombol `.spdl` di plugin hanya muncul kalau URL-nya track Spotify, dan
  // caption "tekan tombol di bawah ini" hanya benar kalau tombol itu ada.
  assert.match(keluar.data.tracks[0].url, /^https:\/\/open\.spotify\.com\/track\//);

  assert.deepEqual(PANGGILAN_SEARCH, [], "ytmusic-api tidak boleh dipakai untuk pencarian Spotify");
  assert.equal(
    PANGGILAN_HTTP.filter((c) => c.verb === "post").length,
    0,
    "pencarian tidak boleh menembak spotyloader: dia hanya menerima URL",
  );
});

test("pencarian: aggregator hidup tapi nihil → tracks kosong, plugin bisa bilang tidak ditemukan", async () => {
  // Jawaban: dengan hanya satu backend pencarian, daftar kosong adalah jawaban
  // yang jujur — bukan kegagalan yang perlu dilemparkan ke backend lain.
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });
  const resolver = resolverUji();
  const keluar = await resolver.resolve("spotify", { q: "tidak ada lagu ini" });
  assert.equal(keluar.source, "nexray");
  assert.deepEqual(keluar.data, { tracks: [] });
});

test("pencarian: aggregator mati → CapabilityError, tidak dilayani apa-apa", async () => {
  hasilAggregator = async () => {
    throw new Error("getaddrinfo ENOTFOUND api.nexray.eu.cc");
  };
  const resolver = resolverUji();
  await assert.rejects(
    () => resolver.resolve("spotify", { q: "neffex" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(
        error.tried.map((t) => t.name),
        ["nexray"],
        "spotyloader tidak berlaku untuk { q }, jadi tidak boleh ikut dicoba",
      );
      return true;
    },
  );
});

// ── budget: abort harus sampai ke lapisan HTTP ───────────────────────────────

test("signal dari resolver diteruskan ke config httpAxios", async () => {
  const controller = new AbortController();
  balasSpotyloader = async () => ({
    data: { downloadLink: "https://cdn/a.mp3", post: { name: "J", artist: "A", mime: "audio/mpeg" } },
  });
  await backendLokal.run(
    { url: "https://open.spotify.com/track/abc" },
    { signal: controller.signal },
  );
  const config = PANGGILAN_HTTP.at(-1).config;
  assert.equal(
    config?.signal,
    controller.signal,
    "backend wajib meneruskan ctx.signal ke lapisan HTTP",
  );
});

test("budget habis → request spotyloader benar-benar dibatalkan, bukan cuma ditinggalkan", async () => {
  // resolve.js:38-41 menjanjikan timeout memutus kerja. Kalau signal tidak
  // sampai ke httpAxios, resolve() selesai duluan tapi POST-nya masih berjalan
  // sampai timeout 15 detik milik httpAxios, memegang socket sepanjang itu.
  let sinyal = null;
  let terputus = false;
  balasSpotyloader = (url, body, config) =>
    new Promise((_, reject) => {
      sinyal = config?.signal ?? null;
      config?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });
  const resolver = createResolver({
    capabilities: { spotify: () => kapspotify },
    budget: { localMs: 60, totalMs: 400 },
  });

  await resolver.resolve("spotify", { url: "https://open.spotify.com/track/abc" }).catch(() => {});

  assert.ok(sinyal instanceof AbortSignal, "config httpAxios harus menerima AbortSignal");
  assert.equal(
    terputus,
    true,
    "abort() harus benar-benar mencabut request yang sedang berjalan",
  );
  assert.equal(sinyal.aborted, true);
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

const PLUGIN_SPOTIFY = [
  "plugins/download/spotifydl.js",
  "plugins/search/spotify.js",
  "plugins/search/spotplay.js",
];

test("tiga plugin tidak menyebut domain agregator lagi", () => {
  const pola = /nexray|neoxr|izuka|cuki|siputzx|azbry/i;
  const ketemu = [];
  for (const file of PLUGIN_SPOTIFY) {
    fs.readFileSync(path.join(process.cwd(), file), "utf8")
      .split("\n")
      .forEach((baris, i) => {
        if (pola.test(baris)) ketemu.push(`${file}:${i + 1}: ${baris.trim()}`);
      });
  }
  assert.deepEqual(ketemu, [], `domain agregator masih ada:\n${ketemu.join("\n")}`);
});

test("tiga plugin memakai resolver, bukan axios/aggregator langsung", () => {
  for (const file of PLUGIN_SPOTIFY) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    assert.match(
      sumber,
      /from "\.\.\/\.\.\/src\/lib\/resolve\.js"/,
      `${file} harus mengimpor resolver`,
    );
    assert.match(sumber, /resolver\.resolve\("spotify"/, `${file} harus resolve lewat resolver`);
  }
});
// ── applies: spotyloader tidak boleh terseret oleh lalu lintas pencarian ──────

test("tiga pencarian tidak boleh menyusun breaker spotyloader", async () => {
  // Breaker menghitung kegagalan backend sebagai kegagalan host. Backend yang
  // tidak berlaku untuk { q } tidak pernah dihubungi, jadi tidak boleh
  // menghitung kegagalan — kalau tidak, tiga ketikan `.spotify` sudah cukup
  // untuk mengeluarkan spotyloader yang sehat dari rotasi selama 30 detik.
  hasilAggregator = async () => {
    throw new Error("nexray mati");
  };
  const resolver = resolverUji();
  for (const q of ["satu", "dua", "tiga"]) {
    await resolver.resolve("spotify", { q }).catch(() => {});
  }

  const state = resolver.breaker.snapshot();
  assert.deepEqual(
    state.filter((s) => s.name === "spotyloader"),
    [],
    "spotyloader tidak boleh punya slot breaker setelah tiga pencarian",
  );
  assert.equal(
    resolver.breaker.isOpen("spotyloader"),
    false,
    "backend yang sehat harus tetap bisa dipakai",
  );
  assert.equal(
    state.find((s) => s.name === "nexray")?.failures,
    3,
    "hanya aggregator yang benar-benar dipanggil yang boleh dihitung",
  );
});

test("setelah tiga pencarian, unduhan tetap memakai spotyloader", async () => {
  hasilAggregator = async () => {
    throw new Error("nexray mati");
  };
  const resolver = resolverUji();
  for (const q of ["satu", "dua", "tiga"]) await resolver.resolve("spotify", { q }).catch(() => {});

  balasSpotyloader = async () => ({
    data: { downloadLink: "https://cdn/a.mp3", post: { name: "J", artist: "A", mime: "audio/mpeg" } },
  });
  const keluar = await resolver.resolve("spotify", { url: "https://open.spotify.com/track/abc" });
  assert.equal(keluar.source, "spotyloader", "pencarian sebelumnya tidak boleh mengganggu unduhan berikutnya");
});

test("argumen yang tidak dilayani backend mana pun → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverUji();
  await assert.rejects(
    () => resolver.resolve("spotify", {}),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, [], "tidak ada host yang gagal di sini");
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), []);
});

test("signal diteruskan ke request aggregator, bukan hanya ke spotyloader", async () => {
  const controller = new AbortController();
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });
  await backendApi.run({ q: "neffex" }, { signal: controller.signal });
  const call = PANGGILAN_HTTP.at(-1);
  assert.equal(call.verb, "get");
  assert.equal(
    call.opts?.signal,
    controller.signal,
    "tier api juga harus menghormati budget resolver",
  );
});

test("budget habis di tier api → request aggregator benar-benar dibatalkan", async () => {
  let terputus = false;
  let sinyal = null;
  hasilAggregator = (url, opts) =>
    new Promise((_, reject) => {
      sinyal = opts?.signal ?? null;
      opts?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });
  const resolver = createResolver({
    capabilities: { spotify: () => kapspotify },
    budget: { localMs: 60, totalMs: 160 },
  });

  await resolver.resolve("spotify", { q: "neffex" }).catch(() => {});

  assert.ok(sinyal instanceof AbortSignal, "aggregator harus menerima AbortSignal");
  assert.equal(terputus, true, "abort harus mencabut request aggregator");
});

// ═════════════════════════════════════════════════════════════════════════════
// Pinterest
//
// Scraping pinterest (src/scraper/pindl.js) memakai `httpAxios` supaya signal
// budget resolver benar-benar sampai ke request dan supaya transport-nya bisa
// dikendalikan di sini. Dua handler di bawah membungkus handler milik blok
// spotify tanpa mengubahnya: GET/POST yang tidak menyasar ilovepin.net
// diteruskan apa adanya, jadi blok Task 3 tidak ikut berubah.
// ═════════════════════════════════════════════════════════════════════════════

let balasIlovepin = () => {
  throw new Error("ilovepin tidak boleh dipanggil: upstream mati");
};
let balasProxy = () => {
  throw new Error("proxy ilovepin tidak boleh dipanggil: upstream mati");
};

const getSpotify = httpPalsu.get;
httpPalsu.get = async (url, opts) => {
  if (String(url).includes("ilovepin.net")) {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return balasIlovepin(url, opts);
  }
  // azbry masuk lewat `aggregator.hit` → GET juga, tapi harness blok spotify
  // hanya mengarahkan GET ke nexray.
  if (String(url).includes("azbry.com")) {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return hasilAggregator(url, opts);
  }
  return getSpotify(url, opts);
};

const postSpotify = httpPalsu.post;
httpPalsu.post = async (url, body, config) => {
  if (String(url).includes("ilovepin.net")) {
    PANGGILAN_HTTP.push({ verb: "post", url, config });
    return balasProxy(url, body, config);
  }
  return postSpotify(url, body, config);
};

const kappinterest = await import("../src/capabilities/pinterest.js");

const pinLokal = kappinterest.backends.find((b) => b.kind === "local");
const pinApi = kappinterest.backends.find((b) => b.kind === "api");

function resolverPin(opsi = {}) {
  return createResolver({
    capabilities: { pinterest: () => kappinterest },
    ...opsi,
  });
}

/** Bentuk yang benar-benar dikirim ilovepin.net, bukan bentuk kiriman plugin. */
function balasanProxy(mediaItems) {
  return async () => ({
    data: {
      api: {
        status: "OK",
        title: "Judul pin",
        description: "  deskripsi  ",
        userInfo: { name: "H", username: "heyeeog", userAvatar: "https://x/a.jpg" },
        mediaStats: { likesCount: "1K", sharesCount: "40K" },
        mediaItems,
      },
    },
  });
}

const VIDEO_ILOVEPIN = {
  type: "Video",
  mediaQuality: "HD",
  mediaRes: "720x1280",
  mediaExtension: "MP4",
  mediaFileSize: "2.16 MB",
  mediaUrl: "https://s15.mcontent.app/v3/videoProcess/1/720p",
};
const VIDEO_ILOVEPIN_KECIL = {
  type: "Video",
  mediaQuality: "SD",
  mediaRes: "486x864",
  mediaExtension: "MP4",
  mediaFileSize: "1.16 MB",
  mediaUrl: "https://s15.mcontent.app/v3/videoProcess/1/486p",
};
const GAMBAR_ILOVEPIN = {
  type: "Image",
  mediaExtension: "JPG",
  mediaFileSize: "1.20 MB",
  mediaUrl: "https://i.pinimg.com/originals/a.jpg",
};

beforeEach(() => {
  balasIlovepin = () => {
    throw new Error("ilovepin tidak boleh dipanggil: upstream mati");
  };
  balasProxy = () => {
    throw new Error("proxy ilovepin tidak boleh dipanggil: upstream mati");
  };
});

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: aggregator video jadi satu entri media video", () => {
  const keluar = kappinterest.normalize({
    type: "video",
    videos: [{ url: "https://x/a720p.mp4" }],
  });
  assert.deepEqual(keluar, {
    type: "video",
    media: [{ type: "video", url: "https://x/a720p.mp4" }],
  });
});

test("normalize: gambar aggregator memilih entri 'orig', bukan yang pertama", () => {
  const keluar = kappinterest.normalize({
    type: "image",
    images: [
      { name: "small", url: "https://x/s.jpg" },
      { name: "orig", url: "https://x/o.jpg" },
    ],
  });
  assert.deepEqual(keluar, {
    type: "image",
    media: [{ type: "image", url: "https://x/o.jpg" }],
  });
});

test("normalize: aggregator yang hanya mengirim `download` tetap jadi media gambar", () => {
  assert.deepEqual(kappinterest.normalize({ type: "image", download: "https://x/d.jpg" }), {
    type: "image",
    media: [{ type: "image", url: "https://x/d.jpg" }],
  });
});

test("normalize: gambar kosong tanpa download melempar, bukan media kosong", () => {
  // Plugin mengirim file apa yang ada di `media` lalu tetap memberi centang
  // hijau kalau ada satu pun yang terkirim. media: [] lolos berarti user melihat
  // "berhasil" padahal tidak menerima apa-apa, jadi respons kosong harus
  // diperlakukan sebagai kegagalan supaya backend lain sempat mencoba.
  assert.throws(
    () => kappinterest.normalize({ type: "image", images: [] }),
    /tanpa (media|URL)/i,
  );
  assert.throws(
    () => kappinterest.normalize({ type: "video", videos: [], download: "" }),
    /tanpa (media|URL)/i,
  );
});

test("normalize: bentuk scraper lokal jadi { type, media } dan hanya kualitas terbaik", () => {
  // Scraper lokal mengirim `media` yang sudah diurutkan dari ukuran terbesar dan
  // bisa berisi lima_VARIAN video untuk satu pin. Plugin mengirim semua entri
  // mediaList-nya, jadi seluruh varian ikut terkirim kalau normalize tidak
  // memilih satu.
  const keluar = kappinterest.normalize({
    title: "Judul pin",
    description: "  deskripsi  ",
    author: { name: "H", username: "heyeeog", avatar: "https://x/a.jpg" },
    stats: { likes: "1K", shares: "40K" },
    media: [
      { type: "video", quality: "HD", size: "2.16 MB", url: VIDEO_ILOVEPIN.mediaUrl },
      { type: "video", quality: "SD (486x864)", size: "1.16 MB", url: VIDEO_ILOVEPIN_KECIL.mediaUrl },
    ],
  });
  assert.deepEqual(keluar, {
    type: "video",
    media: [{ type: "video", url: VIDEO_ILOVEPIN.mediaUrl }],
  });
});

test("normalize: gambar dari scraper lokal jadi satu entri image", () => {
  assert.deepEqual(
    kappinterest.normalize({
      title: "Judul",
      media: [{ type: "image", quality: "Original", size: "1.2 MB", url: GAMBAR_ILOVEPIN.mediaUrl }],
    }),
    { type: "image", media: [{ type: "image", url: GAMBAR_ILOVEPIN.mediaUrl }] },
  );
});

test("normalize: pin tanpa media (hanya audio) melempar, bukan media kosong", () => {
  // src/scraper/pindl.js:50-91 memakai else-if, jadi pin yang isinya cuma audio
  // berakhir dengan `media: []`. Itu gagal, bukan pin kosong.
  assert.throws(
    () =>
      kappinterest.normalize({
        title: "_audio_",
        media: [],
      }),
    /tanpa (media|URL)/i,
  );
});

test("normalize: pencarian kosong tetap sah, dibedakan dari unduhan gagal", () => {
  // Daftar kosong di pencarian berarti "tidak ada yang cocok" — plugin lalu
  // bilang tidak ditemukan. Unduhan yang tidak punya media berarti gagal, jadi
  // keduanya tidak boleh memakai jalur yang sama.
  assert.deepEqual(kappinterest.normalize({ pins: [] }), { pins: [] });
  assert.deepEqual(kappinterest.normalize([]), { pins: [] });
});

test("normalize: hasil pencarian aggregator jadi { pins } dengan tiga field", () => {
  // Bentuk yang benar-benar dikirim aggregator: array di `result`, dan tiap
  // entri bisa menaruh URL gambarnya di `image` atau `images_url`.
  const keluar = kappinterest.normalize([
    { image: "https://i.pinimg.com/originals/a.jpg", title: "Zhao Lusi", link: "https://pin.it/abc" },
    { images_url: "https://i.pinimg.com/originals/b.jpg" },
  ]);
  assert.deepEqual(keluar, {
    pins: [
      { title: "Zhao Lusi", image: "https://i.pinimg.com/originals/a.jpg", link: "https://pin.it/abc" },
      { title: "", image: "https://i.pinimg.com/originals/b.jpg", link: "" },
    ],
  });
});

test("normalize: entri pencarian tanpa URL gambar dibuang, bukan jadi url kosong", () => {
  // Kedua plugin mengunduh URL itu sendiri; entri tanpa URL hanya menambah satu
  // request yang pasti ditolak.
  const keluar = kappinterest.normalize({
    pins: [{ title: "Tanpa gambar" }, { image: "https://i.pinimg.com/originals/b.jpg" }],
  });
  assert.equal(keluar.pins.length, 1);
  assert.equal(keluar.pins[0].image, "https://i.pinimg.com/originals/b.jpg");
});

test("normalize(null) dan nilai non-objek melempar", () => {
  for (const nilai of [null, undefined, "teks", 7, true]) {
    assert.throws(() => kappinterest.normalize(nilai), /tidak dikenali/);
  }
});

test("normalize: respons tanpa type dan tanpa media tidak ditebak diam-diam", () => {
  assert.throws(
    () => kappinterest.normalize({ foo: 1 }),
    /tidak dikenali/,
  );
});

// ── guard URL ─────────────────────────────────────────────────────────────────

test("guard: link Pinterest salah format ditolak tanpa satu pun request HTTP", async () => {
  await assert.rejects(
    () => pinLokal.run({ url: "https://example.com/bukan-pinterest" }),
    (error) => {
      assert.ok(error instanceof Error);
      // Pesan wajib menyebut format yang diterima: tanpa itu user hanya melihat
      // "gagal" dan tidak tahu apa yang harus diketik.
      assert.match(error.message, /pin\.it|pinterest\.com\/pin/);
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_HTTP, [], "guard harus jalan sebelum scraper, bukan sesudahnya");
});

test("guard: aggregator tidak dihubungi untuk link yang gagal guard", async () => {
  // Tanpa `applies` di backend api, penolakan guard di backend lokal akan
  // diteruskan ke aggregator: satu request yang pasti ditolak plus satu
  // kegagalan host untuk host yang tidak salah apa-apa. Yang tetap dihitung
  // kegagalan hanya backend yang benar-benar dipanggil.
  const resolver = resolverPin();
  await assert.rejects(() => resolver.resolve("pinterest", { url: "https://example.com/bukan-pinterest" }));

  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => String(c.url).includes("azbry.com")),
    [],
    "aggregator tidak boleh dihubungi untuk link yang gagal guard",
  );
  assert.deepEqual(
    resolver.breaker.snapshot().map((s) => s.name),
    ["ilovepin"],
    "slot breaker hanya boleh berisi host yang benar-benar dihubungi",
  );
});

test("guard: resolve() melempar CapabilityError dan tetap tanpa request", async () => {
  const resolver = resolverPin();
  await assert.rejects(
    () => resolver.resolve("pinterest", { url: "https://example.com/bukan-pinterest" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.capability, "pinterest");
      assert.ok(error.tried.length > 0, "backend yang dicoba harus tercatat");
      assert.match(error.tried[0].reason, /pin\.it|pinterest\.com\/pin/);
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_HTTP, []);
});

test("guard: bentuk link yang sah diterima — sub-domain, pin.it, huruf besar", async () => {
  const sah = [
    "https://pinterest.com/pin/87186942777228203/",
    "https://id.pinterest.com/pin/87186942777228203/",
    "https://ru.pinterest.com/pin/87186942777228203/?nic_v3=1",
    "https://www.pinterest.co.uk/pin/87186942777228203/",
    "https://pin.it/3abcd",
    "HTTPS://PIN.IT/3abcd",
  ];
  for (const url of sah) {
    balasIlovepin = async () => ({ headers: { "set-cookie": ["s=1"] } });
    balasProxy = balasanProxy([GAMBAR_ILOVEPIN]);
    const keluar = await pinLokal.run({ url });
    assert.equal(keluar.media.length, 1, `harus diterima: ${url}`);
  }
});

test("guard: halaman Pinterest yang bukan pin ditolak", async () => {
  for (const url of [
    "https://pinterest.com/heyeeog/",
    "https://id.pinterest.com/search/pins/?q=cewe",
    "https://pin.it.evil.example/3abcd",
  ]) {
    await assert.rejects(() => pinLokal.run({ url }), /pin\.it|pinterest\.com\/pin/);
  }
  assert.deepEqual(PANGGILAN_HTTP, []);
});

// ── wiring resolver ──────────────────────────────────────────────────────────

test("stable false: pin bisa dihapus kapan saja dan URL unduhan bisa kedaluwarsa", () => {
  assert.equal(
    kappinterest.stable,
    false,
    "dari cache tidak boleh ada: pin dihapus owner-nya dan URL ilovepin basi",
  );
});

test("minimal satu backend local dan satu api sebagai cadangan", () => {
  const lokal = kappinterest.backends.filter((b) => b.kind === "local");
  const api = kappinterest.backends.filter((b) => b.kind !== "local");
  assert.ok(lokal.length >= 1, `backend local minimal satu, dapat ${lokal.length}`);
  assert.ok(api.length >= 1, "harus ada cadangan aggregator, scraper lokal bisa mati");
  assert.equal(lokal[0].name, "ilovepin");
  assert.equal(api[0].name, "azbry");
  assert.equal(new Set(kappinterest.backends.map((b) => b.name)).size, kappinterest.backends.length);
});

test("nama backend per-host, bukan per-URL atau per-kueri", async () => {
  const resolver = resolverPin();
  const nama = new Set(kappinterest.backends.map((b) => b.name));
  const varian = [
    { q: "satu" },
    { q: "dua" },
    { url: "https://pin.it/a" },
    { url: "https://pin.it/b" },
    { url: "https://example.com/bukan-pinterest" },
  ];
  for (const args of varian) await resolver.resolve("pinterest", args).catch(() => {});
  const slot = new Set(resolver.breaker.snapshot().map((s) => s.name));
  assert.ok(
    [...slot].every((n) => nama.has(n)),
    `slot breaker hanya boleh berisi nama host yang dideklarasikan: ${[...slot].join(", ")}`,
  );
  assert.ok(slot.size <= nama.size, `jumlah slot melebihi jumlah nama: ${slot.size}`);
});

test("unduhan: scraper lokal dulu, aggregator tidak boleh diakses lebih awal", async () => {
  const dipanggil = [];
  balasIlovepin = async () => {
    dipanggil.push("ilovepin:get");
    return { headers: { "set-cookie": ["s=1"] } };
  };
  balasProxy = async () => {
    dipanggil.push("ilovepin:post");
    return { data: { api: { status: "OK", mediaItems: [VIDEO_ILOVEPIN, VIDEO_ILOVEPIN_KECIL] } } };
  };
  hasilAggregator = async () => {
    dipanggil.push("azbry");
    return { status: 200, data: { status: true, result: { type: "image", images: [{ name: "orig", url: "https://x/o.jpg" }] } } };
  };

  const keluar = await resolverPin().resolve("pinterest", { url: "https://pin.it/abc" });

  assert.deepEqual(dipanggil, ["ilovepin:get", "ilovepin:post"], "aggregator tidak boleh diakses sebelum host lokal");
  assert.equal(keluar.source, "ilovepin");
  assert.deepEqual(keluar.data, {
    type: "video",
    media: [{ type: "video", url: VIDEO_ILOVEPIN.mediaUrl }],
  });
});

test("unduhan: scraper lokal gagal → aggregator jadi cadangan", async () => {
  balasIlovepin = async () => ({ headers: {} });
  balasProxy = async () => {
    throw new Error("ilovepinproxy 502");
  };
  hasilAggregator = async () => ({
    status: 200,
    data: { status: true, result: { type: "image", images: [{ name: "small", url: "https://x/s.jpg" }, { name: "orig", url: "https://x/o.jpg" }] } },
  });

  const keluar = await resolverPin().resolve("pinterest", { url: "https://pin.it/abc" });
  assert.equal(keluar.source, "azbry");
  assert.deepEqual(keluar.data.media, [{ type: "image", url: "https://x/o.jpg" }]);
});

test("unduhan: pin yang dihapus dari aggregator → CapabilityError, bukan media basi", () => {
  balasIlovepin = async () => ({ headers: {} });
  balasProxy = async () => ({ data: { api: { status: "OK", title: "x", mediaItems: [] } } });
  hasilAggregator = async () => ({
    status: 200,
    data: { status: true, result: { type: "image", images: [] } },
  });

  return assert.rejects(
    () => resolverPin().resolve("pinterest", { url: "https://pin.it/abc" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["ilovepin", "azbry"]);
      return true;
    },
  );
});

test("pencarian: { q } tidak pernah menyentuh scraper lokal", async () => {
  hasilAggregator = async () => ({
    status: 200,
    data: {
      status: true,
      result: [
        { image: "https://i.pinimg.com/originals/a.jpg", title: "cewe", link: "https://pin.it/1" },
        { images_url: "https://i.pinimg.com/originals/b.jpg" },
      ],
    },
  });

  const keluar = await resolverPin().resolve("pinterest", { q: "cewe cantik indonesia" });

  assert.equal(keluar.source, "azbry");
  assert.equal(keluar.data.pins.length, 2);
  assert.equal(keluar.data.pins[0].image, "https://i.pinimg.com/originals/a.jpg");
  assert.equal(keluar.data.pins[1].image, "https://i.pinimg.com/originals/b.jpg");
  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => String(c.url).includes("ilovepin.net")),
    [],
    "pencarian tidak boleh menembak scraper lokal: dia hanya menerima link pin",
  );
});

test("pencarian: aggregator hidup tapi nihil → pins kosong, plugin bisa bilang tidak ditemukan", async () => {
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });
  const keluar = await resolverPin().resolve("pinterest", { q: "tidak ada ini" });
  assert.equal(keluar.source, "azbry");
  assert.deepEqual(keluar.data, { pins: [] });
});

test("pencarian: aggregator mati → CapabilityError tanpa mencoba scraper lokal", async () => {
  hasilAggregator = async () => {
    throw new Error("getaddrinfo ENOTFOUND api.azbry.com");
  };
  await assert.rejects(
    () => resolverPin().resolve("pinterest", { q: "cewe" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["azbry"]);
      return true;
    },
  );
});

test("tiga pencarian tidak boleh menyusun breaker scraper lokal", async () => {
  // Backend yang tidak berlaku untuk { q } tidak pernah dihubungi, jadi tidak
  // boleh menghitung kegagalan: kalau tidak, tiga ketikan `.pap` sudah cukup
  // untuk mengeluarkan scraper yang sehat dari rotasi selama 30 detik.
  hasilAggregator = async () => {
    throw new Error("azbry mati");
  };
  const resolver = resolverPin();
  for (const q of ["satu", "dua", "tiga"]) await resolver.resolve("pinterest", { q }).catch(() => {});

  const state = resolver.breaker.snapshot();
  assert.deepEqual(
    state.filter((s) => s.name === "ilovepin"),
    [],
    "scraper lokal tidak boleh punya slot breaker setelah tiga pencarian",
  );
  assert.equal(state.find((s) => s.name === "azbry")?.failures, 3);
});

test("setelah tiga pencarian, unduhan tetap memakai scraper lokal", async () => {
  hasilAggregator = async () => {
    throw new Error("azbry mati");
  };
  const resolver = resolverPin();
  for (const q of ["satu", "dua", "tiga"]) await resolver.resolve("pinterest", { q }).catch(() => {});

  balasIlovepin = async () => ({ headers: {} });
  balasProxy = balasanProxy([GAMBAR_ILOVEPIN]);
  const keluar = await resolver.resolve("pinterest", { url: "https://pin.it/abc" });
  assert.equal(keluar.source, "ilovepin", "pencarian sebelumnya tidak boleh mengganggu unduhan berikutnya");
});

test("argumen kosong → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverPin();
  await assert.rejects(
    () => resolver.resolve("pinterest", {}),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, []);
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), []);
});

// ── budget: abort harus sampai ke lapisan HTTP ───────────────────────────────

test("signal diteruskan ke request scraper lokal", async () => {
  const controller = new AbortController();
  balasIlovepin = async () => ({ headers: {} });
  balasProxy = balasanProxy([GAMBAR_ILOVEPIN]);
  await pinLokal.run({ url: "https://pin.it/abc" }, { signal: controller.signal });

  const get = PANGGILAN_HTTP.find((c) => c.verb === "get" && c.url.includes("ilovepin.net"));
  const post = PANGGILAN_HTTP.find((c) => c.verb === "post" && c.url.includes("ilovepin.net"));
  assert.equal(get?.opts?.signal, controller.signal, "request pertama harus menerima AbortSignal");
  assert.equal(post?.config?.signal, controller.signal, "request kedua harus menerima AbortSignal");
});

test("signal diteruskan ke request aggregator, bukan hanya ke scraper lokal", async () => {
  const controller = new AbortController();
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });
  await pinApi.run({ q: "cewe" }, { signal: controller.signal });
  assert.equal(
    PANGGILAN_HTTP.at(-1).opts?.signal,
    controller.signal,
    "tier api juga harus menghormati budget resolver",
  );
});

test("budget habis di tier lokal → request scraper lokal benar-benar dibatalkan", async () => {
  // Request yang menggantung di sini hanya bisa berhenti kalau signal benar-benar
  // sampai ke lapisan HTTP; kalau tidak, resolve() selesai duluan sementara
  // socket ke ilovepin.net masih hidup sampai timeout httpAxios.
  let terputus = false;
  balasIlovepin = async (url, opts) =>
    new Promise((_, reject) => {
      opts?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });

  const resolver = resolverPin({ budget: { localMs: 60, totalMs: 400 } });
  await resolver.resolve("pinterest", { url: "https://pin.it/abc" }).catch(() => {});
  assert.equal(terputus, true, "abort harus mencabut request scraper lokal");
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

const PLUGIN_PINTEREST = [
  "plugins/download/pindl.js",
  "plugins/search/pin.js",
  "plugins/search/pap.js",
];

test("tiga plugin tidak menyebut domain agregator lagi", () => {
  const pola = /nexray|neoxr|izuka|cuki|siputzx|azbry/i;
  const ketemu = [];
  for (const file of PLUGIN_PINTEREST) {
    fs.readFileSync(path.join(process.cwd(), file), "utf8")
      .split("\n")
      .forEach((baris, i) => {
        if (pola.test(baris)) ketemu.push(`${file}:${i + 1}: ${baris.trim()}`);
      });
  }
  assert.deepEqual(ketemu, [], `domain agregator masih ada:\n${ketemu.join("\n")}`);
});

test("tiga plugin memakai resolver dan tidak memanggil axios ke host aggregator", () => {
  for (const file of PLUGIN_PINTEREST) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    assert.match(sumber, /from "\.\.\/\.\.\/src\/lib\/resolve\.js"/, `${file} harus mengimpor resolver`);
    assert.match(sumber, /resolver\.resolve\("pinterest"/, `${file} harus resolve lewat resolver`);
    assert.doesNotMatch(
      sumber,
      /axios\.(get|post)\(\s*[`"']https?:\/\/api\./,
      `${file} masih menembak aggregator lewat axios`,
    );
  }
});

test("pindl: guard level plugin dipertahankan, bukan diganti guard yang lebih longgar", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/download/pindl.js"), "utf8");
  assert.match(
    sumber,
    /url\.includes\("pinterest"\)\s*&&\s*!url\.includes\("pin\.it"\)/,
    "guard plugin adalah lapis kedua dan tidak boleh dilonggarkan",
  );
  assert.match(sumber, /for \(const media of mediaList\)/, "plugin harus iterate data.media");
  assert.doesNotMatch(sumber, /res\.data\.result/, "blok parsing aggregator lama harus hilang");
});

test("pap: ekspansi query tetap milik plugin, satu resolve per query", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/search/pap.js"), "utf8");
  assert.match(sumber, /for \(const q of queries\)/, "loop per query harus tetap ada");
  assert.match(sumber, /resolver\.resolve\("pinterest", \{ q \}\)/, "setiap query harus lewat resolver");
  assert.match(sumber, /cewe:\s*\[/, "peta QUERIES tidak boleh hilang");
  assert.doesNotMatch(sumber, /__setDeps/, "hook DI sudah tidak dipakai test mana pun");
});

test("pin: album tetap dibangun dari buffer, bukan dari URL mentah", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/search/pin.js"), "utf8");
  assert.match(sumber, /responseType:\s*"arraybuffer"/, "gambar Pinterest harus diunduh jadi buffer");
  assert.match(sumber, /Referer:\s*"https:\/\/www\.pinterest\.com\/"/, "tanpa Referer Pinterest balas 403");
  assert.match(sumber, /expectedImageCount:\s*mediaList\.length/, "album harus tetap dikirim");
  assert.doesNotMatch(sumber, /from "\.\.\/\.\.\/src\/lib\/http\.js"/, "f() tidak lagi dipakai untuk aggregator");
});

// ═════════════════════════════════════════════════════════════════════════════
// Temuan review Task 4
//
// Di sini dikunci dua hal yang sebelumnya tidak pernah dicek: path aggregator
// yang lupa prefix /api, dan dua tebakan diam-diam di normalize. Uji level
// plugin (guard dan jalur unduhan video) ada di tests/pindl-plugin.test.mjs.
// ═════════════════════════════════════════════════════════════════════════════


// ── Critical: prefix /api pada host azbry ────────────────────────────────────

test("azbry: pathname harus persis /api/download/pinterest dan /api/search/pinterest", async () => {
  // `AGGREGATORS.azbry.base` di src/lib/aggregator.js hanya berisi host, dan
  // `aggregator.hit` menempelkan path apa adanya. Tujuh panggilan langsung dan
  // enam metode AzbryApiProvider di repo ini semuanya memakai /api lebih dulu,
  // jadi path yang lupa /api akan dijawab 404 — fallback yang justru jadi
  // alasan kapabilitas ini ada. Routing test lain berbasis substring tidak
  // bisa membedakan path benar dari path salah, jadi pathname dikunci di sini.
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });

  await pinApi.run({ url: "https://pin.it/abc" });
  await pinApi.run({ q: "cewe cantik indonesia" });

  const [unduh, cari] = PANGGILAN_HTTP.map((c) => new URL(c.url));
  assert.equal(unduh.origin, "https://api.azbry.com");
  assert.equal(unduh.pathname, "/api/download/pinterest");
  assert.equal(unduh.searchParams.get("url"), "https://pin.it/abc");
  assert.equal(cari.pathname, "/api/search/pinterest");
  assert.equal(cari.searchParams.get("q"), "cewe cantik indonesia");
});

test("azbry: setiap literal path aggregator di kapabilitas ini berawalan /api", () => {
  // Cakupan test ini sengaja sempit dan disebutkan apa adanya: dia memeriksa
  // literal yang ditulis di dalam berkas kapabilitas ini, bukan aturan per host.
  // Yang menegakkan aturan per host adalah test pathname di atas, karena ia
  // membaca URL yang benar-benar dibangun.
  //
  // Yang diperiksa di sini adalah setiap literal yang diawali "/" pada baris
  // kode, bukan hanya yang menempel pada `aggregator.hit`: memindahkan path ke
  // variabel, memakai template literal, atau menambah endpoint ketiga tidak akan
  // lolos diam-diam. Baris komentar dilewati supaya `/api` yang disebut di
  // dalam penjelasan tidak ikut terhitung.
  // Nexray memang tanpa prefix (`https://api.nexray.eu.cc/downloader/v2/...`),
  // jadi prefix ini harus ditulis eksplisit per host.
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/pinterest.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)].map((m) => m[0].slice(1, -1));
  assert.ok(literalPath.length >= 2, `path aggregator harus detectable, dapat ${literalPath.length}`);
  const salah = literalPath.filter((p) => !p.startsWith("/api/"));
  assert.deepEqual(salah, [], `path tanpa /api: ${salah.join(", ")}`);
});

// ── Minor: normalize jangan menebak diam-diam ────────────────────────────────

test("normalize: image kosong dari host tidak memblokir images_url", () => {
  // `??` hanya bereaksi pada null/undefined, jadi string kosong memblokir
  // images_url yang justru berisi URL. Plugin sebelum Phase 1 memakai `||`.
  const keluar = kappinterest.normalize({ pins: [{ image: "", images_url: "https://x/b.jpg" }] });
  assert.equal(keluar.pins.length, 1);
  assert.equal(keluar.pins[0].image, "https://x/b.jpg");
});

test("normalize: entri media dengan type tak dikenal dilempar, bukan ditebak jadi gambar", () => {
  // Audio — dan apa pun yang tidak dikenal — tidak boleh jadi
  // `{ type: "image" }`: plugin akan mengirim .mp3 sebagai gambar dan user
  // menerima file rusak yang dilaporkan sukses.
  assert.throws(
    () => kappinterest.normalize({ media: [{ type: "audio", url: "https://x/a.mp3" }] }),
    /tidak dikenal|tanpa media/i,
  );
});
// ═════════════════════════════════════════════════════════════════════════════
// Douyin
//
// Dua bentuk yang disagreed di sini bukan tebakan. Bentuk lokal dibaca dari
// src/scraper/douyin.js: `DouyinDL` mengembalikan objek yang SELALU ada —
// `{ status: false, error }` saat gagal, `{ status: true, video: null }` saat
// host menjawab tanpa media video. Bentuk aggregator dibaca dari plugin
// sebelum Phase 1: `res.data.status && res.data.result`, lalu `result.platform`,
// `result.title`, `result.video`, `result.audio`. Keduanya dinormalisasi ke
// empat field yang benar-benar dikonsumsi plugin.
// ═════════════════════════════════════════════════════════════════════════════

let balasSnap = () => {
  throw new Error("snapvideotools tidak boleh dipanggil: upstream mati");
};

const postPinterest = httpPalsu.post;
httpPalsu.post = async (url, body, config) => {
  if (String(url).includes("snapvideotools.com")) {
    PANGGILAN_HTTP.push({ verb: "post", url, config });
    return balasSnap(url, body, config);
  }
  return postPinterest(url, body, config);
};

const kapdouyin = await import("../src/capabilities/douyin.js");

const douyinLokal = kapdouyin.backends.find((b) => b.kind === "local");
const douyinApi = kapdouyin.backends.find((b) => b.kind === "api");

function resolverDouyin(opsi = {}) {
  return createResolver({ capabilities: { douyin: () => kapdouyin }, ...opsi });
}

/** Bentuk yang benar-benar dikirim snapvideotools.com, bukan bentuk keluar plugin. */
const SNAP_VIDEO = "https://v3.douyinvod.com/abc/video.mp4";
const SNAP_AUDIO = "https://v3.douyinvod.com/abc/audio.mp3";

// `DouyinDL` membaca `response.data.data`, jadi amplop host ada dua lapis:
// `data` (badan respons) lalu `data` (hasil scraping). Menulis satu lapis saja
// membuat scraper melihat `undefined` dan melaporkan "Data tidak ditemukan" —
// persis gejala yang akan muncul kalaubingkuk fixture ini dipakai.
function snapSukses(mediaUrls = [
  { type: "video", url: SNAP_VIDEO },
  { type: "audio", url: SNAP_AUDIO },
]) {
  balasSnap = async () => ({
    data: { data: { title: "Judul Douyin", platformName: "Douyin", mediaUrls } },
  });
}

/** Bentuk yang benar-benar dikirim azbry /downloader/douyin. */
const AZBRY = {
  status: 200,
  data: {
    status: true,
    result: {
      platform: "Douyin",
      title: "Judul Douyin",
      video: SNAP_VIDEO,
      audio: SNAP_AUDIO,
    },
  },
};

beforeEach(() => {
  balasSnap = () => {
    throw new Error("snapvideotools tidak boleh dipanggil: upstream mati");
  };
});

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: bentuk scraper lokal jadi empat field", () => {
  const keluar = kapdouyin.normalize({
    status: true,
    title: "Judul Douyin",
    platform: "Douyin",
    video: SNAP_VIDEO,
    audio: SNAP_AUDIO,
  });
  assert.deepEqual(keluar, {
    title: "Judul Douyin",
    platform: "Douyin",
    video: SNAP_VIDEO,
    audio: SNAP_AUDIO,
  });
});

test("normalize: bentuk aggregator punya field yang sama persis", () => {
  // Kegagalan Phase 4 ada di sini kalau nama field aggregator dan lokal
  // disamakan di normalize: dikunci ke bentuk yang benar-benar dikirim host,
  // bukan ke bentuk kiriman plugin.
  assert.deepEqual(
    kapdouyin.normalize({
      platform: "Douyin",
      title: "Judul Douyin",
      video: SNAP_VIDEO,
      audio: SNAP_AUDIO,
    }),
    { title: "Judul Douyin", platform: "Douyin", video: SNAP_VIDEO, audio: SNAP_AUDIO },
  );
});

test("normalize: status false adalah kegagalan, bukan data kosong", () => {
  // `DouyinDL` tidak melempar saat gagal — ia mengembalikan objek yang isinya
  // cuma `error`. Kalau normalize menerimanya, plugin akan mengirim tidak ada
  // file lalu tetap memberi centang hijau.
  assert.throws(
    () => kapdouyin.normalize({ status: false, error: "Data tidak ditemukan" }),
    /tidak ditemukan/i,
  );
});

test("normalize: video null melempar — plugin tidak boleh mengirim apa-apa", () => {
  //snapvideotools menjawab 200 dengan `mediaUrls` tanpa entri video. Objeknya
  //ada dan `status`-nya `true`, jadi tanpa pemeriksaan ini backend dianggap
  //berhasil dan aggregator tidak pernah diberi giliran.
  assert.throws(
    () =>
      kapdouyin.normalize({
        status: true,
        title: "Tanpa video",
        platform: "Douyin",
        video: null,
        audio: SNAP_AUDIO,
      }),
    /video/i,
  );
});

test("normalize: audio null tetap sah, post tanpa trek audio itu nyata", () => {
  assert.deepEqual(
    kapdouyin.normalize({
      status: true,
      title: "Tanpa audio",
      platform: "Douyin",
      video: SNAP_VIDEO,
      audio: null,
    }),
    { title: "Tanpa audio", platform: "Douyin", video: SNAP_VIDEO, audio: "" },
  );
});

test("normalize: URL media yang bukan http(s) ditolak, bukan diteruskan", () => {
  assert.throws(
    () =>
      kapdouyin.normalize({
        status: true,
        title: "x",
        platform: "Douyin",
        video: "data:video/mp4;base64,AAAA",
        audio: "",
      }),
    /video/i,
  );
});

test("normalize(null), normalize({}) dan nilai non-objek melempar", () => {
  for (const nilai of [null, undefined, {}, "teks", 7, true]) {
    assert.throws(() => kapdouyin.normalize(nilai), /tidak dikenali|video/i, `nilai: ${String(nilai)}`);
  }
});

// ── guard URL ─────────────────────────────────────────────────────────────────

test("guard: host yang bukan Douyin ditolak tanpa satu pun request", async () => {
  for (const url of [
    "https://douyin.com.evil.example/video/1",
    "https://notdouyin.com/video/1",
    "https://example.com/?u=douyin.com",
    "https://v.douyin.com.evil.example/abc/",
  ]) {
    await assert.rejects(
      () => douyinLokal.run({ url }),
      (error) => {
        // Pesan wajib menyebut format yang diterima: tanpa itu user hanya
        // melihat "gagal" dan tidak tahu apa yang harus diketik.
        assert.match(error.message, /douyin\.com|iesdouyin\.com/);
        return true;
      },
      `harus ditolak: ${url}`,
    );
  }
  assert.deepEqual(PANGGILAN_HTTP, [], "guard harus jalan sebelum scraper, bukan sesudahnya");
});

test("guard: link Douyin yang sah diterima — subdomain, iesdouyin, huruf besar", async () => {
  const sah = [
    "https://v.douyin.com/abc123/",
    "https://www.douyin.com/video/7123456789",
    "https://www.iesdouyin.com/share/video/7123456789/",
    "HTTPS://V.DOUYIN.COM/ABC123/",
  ];
  for (const url of sah) {
    snapSukses();
    const keluar = await douyinLokal.run({ url });
    assert.equal(keluar.video, SNAP_VIDEO, `harus diterima: ${url}`);
  }
});

test("guard: aggregator tidak dihubungi untuk link yang gagal guard", async () => {
  // Tanpa `applies` di backend api, penolakan guard diteruskan ke aggregator:
  // satu request yang pasti ditolak plus satu kegagalan untuk host yang tidak
  // salah apa-apa. Yang tetap dihitung kegagalan hanya backend yang benar-benar
  // dipanggil.
  hasilAggregator = async () => AZBRY;
  const resolver = resolverDouyin();
  await assert.rejects(() => resolver.resolve("douyin", { url: "https://douyin.com.evil.example/v/1" }));
  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => String(c.url).includes("azbry.com")),
    [],
    "aggregator tidak boleh dihubungi untuk link yang gagal guard",
  );
  assert.deepEqual(
    resolver.breaker.snapshot().map((s) => s.name),
    ["snapvideotools"],
    "slot breaker hanya boleh berisi host yang benar-benar dihubungi",
  );
});

test("guard: resolve() melempar CapabilityError dan tetap tanpa request", async () => {
  await assert.rejects(
    () => resolverDouyin().resolve("douyin", { url: "https://notdouyin.com/video/1" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.equal(error.capability, "douyin");
      assert.deepEqual(error.tried.map((t) => t.name), ["snapvideotools"]);
      assert.match(error.tried[0].reason, /douyin\.com/);
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_HTTP, []);
});

// ── wiring resolver ──────────────────────────────────────────────────────────

test("stable false: URL snapvideotools kedaluwarsa dan post bisa dihapus", () => {
  assert.equal(kapdouyin.stable, false, "dari cache tidak boleh ada: tautan unduhan berumur pendek");
});

test("minimal satu backend local dan satu api sebagai cadangan", () => {
  const lokal = kapdouyin.backends.filter((b) => b.kind === "local");
  const api = kapdouyin.backends.filter((b) => b.kind !== "local");
  assert.equal(lokal.length, 1, `backend local minimal satu, dapat ${lokal.length}`);
  assert.equal(api.length, 1, "harus ada cadangan aggregator, snapvideotools bisa mati");
  assert.equal(api[0].name, "azbry");
  assert.deepEqual(
    kapdouyin.backends.map((b) => b.name).sort(),
    [...new Set(kapdouyin.backends.map((b) => b.name))].sort(),
    "nama backend wajib unik per host",
  );
});

test("nama backend per-host: label host, bukan nama perintah atau per-URL", () => {
  for (const n of kapdouyin.backends.map((b) => b.name)) {
    assert.match(n, /^[a-z0-9][a-z0-9-]*$/, `nama backend bukan label host yang wajar: ${n}`);
  }
  assert.ok(
    kapdouyin.backends.some((b) => b.name === "snapvideotools"),
    `backend lokal harus bernama host snapvideotools, dapat ${kapdouyin.backends.map((b) => b.name).join(", ")}`,
  );
});

test("unduhan: scraper lokal dulu, aggregator tidak boleh diakses lebih awal", async () => {
  const dipanggil = [];
  balasSnap = async () => {
    dipanggil.push("snapvideotools");
    return { data: { data: { title: "Judul", platformName: "Douyin", mediaUrls: [{ type: "video", url: SNAP_VIDEO }] } } };
  };
  hasilAggregator = async () => {
    dipanggil.push("azbry");
    return AZBRY;
  };

  const keluar = await resolverDouyin().resolve("douyin", { url: "https://v.douyin.com/abc/" });

  assert.deepEqual(dipanggil, ["snapvideotools"]);
  assert.equal(keluar.source, "snapvideotools");
  assert.equal(keluar.data.video, SNAP_VIDEO);
});

test("unduhan: scraper lokal gagal → aggregator jadi cadangan", async () => {
  snapSukses([{ type: "audio", url: SNAP_AUDIO }]);
  hasilAggregator = async () => AZBRY;

  const keluar = await resolverDouyin().resolve("douyin", { url: "https://v.douyin.com/abc/" });

  assert.equal(keluar.source, "azbry");
  assert.equal(keluar.data.video, SNAP_VIDEO);
});

test("unduhan: aggregator hidup tapi nihil → CapabilityError, bukan URL basi", async () => {
  snapSukses([{ type: "audio", url: SNAP_AUDIO }]);
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: { platform: "Douyin" } } });

  await assert.rejects(
    () => resolverDouyin().resolve("douyin", { url: "https://v.douyin.com/abc/" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["snapvideotools", "azbry"]);
      return true;
    },
  );
});

// ── amplop `status` azbry tidak boleh hilang ─────────────────────────────────
//
// `lewatAzbry` pernah mengembalikan `body.result` apa adanya. `status` azbry ada
// satu level di atas `result`, jadi begitu amplop itu dibuang, tidak ada lagi
// yang bisa membacanya: `normalize` hanya punya `result`, dan `result` tidak
// pernah punya `status`. Plugin sebelum Phase 1 mensyaratkan
// `res.data?.status && res.data?.result` (plugins/download/douyindl.js) —
// pemeriksaan itu hilang di rewire.

test("backend aggregator wajib menolak status false, meski result-nya penuh", async () => {
  // `result` sengaja lengkap: kalau pemeriksaan status hilang, bentuk ini
  // normalize-nya sukses dan `video` sampai ke plugin.
  const body = {
    status: false,
    msg: "gagal",
    result: { platform: "Douyin", title: "Judul Douyin", video: SNAP_VIDEO, audio: SNAP_AUDIO },
  };
  hasilAggregator = async () => ({ status: 200, data: body });

  await assert.rejects(
    () => douyinApi.run({ url: "https://v.douyin.com/abc/" }),
    /aggregator menandai gagal/,
    "status false adalah kegagalan, bukan data",
  );
});

test("status aggregator yang hilang total juga gagal", async () => {
  // Bentuk tanpa `status` sama sekali bukan mungkin, tapi `hit` tidak menjamin
  // host menjawab sesuai skema, jadi backend tidak boleh menebak `false` berarti
  // baik. Plugin lama juga menolak apa pun yang bukan `status` truthy.
  for (const body of [undefined, null, {}, { result: { platform: "Douyin", video: SNAP_VIDEO } }]) {
    hasilAggregator = async () => ({ status: 200, data: body });
    await assert.rejects(
      () => douyinApi.run({ url: "https://v.douyin.com/abc/" }),
      /aggregator menandai gagal/,
      `harus gagal: ${JSON.stringify(body)}`,
    );
  }
});

test("status false yang tetap membawa video → tidak ada yang dikirim ke user", async () => {
  balasSnap = async () => {
    const e = new Error("Request failed with status code 503");
    e.response = { status: 503 };
    throw e;
  };
  hasilAggregator = async () => ({
    status: 200,
    data: {
      status: false,
      msg: "gagal",
      result: { platform: "Douyin", title: "Judul Douyin", video: SNAP_VIDEO, audio: SNAP_AUDIO },
    },
  });

  await assert.rejects(
    () => resolverDouyin().resolve("douyin", { url: "https://v.douyin.com/abc/" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["snapvideotools", "azbry"]);
      return true;
    },
  );
});

test("argumen kosong → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverDouyin();
  await assert.rejects(
    () => resolver.resolve("douyin", {}),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, []);
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), []);
});

// ── aggregator: URL harus dikunci persis ──────────────────────────────────────

test("azbry: pathname harus persis /api/downloader/douyin dan param url", async () => {
  // Routing berbasis substring tidak bisa membedakan path benar dari path
  // salah, dan path yang lupa `/api` dijawab 404 — fallback yang justru jadi
  // alasan kapabilitas ini ada.
  hasilAggregator = async () => AZBRY;

  await douyinApi.run({ url: "https://v.douyin.com/abc/" });

  const [req] = PANGGILAN_HTTP.map((c) => new URL(c.url));
  assert.equal(req.origin, "https://api.azbry.com");
  assert.equal(req.pathname, "/api/downloader/douyin");
  assert.equal(req.searchParams.get("url"), "https://v.douyin.com/abc/");
  // Berbeda dengan neoxr: azbry tidak punya key sama sekali (`AGGREGATORS.azbry.key`
  // = null), jadi memang tidak ada apikey yang boleh ikut terkirim. Ini sebabnya
  // assertion ini tetap sah meski bentuknya sama persis dengan assertion neoxr
  // yang lama. Bedanya: azbry memang tidak punya key.
  assert.equal(req.searchParams.has("apikey"), false);
  assert.deepEqual([...req.searchParams.keys()], ["url"], "param lain tidak boleh ikut terkirim");
  assert.ok(!("apikey" in (PANGGILAN_HTTP[0].opts?.headers ?? {})));
});

test("azbry: setiap literal path aggregator di kapabilitas ini berawalan /api", () => {
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/douyin.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)].map((m) => m[0].slice(1, -1));
  // `length >= 1` tidak bisa gagal di sini: regex di atas menangkap SEMUA string
  // literal yang diawali "/", jadi isinya pasti ada. Yang bisa gagal — dan yang
  // sebenarnya bermakna — adalah path yang benar-benar dipakai `aggregator.hit`
  // ikut di-scan. Kalau path itu dipindah ke variabel atau konstanta, `salah`
  // jadi kosong dan seluruh test ini hijau tanpa memeriksa apa pun.
  assert.ok(
    literalPath.includes("/api/downloader/douyin"),
    `path yang dipakai aggregator.hit harus terdeteksi, dapat: ${literalPath.join(", ")}`,
  );
  const salah = literalPath.filter((p) => !p.startsWith("/api/"));
  assert.deepEqual(salah, [], `path tanpa /api: ${salah.join(", ")}`);
});

// ── budget: abort harus sampai ke lapisan HTTP ───────────────────────────────

test("signal diteruskan ke request scraper lokal", async () => {
  const controller = new AbortController();
  snapSukses();
  await douyinLokal.run({ url: "https://v.douyin.com/abc/" }, { signal: controller.signal });
  assert.equal(PANGGILAN_HTTP.at(-1).config?.signal, controller.signal);
});

test("budget habis → request scraper lokal benar-benar dibatalkan", async () => {
  let terputus = false;
  balasSnap = (url, body, config) =>
    new Promise((_resolve, reject) => {
      config?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });

  await resolverDouyin({ budget: { localMs: 60, totalMs: 400 } })
    .resolve("douyin", { url: "https://v.douyin.com/abc/" })
    .catch(() => {});

  assert.equal(terputus, true, "abort harus mencabut request scraper lokal, bukan hanya resolver");
});

test("signal diteruskan ke request aggregator", async () => {
  const controller = new AbortController();
  hasilAggregator = async () => AZBRY;
  await douyinApi.run({ url: "https://v.douyin.com/abc/" }, { signal: controller.signal });
  assert.equal(PANGGILAN_HTTP.at(-1).opts?.signal, controller.signal);
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

const PLUGIN_DOUYIN = ["plugins/download/douyindl.js"];

test("plugin douyin tidak menyebut domain agregator lagi", () => {
  const pola = /nexray|neoxr|izuka|cuki|siputzx|azbry/i;
  const ketemu = [];
  for (const file of PLUGIN_DOUYIN) {
    fs.readFileSync(path.join(process.cwd(), file), "utf8")
      .split("\n")
      .forEach((baris, i) => {
        if (pola.test(baris)) ketemu.push(`${file}:${i + 1}: ${baris.trim()}`);
      });
  }
  assert.deepEqual(ketemu, [], `domain agregator masih ada:\n${ketemu.join("\n")}`);
});

test("plugin douyin memakai resolver, bukan axios/aggregator langsung", () => {
  for (const file of PLUGIN_DOUYIN) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    assert.match(
      sumber,
      /from "\.\.\/\.\.\/src\/lib\/resolve\.js"/,
      `${file} harus mengimpor resolver`,
    );
    assert.match(sumber, /resolver\.resolve\("douyin"/, `${file} harus resolve lewat resolver`);
    assert.doesNotMatch(
      sumber,
      /axios\.(get|post)\(\s*[`"']https?:\/\//,
      `${file} masih menembak host luar lewat axios`,
    );
    assert.doesNotMatch(sumber, /douyinFetch/, "blok parsing aggregator lama harus hilang");
  }
});

test("plugin douyin: guard host bertahan, dan lebih ketat dari `includes`", () => {
  // Guard plugin adalah lapis kedua dan tidak boleh dilonggarkan: tanpa itu
  // guard kapabilitas yang menolak, dan tiga link palsu dari satu user sudah
  // cukup menyusun breaker host yang sehat.
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/download/douyindl.js"), "utf8");
  assert.match(sumber, /host\.endsWith\("\.douyin\.com"\)/, "guard harus cek batas label host");
  assert.match(sumber, /host\.endsWith\("\.iesdouyin\.com"\)/);
  assert.match(
    sumber,
    /if \(!hostDouyin\(text\)\)/,
    "penolakan guard harus terjadi sebelum resolve",
  );
  assert.ok(
    sumber.indexOf("hostDouyin(text)") < sumber.indexOf('resolver.resolve("douyin"'),
    "guard harus mendahului panggilan resolver",
  );
});

test("plugin douyin: permukaan perintah dan reaksi tidak berubah", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/download/douyindl.js"), "utf8");
  assert.match(sumber, /name: "douyindl"/);
  assert.match(sumber, /alias: \["douyin", "dydl"\]/);
  assert.match(sumber, /category: "download"/);
  assert.match(sumber, /cooldown: 10/);
  assert.match(sumber, /energi: 1/);
  for (const reaksi of ['"🕕"', '"✅"', '"☢"', '"❌"']) {
    assert.match(sumber, new RegExp(`m\\.react\\(${reaksi}\\)`), `reaksi ${reaksi} harus tetap ada`);
  }
  // Dua media tetap dikirim seperti sebelumnya: video dengan caption, audio
  // tanpa caption. `normalize` menjamin `video` terisi, jadi blok
  // `if (data.video)` bukan lagi syarat yang bisa dilewati diam-diam.
  assert.match(sumber, /type: "video"/);
  assert.match(sumber, /type: "audio"/);
});

// ═════════════════════════════════════════════════════════════════════════════
// Sfile
//
// Bentuk lokal dibaca dari src/scraper/sfiledl.js: nama fieldnya `file_name`
// dan `size_from_text` — bukan `filename` dan `size` seperti yang diasumsikan
// di brief — dan `download_url` boleh `null` pada tiga jalur keluar yang berbeda
// (tanpa `og:url`, tanpa `#download`, dan regex gate yang tidak cocok). Bentuk
// aggregator dibaca dari plugin sebelum Phase 1: `neoxr/api/sfile` dibaca lewat
// `res.data.url`, `res.data.filename`, dan `res.data.mime`.
// ═════════════════════════════════════════════════════════════════════════════

const HAL_SFILE = [
  '<html><head><meta property="og:url" content="https://sfile.mobi/abc123" />' +
    '<meta property="og:description" content="uploaded by planetvdn on 22 January 2026" /></head>' +
    "<body><h1>Spotify PREMIUM_9.1.14.864</h1><p>100.42 MB</p>" +
    "<span>1.2k downloads</span></body></html>",
  '<html><body><a id="download" href="https://sfile.mobi/gate/abc123">Download</a></body></html>',
  '<html><body><script>var u="https:\\/\\/download0426.sfile.co\\/downloadfile\\/2200881\\/724157\\/' +
    '99f1f1d69f8fd931e1447af03be9cec0\\/spotify-premium_9.1.14.864.apk?k=8a782a007c623b62889412b2f5a49424";' +
    "</script></body></html>",
];

const SFILE_UNDUHAN =
  "https://download0426.sfile.co/downloadfile/2200881/724157/99f1f1d69f8fd931e1447af03be9cec0/" +
  "spotify-premium_9.1.14.864.apk?k=8a782a007c623b62889412b2f5a49424";

let halamanSfile = () => {
  throw new Error("sfile.mobi tidak boleh dipanggil: upstream mati");
};

const getDouyin = httpPalsu.get;
httpPalsu.get = async (url, opts) => {
  // Dicocokkan ke host, bukan ke seluruh string: URL aggregator neoxr untuk
  // kapabilitas ini memuat "sfile.mobi" di query string-nya, jadi pencocokan
  // substring akan mengarahkan request aggregator ke stub halaman sfile.
  const host = new URL(String(url)).hostname;
  if (host.endsWith("sfile.mobi") || host.endsWith("sfile.co")) {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return halamanSfile(url, opts);
  }
  if (host === "api.neoxr.eu") {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return hasilAggregator(url, opts);
  }
  return getDouyin(url, opts);
};

// `aggregator.hit` menolak neoxr sebelum ada request kalau key kosong, jadi
// key dipin di sini. Nilainya dikembalikan apa adanya di setiap respons —
// hanya yang dibaca test adalah hostname, pathname, dan param.
const configUji = (await import("../config.js")).default;
configUji.APIkey.neoxr = "k-neoxr-untuk-test";

const kapsfile = await import("../src/capabilities/sfile.js");

const sfileLokal = kapsfile.backends.find((b) => b.kind === "local");
const sfileApi = kapsfile.backends.find((b) => b.kind === "api");

function resolverSfile(opsi = {}) {
  return createResolver({ capabilities: { sfile: () => kapsfile }, ...opsi });
}

/**
 * Bentuk amplop neoxr yang sebenarnya. `f()` di plugin lama mengembalikan
 * body respons apa adanya, dan plugin itu membaca `res.data.url` — jadi `data`
 * adalah field di dalam body, bukan field HTTP axios. Stub di test memakai
 * `{ status, data }` sebagai respons HTTP axios, jadi body neoxr ada dua lapis.
 */
function neoxrSfile(body) {
  return async () => ({ status: 200, data: { data: body } });
}

beforeEach(() => {
  halamanSfile = () => {
    throw new Error("sfile.mobi tidak boleh dipanggil: upstream mati");
  };
});

/** Tiga request sfile() memang dijalankan berurutan; ini urutan itu. */
function sfileLokalSukses(hal = HAL_SFILE, catat = null) {
  let i = 0;
  halamanSfile = async (url, opts) => {
    if (catat) catat(url);
    return { data: hal[i++] ?? hal[hal.length - 1], headers: {} };
  };
}

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: bentuk scraper lokal dengan nama field aslinya", () => {
  // Nama field lokal adalah `file_name` dan `size_from_text`. Test ini gagal
  // kalau normalize mengira bentuk lokal memakai nama yang sama dengan
  // aggregator — asumsi yang membuat Task 3 sempat hampir salah di kapabilitas
  // spotify.
  const keluar = kapsfile.normalize({
    file_name: "Spotify PREMIUM_9.1.14.864",
    size_from_text: "100.42 MB",
    author_name: "Planetvdn",
    upload_date: "22 January 2026",
    download_count: "1",
    download_url: SFILE_UNDUHAN,
  });
  assert.deepEqual(keluar, {
    filename: "Spotify PREMIUM_9.1.14.864",
    url: SFILE_UNDUHAN,
    size: "100.42 MB",
    mime: "",
  });
});

test("normalize: bentuk aggregator neoxr jadi empat field yang sama", () => {
  // Plugin sebelum Phase 1 memakai `url`, `filename`, dan `mime`; `size` tidak
  // pernah dikirim host itu. Field yang tidak ada harus jadi string kosong,
  // bukan `undefined` yang tercetak di UI.
  assert.deepEqual(
    kapsfile.normalize({
      url: SFILE_UNDUHAN,
      filename: "Spotify PREMIUM_9.1.14.864",
      mime: "application/vnd.android.package-archive",
    }),
    {
      filename: "Spotify PREMIUM_9.1.14.864",
      url: SFILE_UNDUHAN,
      size: "",
      mime: "application/vnd.android.package-archive",
    },
  );
});

test("normalize: download_url null melempar, bukan jadi record kosong", () => {
  // `sfile()` mengembalikan `download_url: null` pada tiga jalur keluar yang
  // berbeda. Tanpa penolakan, plugin akan menjalankan sendMedia dengan `undefined`
  // lalu memberi centang hijau ke user yang tidak menerima apa-apa.
  assert.throws(
    () => kapsfile.normalize({ file_name: "x", size_from_text: "1 MB", download_url: null }),
    /URL|unduhan/i,
  );
});

test("normalize: respons kosong dan nilai non-objek melempar", () => {
  for (const nilai of [null, undefined, {}, "teks", 7, true]) {
    assert.throws(() => kapsfile.normalize(nilai), /tidak dikenali|URL|unduhan/i, `nilai: ${String(nilai)}`);
  }
});

test("normalize: URL download non-http ditolak", () => {
  assert.throws(
    () => kapsfile.normalize({ file_name: "x", download_url: "ftp://download.sfile.co/a.apk" }),
    /URL|unduhan/i,
  );
});

// ── guard URL ─────────────────────────────────────────────────────────────────

test("guard: host selain sfile ditolak tanpa satu pun request", async () => {
  for (const url of [
    "https://sfile.mobi.evil.example/abc",
    "https://notsfile.mobi/abc",
    "https://example.com/sfile.mobi",
  ]) {
    await assert.rejects(
      () => sfileLokal.run({ url }),
      (error) => {
        assert.match(error.message, /sfile\.mobi/);
        return true;
      },
      `harus ditolak: ${url}`,
    );
  }
  assert.deepEqual(PANGGILAN_HTTP, [], "guard harus jalan sebelum scraper");
});

test("guard: aggregator tidak dihubungi untuk link yang gagal guard", async () => {
  hasilAggregator = neoxrSfile({ url: SFILE_UNDUHAN, filename: "a.apk" });
  const resolver = resolverSfile();
  await assert.rejects(() => resolver.resolve("sfile", { url: "https://notsfile.mobi/abc" }));
  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => new URL(String(c.url)).hostname === "api.neoxr.eu"),
    [],
    "aggregator tidak boleh dihubungi untuk link yang gagal guard",
  );
  assert.deepEqual(
    resolver.breaker.snapshot().map((s) => s.name),
    ["sfile-mobi"],
  );
});

test("guard: link sfile.mobi dan sfile.co diterima", async () => {
  for (const url of [
    "https://sfile.mobi/abc123",
    "https://www.sfile.mobi/abc123",
    "https://sfile.co/abc123",
  ]) {
    sfileLokalSukses();
    const keluar = await sfileLokal.run({ url });
    assert.equal(keluar.download_url, SFILE_UNDUHAN, `harus diterima: ${url}`);
  }
});

// ── wiring resolver ──────────────────────────────────────────────────────────

test("stable false: URL unduhan sfile berbau token sekali pakai", () => {
  assert.equal(kapsfile.stable, false, "dari cache tidak boleh ada: URL gate sambil kedaluwarsa");
});

test("minimal satu backend local dan satu api sebagai cadangan", () => {
  const lokal = kapsfile.backends.filter((b) => b.kind === "local");
  const api = kapsfile.backends.filter((b) => b.kind !== "local");
  assert.equal(lokal.length, 1);
  assert.equal(api.length, 1);
  assert.equal(lokal[0].name, "sfile-mobi");
  assert.equal(api[0].name, "neoxr");
  assert.equal(new Set(kapsfile.backends.map((b) => b.name)).size, kapsfile.backends.length);
});

test("unduhan: scraper lokal dulu, aggregator tidak boleh diakses lebih awal", async () => {
  const dipanggil = [];
  sfileLokalSukses(undefined, (url) => dipanggil.push(new URL(String(url)).hostname));
  hasilAggregator = async () => {
    dipanggil.push("neoxr");
    return neoxrSfile({ url: SFILE_UNDUHAN, filename: "a.apk" })();
  };

  const keluar = await resolverSfile().resolve("sfile", { url: "https://sfile.mobi/abc123" });

  assert.equal(keluar.source, "sfile-mobi");
  assert.equal(keluar.data.filename, "Spotify PREMIUM_9.1.14.864");
  assert.equal(keluar.data.size, "100.42 MB");
  assert.equal(dipanggil.includes("neoxr"), false, "aggregator tidak boleh diakses sebelum host lokal");
});

test("unduhan: gate sfile memblokir → aggregator jadi cadangan", async () => {
  // `#download` hilang: scraper mengembalikan `download_url: null`, normalize
  // melempar, dan aggregator harus dapat giliran.
  sfileLokalSukses([HAL_SFILE[0], HAL_SFILE[2]]);
  hasilAggregator = neoxrSfile({
    url: SFILE_UNDUHAN,
    filename: "a.apk",
    mime: "application/vnd.android.package-archive",
  });

  const keluar = await resolverSfile().resolve("sfile", { url: "https://sfile.mobi/abc123" });

  assert.equal(keluar.source, "neoxr");
  assert.equal(keluar.data.url, SFILE_UNDUHAN);
  assert.equal(keluar.data.mime, "application/vnd.android.package-archive");
});

test("semua backend gagal → CapabilityError, bukan URL lama dari cache", async () => {
  sfileLokalSukses([HAL_SFILE[0], HAL_SFILE[2]]);
  hasilAggregator = neoxrSfile({ message: "not found" });

  await assert.rejects(
    () => resolverSfile().resolve("sfile", { url: "https://sfile.mobi/abc123" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["sfile-mobi", "neoxr"]);
      return true;
    },
  );
});

test("argumen kosong → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverSfile();
  await assert.rejects(
    () => resolver.resolve("sfile", {}),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, []);
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), []);
});

// ── aggregator: URL harus dikunci persis ──────────────────────────────────────

test("neoxr: pathname harus persis /api/sfile dan param url", async () => {
  // Konvensi per host di repo ini: neoxr, izuka, cuki, siputzx, dan azbry
  // semuanya memakai prefix `/api`; hanya nexray yang tidak.
  hasilAggregator = neoxrSfile({ url: SFILE_UNDUHAN, filename: "a.apk" });

  await sfileApi.run({ url: "https://sfile.mobi/abc123" });

  const [req] = PANGGILAN_HTTP.map((c) => new URL(c.url));
  assert.equal(req.origin, "https://api.neoxr.eu");
  assert.equal(req.pathname, "/api/sfile");
  assert.equal(req.searchParams.get("url"), "https://sfile.mobi/abc123");
  // neoxr membaca key dari query string (dicek live: `apikey` di header dijawab
  // `Parameter "apikey" is required in the request query string.`), jadi key
  // WAJIB ada di query. Dikirim juga sebagai header supaya host yang membaca di
  // sana tetap jalan — yang diuji di sini adalah query, karena itulah yang
  // benar-benar dibaca.
  assert.equal(req.searchParams.get("apikey"), "k-neoxr-untuk-test", "key harus sampai ke query string");
  assert.deepEqual(
    [...req.searchParams.keys()].sort(),
    ["apikey", "url"],
    "param lain tidak boleh ikut terkirim",
  );
  assert.equal(PANGGILAN_HTTP[0].opts?.headers?.apikey, "k-neoxr-untuk-test", "header tetap dikirim");
});

test("neoxr: setiap literal path aggregator di kapabilitas ini berawalan /api", () => {
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/sfile.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)].map((m) => m[0].slice(1, -1));
  // `length >= 1` tidak bisa gagal di sini: regex di atas menangkap SEMUA string
  // literal yang diawali "/", jadi isinya pasti ada. Yang bisa gagal — dan yang
  // sebenarnya bermakna — adalah path yang benar-benar dipakai `aggregator.hit`
  // ikut di-scan. Kalau path itu dipindah ke variabel atau konstanta, `salah`
  // jadi kosong dan seluruh test ini hijau tanpa memeriksa apa pun.
  assert.ok(
    literalPath.includes("/api/sfile"),
    `path yang dipakai aggregator.hit harus terdeteksi, dapat: ${literalPath.join(", ")}`,
  );
  const salah = literalPath.filter((p) => !p.startsWith("/api/"));
  assert.deepEqual(salah, [], `path tanpa /api: ${salah.join(", ")}`);
});

// ── budget: abort harus sampai ke lapisan HTTP ───────────────────────────────

test("signal diteruskan ke ketiga request scraper lokal", async () => {
  const controller = new AbortController();
  sfileLokalSukses();
  await sfileLokal.run({ url: "https://sfile.mobi/abc123" }, { signal: controller.signal });

  const lokal = PANGGILAN_HTTP.filter((c) => String(c.url).includes("sfile"));
  assert.equal(lokal.length, 3, "sfile() memang tiga request berurutan");
  for (const call of lokal) {
    assert.equal(call.opts?.signal, controller.signal, `request ${call.url} harus menerima signal`);
  }
});

test("budget habis → request scraper lokal benar-benar dibatalkan", async () => {
  let terputus = false;
  halamanSfile = (url, opts) =>
    new Promise((_resolve, reject) => {
      opts?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });

  await resolverSfile({ budget: { localMs: 60, totalMs: 400 } })
    .resolve("sfile", { url: "https://sfile.mobi/abc123" })
    .catch(() => {});

  assert.equal(terputus, true, "abort harus mencabut request scraper lokal");
});

test("signal diteruskan ke request aggregator", async () => {
  const controller = new AbortController();
  hasilAggregator = neoxrSfile({ url: SFILE_UNDUHAN, filename: "a.apk" });
  await sfileApi.run({ url: "https://sfile.mobi/abc123" }, { signal: controller.signal });
  assert.equal(PANGGILAN_HTTP.at(-1).opts?.signal, controller.signal);
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

test("plugin sfiledl tidak menyebut domain agregator lagi", () => {
  const pola = /nexray|neoxr|izuka|cuki|siputzx|azbry/i;
  const ketemu = [];
  fs.readFileSync(path.join(process.cwd(), "plugins/download/sfiledl.js"), "utf8")
    .split("\n")
    .forEach((baris, i) => {
      if (pola.test(baris)) ketemu.push(`plugins/download/sfiledl.js:${i + 1}: ${baris.trim()}`);
    });
  assert.deepEqual(ketemu, [], `domain agregator masih ada:\n${ketemu.join("\n")}`);
});

test("plugin sfiledl memakai resolver, dan guard URL-nya dipertahankan", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/download/sfiledl.js"), "utf8");
  // Plugin ini memakai gaya kutip tunggal, seperti file aslinya. PenAssertion di
  // sini sengaja tidak mengunci gaya kutip plugin lain.
  assert.match(sumber, /import \{ resolver \} from '..\/..\/src\/lib\/resolve\.js'/);
  assert.match(sumber, /resolver\.resolve\('sfile'/);
  assert.doesNotMatch(
    sumber,
    /axios\.(get|post)\(\s*[`"']https?:\/\//,
    "masih menembak host luar lewat axios",
  );
  assert.doesNotMatch(sumber, /api\.neoxr|apikey=/, "URL aggregator lama harus hilang");
  // Guard plugin adalah lapis kedua. Plugin ini sudah punya guard `includes`
  // dari sebelum Phase 1; ia tidak boleh hilang, dan penolakannya harus terjadi
  // sebelum resolve.
  assert.match(sumber, /hostSfile\(url\)/, "guard host level plugin harus ada");
  assert.ok(
    sumber.indexOf("hostSfile(url)") < sumber.indexOf("resolver.resolve('sfile'"),
    "guard harus mendahului panggilan resolver",
  );
  assert.match(sumber, /name: 'sfiledl'/);
  assert.match(sumber, /alias: \['sfile', 'sfiledownload'\]/);
  assert.match(sumber, /category: 'download'/);
  assert.match(sumber, /cooldown: 15/);
});


test("neoxr: key aggregator kosong → gagal tanpa request, bukan 401 dari server", async () => {
  // `aggregator.hit` memeriksa key sebelum ada request (src/lib/aggregator.js:
  // key dicek sebelum ada request). Konsekuensinya untuk kapabilitas ini: key
  // kosong berarti cadanganAggregator tidak bisa dipakai sama sekali, dan itu
  // harus terlihat sebagai kegagalan backend yang jujur, bukan sebagai
  // "semua backend gagal" tanpa penjelasan.
  configUji.APIkey.neoxr = "";
  const resolver = resolverSfile();
  sfileLokalSukses([HAL_SFILE[0], HAL_SFILE[2]]);

  await assert.rejects(
    () => resolver.resolve("sfile", { url: "https://sfile.mobi/abc123" }),
    (error) => {
      assert.deepEqual(error.tried.map((t) => t.name), ["sfile-mobi", "neoxr"]);
      // Pesan aggregator menyebut nama env, bukan hanya nama host: plugin
      // membutuhkannya untuk menjelaskan ke user tanpa tahu host mana.
      assert.match(error.tried[1].reason, /API key neoxr belum diisi, set APIKEY_NEOXR di \.env/);
      return true;
    },
  );
  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => new URL(String(c.url)).hostname === "api.neoxr.eu"),
    [],
    "key kosong tidak boleh menghasilkan request apa pun",
  );
});

// ═════════════════════════════════════════════════════════════════════════════
// Videy
//
// Dua bentuk yang disagreed di sini dibaca dari sumber yang nyata, bukan dari
// brief. Bentuk aggregator dibaca dari plugin sebelum Phase 1
// (git show 5ea7567^:plugins/download/videy.js): `f()` mengembalikan body apa
// adanya dan plugin membaca `res.data.status && res.data.data.url`, jadi
// amplop neoxr adalah `{ status, data: { url } }` — bukan `{ title, thumbnail,
// formats }` seperti duga brief. Bentuk lokal TIDAK berasal dari
// src/scraper/videy.js: default export file itu mengunggah file lokal ke
// videy.co (`fs.existsSync(file)` lalu POST multipart dengan maxBodyLength
// Infinity), sedangkan plugin ini menerima link berbagi dan butuh URL video
// langsung. Jadi resolve lokal dibangun dari permukaan yang dipakai sendiri oleh
// frontend videy, bukan dari uploader itu.
// ═════════════════════════════════════════════════════════════════════════════

const VIDEO_VIDEY = "https://cdn.videy.co/7ZH1ZRIF.mp4";

/** Bentuk yang benar-benar dikirim frontend videy: `cdn.videy.co/<id>.<ext>`. */
function cdnVideySukses(ext = "mp4") {
  return async () => ({
    status: 206,
    headers: { "content-type": "video/mp4" },
    data: Buffer.from([0x00]),
  });
}

let balasCdn = () => {
  throw new Error("cdn.videy.co tidak boleh dipanggil: upstream mati");
};
// `hasilAggregator` sengaja dipakai ulang dari harness paling atas, bukan
// dideklarasikan ulang: satu variabel untuk semua kapabilitas di berkas ini.

const getSfile = httpPalsu.get;
httpPalsu.get = async (url, opts) => {
  // Dicocokkan ke host, bukan ke seluruh string: URL aggregator neoxr untuk
  // kapabilitas ini memuat "videy.co" di query string-nya, jadi pencocokan
  // substring akan mengarahkan request aggregator ke stub CDN.
  const host = new URL(String(url)).hostname;
  if (host === "cdn.videy.co") {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return balasCdn(url, opts);
  }
  if (host === "api.neoxr.eu") {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return hasilAggregator(url, opts);
  }
  return getSfile(url, opts);
};

/**
 * Bentuk amplop neoxr yang sebenarnya. `f()` di plugin lama mengembalikan body
 * respons apa adanya, dan plugin itu membaca `res.data.status` lalu
 * `res.data.data.url` — jadi `data` adalah field di dalam body neoxr, bukan
 * field HTTP axios, dan hanya ada SATU lapis `data`. Stub di test memakai
 * `{ status, data }` sebagai respons HTTP axios, jadi body neoxr dikirim apa
 * adanya dan `aggregator.hit` yang membungkusnya.
 */
function neoxrVidey(body) {
  return async () => ({ status: 200, data: body });
}

const kapvidey = await import("../src/capabilities/videy.js");

const videyLokal = kapvidey.backends.find((b) => b.kind === "local");
const videyApi = kapvidey.backends.find((b) => b.kind === "api");

function resolverVidey(opsi = {}) {
  return createResolver({ capabilities: { videy: () => kapvidey }, ...opsi });
}

beforeEach(() => {
  balasCdn = () => {
    throw new Error("cdn.videy.co tidak boleh dipanggil: upstream mati");
  };
  // Blok sfile di atas mengosongkan key neoxr di salah satu testnya dan tidak
  // memulihkannya, jadi test setelah blok itu — termasuk blok ini — akan gagal
  // dengan "API key neoxr belum diisi" yang bukan apa pun soal videy. Dipin di
  // sini, persis seperti yang dilakukan blok sfile saat mengatur harness-nya.
  configUji.APIkey.neoxr = "k-neoxr-untuk-test";
});

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: bentuk scraper lokal jadi { url }", () => {
  // Field lokal adalah `id` dan `ext` hasil aturan frontend videy; yang
  // dibutuhkan plugin hanya `url`.
  assert.deepEqual(
    kapvidey.normalize({ id: "7ZH1ZRIF", ext: "mp4", url: VIDEO_VIDEY }),
    { url: VIDEO_VIDEY },
  );
});

test("normalize: bentuk aggregator neoxr jadi { url } yang sama", () => {
  // Plugin sebelum Phase 1 membaca `res.data.data.url`, jadi amplop `data` ada
  // di dalam body dan harus dibuka di sini, bukan di backend.
  assert.deepEqual(
    kapvidey.normalize({ status: true, data: { url: VIDEO_VIDEY } }),
    { url: VIDEO_VIDEY },
  );
});

test("normalize: status false adalah kegagalan walau URL-nya ada", () => {
  // Plugin lama menolak `!data.status`. Amplop `status` harus tetap berpengaruh:
  // membawanya sebagai `body.data` saja membuat `{ status: false, data: { url } }`
  // lolos dan plugin mengirim video dari respons error.
  assert.throws(
    () => kapvidey.normalize({ status: false, data: { url: VIDEO_VIDEY } }),
    /status|gagal/i,
  );
});

test("normalize: respons tanpa URL melempar — plugin tidak boleh mengirim apa-apa", () => {
  // Sama seperti bentuk brief (`formats: []`): array kosong berarti plugin
  // menjalankan sendMedia dengan tidak ada file lalu tetap memberi centang hijau.
  for (const nilai of [
    {},
    { status: true, data: {} },
    { status: true, data: { url: null } },
    { status: true, data: { url: "" } },
    { url: "ftp://cdn.videy.co/7ZH1ZRIF.mp4" },
  ]) {
    assert.throws(
      () => kapvidey.normalize(nilai),
      /tidak dikenali|URL|video/i,
      `nilai: ${JSON.stringify(nilai)}`,
    );
  }
});

test("normalize: nilai non-objek dan null melempar", () => {
  for (const nilai of [null, undefined, "teks", 7, true]) {
    assert.throws(() => kapvidey.normalize(nilai), /tidak dikenali|URL|video/i);
  }
});

// ── guard URL ─────────────────────────────────────────────────────────────────

test("guard: link yang bukan videy.co ditolak tanpa satu pun request", async () => {
  for (const url of [
    "https://videy.co.evil.example/v?id=7ZH1ZRIF",
    "https://notvidey.co/v?id=7ZH1ZRIF",
    "https://example.com/videy.co",
    "halo",
  ]) {
    await assert.rejects(
      () => videyLokal.run({ url }),
      (error) => {
        assert.match(error.message, /videy\.co/);
        return true;
      },
      `harus ditolak: ${url}`,
    );
  }
  assert.deepEqual(PANGGILAN_HTTP, [], "guard harus jalan sebelum request CDN");
});

test("guard: aggregator tidak dihubungi untuk link yang gagal guard", async () => {
  hasilAggregator = neoxrVidey({ status: true, data: { url: VIDEO_VIDEY } });
  const resolver = resolverVidey();
  await assert.rejects(() =>
    resolver.resolve("videy", { url: "https://notvidey.co/v?id=7ZH1ZRIF" }),
  );
  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => new URL(String(c.url)).hostname === "api.neoxr.eu"),
    [],
    "aggregator tidak boleh dihubungi untuk link yang gagal guard",
  );
  assert.deepEqual(
    resolver.breaker.snapshot(),
    [],
    "link salah host bukan kegagalan host mana pun",
  );
});

test("guard: link tanpa id dilewati tanpa menyentuh breaker", async () => {
  // `https://videy.co/` lolos guard host tapi tidak punya `?id=`, jadi backend
  // lokal tidak bisa dipakai. Kalau ini dihitung kegagalan, tiga user yang
  // mengirim link beranda akan membuka breaker cdn-videy untuk semua orang —
  // termasuk yang sedang mengunduh link yang sebenarnya hidup.
  balasCdn = cdnVideySukses();
  hasilAggregator = neoxrVidey({ status: true, data: { url: VIDEO_VIDEY } });
  const resolver = resolverVidey();

  await resolver.resolve("videy", { url: "https://videy.co/" });

  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => new URL(String(c.url)).hostname === "cdn.videy.co"),
    [],
    "tanpa id tidak ada yang bisa diprobe",
  );
  // `neoxr` boleh muncul di snapshot karena resolve-nya memang sukses, tapi
  // `cdn-videy` tidak boleh muncul sama sekali dan tidak ada host yang boleh
  // terbuka — backend yang dilewati `applies` tidak pernah menyentuh breaker.
  const breaker = resolver.breaker.snapshot();
  assert.deepEqual(
    breaker.filter((s) => s.name === "cdn-videy"),
    [],
    "backend yang dilewati tidak boleh masuk breaker",
  );
  assert.equal(
    breaker.some((s) => s.openedAt !== null || s.failures > 0),
    false,
    "tidak boleh ada kegagalan yang tercatat",
  );
});

test("guard: id dengan bentuk aneh ditolak sebelum masuk ke path CDN", async () => {
  // Id masuk ke pathname CDN. Tanpa batas karakter, `?id=../x` bisa menulis
  // keluar dari `cdn.videy.co` — dan request sia-sia ke host lain.
  for (const id of ["../secret", "a b", "x".repeat(200), "%2e%2e%2f"]) {
    balasCdn = cdnVideySukses();
    await assert.rejects(
      () => videyLokal.run({ url: `https://videy.co/v?id=${id}` }),
      /id|tidak dikenal/i,
      `harus ditolak: ${id}`,
    );
  }
  assert.deepEqual(
    PANGGILAN_HTTP.filter((c) => new URL(String(c.url)).hostname === "cdn.videy.co"),
    [],
    "id buruk tidak boleh sampai ke jaringan",
  );
});

// ── wiring resolver ──────────────────────────────────────────────────────────

test("stable false: tautan media videy bisa kedaluwarsa dan dihapus kapan saja", () => {
  assert.equal(kapvidey.stable, false, "dari cache tidak boleh ada: URL CDN bisa mati");
});

test("minimal satu backend local dan satu api sebagai cadangan", () => {
  const lokal = kapvidey.backends.filter((b) => b.kind === "local");
  const api = kapvidey.backends.filter((b) => b.kind !== "local");
  assert.equal(lokal.length, 1);
  assert.equal(api.length, 1);
  // Nama per-host, bukan per-URL: `?id=` berbeda tiap link, jadi nama per-URL
  // akan menghabiskan satu dari 64 slot LRU breaker per permintaan.
  assert.equal(lokal[0].name, "cdn-videy");
  assert.equal(api[0].name, "neoxr");
  assert.equal(new Set(kapvidey.backends.map((b) => b.name)).size, kapvidey.backends.length);
});

test("unduhan: CDN videy dulu, aggregator tidak boleh diakses lebih awal", async () => {
  const dipanggil = [];
  balasCdn = async (url) => {
    dipanggil.push(new URL(String(url)).hostname);
    return { status: 206, headers: { "content-type": "video/mp4" }, data: Buffer.from([0x00]) };
  };
  hasilAggregator = async () => {
    dipanggil.push("neoxr");
    return neoxrVidey({ status: true, data: { url: VIDEO_VIDEY } })();
  };

  const keluar = await resolverVidey().resolve("videy", { url: "https://videy.co/v?id=7ZH1ZRIF" });

  assert.equal(keluar.source, "cdn-videy");
  assert.deepEqual(keluar.data, { url: VIDEO_VIDEY });
  assert.equal(dipanggil.includes("neoxr"), false, "aggregator tidak boleh diakses sebelum host lokal");
});

test("unduhan: aturan ekstensi mengikuti frontend videy", async () => {
  // Diambil dari frontend videy (`cdn.videy.co/${id}.${ext}`): mp4 kecuali id
  // 9 karakter berakhiran "2", yang mov. Salah di sini menghasilkan URL 404.
  const probe = async (id) => {
    PANGGILAN_HTTP.length = 0;
    balasCdn = cdnVideySukses();
    await videyLokal.run({ url: `https://videy.co/v?id=${id}` });
    return new URL(String(PANGGILAN_HTTP[0].url)).pathname;
  };
  assert.equal(await probe("7ZH1ZRIF"), "/7ZH1ZRIF.mp4");
  assert.equal(await probe("AbCdEfGh"), "/AbCdEfGh.mp4");
  assert.equal(await probe("7ZH1ZRIF2"), "/7ZH1ZRIF2.mov");
  assert.equal(await probe("7ZH1ZRIF1"), "/7ZH1ZRIF1.mp4");
});

test("unduhan: link sudah mati di CDN → aggregator jadi cadangan", async () => {
  balasCdn = async () => {
    const e = new Error("Request failed with status code 404");
    e.response = { status: 404 };
    throw e;
  };
  hasilAggregator = neoxrVidey({ status: true, data: { url: VIDEO_VIDEY } });

  const keluar = await resolverVidey().resolve("videy", { url: "https://videy.co/v?id=7ZH1ZRIF" });

  assert.equal(keluar.source, "neoxr");
  assert.deepEqual(keluar.data, { url: VIDEO_VIDEY });
});

test("semua backend gagal → CapabilityError, bukan URL lama dari cache", async () => {
  balasCdn = async () => {
    const e = new Error("Request failed with status code 404");
    e.response = { status: 404 };
    throw e;
  };
  hasilAggregator = neoxrVidey({ status: false, error: "expired" });

  await assert.rejects(
    () => resolverVidey().resolve("videy", { url: "https://videy.co/v?id=7ZH1ZRIF" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["cdn-videy", "neoxr"]);
      return true;
    },
  );
});

// ── amplop `status` harus bertahan melewati backend ───────────────────────────
//
// Test `normalize` di atas hanya membuktikan `normalize` melempar. Itu tidak
// bisa menangkap backend yang membuang amplop SEBELUM `normalize` melihatnya,
// dan itulah perubahan yang paling menentukan di kapabilitas ini.

test("backend aggregator wajib mengembalikan amplop, bukan body.data", async () => {
  // `lewatNeoxr` sengaja mengembalikan body aggregator utuh: `status` ada satu
  // level di atas `data`, dan plugin sebelum Phase 1 menolak `!data.status`.
  // Kalau backend mengembalikan `body.data`, respons `{ status: false, data: { url } }`
  // menjadi `{ url }` — `normalize` tidak punya apa pun untuk diperiksa dan
  // plugin mengirim video yang dianggap berhasil.
  //
  // URL-nya sengaja menunjuk domain lain supaya jelas bahwa yang diuji adalah
  // "backend tidak menghasilkan URL yang bisa dipakai", bukan jaring.
  hasilAggregator = neoxrVidey({
    status: false,
    error: "expired",
    data: { url: "https://evil.example/bad.mp4" },
  });

  const keluar = await videyApi.run({ url: "https://videy.co/v?id=7ZH1ZRIF" });

  assert.equal(
    keluar?.status,
    false,
    `backend tidak boleh membuang amplop status: ${JSON.stringify(keluar)}`,
  );
  assert.throws(
    () => kapvidey.normalize(keluar),
    /aggregator menandai gagal/,
    "keluaran backend tidak boleh bisa dipakai langsung sebagai { url }",
  );
});

test("status false yang tetap membawa URL → tidak menghasilkan apa pun yang bisa dikirim", async () => {
  // Bentuk yang sama, diuji sampai lewat resolver: dengan CDN 404 dan
  // aggregator yang menolak, tidak boleh ada hasil resolve sama sekali. Bentuk
  // plugin-nya (tidak ada `sendMedia`, tidak ada centang hijau) dipin di
  // tests/videy-plugin.test.mjs.
  balasCdn = async () => {
    const e = new Error("Request failed with status code 404");
    e.response = { status: 404 };
    throw e;
  };
  hasilAggregator = neoxrVidey({
    status: false,
    error: "expired",
    data: { url: "https://evil.example/bad.mp4" },
  });

  await assert.rejects(
    () => resolverVidey().resolve("videy", { url: "https://videy.co/v?id=7ZH1ZRIF" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["cdn-videy", "neoxr"]);
      return true;
    },
  );
});

test("argumen kosong → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverVidey();
  await assert.rejects(
    () => resolver.resolve("videy", {}),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, []);
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), []);
});

// ── aggregator: URL harus dikunci persis ──────────────────────────────────────

test("neoxr: pathname harus persis /api/videy dan param url", async () => {
  // Konvensi per host di repo ini: neoxr, izuka, cuki, siputzx, dan azbry
  // semuanya memakai prefix `/api`; hanya nexray yang tidak.
  hasilAggregator = neoxrVidey({ status: true, data: { url: VIDEO_VIDEY } });

  await videyApi.run({ url: "https://videy.co/v?id=7ZH1ZRIF" });

  const [req] = PANGGILAN_HTTP.map((c) => new URL(c.url));
  assert.equal(req.origin, "https://api.neoxr.eu");
  assert.equal(req.pathname, "/api/videy");
  assert.equal(req.searchParams.get("url"), "https://videy.co/v?id=7ZH1ZRIF");
  // neoxr membaca key dari query string (dicek live: `apikey` di header dijawab
  // `Parameter "apikey" is required in the request query string.`), jadi key
  // WAJIB ada di query. Dikirim juga sebagai header supaya host yang membaca di
  // sana tetap jalan — yang diuji di sini adalah query, karena itulah yang
  // benar-benar dibaca.
  assert.equal(req.searchParams.get("apikey"), "k-neoxr-untuk-test", "key harus sampai ke query string");
  assert.deepEqual(
    [...req.searchParams.keys()].sort(),
    ["apikey", "url"],
    "param lain tidak boleh ikut terkirim",
  );
  assert.equal(PANGGILAN_HTTP[0].opts?.headers?.apikey, "k-neoxr-untuk-test", "header tetap dikirim");
});

test("neoxr: setiap literal path aggregator di kapabilitas ini berawalan /api", () => {
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/videy.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)].map((m) => m[0].slice(1, -1));
  // `length >= 1` tidak bisa gagal di sini: regex di atas menangkap SEMUA string
  // literal yang diawali "/", jadi isinya pasti ada. Yang bisa gagal — dan yang
  // sebenarnya bermakna — adalah path yang benar-benar dipakai `aggregator.hit`
  // ikut di-scan. Kalau path itu dipindah ke variabel atau konstanta, `salah`
  // jadi kosong dan seluruh test ini hijau tanpa memeriksa apa pun.
  assert.ok(
    literalPath.includes("/api/videy"),
    `path yang dipakai aggregator.hit harus terdeteksi, dapat: ${literalPath.join(", ")}`,
  );
  const salah = literalPath.filter((p) => !p.startsWith("/api/"));
  assert.deepEqual(salah, [], `path tanpa /api: ${salah.join(", ")}`);
});

// ── budget: abort harus sampai ke lapisan HTTP ───────────────────────────────

test("probe CDN dibatasi: satu byte dan respons kecil, bukan unduhan penuh", async () => {
  // Bahaya yang nyata di kapabilitas ini bukan "baca tanpa batas", tapi kebalikannya:
  // uploader di src/scraper/videy.js memakai maxBodyLength/maxContentLength
  // Infinity untuk mengirim file milik user. Jalur resolve tidak boleh mewarisi
  // konfigurasi itu — kalau tidak, satu probe bisa menarik video 2GB ke kotak 1GB.
  balasCdn = cdnVideySukses();
  await videyLokal.run({ url: "https://videy.co/v?id=7ZH1ZRIF" });

  const probe = PANGGILAN_HTTP.at(-1);
  assert.equal(new URL(String(probe.url)).hostname, "cdn.videy.co");
  assert.match(probe.opts?.headers?.Range ?? "", /bytes=0-0/, "hanya minta satu byte");
  const batas = probe.opts?.maxContentLength;
  assert.equal(typeof batas, "number", "harus ada batas eksplisit, bukan batas bawaan yang longgar");
  assert.ok(batas > 0 && batas <= 64 * 1024, `batas terlalu longgar: ${batas}`);
  assert.ok(
    probe.opts?.maxContentLength !== Infinity && probe.opts?.maxBodyLength !== Infinity,
    "batas Infinity dari uploader tidak boleh bocor ke jalur resolve",
  );
});

test("signal diteruskan ke probe CDN", async () => {
  const controller = new AbortController();
  balasCdn = cdnVideySukses();
  await videyLokal.run({ url: "https://videy.co/v?id=7ZH1ZRIF" }, { signal: controller.signal });
  assert.equal(PANGGILAN_HTTP.at(-1).opts?.signal, controller.signal);
});

test("signal diteruskan ke request aggregator, bukan hanya ke CDN", async () => {
  const controller = new AbortController();
  hasilAggregator = neoxrVidey({ status: true, data: { url: VIDEO_VIDEY } });
  await videyApi.run({ url: "https://videy.co/v?id=7ZH1ZRIF" }, { signal: controller.signal });
  assert.equal(PANGGILAN_HTTP.at(-1).opts?.signal, controller.signal);
});

test("budget habis di tier lokal → probe CDN benar-benar dibatalkan", async () => {
  let terputus = false;
  balasCdn = (url, opts) =>
    new Promise((_resolve, reject) => {
      opts?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });

  await resolverVidey({ budget: { localMs: 60, totalMs: 400 } })
    .resolve("videy", { url: "https://videy.co/v?id=7ZH1ZRIF" })
    .catch(() => {});

  assert.equal(terputus, true, "abort harus mencabut probe CDN");
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

test("plugin videy tidak menyebut domain agregator lagi", () => {
  const pola = /nexray|neoxr|izuka|cuki|siputzx|azbry/i;
  const ketemu = [];
  fs.readFileSync(path.join(process.cwd(), "plugins/download/videy.js"), "utf8")
    .split("\n")
    .forEach((baris, i) => {
      if (pola.test(baris)) ketemu.push(`plugins/download/videy.js:${i + 1}: ${baris.trim()}`);
    });
  assert.deepEqual(ketemu, [], `domain agregator masih ada:\n${ketemu.join("\n")}`);
});

test("plugin videy memakai resolver, dan guard host-nya lebih ketat dari `includes`", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/download/videy.js"), "utf8");
  // Plugin ini memakai gaya kutip tunggal, seperti file aslinya.
  assert.match(sumber, /import \{ resolver \} from '..\/..\/src\/lib\/resolve\.js'/);
  assert.match(sumber, /resolver\.resolve\('videy'/);
  assert.doesNotMatch(
    sumber,
    /axios\.(get|post)\(\s*[`"']https?:\/\//,
    "masih menembak host luar lewat axios",
  );
  assert.doesNotMatch(sumber, /api\.neoxr|apikey=/, "URL aggregator lama harus hilang");
  assert.doesNotMatch(sumber, /from '\.\.\/\.\.\/src\/lib\/http\.js'/, "helper HTTP lama harus hilang");
  // Guard plugin adalah lapis pertama dan harus tetap ada. Versi lama
  // (`url.match(/videy\.co/i)`) adalah pemeriksaan substring, jadi
  // `videy.co.evil.example` lolos; Task 5 dan Task 6 sudah menaikkan `douyindl`
  // dan `sfiledl` ke pencocokan batas label host, dan ketiga plugin harus seragam.
  assert.match(sumber, /host === 'videy\.co' \|\| host\.endsWith\('\.videy\.co'\)/, "guard harus cek batas label host");
  assert.doesNotMatch(
    sumber,
    /url\.match\(\/videy|includes\('videy\.co'\)|includes\("videy\.co"\)/,
    "guard plugin tidak boleh kembali ke substring",
  );
  assert.ok(
    sumber.indexOf("if (!hostVidey(url))") < sumber.indexOf("resolver.resolve('videy'"),
    "guard harus mendahului panggilan resolver",
  );
  // Pesan yang dilihat user tidak berubah; yang longgar hanya isi validasinya.
  assert.match(sumber, /❌ URL tidak valid\. Gunakan link dari videy\.co/);
});

test("permukaan plugin videy tetap sama: config, pesan, dan reaksi", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/download/videy.js"), "utf8");
  assert.match(sumber, /name: 'videy'/);
  assert.match(sumber, /alias: \['vdl', 'videydownload', 'videydl'\]/);
  assert.match(sumber, /category: 'download'/);
  assert.match(sumber, /cooldown: 10/);
  assert.match(sumber, /energi: 1/);
  // Semua pesan dan reaksi yang sudah ada sebelum Phase 1 harus tetap hidup.
  for (const bagian of [
    /🎬 \*ᴠɪᴅᴇʏ ᴅᴏᴡɴʟᴏᴀᴅ\*/,
    /Masukkan URL videy\.co/,
    /❌ URL tidak valid\. Gunakan link dari videy\.co/,
    /❌ Gagal mengambil video\. Link tidak valid atau sudah expired\./,
  ]) {
    assert.match(sumber, bagian, `pesan hilang: ${bagian}`);
  }
  for (const emoji of ["🕕", "✅", "❌", "☢"]) {
    assert.ok(sumber.includes(emoji), `reaksi hilang: ${emoji}`);
  }
});

// ═══════════════════════════════════════════════════════════════════════════════
// Task 8 — kapabilitas `youtube` (lima plugin)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Yang diuji di sini adalah bagian yang tidak butuh transport axios: nama
// backend, `stable`, path aggregator yang harus dikunci persis, dan permukaan
// lima plugin. Backend lokal `ytdl-native`/`youtube-fallback` memanggil axios
// langsung lewat src/scraper/ytdl.js dan src/scraper/youtube.js, jadi
// pengujiannya harus mock `axios` — dan `axios` sudah dimock di berkas lain.
// Semua pengujian short-link dan scraper lokal ada di
// tests/youtube-capability.test.mjs, yang menguji kedua backend lokal secara
// langsung.
//
// Blok ini tidak memakai `mock.module` baru, jadi blok-blok Task 3 sampai Task 7
// di atas tidak tersentuh. Satu-satunya hal bersama yang dipakai blok ini adalah
// `httpPalsu.get`, dan itu di-wrap di bawah — bukan diganti: host aggregator
// YouTube ditangani di sini, sisanya diteruskan ke `get` yang lama apa adanya
// supaya blok-blok sebelumnya tetap berperilaku persis seperti sebelumnya.

const getYoutube = httpPalsu.get;
httpPalsu.get = async (url, opts) => {
  const host = new URL(String(url)).hostname;
  // Host yang dipakai kapabilitas ini. Dicocokkan ke host, bukan ke substring:
  // URL aggregator memuat "youtube.com/watch?v=…" di query string-nya.
  if (host === "my.izuka-api.xyz" || host === "api.azbry.com") {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return hasilAggregator(url, opts);
  }
  return getYoutube(url, opts);
};

const kapyoutube = await import("../src/capabilities/youtube.js");

const PLUGIN_YOUTUBE = [
  "plugins/download/ytmp3.js",
  "plugins/download/ytmp4.js",
  "plugins/search/playvid.js",
  "plugins/search/playcall.js",
  "plugins/search/playch.js",
];

test("kapabilitas youtube: stable false, URL unduhan YouTube punya masa berlaku pendek", () => {
  assert.equal(kapyoutube.stable, false, "dari cache tidak boleh ada: URL YouTube bisa mati");
});

test("kapabilitas youtube: nama per-host, unik, dan ada dua tier", () => {
  const lokal = kapyoutube.backends.filter((b) => b.kind === "local");
  const api = kapyoutube.backends.filter((b) => b.kind !== "local");
  assert.deepEqual(lokal.map((b) => b.name).sort(), ["youtube-fallback", "ytdl-native"]);
  assert.deepEqual(api.map((b) => b.name).sort(), ["azbry", "izuka"]);
  assert.equal(
    new Set(kapyoutube.backends.map((b) => b.name)).size,
    kapyoutube.backends.length,
    "nama ganda akan berbagi satu slot breaker untuk dua host berbeda",
  );
  // Per-URL atau per-user akan menghabiskan LRU breaker 64 slot per permintaan.
  for (const backend of kapyoutube.backends) {
    // `ytdl-native` juga 11 karakter, jadi yang diuji bukan panjangnya:
    // nama backend tidak boleh memuat URL, path, nomor video, atau alamat Junction.
    assert.doesNotMatch(backend.name, /https?:|[/?@]|\d{4,}/, `nama bukan per-host: ${backend.name}`);
  }
});

// ── aggregator: URL harus dikunci persis ──────────────────────────────────────
//
// Tiga path yang tercatat di repo ini, semuanya dibaca plugin sebelum Phase 1:
// `plugins/download/ytmp3.js:23` dan `plugins/search/playcall.js:28` memakai
// `/api/downloader/ytmp3`, `plugins/download/ytmp4.js:19` dan
// `plugins/search/playvid.js:27` memakai `/api/downloader/ytmp4`, dan
// `plugins/search/playch.js:99` memakai `/api/download/ytmp3`. Melewatkan `/api`
// dijawab 404, jadi path-nya di sini dikunci, bukan cuma "aggregator dipanggil".

const backendIzuka = kapyoutube.backends.find((b) => b.name === "izuka");
const backendAzbry = kapyoutube.backends.find((b) => b.name === "azbry");

/** Jalankan satu backend aggregator dan kembalikan URL request yang benar-benar keluar. */
async function reqAggregator(backend, body, args) {
  PANGGILAN_HTTP.length = 0;
  hasilAggregator = async () => ({ status: 200, data: body });
  await backend.run(args);
  return PANGGILAN_HTTP.map((c) => new URL(String(c.url))).at(-1);
}

const IZUKA_MP3_BODY = { status: true, result: { download_url: "https://cdn.example/a.mp3", title: "Lagu" } };
const IZUKA_MP4_BODY = {
  status: true,
  result: { title: "Video", video_normal: [{ ext: "mp4", quality: "720", url: "https://cdn.example/v.mp4" }] },
};
const AZBRY_MP3_BODY = { status: true, result: { download: "https://cdn.example/a.mp3", title: "Lagu" } };

test("izuka mp3: pathname /api/downloader/ytmp3 dan hanya param url", async () => {
  const req = await reqAggregator(backendIzuka, IZUKA_MP3_BODY, { url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ", format: "mp3" });
  assert.equal(req.origin, "https://my.izuka-api.xyz");
  assert.equal(req.pathname, "/api/downloader/ytmp3");
  assert.equal(req.searchParams.get("url"), "https://www.youtube.com/watch?v=dQw4w9WgXcQ");
  // izuka tidak punya key di AGGREGATORS (src/lib/aggregator.js:26), jadi query
  // yang hanya berisi `url` adalah hasil yang benar. `apikey` karangan akan
  // menjadi parameter asing yang tidak pernah dibaca host ini.
  assert.deepEqual([...req.searchParams.keys()], ["url"], "param lain tidak boleh ikut terkirim");
});

test("izuka mp4: pathname /api/downloader/ytmp4, bukan endpoint mp3", async () => {
  const req = await reqAggregator(backendIzuka, IZUKA_MP4_BODY, { url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ", format: "mp4" });
  assert.equal(req.pathname, "/api/downloader/ytmp4");
  assert.equal(req.origin, "https://my.izuka-api.xyz");
  assert.deepEqual([...req.searchParams.keys()], ["url"]);
});

test("azbry mp3: pathname /api/download/ytmp3 (download, bukan downloader)", async () => {
  // Path azbry berbeda satu kata dari izuka; tertukar di sini berarti mp3 dari
  // channelCfg salah host dan salah bentuk respons.
  const req = await reqAggregator(backendAzbry, AZBRY_MP3_BODY, { url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ", format: "mp3" });
  assert.equal(req.origin, "https://api.azbry.com");
  assert.equal(req.pathname, "/api/download/ytmp3");
  assert.deepEqual([...req.searchParams.keys()], ["url"]);
});

test("short link diteruskan ke aggregator sebagai URL kanonik", async () => {
  const req = await reqAggregator(backendIzuka, IZUKA_MP3_BODY, { url: "https://youtu.be/dQw4w9WgXcQ", format: "mp3" });
  assert.equal(
    req.searchParams.get("url"),
    "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    "short link harus menjadi watch?v=<id> juga di aggregator",
  );
});

test("semua path aggregator di kapabilitas ini berawalan /api", () => {
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/youtube.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  // `"/"` adalah pemisah path di `pathname.split("/")`, bukan path aggregator,
  // jadi dikecualikan supaya pemindaian tidak melapor terus-menerus.
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)]
    .map((m) => m[0].slice(1, -1))
    .filter((p) => p !== "/");
  assert.ok(
    literalPath.includes("/api/downloader/ytmp3"),
    `path yang dipakai aggregator.hit harus terdeteksi, dapat: ${literalPath.join(", ")}`,
  );
  const salah = literalPath.filter((p) => !p.startsWith("/api/"));
  assert.deepEqual(salah, [], `path tanpa /api: ${salah.join(", ")}`);
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

test("lima plugin youtube tidak menyebut domain agregator lagi", () => {
  const pola = /nexray|neoxr|izuka|cuki|siputzx|azbry/i;
  const ketemu = [];
  for (const file of PLUGIN_YOUTUBE) {
    fs.readFileSync(path.join(process.cwd(), file), "utf8")
      .split("\n")
      .forEach((baris, i) => {
        if (pola.test(baris)) ketemu.push(`${file}:${i + 1}: ${baris.trim()}`);
      });
  }
  assert.deepEqual(ketemu, [], `domain agregator masih ada:\n${ketemu.join("\n")}`);
});

test("lima plugin youtube memakai resolver youtube dengan format yang tepat", () => {
  const diharapkan = {
    "plugins/download/ytmp3.js": "mp3",
    "plugins/download/ytmp4.js": "mp4",
    "plugins/search/playvid.js": "mp4",
    "plugins/search/playcall.js": "mp3",
    "plugins/search/playch.js": "mp3",
  };
  for (const [file, format] of Object.entries(diharapkan)) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    assert.match(sumber, /from "\.\.\/\.\.\/src\/lib\/resolve\.js"|from '\.\.\/\.\.\/src\/lib\/resolve\.js'/, `${file}: resolver belum diimpor`);
// Panggilan ada di dalam helper; yang diuji adalah kapabilitas dan format
    // yang diteruskan ke sana, bukan nama variabel yang kebetulan memegang
    // closure. Bentuknya berubah dari `run("youtube", …)` ke
    // `resolver.resolve("youtube", …)` setelah seam `deps.resolve` dihapus dari
    // empat plugin yang tidak pernah memakainya.
    assert.match(sumber, /\(\s*["']youtube["']/, `${file}: kapabilitas yang dipanggil bukan youtube`);
    assert.match(sumber, new RegExp(`format: "${format}"`), `${file}: format harus ${format}`);
    // `playcall` dan `playch` masih mengunduh media ke berkas dengan axios, jadi
    // yang dilarang adalah axios ke URL absolut — memuat `audioUrl` lokal sah.
    assert.doesNotMatch(
      sumber,
      /axios\.(get|post)\(\s*[`"']https?:\/\//,
      `${file}: masih menembak host luar lewat axios`,
    );
    assert.doesNotMatch(sumber, /apikey=/, `${file}: URL aggregator lama harus hilang`);
  }
});

test("ytmp3 dan ytmp4: guard URL dipertahankan dan mendahului resolver", () => {
  for (const file of ["plugins/download/ytmp3.js", "plugins/download/ytmp4.js"]) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    // Pesan user tidak berubah; yang berubah hanya isi validasinya, dari
    // substring ke batas label host.
    assert.match(sumber, /❌ URL harus YouTube/, `${file}: pesan guard hilang`);
    assert.match(sumber, /host === "youtu\.be" \|\| host === "youtube\.com" \|\| host\.endsWith\("\.youtube\.com"\)/, `${file}: guard harus cek batas label host`);
    assert.doesNotMatch(
      sumber,
      /url\.includes\("youtube\.com"\)|url\.includes\('youtube\.com'\)/,
      `${file}: guard plugin tidak boleh kembali ke substring`,
    );
    assert.ok(
      sumber.indexOf("if (!hostYoutube(url))") < sumber.indexOf("await get"),
      `${file}: guard harus mendahului panggilan resolver`,
    );
  }
});

test("pesan dan reaksi kelima plugin youtube tetap hidup", () => {
  const wajib = {
    "plugins/download/ytmp3.js": [
      /ytmp3 https:\/\/youtube\.com\/watch\?v=xxx/,
      /❌ URL harus YouTube/,
      /Gagal mengunduh audio\./,
    ],
    "plugins/download/ytmp4.js": [
      /ytmp4 https:\/\/youtube\.com\/watch\?v=xxx/,
      /❌ URL harus YouTube/,
      /Gagal mengunduh video\./,
    ],
    "plugins/search/playvid.js": [
      /playvid <judul video>/,
      /Sedang mengunduh video, harap tunggu sebentar ya/,
      /fitur putar videonya sedang ada kendala/,
    ],
    "plugins/search/playcall.js": [
      /PANGGILAN MUSIK \(PLAYCALL\)/,
      /LAYANAN BELUM SIAP/,
      /MEMULAI PANGGILAN/,
      /TERHUBUNG/,
      /PANGGILAN BERAKHIR/,
    ],
    "plugins/search/playch.js": [
      /PLAY SALURAN/,
      /❌ Saluran belum diatur/,
      /NOW PLAYING \(SALURAN\)/,
      /berhasil dikirim ke saluran/,
    ],
  };
  for (const [file, bagian] of Object.entries(wajib)) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    for (const pola of bagian) assert.match(sumber, pola, `${file}: pesan hilang ${pola}`);
  }
  // Reaksi tidak boleh hilang di salah satu pun plugin: centang hijau tanpa file
  // yang terkirim adalah kebohongan yang paling merusak.
  const reaksiHarusAda = {
    "plugins/download/ytmp3.js": ["🕕", "✅", "❌"],
    "plugins/download/ytmp4.js": ["🕕", "✅", "❌"],
    "plugins/search/playvid.js": ["🕕", "✅", "❌"],
    "plugins/search/playcall.js": ["🕕", "📞", "❌", "☢"],
    "plugins/search/playch.js": ["🔎", "🎵", "✅", "☢"],
  };
  for (const [file, emoji] of Object.entries(reaksiHarusAda)) {
    const sumber = fs.readFileSync(path.join(process.cwd(), file), "utf8");
    for (const e of emoji) assert.ok(sumber.includes(e), `${file}: reaksi hilang ${e}`);
  }
});

test("ytmp3 tidak lagi mengunduh audio ke memori sebagai jalur utama", async () => {
  // `fallbackToMp3Buffer` menarik seluruh audio ke satu Buffer. Sebelum Phase 1
  // jalur itu hanya tersentuh kalau aggregator gagal; setelah rewire scraper lokal
  // jalan pertama, jadi jalur buffer jadi jalur utama dan kotak 1GB menarik
  // video 2 jam setiap kali `.ytmp3` dipakai.
  // Komentar dibuang dulu: nama fungsi lama memang masih disebut di komentar
  // yang menjelaskan kenapa cabangnya dihapus, dan yang dilarang adalah kodenya.
  const sumber = fs
    .readFileSync(path.join(process.cwd(), "plugins/download/ytmp3.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  assert.doesNotMatch(sumber, /fallbackToMp3Buffer/, "unduhan penuh tidak boleh jadi jalur plugin");
  assert.doesNotMatch(sumber, /isFallback/, "cabang fallback harus hilang bersama pemanggilnya");
});

// ── budget resolver harus benar-benar mencapai aggregator ─────────────────────
//
// Paruh yang bisa diuji di kapabilitas ini adalah tier `api`: `ytdl` dan
// SaveTube tidak punya parameter signal — lihat catatan di `lewatYtdlNative`
// dan `lewatYoutubeFallback` di src/capabilities/youtube.js — jadi hanya
// aggregator.hit yang bisa dihentikan oleh budget resolver. Kalau `ctx.signal`
// tidak diteruskan di sini, seluruh `.ytmp3` yang jatuh ke aggregator berjalan
// sampai timeout 5 detik milik `aggregator.hit` tanpa bisa dihentikan — dan
// tidak ada test lain yang akan menangkapnya.

test("signal diteruskan ke request aggregator youtube, bukan hanya ke scraper lokal", async () => {
  const controller = new AbortController();
  for (const [backend, body, format] of [
    [backendIzuka, IZUKA_MP3_BODY, "mp3"],
    [backendIzuka, IZUKA_MP4_BODY, "mp4"],
    [backendAzbry, AZBRY_MP3_BODY, "mp3"],
  ]) {
    PANGGILAN_HTTP.length = 0;
    hasilAggregator = async () => ({ status: 200, data: body });
    await backend.run({ url: "https://youtu.be/dQw4w9WgXcQ", format }, { signal: controller.signal });
    const call = PANGGILAN_HTTP.at(-1);
    assert.equal(call?.verb, "get", `${backend.name} harus lewat GET aggregator`);
    assert.equal(
      call?.opts?.signal,
      controller.signal,
      `tier api ${backend.name} (${format}) harus menghormati budget resolver`,
    );
  }
});

test("budget habis di tier aggregator youtube → request aggregator benar-benar dibatalkan", async () => {
  // Bukti yang tidak bisa dipalsukan oleh test "signal diteruskan": kalau
  // `opts.signal` benar-benar sampai ke transport, `abort()` memanggil daftar
  // listener-nya dan request ini menolak — bukan menunggu timeout 5 detik.
  const controller = new AbortController();
  let terputus = false;
  PANGGILAN_HTTP.length = 0;
  hasilAggregator = (url, opts) =>
    new Promise((_resolve, reject) => {
      opts?.signal?.addEventListener("abort", () => {
        terputus = true;
        reject(new Error("dibatalkan oleh budget"));
      });
    });

  const jalan = backendIzuka.run({ url: "https://youtu.be/dQw4w9WgXcQ", format: "mp3" }, { signal: controller.signal });
  controller.abort();

  // Tanpa `opts.signal` di transport, promise di atas tidak akan pernah selesai,
  // jadi ada batas waktu pendek supaya kegagalan muncul sebagai merah yang jelas
  // dan tidak menggantung seluruh berkas. Timer dibersihkan di `finally` supaya
  // jalur hijau tidak meninggalkan pekerjaan yang belum selesai.
  let ganti;
  const yolk = new Promise((_, reject) => {
    ganti = setTimeout(
      () => reject(new Error("request tidak dibatalkan: signal tidak sampai ke transport")),
      300,
    );
  });
  try {
    await assert.rejects(() => Promise.race([jalan, yolk]), /dibatalkan oleh budget/);
  } finally {
    clearTimeout(ganti);
  }
  assert.equal(terputus, true, "request aggregator harus benar-benar dibatalkan");
});

// ── `fallbackToMp3Buffer` tidak boleh tetap hidup sebagai export ──────────────
//
// Setelah rewire `.ytmp3` tidak pernah menyentuhnya, dan nol pemanggil tersisa di
// repo. Fungsi itu `axios.get(url, {responseType:"arraybuffer"})` tanpa batas
// ukuran lalu `readFileSync` hasilnya — persis mode yang dilarang di kotak 1GB.
// Export yang tidak terpakai adalah gubernak yang menganggur: plugin berikutnya
// bisa mengimpornya tanpa jejak karena tidak ada yang menunjuk. Jadi dihapus, dan
// ketidakhadirannya ikut dikunci supaya tidak kembali diam-diam.

test("scraper ytdl tidak lagi mengekspor fallbackToMp3Buffer", async () => {
  const mod = await import("../src/scraper/ytdl.js");
  assert.equal(
    mod.fallbackToMp3Buffer,
    undefined,
    "unduhan penuh tanpa batas ke RAM tidak boleh tetap tersedia sebagai export",
  );
  // Hapus exportnya tidak boleh merusak yang masih dipakai plugin di luar Phase 1.
  assert.equal(typeof mod.ytdl, "function");
  assert.equal(typeof mod.Youtube, "function");
  assert.equal(typeof mod.default, "function");
  const sumber = fs.readFileSync(path.join(process.cwd(), "src/scraper/ytdl.js"), "utf8");
  assert.doesNotMatch(sumber, /fallbackToMp3Buffer/, "kodenya harus hilang, bukan cuma export-nya");
});

// ═══════════════════════════════════════════════════════════════════════════════
// Task 9 — kapabilitas `ytmusic` (plugin `applemusic`)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Blok ini hanya memakai `mock.module` yang sudah ada di berkas ini
// (`../src/lib/http.js` dan `ytmusic-api`), jadi blok Task 3 sampai Task 8 tidak
// tersentuh. Tapi `ytmusic-api` di-mock sebagai tripwire untuk spotify
// (baris 52): `search()` di situ melempar. Karena itu backend lokal kapabilitas
// ini — yang memang memanggil `search()` — diuji di berkas terpisah
// tests/ytmusic-capability.test.mjs, yang mock `ytmusic-api`-nya sendiri.
//
// Bentuk yang diuji di sini adalah hasil probe langsung pada 2026-10-03, bukan
// tebakan:
//
//   curl 'https://api.nexray.eu.cc/search/applemusic?q=best+friend'
//     → {"status":true,"author":"@nexray - ElrayyXml","result":[{title,subtitle,link,image}, …]}
//
// `status` ada, jadi amplopnya wajib sampai ke `normalize`. Catatan kedua dari
// probe itu: `result` bercampur — entri `Song`, `Artist`, dan `Album` semua
// punya `link`, jadi penyaringan di `normalize` tidak boleh berbasis `type`.

const kapytmusic = await import("../src/capabilities/ytmusic.js");

const ytmLokal = kapytmusic.backends.find((b) => b.kind === "local");
const ytmApi = kapytmusic.backends.find((b) => b.kind === "api");

function resolverYtmusic(opsi = {}) {
  return createResolver({ capabilities: { ytmusic: () => kapytmusic }, ...opsi });
}

/** Bentuk entri SONG yang benar-benar dikirim `ytmusic-api`. */
const SONG_YTM = {
  type: "SONG",
  videoId: "dQw4w9WgXcQ",
  name: "Best Friend",
  artists: [{ name: "Rex Orange County" }, { name: "Tyler" }],
  duration: 225,
  thumbnails: [{ url: "https://lh3.example/ kecil.jpg" }, { url: "https://lh3.example/besar.jpg" }],
};

// ── normalisasi bentuk: sumber lokal (`ytmusic.search()`) ─────────────────────

test("normalize: entri SONG mentah jadi lima field dengan nama field ytmusic", () => {
  const keluar = kapytmusic.normalize([SONG_YTM]);
  assert.deepEqual(keluar, {
    tracks: [
      {
        title: "Best Friend",
        artist: "Rex Orange County, Tyler",
        // Durasi ytmusic dalam DETIK, bukan teks "3:45" seperti bentuk agregator.
        durationSec: 225,
        // Thumbnail diurutkan kecil → besar, jadi yang dipakai adalah entri terakhir.
        cover: "https://lh3.example/besar.jpg",
        url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
      },
    ],
  });
});

test("normalize: entri dengan title (bukan name) dan artist tunggal tetap jalan", () => {
  // `ytmusic-api` memakai `name` untuk SONG dan `title` untuk VIDEO, dan
  // `artists` (array) untuk satu tipe tapi `artist` (objek) untuk tipe lain.
  // Salah satu membacaan yang keliru di sini berarti hasil kosong atau
  // `artist: "undefined"` di daftar.
  const keluar = kapytmusic.normalize([
    { type: "VIDEO", videoId: "abc12345678", title: "Live", artist: { name: "Some Artist" } },
  ]);
  assert.deepEqual(keluar.tracks, [
    { title: "Live", artist: "Some Artist", durationSec: 0, cover: "", url: "https://www.youtube.com/watch?v=abc12345678" },
  ]);
});

test("normalize: ARTIST dan PLAYLIST tanpa videoId dilewati, bukan jadi error", () => {
  // `search()` mengembalikan daftar campuran dan entri non-lagu datang LEBIH
  // DAHULU. Kalau tidak disaring, hasil pertama yang dipakai adalah artist.
  const keluar = kapytmusic.normalize([
    { type: "ARTIST", name: "Rex Orange County", subscribers: "1.2M" },
    { type: "PLAYLIST", title: "Mix Lama", browseId: "VLCA" },
    SONG_YTM,
  ]);
  assert.equal(keluar.tracks.length, 1);
  assert.equal(keluar.tracks[0].title, "Best Friend");
});

test("normalize: daftar kosong sah, bukan kegagalan", () => {
  // Pencarian tanpa hasil itu jawaban yang valid — plugin yang harus
  // menjemputnya, bukan resolver yang harus melempar.
  assert.deepEqual(kapytmusic.normalize([]), { tracks: [] });
});

test("normalize: null dan nilai non-objek melempar", () => {
  assert.throws(() => kapytmusic.normalize(null), /tidak dikenali/);
  assert.throws(() => kapytmusic.normalize("dua"), /tidak dikenali/);
  assert.throws(() => kapytmusic.normalize(7), /tidak dikenali/);
  // Amplop agregator tanpa `result` bukan daftar kosong: itu bentuk tak dikenal,
  // dan membiarkaninya jadi `{ tracks: [] }` menutup kegagalan diam-diam.
  assert.throws(() => kapytmusic.normalize({ status: true, msg: "ok" }), /bukan daftar lagu/);
});

// ── normalisasi bentuk: sumber agregator (`nexray`) ───────────────────────────

test("normalize: bentuk agregator nexray memetakan subtitle→artist dan image→cover", () => {
  // Bentuk nyata dari probe 2026-10-03. `subtitle` berisi "Song · Rex Orange
  // County" dan itu justru yang dicetak plugin sebelum Phase 1, jadi teksnya
  // diteruskan apa adanya — plugin menampilkan kalimat yang sama seperti dulu.
  const keluar = kapytmusic.normalize({
    status: true,
    author: "@nexray - ElrayyXml",
    result: [
      {
        title: "Best Friend",
        subtitle: "Song · Rex Orange County",
        link: "https://music.apple.com/id/album/best-friend/1286662463?i=1286662899",
        image: "https://is1-ssl.mzstatic.com/image/thumb/Music115/110x110bb-60.jpg",
      },
    ],
  });
  assert.deepEqual(keluar, {
    tracks: [
      {
        title: "Best Friend",
        artist: "Song · Rex Orange County",
        // nexray tidak mengirim durasi. Nol berarti "tidak dilaporkan", bukan
        // durasi yang salah — plugin tidak memakainya, dan menebak "3:45" lebih
        // buruk daripada tidak punya.
        durationSec: 0,
        cover: "https://is1-ssl.mzstatic.com/image/thumb/Music115/110x110bb-60.jpg",
        url: "https://music.apple.com/id/album/best-friend/1286662463?i=1286662899",
      },
    ],
  });
});

test("normalize: entri agregator tanpa link dibuang, bukan jadi track rusak", () => {
  const keluar = kapytmusic.normalize({
    status: true,
    result: [{ title: "Tanpa Tautan", subtitle: "Song", image: "https://cdn.example/a.jpg" }],
  });
  assert.deepEqual(keluar, { tracks: [] });
});

// ── guard dan wiring resolver ─────────────────────────────────────────────────

test("stable false: judul lagu dan tautannya berubah terus di host", () => {
  assert.equal(kapytmusic.stable, false, "hasil pencarian tidak boleh keluar dari cache");
});

test("nama backend per-host, unik, dan ada dua tier", () => {
  const lokal = kapytmusic.backends.filter((b) => b.kind === "local");
  const api = kapytmusic.backends.filter((b) => b.kind !== "local");
  assert.deepEqual(lokal.map((b) => b.name), ["ytmusic"]);
  assert.deepEqual(api.map((b) => b.name), ["nexray"]);
  assert.equal(
    new Set(kapytmusic.backends.map((b) => b.name)).size,
    kapytmusic.backends.length,
    "nama ganda berbagi satu slot breaker untuk dua sumber berbeda",
  );
  for (const backend of kapytmusic.backends) {
    assert.doesNotMatch(backend.name, /https?:|[/?@]|\d{4,}/, `nama bukan per-host: ${backend.name}`);
  }
});

test("argumen kosong → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverYtmusic();
  await assert.rejects(
    () => resolver.resolve("ytmusic", {}),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, []);
      assert.deepEqual(resolver.breaker.snapshot(), []);
      return true;
    },
  );
});

test("kueri kosong tidak dianggap sebagai backend yang berlaku", () => {
  assert.equal(ytmLokal.applies({ q: "   " }), false);
  assert.equal(ytmApi.applies({ q: "" }), false);
  assert.equal(ytmLokal.applies({ q: "best friend" }), true);
});

test("pencarian lokal dipakai lebih dulu; agregator tidak boleh diakses lebih awal", async () => {
  // Backend lokal di berkas ini adalah tripwire `YTMusicPalsu`, jadi `search()`
  // melempar — persis keadaan "host lokal mati" yang harus membuat agregator
  // mendapat giliran.
  PANGGILAN_SEARCH.length = 0;
  hasilAggregator = async () => ({
    status: 200,
    data: { status: true, result: [{ title: "Best Friend", subtitle: "Song · Rex Orange County", link: "https://music.apple.com/x", image: "https://cdn.example/a.jpg" }] },
  });

  const keluar = await resolverYtmusic().resolve("ytmusic", { q: "best friend" });

  assert.deepEqual(PANGGILAN_SEARCH, ["best friend"], "backend lokal harus dicoba sebelum agregator");
  assert.equal(keluar.source, "nexray");
  assert.equal(keluar.data.tracks[0].url, "https://music.apple.com/x");
});

// ── amplop `status` harus bertahan melewati backend ───────────────────────────

test("backend agregator wajib mengembalikan amplop, bukan body.result", async () => {
  // Test `normalize` di atas hanya membuktikan `normalize` melempar. Itu tidak
  // bisa menangkap backend yang membuang amplop SEBELUM `normalize` melihatnya.
  // Probe 2026-10-03 membuktikan nexray memang mengirim `status`, jadi backend
  // yang mengembalikan `body.result` menghapus satu-satunya tempat `status`
  // masih bisa dibaca.
  hasilAggregator = async () => ({
    status: 200,
    data: { status: false, msg: "kuota habis", result: [{ title: "Palsu", subtitle: "Song", link: "https://evil.example/bogus" }] },
  });

  const keluar = await ytmApi.run({ q: "best friend" });

  assert.equal(
    keluar?.status,
    false,
    `backend tidak boleh membuang amplop status: ${JSON.stringify(keluar)}`,
  );
  assert.throws(() => kapytmusic.normalize(keluar), /aggregator menandai gagal/);
});

test("status false yang tetap membawa result → tidak ada satu pun track yang lolos", async () => {
  // Diuji sampai lewat resolver, supaya bentuk plugin-nya (pesan "tidak
  // ditemukan", bukan daftar kosong) bisa dijamin.
  hasilAggregator = async () => ({
    status: 200,
    data: { status: false, msg: "kuota habis", result: [{ title: "Palsu", subtitle: "Song", link: "https://evil.example/bogus" }] },
  });

  await assert.rejects(
    () => resolverYtmusic().resolve("ytmusic", { q: "best friend" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["ytmusic", "nexray"]);
      assert.match(error.tried.at(-1).reason, /aggregator menandai gagal/);
      return true;
    },
  );
});

// ── aggregator: URL harus dikunci persis ──────────────────────────────────────

test("nexray: pathname harus persis /search/applemusic dan hanya param q", async () => {
  PANGGILAN_HTTP.length = 0;
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });

  await ytmApi.run({ q: "best friend" });

  const req = PANGGILAN_HTTP.map((c) => new URL(String(c.url))).at(-1);
  assert.equal(req.origin, "https://api.nexray.eu.cc");
  // nexray satu-satunya host di AGGREGATORS yang tidak memakai prefix `/api`.
  // Path lama di plugin adalah `/search/applemusic` (tanpa `/api`) dan probe
  // 2026-10-03 membalas 200 dengan `result` berisi link — menambahkan `/api`
  // di sini akan mengubahnya jadi 404.
  assert.equal(req.pathname, "/search/applemusic");
  assert.equal(req.searchParams.get("q"), "best friend");
  // nexray `key: null` di src/lib/aggregator.js:25, jadi `apikey` di query akan
  // jadi parameter asing yang tidak pernah dibaca host ini.
  assert.deepEqual([...req.searchParams.keys()], ["q"], "param lain tidak boleh ikut terkirim");
  assert.equal(PANGGILAN_HTTP[0].opts?.headers?.apikey, undefined, "nexray tidak punya key");
});

test("path agregator di kapabilitas ini tetap yang tanpa /api, dan tidak ada yang lain", () => {
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/ytmusic.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)].map((m) => m[0].slice(1, -1));
  // `https://www.youtube.com/watch?v=…` juga|POL string yang diawali "/", jadi
  // path aggregator harus dideteksi lewat nama yang pasti muncul supaya test
  // ini tidak bisa hijau karena tidak memindai apa pun.
  assert.ok(
    literalPath.includes("/search/applemusic"),
    `path yang dipakai aggregator.hit harus terdeteksi, dapat: ${literalPath.join(", ")}`,
  );
  const salah = literalPath.filter((p) => p !== "/" && !p.startsWith("/search/applemusic"));
  assert.deepEqual(salah, [], `path agregator yang tidak terduga: ${salah.join(", ")}`);
});

// ── budget: abort harus sampai ke lapisan HTTP ───────────────────────────────

test("signal diteruskan ke request agregator", async () => {
  const controller = new AbortController();
  PANGGILAN_HTTP.length = 0;
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });

  await ytmApi.run({ q: "best friend" }, { signal: controller.signal });

  assert.equal(PANGGILAN_HTTP.at(-1).opts?.signal, controller.signal);
});

test("budget habis di tier lokal → agregator tetap sempat dipanggil", async () => {
  // Backend lokal di berkas ini melempar seketika (tripwire), jadi ini belum
  // membuktikan apa pun soal budget. Yang dibuktikan di sini adalah urutannya:
  // kegagalan tier lokal tidak boleh mencegah giliran tier api.
  PANGGILAN_SEARCH.length = 0;
  PANGGILAN_HTTP.length = 0;
  hasilAggregator = async () => ({ status: 200, data: { status: true, result: [] } });

  const keluar = await resolverYtmusic().resolve("ytmusic", { q: "best friend" });
  assert.equal(keluar.source, "nexray");
  assert.equal(PANGGILAN_SEARCH.length, 1);
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

test("plugin applemusic tidak menyebut domain agregator atau axios lagi", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/search/applemusic.js"), "utf8");
  assert.doesNotMatch(sumber, /nexray|neoxr|axios/i, "plugin harus lewat aggregator.hit di kapabilitas");
});

test("plugin applemusic memakai resolver ytmusic dan menampilkan lima field", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/search/applemusic.js"), "utf8");
  assert.match(sumber, /from ['"]\.\.\/\.\.\/src\/lib\/resolve\.js['"]/, "plugin harus mengimpor resolver");
  assert.match(sumber, /resolve\(\s*['"]ytmusic['"]\s*,\s*\{\s*q\s*:\s*query\s*\}\s*\)/, "argumen harus { q: query }");
  assert.match(sumber, /data\?\.tracks/, "plugin harus membaca data.tracks");
});

test("permukaan plugin applemusic tetap sama: config dan pesan", () => {
  const sumber = fs.readFileSync(path.join(process.cwd(), "plugins/search/applemusic.js"), "utf8");
  // Nilai config di-pin persis seperti sebelum rewire: nama, alias, kategori,
  // cooldown, dan energi.
  assert.match(sumber, /name:\s*'applemusic'/);
  assert.match(sumber, /alias:\s*\[\s*'amusic'\s*,\s*'am'\s*\]/);
  assert.match(sumber, /category:\s*'search'/);
  assert.match(sumber, /cooldown:\s*5/);
  assert.match(sumber, /energi:\s*0/);
  // Dua pesan yang decides clamp harus utuh, huruf demi huruf.
  assert.ok(
    sumber.includes("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*") && sumber.includes("> Contoh:"),
    "pesan cara pakai tidak boleh berubah",
  );
  assert.ok(
    sumber.includes("❌ Tidak ditemukan hasil untuk: ") &&
      sumber.includes("🍎 *ᴀᴘᴘʟᴇ ᴍᴜsɪᴄ sᴇᴀʀᴄʜ*") &&
      sumber.includes("├ 📀") &&
      sumber.includes("└ 🔗"),
    "pesan hasil dan penanda baris tidak boleh berubah",
  );
});

// ═══════════════════════════════════════════════════════════════════════════════
// Task 10 — kapabilitas `hd` (plugin `hd2` imglarger dan `hd3` unblur)
// ═══════════════════════════════════════════════════════════════════════════════
//
// Backend lokal di blok ini adalah scraper sungguhan (`src/scraper/hd.js` lewat
// `httpAxios` yang sudah dimock di berkas ini), jadi alur upload → CheckStatus →
// URL diuji tanpa satu pun request ke jaringan. Yang butuh mock modul terpisah
// adalah `src/scraper/hdvid.js` — ia memakai `axios` polos, bukan `httpAxios` —
// dan itu ada di tests/hd-unblur-local.test.mjs.
//
// Bentuk respons di bawah diturunkan dari kode repo sendiri, bukan dari probe
// jaringan — dan bagian yang tidak bisa diturunkan ditandai apa adanya.
//
// Dari `src/scraper/hd.js`:
//   `upload(filePath)` mengembalikan `data.data`, jadi bentuk yang dilihat
//     kapabilitas adalah `{ code, imageId, type }` — `code` di level TERATAS.
//     Bandingkan `src/scraper/imglarger.js:84` yang membaca `upload?.data?.code`
//     dari respons yang belum dibuka amplopnya; itulah yang membedakan keduanya.
//   `get(code)` juga mengembalikan `data.data`, jadi yang terlihat adalah
//     `{ downloadUrls, filesize, status, … }`.
//   `status` bernilai `"success"` atau `"waiting"` — dibuktikan
//     `src/scraper/imglarger.js:70` yang membandingkan `data.status ===
//     "success"`.
//
// Yang TIDAK bisa diverifikasi dari repo dan tidak boleh diklaim sebagai fakta:
// nilai persis `downloadUrls` saat `status: "waiting"` (apakah URL akar host
// atau apa adanya). Yang dikunci di sini hanya perilaku yang benar dengan
// sendirinya: `waiting` adalah kegagalan, apa pun isi array-nya.
//
// Dari `src/scraper/hdvid.js:75-85`: polling mengembalikan `{ …result }` dengan
// `result.res_url`, dan `pollEnhanceTask` sudah melempar sendiri kalau `res_url`
// kosong — jadi `res_url` adalah satu-satunya nama yang bisa muncul dari sana.
//
// Bentuk agregator berasal dari plugin lama sebelum Phase 1, keduanya bisa dibaca
// di `git show ff397d2^:plugins/tools/hd2.js` dan `hd3.js`: `result` berupa
// string URL untuk imglarger, dan `result.output_url[0]` untuk unblur.

const kapListrik = await import("../src/capabilities/hd.js");

const hdImglarger = kapListrik.backends.find((b) => b.name === "photoai-imglarger");
const hdUnblur = kapListrik.backends.find((b) => b.name === "fgsi-enchantvideo");
const hdIzuka = kapListrik.backends.find((b) => b.name === "izuka");

function resolverHd(opsi = {}) {
  return createResolver({ capabilities: { hd: () => kapListrik }, ...opsi });
}

const KODE_UPLOAD = { code: 200, data: { code: "lgIhxcRf", imageId: "1791017793415", type: 13 }, msg: "Success" };
const SELESAI = {
  code: 200,
  data: {
    downloadUrls: ["https://photoai.imglarger.com/upscaler/lgIhxcRf.jpg"],
    filesize: 1190,
    imagemimetype: "jpg",
    originalfilename: "lgIhxcRf.jpg",
    status: "success",
  },
  msg: "Success",
};
const MASIH_PROSES = {
  code: 200,
  data: {
    // Bentuk `waiting` yang paling merusak: statusnya belum selesai tapi
    // `downloadUrls` tetap terisi. Isi URL-nya di sini adalah kasus yang paling
    // buruk, bukan hasil yang sudah dibuktikan host benar-benar mengirimnya —
    // yang diuji adalah bahwa status yang jadi acuan, bukan isi array.
    downloadUrls: ["https://photoai.imglarger.com/"],
    filesize: 0,
    imagemimetype: "jpg",
    status: "waiting",
  },
  msg: "Success",
};

const PANGGILAN_IZUKA = [];
const PANGGILAN_UPLOAD = [];
// `form-data` menyimpan stream di dalam `DelayedStream`, jadi `getBuffer()`
// melempar untuk field gambar. Yang direkam adalah `append`-nya, supaya nama
// field, opsi, dan jenis nilainya bisa diperiksa tanpa memaksa stream jadi Buffer.
const PANGGILAN_FORM = [];
const appendAsli = FormData.prototype.append;
FormData.prototype.append = function appendRekaman(nama, nilai, opsi) {
  PANGGILAN_FORM.push({ nama, nilai, opsi });
  return appendAsli.call(this, nama, nilai, opsi);
};

/** Stub transport: bedakan dua host scraper lokal dan aggregator izuka. */
function balasHd(url, opts) {
  const alamat = String(url);
  if (alamat.includes("photoai.imglarger.com/api/PhoAi/Upload")) {
    PANGGILAN_UPLOAD.push({ url: alamat, opts });
    return KODE_UPLOAD;
  }
  if (alamat.includes("photoai.imglarger.com/api/PhoAi/CheckStatus")) {
    PANGGILAN_UPLOAD.push({ url: alamat, opts });
    return LevelImglarger;
  }
  if (alamat.includes("my.izuka-api.xyz")) {
    PANGGILAN_IZUKA.push({ url: alamat, opts });
    return IzukaBadal;
  }
  throw new Error(`host tak terduga di test ini: ${alamat}`);
}

let LevelImglarger = SELESAI;
let IzukaBadal = { status: true, result: "https://cdn.example/naik.jpg" };

beforeEach(() => {
  PANGGILAN_UPLOAD.length = 0;
  PANGGILAN_IZUKA.length = 0;
  PANGGILAN_FORM.length = 0;
  LevelImglarger = SELESAI;
  IzukaBadal = { status: true, result: "https://cdn.example/naik.jpg" };
  // Stub ini HANYA-chief untuk dua host di atas. Host lain tetap mendapat
  // tripwire yang dipasang `beforeEach` pertama, karena hook top-level berlaku
  // ke SELURUH test di berkas ini — bukan hanya yang ditulis setelahnya. Tanpa
  // penjaga itu, blok ini diam-diam melemahkan jaring pengaman blok Task 3-9.
  balasSpotyloader = async (url, opts) => {
    const alamat = String(url);
    if (alamat.includes("photoai.imglarger.com") || alamat.includes("my.izuka-api.xyz")) {
      return { status: 200, data: balasHd(alamat, opts) };
    }
    throw new Error("http tidak boleh dipanggil: upstream mati");
  };
});

// ── file sementara untuk pengujian ────────────────────────────────────────────
//
// Backend lokal dan aggregator sama-sama butuh file di disk: `upload(filePath)`
// dan `createEnhanceTask(filePath)` menerima path, dan FormData ke izuka harus
// mengalirkan bytes dari disk. Berkas ini dibuat sekali dan dihapus di akhir.

const MEDIA_UJI = path.join(process.cwd(), "tmp", "hd-uji.jpg");
fs.mkdirSync(path.join(process.cwd(), "tmp"), { recursive: true });
fs.writeFileSync(MEDIA_UJI, Buffer.from("bukan jpeg sungguhan, tapi cukup untuk FormData", "utf8"));
process.on("exit", () => {
  try {
    fs.unlinkSync(MEDIA_UJI);
  } catch {}
});

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: bentuk lokal get(code) dengan downloadUrls jadi { url }", () => {
  const keluar = kapListrik.normalize(SELESAI.data);
  assert.deepEqual(keluar, { url: "https://photoai.imglarger.com/upscaler/lgIhxcRf.jpg" });
});

test("normalize: status waiting beserta downloadUrls terisi TIDAK boleh lolos", () => {
  // Bentuk paling merusak di kapabilitas ini: `get(code)` adalah SATU panggilan
  // CheckStatus tanpa polling, jadi host yang masih mengolah menjawab
  // `status: "waiting"` sementara `downloadUrls` tetap terisi. Menerimanya
  // berarti plugin mengirim URL yang belum di-render sambil tetap memberi
  // centang hijau. Statusnya yang jadi acuan, bukan isi arraynya.
  assert.throws(
    () => kapListrik.normalize(MASIH_PROSES.data),
    /belum selesai/,
    "status waiting harus jadi kegagalan, bukan URL apa adanya",
  );
});

test("normalize: bentuk agregator imglarger — result berupa string URL", () => {
  // Plugin lama membaca `data.result` dan memakainya
  // langsung sebagai `document.url`, jadi `result` di sini adalah string.
  assert.deepEqual(kapListrik.normalize({ status: true, result: "https://cdn.example/naik.jpg" }), {
    url: "https://cdn.example/naik.jpg",
  });
});

test("normalize: bentuk agregator unblur — result.output_url[0]", () => {
  // Plugin lama hd3 menuntut `data.result.output_url[0]`; ini bentuk
  // yang harus jadi `{ url }` yang sama supaya plugin tidak perlu tahu backend
  // mana yang menjawab.
  assert.deepEqual(
    kapListrik.normalize({ status: true, result: { output_url: ["https://cdn.example/jelas.jpg", "https://cdn.example/lain.jpg"] } }),
    { url: "https://cdn.example/jelas.jpg" },
  );
});

test("normalize: bentuk pollEnhanceTask memakai res_url", () => {
  // src/scraper/hdvid.js:77-85 melempar kalau `result.res_url` kosong, jadi itu
  // satu-satunya nama field yang bisa muncul dari jalur unblur lokal.
  assert.deepEqual(
    kapListrik.normalize({ taskId: "t1", createdAt: "2026-10-03", res_url: "https://cdn.example/hd.mp4" }),
    { url: "https://cdn.example/hd.mp4" },
  );
});

test("normalize: bentuk { url }, { data: { url } }, dan { result: { url } } satu bentuk", () => {
  for (const bentuk of [
    { url: "https://cdn.example/a.jpg" },
    { data: { url: "https://cdn.example/a.jpg" } },
    { result: { url: "https://cdn.example/a.jpg" } },
  ]) {
    assert.deepEqual(kapListrik.normalize(bentuk), { url: "https://cdn.example/a.jpg" }, `gagal: ${JSON.stringify(bentuk)}`);
  }
});

test("normalize: nilai yang bukan URL http dilolak, termasuk path lokal dan string kosong", () => {
  // `sendMedia` dengan `file:///...` atau `javascript:` bukan enhancement.
  for (const buruk of [
    { url: "file:///etc/passwd" },
    { url: "/tmp/hasil.jpg" },
    { result: "" },
    { downloadUrls: [null] },
    { url: "   " },
  ]) {
    assert.throws(() => kapListrik.normalize(buruk), /tanpa URL hasil/, `lolos: ${JSON.stringify(buruk)}`);
  }
});

test("normalize: nilai non-objek, null, dan bentuk tanpa URL melempar", () => {
  for (const buruk of [null, undefined, "https://cdn.example/a.jpg", 7, {}, { status: true }, { filesize: 10 }]) {
    assert.throws(() => kapListrik.normalize(buruk), /tidak dikenali|tanpa URL hasil/, `lolos: ${JSON.stringify(buruk)}`);
  }
});

test("normalize: amplop status false ditolak walau URL-nya ada", () => {
  assert.throws(
    () => kapListrik.normalize({ status: false, msg: "kuota habis", result: "https://evil.example/bogus.jpg" }),
    /menandai gagal/,
  );
});

// ── wiring resolver dan dispatch kind ─────────────────────────────────────────

test("stable false: file hasil enhancement berumur pendek dan tidak boleh dicache", () => {
  assert.equal(kapListrik.stable, false, "URL hasil render tidak boleh keluar dari cache");
});

test("nama backend per-host, unik, dan dua backend lokal lalu satu api", () => {
  const lokal = kapListrik.backends.filter((b) => b.kind === "local");
  const api = kapListrik.backends.filter((b) => b.kind !== "local");
  assert.deepEqual(lokal.map((b) => b.name).sort(), ["fgsi-enchantvideo", "photoai-imglarger"]);
  assert.deepEqual(api.map((b) => b.name), ["izuka"]);
  assert.equal(
    new Set(kapListrik.backends.map((b) => b.name)).size,
    kapListrik.backends.length,
    "nama ganda berbagi satu slot breaker untuk dua host berbeda",
  );
  for (const backend of kapListrik.backends) {
    assert.doesNotMatch(backend.name, /https?:|[/?@]|\d{4,}/, `nama bukan per-host: ${backend.name}`);
  }
});

test("kind di luar dua yang dikenal ditolak, dan pesannya menyebut keduanya", async () => {
  // Dipanggil langsung supaya pesan yang dilihat operator ada di test. Lewat
  // resolver, tidak ada backend yang berlaku dan yang muncul adalah
  // `no-applicable-backend` — diuji terpisah di bawah.
  await assert.rejects(
    () => hdImglarger.run({ kind: " sharpen ", media: MEDIA_UJI }),
    (error) => {
      assert.match(error.message, /imglarger/);
      assert.match(error.message, /unblur/);
      return true;
    },
  );
  await assert.rejects(() => hdUnblur.run({ kind: null, media: MEDIA_UJI }), /imglarger[\s\S]*unblur/);
  await assert.rejects(() => hdIzuka.run({ kind: "sharpen", media: MEDIA_UJI }), /imglarger[\s\S]*unblur/);
});

test("kind yang dikenal dibaca longgar: huruf besar dan spasi excess tetap dilayani", () => {
  for (const backend of kapListrik.backends) {
    assert.equal(backend.applies({ kind: "IMGLARGER", media: MEDIA_UJI }), backend.name !== "fgsi-enchantvideo");
    assert.equal(backend.applies({ kind: " unblur ", media: MEDIA_UJI }), backend.name !== "photoai-imglarger");
  }
});

test("media tanpa path yang bisa dibaca tidak cocok dengan backend mana pun", () => {
  for (const backend of kapListrik.backends) {
    assert.equal(backend.applies({ kind: "imglarger" }), false, backend.name);
    assert.equal(backend.applies({ kind: "imglarger", media: "   " }), false, backend.name);
    assert.equal(backend.applies({ kind: "unblur" }), false, backend.name);
  }
});

test("argumen kosong → no-applicable-backend, breaker bersih", async () => {
  const resolver = resolverHd();
  await assert.rejects(
    () => resolver.resolve("hd", {}),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(resolver.breaker.snapshot(), []);
      return true;
    },
  );
});

// `{}` di atas belum membuktikan apa pun soal `applies`: tidak ada kind sama
// sekali, jadi setiap backend ditolak karena media kosong. Kasus ini lebih
// tajam — media ADA dan valid, hanya `kind` yang tidak dikenal. Kalau `applies`
// tidak ada, ketiga backend akan-contacted dan ikut dicatat sebagai kegagalan,
// yaitu persis kelas bug yang `applies` diciptakan untuk mencegah.
test("kind yang tidak dikenal: tidak ada backend yang dihubungi, breaker tetap bersih", async () => {
  const resolver = resolverHd();
  await assert.rejects(
    () => resolver.resolve("hd", { kind: "sharpen", media: MEDIA_UJI }),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, [], "backend yang tidak berlaku tidak boleh masuk daftar percobaan");
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_UPLOAD, [], "tidak boleh ada request ke host mana pun");
  assert.deepEqual(PANGGILAN_IZUKA, [], "tidak boleh ada request ke aggregator");
  assert.deepEqual(resolver.breaker.snapshot(), [], "kind asing bukan kegagalan host");
});

// `.hd3` tidak boleh membuka breaker `photoai-imglarger` hanya karena
// `resolve('hd', {kind:'unblur'})` tidak menyentuh host itu — backend yang tidak
// berlaku tidak pernah dihubungi, jadi tidak bisa gagal.
test("unblur tidak pernah menyentuh imglarger, jadi breaker-nya tidak terganggu", async () => {
  LevelImglarger = MASIH_PROSES; // kalau dihubungi, ini akan jadi kegagalan
  IzukaBadal = { status: true, result: { output_url: ["https://cdn.example/jelas.jpg"] } };

  const resolver = resolverHd();
  const keluar = await resolver.resolve("hd", { kind: "unblur", media: MEDIA_UJI });

  assert.equal(keluar.source, "izuka", "imglarger tidak berlaku untuk kind unblur");
  assert.deepEqual(keluar.data, { url: "https://cdn.example/jelas.jpg" });
  assert.deepEqual(PANGGILAN_UPLOAD, [], "host imglarger tidak boleh dihubungi");
  // Breaker yang terbuka menandai `photoai-imglarger` bermasalah, padahal host
  // itu tidak pernah dihubungi sama sekali.
  const terbuka = resolver.breaker.snapshot().filter((s) => s.name === "photoai-imglarger");
  assert.deepEqual(terbuka, [], "backend yang tidak berlaku tidak boleh dihitung sebagai kegagalan");
});

// ── backend lokal imglarger: scraper sungguhan lewat transport palsu ──────────

test("imglarger lokal: upload lalu CheckStatus mengembalikan URL hasil", async () => {
  const keluar = await resolverHd().resolve("hd", { kind: "imglarger", media: MEDIA_UJI });

  assert.equal(keluar.source, "photoai-imglarger");
  assert.deepEqual(keluar.data, { url: "https://photoai.imglarger.com/upscaler/lgIhxcRf.jpg" });
  // Urutan dua request itu juga bagian dari kontrak: `get(code)` tanpa `code`
  // dari `upload` tidak akan pernah dipanggil.
  assert.deepEqual(
    PANGGILAN_UPLOAD.map((c) => new URL(c.url).pathname),
    ["/api/PhoAi/Upload", "/api/PhoAi/CheckStatus"],
  );
  assert.deepEqual(PANGGILAN_IZUKA, [], "backend lokal tidak boleh menyentuh aggregator");
});

test("imglarger lokal: host masih mengolah → kegagalan, aggregator jadi cadangan", async () => {
  LevelImglarger = MASIH_PROSES;

  const keluar = await resolverHd().resolve("hd", { kind: "imglarger", media: MEDIA_UJI });

  assert.equal(keluar.source, "izuka", "aggregator harus dipakai saat host lokal belum selesai");
  assert.deepEqual(keluar.data, { url: "https://cdn.example/naik.jpg" });
});

test("imglarger lokal: file yang tidak ada berhenti sebelum request", async () => {
  await assert.rejects(
    () => resolverHd().resolve("hd", { kind: "imglarger", media: path.join(process.cwd(), "tmp", "tidak-ada.jpg") }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["photoai-imglarger", "izuka"]);
      return true;
    },
  );
  assert.deepEqual(PANGGILAN_UPLOAD, [], "tidak boleh ada request untuk file yang tidak ada");
});

test("ctx.signal tidak bisa sampai ke scraper lokal, dan itu dibiarkan terang-terangan", async () => {
  // `upload(filePath)` dan `get(code)` di src/scraper/hd.js tidak menerima
  // opsi, jadi `aggregator.hit`-style `signal` tidak punya jalan masuk. Test ini
  // mengunci kenyataan itu sebagai catatan TERANG: kalau scraper suatu saat
  // sudah menerima signal, test ini harus ikut berubah. Bandingkan
  // src/capabilities/youtube.js, yang mencatat hal yang sama untuk SaveTube.
  const controller = new AbortController();
  await hdImglarger.run({ kind: "imglarger", media: MEDIA_UJI }, { signal: controller.signal });

  assert.equal(PANGGILAN_UPLOAD.length, 2);
  for (const panggilan of PANGGILAN_UPLOAD) {
    assert.equal(panggilan.opts?.signal, undefined, "scraper lokal tidak menerima signal; dokumentasi harus diperbarui");
  }
});

// Bukti yang tidak bisa dipalsukan oleh assertion "signal ada di opts": kalau
// `signal` benar-benar sampai ke transport, `abort()` memanggil listener-nya.
// Request di sini sengaja menggantung, jadi tanpa signal yang sampai ke
// `httpAxios` promise-nya tidak akan pernah batal.
//
// Punya batas waktu sendiri supaya kegagalan dilaporkan sebagai SATU test merah
// dengan pesan jelas. Tanpa itu, promise yang menggantung membuat node:test
// menguras event loop dan membatalkan seluruh test setelahnya — 15 test lain
// ikut mati karena satu bug, dan penyebabnya tidak terlihat.
test("signal aggregator benar-benar membatalkan request, bukan sekadar ada di opts", async () => {
  balasSpotyloader = (url, opts) => {
    const alamat = String(url);
    PANGGILAN_IZUKA.push({ url: alamat, opts });
    if (alamat.includes("photoai.imglarger.com")) return { status: 200, data: LevelImglarger };
    return new Promise((resolve, reject) => {
      opts?.signal?.addEventListener("abort", () => reject(new Error("dibatalkan oleh signal")));
    });
  };

  const controller = new AbortController();
  const jalan = hdIzuka.run({ kind: "imglarger", media: MEDIA_UJI }, { signal: controller.signal });
  controller.abort();

  // Jam dijaga TIDAK di-unref dan selalu dibersihkan: kalau signal tidak
  // sampai, `jalan` menggantung selamanya. Tanpa jam yang menahan event loop,
  // node:test menguras loop dan membatalkan 15 test berikutnya — penyebabnya
  // jadi tidak terlihat di antara kegagalan yang tidak terkait.
  let jam;
  try {
    const batasWaktu = new Promise((_, reject) => {
      jam = setTimeout(() => reject(new Error("signal tidak mencapai transport: request tidak pernah batal")), 2000);
    });
    await assert.rejects(
      () => Promise.race([jalan, batasWaktu]),
      /dibatalkan oleh signal/,
      "signal tidak mencapai transport: request menggantung dan tidak pernah batal",
    );
  } finally {
    clearTimeout(jam);
  }

  assert.equal(
    PANGGILAN_IZUKA.at(-1).opts?.signal,
    controller.signal,
    "backend wajib meneruskan ctx.signal ke lapisan HTTP",
  );
});

// ── amplop `status` harus bertahan melewati backend ───────────────────────────

test("backend izuka wajib mengembalikan amplop, bukan body.result", async () => {
  // Bentuk amplopnya pasti dari plugin lama, bukan dari probe: kedua plugin
  // lama membaca `data.status` sebelum memakai `data.result`.
  // Kalau backend mengembalikan `body.result`, `status: false` yang masih
  // membawa URL akan lolos ke `normalize` dan dikirim sebagai hasil.
  IzukaBadal = { status: false, msg: "kuota habis", result: "https://evil.example/bogus.jpg" };

  const keluar = await hdIzuka.run({ kind: "imglarger", media: MEDIA_UJI });

  assert.equal(keluar?.status, false, `amplop dibuang: ${JSON.stringify(keluar)}`);
  assert.throws(() => kapListrik.normalize(keluar), /menandai gagal/);
});

test("status false yang tetap membawa URL → tidak ada hasil yang bisa dikirim", async () => {
  // Tier lokal harus gagal lebih dulu, kalau tidak test ini hanya membuktikan
  // bahwa backend lokal menang dan tidak pernah menyentuh amplop sama sekali.
  LevelImglarger = MASIH_PROSES;
  IzukaBadal = { status: false, msg: "kuota habis", result: "https://evil.example/bogus.jpg" };

  await assert.rejects(
    () => resolverHd().resolve("hd", { kind: "imglarger", media: MEDIA_UJI }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(error.tried.map((t) => t.name), ["photoai-imglarger", "izuka"]);
      assert.match(error.tried.at(-1).reason, /menandai gagal/);
      return true;
    },
  );
});

// ── aggregator: URL harus dikunci persis ──────────────────────────────────────

test("izuka imglarger: POST persis ke /api/tools/imglarger, tanpa key", async () => {
  await hdIzuka.run({ kind: "imglarger", media: MEDIA_UJI });

  assert.equal(PANGGILAN_IZUKA.length, 1);
  // URL LENGAP dikunci, bukan hanya pathname-nya: `AGGREGATORS.izuka.base` hanya
  // berisi host dan `aggregator.hit` menempelkan path apa adanya, jadi satu
  // karakter `/` yang hilang membuat host membalas 404 — cadangan yang lalu
  // menjadi sumber utama kegagalan. Ini persis bug Critical yang lolos di Task 4.
  assert.equal(
    String(PANGGILAN_IZUKA[0].url),
    "https://my.izuka-api.xyz/api/tools/imglarger",
    "URL lengkap harus sama dengan yang plugins/tools/hd2.js:56 panggil sebelum Phase 1",
  );
  const req = new URL(PANGGILAN_IZUKA[0].url);
  assert.equal(req.origin, "https://my.izuka-api.xyz");
  assert.equal(req.pathname, "/api/tools/imglarger");
  assert.deepEqual([...req.searchParams.keys()], [], "endpoint ini tidak punya parameter query");
  const opts = PANGGILAN_IZUKA[0].opts;
  assert.equal(opts.method, undefined, "aggregator.hit memakai method sebagai argumen, bukan di dalam opts");
  assert.equal(opts.headers?.apikey, undefined, "izuka tidak punya key di AGGREGATORS");
  // Bukti method-nya benar-benar POST, bukan hanya "opts tidak punya method":
  // `httpAxios.post` adalah satu-satunya handler yang menerima FormData, dan
  // `httpPalsu.get` di berkas ini menolak semua host selain nexray — jadi
  // request yang tercatat di sini sudah lewat jalur POST.
  assert.equal(PANGGILAN_HTTP.at(-1).verb, "post");
});

test("izuka unblur: URL persis /api/tools/unblur, bukan endpoint imglarger", async () => {
  // Dua endpoint hidup berdampingan di host yang sama; tertukar berarti gambar
  // tetap buram (atau salah efek) tanpa satu pesan pun.
  IzukaBadal = { status: true, result: { output_url: ["https://cdn.example/jelas.jpg"] } };
  await hdIzuka.run({ kind: "unblur", media: MEDIA_UJI });

  assert.equal(
    String(PANGGILAN_IZUKA[0].url),
    "https://my.izuka-api.xyz/api/tools/unblur",
    "URL lengkap harus sama dengan yang plugins/tools/hd3.js:50 panggil sebelum Phase 1",
  );
  const req = new URL(PANGGILAN_IZUKA[0].url);
  assert.equal(req.pathname, "/api/tools/unblur");
  assert.equal(req.origin, "https://my.izuka-api.xyz");
});

test("method POST dan field multipart persis seperti plugin lama", async () => {
  await hdIzuka.run({ kind: "imglarger", media: MEDIA_UJI });

  const form = PANGGILAN_IZUKA[0].opts?.data;
  assert.ok(form && typeof form.append === "function", "body harus FormData");
  // Field `type` dan `scale` bukan karangan: plugin lama hd2:53-54
  // mengirim keduanya ke endpoint yang sama, dan menghilangkannya mengubahnya
  // jadi parameter lain.
  assert.deepEqual(
    PANGGILAN_FORM.map((c) => c.nama),
    ["image", "type", "scale"],
    `field multipart tidak sama dengan plugin lama: ${PANGGILAN_FORM.map((c) => c.nama).join(", ")}`,
  );
  assert.deepEqual(
    PANGGILAN_FORM.slice(1).map((c) => c.nilai),
    ["upscale", "2"],
  );
  assert.deepEqual(
    PANGGILAN_FORM[0].opsi,
    { filename: "image.jpg", contentType: "image/jpeg" },
    "nama dan contentType harus sama seperti plugins/tools/hd2.js:52",
  );
});

test("unblur tidak mengirim field type/scale, mengikuti plugin lama", async () => {
  await hdIzuka.run({ kind: "unblur", media: MEDIA_UJI });

  assert.deepEqual(
    PANGGILAN_FORM.map((c) => c.nama),
    ["image"],
    `unblur hanya mengirim image: ${PANGGILAN_FORM.map((c) => c.nama).join(", ")}`,
  );
});

test("gambar dikirim sebagai stream dari disk, bukan Buffer penuh", async () => {
  // Batas keras: file milik user dibaca ke RAM setiap kali ada yang salah. Yang
  // boleh dipegang modul ini adalah stream, dan `form-data` yang mengalirkan
  // bytes-nya.
  await hdIzuka.run({ kind: "imglarger", media: MEDIA_UJI });

  const gambar = PANGGILAN_FORM[0].nilai;
  assert.ok(gambar, "harus ada isi multipart untuk field image");
  assert.equal(Buffer.isBuffer(gambar), false, "gambar tidak boleh dibaca penuh ke memori");
  assert.equal(typeof gambar.pipe, "function", "isi field image harus stream");

  const kode = fs.readFileSync(path.join(process.cwd(), "src/capabilities/hd.js"), "utf8");
  assert.doesNotMatch(kode, /readFileSync|readFile\(/, "backend lokal tidak boleh membaca file penuh");
});

test("timeout aggregator mengikuti timeout lama plugin, dan sinyal diteruskan", async () => {
  const controller = new AbortController();
  await hdIzuka.run({ kind: "imglarger", media: MEDIA_UJI }, { signal: controller.signal });

  const opts = PANGGILAN_IZUKA[0].opts;
  assert.equal(opts?.signal, controller.signal, "signal wajib sampai ke HTTP");
  // 60 detik bukan angka baru: `timeout: 60000` di kedua plugin lama adalah
  // batas yang sudah dipakai produksi. Default aggregator 5
  // detik akan memotong setiap enhancement yang memang butuh waktu.
  assert.equal(opts?.timeout, 60_000, "timeout harus mengikuti batas lama plugin");
});

test("path agregator di kapabilitas ini semuanya berawalan /api", () => {
  const kode = fs
    .readFileSync(path.join(process.cwd(), "src/capabilities/hd.js"), "utf8")
    .split("\n")
    .filter((baris) => !/^\s*(\/\/|\/\*|\*)/.test(baris))
    .join("\n");
  const literalPath = [...kode.matchAll(/["'`]\/[^"'`\s]*["'`]/g)].map((m) => m[0].slice(1, -1)).filter((p) => p !== "/");
  assert.ok(
    literalPath.includes("/api/tools/imglarger") && literalPath.includes("/api/tools/unblur"),
    `kedua path harus terdeteksi, dapat: ${literalPath.join(", ")}`,
  );
  const salah = literalPath.filter((p) => !p.startsWith("/api/"));
  assert.deepEqual(salah, [], `path tanpa /api: ${salah.join(", ")}`);
});

// ── budget ────────────────────────────────────────────────────────────────────

test("budget habis di tier lokal → aggregator tetap sempat dipanggil", async () => {
  // Host lokal yang tidak menjawab dalam budget. Karena `upload()` tidak
  // menerima signal, request lokal tidak bisa benar-benar dibatalkan — resolver
  // menyerah sesuai budgetnya dan pindah backend, dan itulah yang diuji di
  // sini: tier api tetap mendapat giliran.
  balasSpotyloader = (url, opts) => {
    if (String(url).includes("my.izuka-api.xyz")) {
      PANGGILAN_IZUKA.push({ url, opts });
      return { status: 200, data: IzukaBadal };
    }
    return new Promise(() => {});
  };

  const keluar = await resolverHd({ budget: { localMs: 40, totalMs: 600 } }).resolve("hd", {
    kind: "imglarger",
    media: MEDIA_UJI,
  });

  assert.equal(keluar.source, "izuka");
  assert.deepEqual(keluar.data, { url: "https://cdn.example/naik.jpg" });
  assert.equal(PANGGILAN_IZUKA.length, 1, "aggregator harus tetap dipanggil setelah tier lokal kehabisan waktu");
});

// ── plugin sudah tidak bicara langsung ke agregator ──────────────────────────

test("hd2 dan hd3 tidak menyebut domain agregator, axios, atau FormData lagi", () => {
  for (const berkas of ["plugins/tools/hd2.js", "plugins/tools/hd3.js"]) {
    const sumber = fs.readFileSync(path.join(process.cwd(), berkas), "utf8");
    assert.doesNotMatch(sumber, /izuka|nexray|neoxr/i, `${berkas} harus lewat aggregator.hit di kapabilitas`);
    assert.doesNotMatch(sumber, /axios/i, `${berkas} tidak boleh bicara ke transport`);
    assert.doesNotMatch(sumber, /FormData/, `${berkas} tidak boleh merakit body multipart sendiri`);
  }
});

test("hd2 dan hd3 memakai resolver hd dengan kind yang tepat", () => {
  const hd2 = fs.readFileSync(path.join(process.cwd(), "plugins/tools/hd2.js"), "utf8");
  const hd3 = fs.readFileSync(path.join(process.cwd(), "plugins/tools/hd3.js"), "utf8");
  for (const [berkas, sumber] of [["hd2", hd2], ["hd3", hd3]]) {
    assert.match(sumber, /from ['"]\.\.\/\.\.\/src\/lib\/resolve\.js['"]/, `${berkas} harus mengimpor resolver`);
    assert.match(sumber, /resolve\(\s*['"]hd['"]\s*,\s*\{\s*kind\s*:/, `${berkas} harus lewat resolve('hd', { kind … })`);
  }
  assert.match(hd2, /kind:\s*['"]imglarger['"]/, "hd2 memakai imglarger");
  assert.match(hd3, /kind:\s*['"]unblur['"]/, "hd3 memakai unblur");
});

test("hd2 dan hd3 mengupload dari file sementara dan membersihkannya", async () => {
  // Upload butuh path (`upload(filePath)`), sementara plugin hanya punya Buffer
  // dari `m.download()`. Berkas sementara harus dihapus apa pun hasilnya —
  // HD adalah fitur yang dipakai sering, dan sisa gambar user menumpuk di disk.
  for (const berkas of ["plugins/tools/hd2.js", "plugins/tools/hd3.js"]) {
    const sumber = fs.readFileSync(path.join(process.cwd(), berkas), "utf8");
    assert.match(sumber, /writeFile\(/, `${berkas} harus menulis berkas sementara`);
    assert.match(sumber, /unlink\(/, `${berkas} harus menghapus berkas sementara`);
    assert.match(sumber, /finally/, `${berkas} pembersihan harus di finally`);
  }
});

test("pesan dan permukaan kedua plugin tetap hidup", () => {
  const hd2 = fs.readFileSync(path.join(process.cwd(), "plugins/tools/hd2.js"), "utf8");
  const hd3 = fs.readFileSync(path.join(process.cwd(), "plugins/tools/hd3.js"), "utf8");
  // Config di-pin nilai demi nilai: nama, alias, kategori, cooldown, energi.
  assert.match(hd2, /name:\s*"hd2"/);
  assert.deepEqual(
    JSON.parse(JSON.stringify([...hd2.matchAll(/alias:\s*\[([^\]]*)\]/g)].flatMap((m) => [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1])))),
    ["enhance2", "upscale2", "aienhancer"],
  );
  assert.match(hd2, /cooldown:\s*30/);
  assert.match(hd2, /energi:\s*2/);
  assert.deepEqual(
    JSON.parse(JSON.stringify([...hd3.matchAll(/alias:\s*\[([^\]]*)\]/g)].flatMap((m) => [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1])))),
    ["enhance3", "upscale3", "unblur"],
  );
  assert.match(hd3, /cooldown:\s*20/);
  assert.match(hd3, /energi:\s*2/);
  // Reaksi dan kalimat yang dilihat user tidak boleh berubah.
  for (const [berkas, sumber] of [["hd2", hd2], ["hd3", hd3]]) {
    assert.ok(sumber.includes('react("🕕")'), `${berkas} harus tetap memberi jam pasir`);
    assert.ok(sumber.includes('react("✅")'), `${berkas} harus tetap memberi centang hijau`);
    assert.ok(sumber.includes('react("❌")'), `${berkas} harus tetap memberi tanda gagal`);
    assert.ok(sumber.includes('react("☢")'), `${berkas} harus tetap memberi tanda error`);
  }
  // Kedua plugin menulis "kamu berikan" SEJAK AWAL — dibuktikan dari
  // `git show ff397d2^:plugins/tools/hd3.js`, bukan dari asumsi. Menyamakan
  // atau "memperbaiki" salah satunya bukan bagian Phase 1, jadi dua-duanya
  // dikunci apa adanya.
  assert.ok(hd2.includes("Maaf, sistem gagal mengunduh gambar yang kamu berikan."));
  assert.ok(hd3.includes("Maaf, sistem gagal mengunduh gambar yang kamu berikan."));
  assert.ok(hd2.includes("Maaf, AI gagal memproses gambarmu kali ini."));
  assert.ok(hd3.includes("Maaf, AI gagal memproses gambarmu kali ini."));
  // Nama file yang dikirim ke user tidak boleh berubah.
  assert.ok(hd2.includes("HD_BY_${config.bot.name}.jpg"), "nama file hd2 harus tetap");
  assert.ok(hd3.includes("UNBLUR_BY_${config.bot.name}.jpg"), "nama file hd3 harus tetap");
});
