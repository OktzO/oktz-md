import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import path from "node:path";
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

test("azbry: prefix /api adalah aturan host, bukan tebakan per kapabilitas", () => {
  // Nexray memang tanpa prefix (`https://api.nexray.eu.cc/downloader/v2/...`),
  // jadi ini tidak bisa disimpulkan dari base URL: harus ditulis per host.
  assert.match(
    kappinterest.backends.find((b) => b.name === "azbry").name,
    /^[a-z0-9-]+$/,
  );
  const sumber = fs.readFileSync(path.join(process.cwd(), "src/capabilities/pinterest.js"), "utf8");
  assert.doesNotMatch(
    sumber,
    /aggregator\.hit\("azbry", "\/(?!api\/)/,
    "path aggregator tidak boleh tanpa /api",
  );
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