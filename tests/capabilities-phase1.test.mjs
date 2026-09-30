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
// spotyloader, dan `ytmusic-api` — sehingga tidak ada satu pun request yang
// keluar ke jaringan dan upstream yang sedang mati tidak bisa membuat test
// berkedip merah.

const PANGGILAN_HTTP = [];
const PANGGILAN_SEARCH = [];

let balasSpotyloader = () => {
  throw new Error("http tidak boleh dipanggil: upstream mati");
};
let hasilSearch = () => {
  throw new Error("ytmusic tidak boleh dipanggil: upstream mati");
};

const httpPalsu = {
  async get(url) {
    PANGGILAN_HTTP.push({ verb: "get", url });
    throw new Error("GET ke sumber daya luar tidak diizinkan di test ini");
  },
  async post(url, body) {
    PANGGILAN_HTTP.push({ verb: "post", url });
    return balasSpotyloader(url, body);
  },
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu } });

class YTMusicPalsu {
  async initialize() {}
  async search(q) {
    PANGGILAN_SEARCH.push(q);
    return hasilSearch(q);
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
  hasilSearch = () => {
    throw new Error("ytmusic tidak boleh dipanggil: upstream mati");
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
  const keluar = kapspotify.normalize({
    tracks: [
      {
        title: "Grateful",
        artist: "Neffex",
        url: "https://open.spotify.com/track/abc",
        cover: "https://cdn.example/c.jpg",
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

test("normalize: durasi YT Music (detik) jadi teks, plugin tidak menampilkan undefined", () => {
  const keluar = kapspotify.normalize({
    tracks: [
      {
        type: "SONG",
        name: "Grateful",
        videoId: "dQw4w9WgXcQ",
        duration_seconds: 185,
      },
    ],
  });
  assert.equal(keluar.tracks[0].duration, "3:05");
  assert.equal(
    kapspotify.normalize({ tracks: [{ title: "x", url: "https://a/b" }] }).tracks[0].duration,
    "0:00",
    "durasi yang tidak ada harus tetap string, bukan undefined",
  );
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

test("normalize: entri YT Music tanpa videoId (ARTIST/PLAYLIST) dibuang", () => {
  // search() YT Music mencampur ARTIST dan PLAYLIST di depan daftar, dan keduanya
  // tidak punya videoId. Kalau tidak dibuang, index 0 yang dipakai plugin
  // sebagai "lagu pertama" bukan lagu.
  const keluar = kapspotify.normalize({
    tracks: [
      { type: "ARTIST", name: "Neffex" },
      { type: "PLAYLIST", name: "Phonk" },
      {
        type: "SONG",
        name: "Grateful",
        videoId: "dQw4w9WgXcQ",
        artists: [{ name: "Neffex" }, { name: "Smit" }],
        thumbnails: [{ url: "https://i.ytimg.com/a.jpg" }],
      },
    ],
  });
  assert.equal(keluar.tracks.length, 1);
  assert.deepEqual(keluar.tracks[0], {
    title: "Grateful",
    artist: "Neffex, Smit",
    url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
    cover: "https://i.ytimg.com/a.jpg",
    duration: "0:00",
  });
});

test("normalize: entri tanpa identitas apa pun dibuang, daftar boleh kosong", () => {
  assert.deepEqual(kapspotify.normalize({ tracks: [{ type: "ARTIST" }] }), { tracks: [] });
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

  // Argumen yang berbeda_total tapi nama sama: kalau nama diturunkan dari
  // argumen, breaker akan allocating slot baru tiap permintaan dan host yang
  // OPEN akan ter-evict lalu terlihat sehat lagi.
  for (const q of ["satu", "dua", "tiga"]) {
    await resolver.resolve("spotify", { q }).catch(() => {});
  }
  assert.equal(
    resolver.breaker.snapshot().length,
    kapspotify.backends.length,
    "slot breaker tidak boleh bertambah dari variasi argumen",
  );
});

test("minimal satu backend local dan satu api sebagai cadangan", () => {
  const lokal = kapspotify.backends.filter((b) => b.kind === "local");
  const api = kapspotify.backends.filter((b) => b.kind !== "local");
  assert.ok(lokal.length >= 1, `backend local minimal satu, dapat ${lokal.length}`);
  assert.ok(api.length >= 1, "harus ada cadangan aggregator, spotyloader bisa mati");
  assert.equal(api[0].name, "nexray", "host cadangan harus nexray (tanpa API key)");
});

test("backend download dipakai hanya untuk { url }, backend search hanya untuk { q }", async () => {
  const backendSearch = kapspotify.backends.find((b) => b.kind === "local" && b !== backendLokal);
  await assert.rejects(
    () => backendSearch.run({ url: "https://open.spotify.com/track/abc" }),
    /q/,
    "backend search tidak boleh menerima { url }",
  );
  await assert.rejects(
    () => backendLokal.run({ q: "neffex" }),
    /url/,
    "backend download tidak boleh menerima { q }",
  );
  assert.deepEqual(PANGGILAN_HTTP, []);
  assert.deepEqual(PANGGILAN_SEARCH, []);
});

test("pencarian: backend local lebih dulu, hasilnya sudah bentuk { tracks }", async () => {
  hasilSearch = async () => [
    { type: "ARTIST", name: "Neffex" },
    { type: "SONG", name: "Grateful", videoId: "dQw4w9WgXcQ", artists: [{ name: "Neffex" }] },
  ];
  const resolver = resolverUji();
  const keluar = await resolver.resolve("spotify", { q: "neffex grateful" });

  assert.equal(keluar.source, "ytmusic");
  assert.equal(keluar.meta.cached, false);
  assert.equal(keluar.data.tracks.length, 1);
  assert.equal(keluar.data.tracks[0].title, "Grateful");
  assert.equal(
    keluar.meta.tookMs >= 0,
    true,
    "initialize() YT Music dipanggil sekali, tidak hang",
  );
});

test("pencarian lokal nihil → backend aggregator dicoba, tidak dijawab kosong", async () => {
  // Daftar yang isinya ARTIST/PLAYLIST saja bukan jawaban: plugin memakai
  // tracks[0] sebagai "lagu pertama", jadi backend bolong ini harus melepas
  // ke backend lain, bukan dijawab dengan tracks kosong.
  hasilSearch = async () => [{ type: "ARTIST", name: "Neffex" }];
  const resolver = resolverUji();
  await assert.rejects(
    () => resolver.resolve("spotify", { q: "neffex" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      const dicoba = error.tried.map((t) => t.name);
      assert.ok(
        dicoba.includes("ytmusic") && dicoba.includes("nexray"),
        `semua backend harus dicoba berurutan: ${dicoba.join(", ")}`,
      );
      return true;
    },
  );
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