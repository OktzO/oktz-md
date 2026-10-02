import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import { createCipheriv } from "node:crypto";
import { CapabilityError, createResolver } from "../src/lib/resolve.js";

// ── harness ───────────────────────────────────────────────────────────────────
//
// Berkas ini menguji lapisan yang menyentuh jaringan sungguhan: `ytdl`
// (src/scraper/ytdl.js) dan SaveTube (src/scraper/youtube.js) memanggil `axios`
// langsung, jadi transport-nya yang dimock — modul kapabilitasnya diimpor
// apa adanya supaya import-nya ikut diuji: file yang hilang harus menggagalkan
// berkas ini, bukan lolos diam-diam.
//
// Dipisah dari tests/capabilities-phase1.test.mjs karena `axios` tidak boleh
// di-mock dua kali dalam satu berkas, dan karena pengujian short-link butuh
// melihat persis request apa yang sampai ke d.ymcdn.org — sesuatu yang tidak
// terlihat lewat mock `httpAxios`.

const PANGGILAN_AXIOS = [];
const PANGGILAN_HTTP = [];

let balasYmdcn = () => {
  throw new Error("d.ymcdn.org tidak boleh dipanggil: upstream mati");
};
let balasSavetube = () => {
  throw new Error("savetube tidak boleh dipanggil: upstream mati");
};
let hasilAggregator = () => {
  throw new Error("aggregator tidak boleh dipanggil: upstream mati");
};

const axiosPalsu = {
  // `ytdl` memakai `axios.create`, SaveTube memakai `axios` modul langsung.
  // `create` harus sinkron: kalau async, hasilnya Promise dan `ytdl` melempar
  // "client.get is not a function" — persis galat yang membuat test ini hijau
  // tanpa pernah menyentuh stub.
  create() {
    return {
      async get(url, opts) {
        PANGGILAN_AXIOS.push({ verb: "get", url, opts });
        return balasYmdcn(url, opts);
      },
      async post(url, body, opts) {
        PANGGILAN_AXIOS.push({ verb: "post", url, body, opts });
        return balasSavetube(url, body, opts);
      },
    };
  },
  async get(url, opts) {
    PANGGILAN_AXIOS.push({ verb: "get", url, opts });
    return balasSavetube(url, opts);
  },
  async post(url, body, opts) {
    PANGGILAN_AXIOS.push({ verb: "post", url, body, opts });
    return balasSavetube(url, body, opts);
  },
};
mock.module("axios", { defaultExport: axiosPalsu });

const httpPalsu = {
  async get(url, opts) {
    PANGGILAN_HTTP.push({ verb: "get", url, opts });
    return hasilAggregator(url, opts);
  },
  async post() {
    throw new Error("POST ke aggregator tidak boleh dipakai di test ini");
  },
};
mock.module("../src/lib/http.js", { namedExports: { httpAxios: httpPalsu } });

const kapyoutube = await import("../src/capabilities/youtube.js");

const ytdlLokal = kapyoutube.backends.find((b) => b.name === "ytdl-native");
const fallbackLokal = kapyoutube.backends.find((b) => b.name === "youtube-fallback");

const ID = "dQw4w9WgXcQ";
const KANONIK = `https://www.youtube.com/watch?v=${ID}`;
const UNDUHAN_MP3 = "https://cdn.example/audio.mp3";
const UNDUHAN_MP4 = "https://cdn.example/video.mp4";
const THUMB = `https://i.ytimg.com/vi/${ID}/hqdefault.jpg`;

function resolverYoutube(opsi = {}) {
  // Registry diganti agar berkas ini tidak ikut bergantung pada modul kapabilitas
  // lain yang belum dibuat (ytmusic, hd).
  return createResolver({ capabilities: { youtube: () => kapyoutube }, ...opsi });
}

beforeEach(() => {
  PANGGILAN_AXIOS.length = 0;
  PANGGILAN_HTTP.length = 0;
  balasYmdcn = () => {
    throw new Error("d.ymcdn.org tidak boleh dipanggil: upstream mati");
  };
  balasSavetube = () => {
    throw new Error("savetube tidak boleh dipanggil: upstream mati");
  };
  hasilAggregator = () => {
    throw new Error("aggregator tidak boleh dipanggil: upstream mati");
  };
});

/**
 * Bentuk respons ymcdn yang tercatat di src/scraper/ytdl.js: `init` memberi
 * `convertURL`, `convert` memberi `progressURL` + `downloadURL`, dan `progress`
 * harus mencapai 3 sebelum `ytdl` mengembalikan `{ status: true, title, dl }`.
 */
function ymdcnSukses({ judul = "Lagu Uji", dl = UNDUHAN_MP3 } = {}) {
  balasYmdcn = async (url) => {
    const u = String(url);
    if (u.includes("/api/v1/init")) return { data: { convertURL: "https://d.ymcdn.org/api/v1/convert" } };
    if (u.includes("/api/v1/convert")) {
      return {
        data: { title: judul, progressURL: "https://d.ymcdn.org/api/v1/progress", downloadURL: dl },
      };
    }
    if (u.includes("/api/v1/progress")) return { data: { progress: 3, title: judul } };
    throw new Error(`host tak terduga: ${u}`);
  };
}

/** Bentuk SaveTube yang tercatat di src/scraper/youtube.js: `data` terenkripsi AES. */
function savetubeSukses({ judul = "Video Uji", dl = UNDUHAN_MP4 } = {}) {
  const iv = Buffer.alloc(16);
  const cipher = createCipheriv("aes-128-cbc", Buffer.from("C5D58EF67A7584E4A29F6C35BBC4EB12", "hex"), iv);
  const terenkripsi = Buffer.concat([
    cipher.update(JSON.stringify({ title: judul, key: "kunci" })),
    cipher.final(),
  ]);
  balasSavetube = async (url) => {
    const u = String(url);
    if (u.includes("/api/random-cdn")) return { data: { cdn: "cdn.example" } };
    if (u.includes("/v2/info")) {
      return { data: { data: Buffer.concat([iv, terenkripsi]).toString("base64") } };
    }
    if (u.includes("/download")) return { data: { data: { downloadUrl: dl } } };
    throw new Error(`host tak terduga: ${u}`);
  };
}

/** Bentuk amplop aggregator yang dibaca `plugins/` sebelum Phase 1. */
function aggregatorSukses(body) {
  return async () => ({ status: 200, data: body });
}

// ── normalisasi bentuk ────────────────────────────────────────────────────────

test("normalize: bentuk ytdl jadi empat field", () => {
  // Bentuk lokal yang sebenarnya (src/scraper/ytdl.js:152) adalah
  // `{ status, title, dl }` — nama field unduhannya `dl`, bukan `url`.
  assert.deepEqual(kapyoutube.normalize({ status: true, title: "Lagu Uji", dl: UNDUHAN_MP3 }), {
    title: "Lagu Uji",
    thumbnail: "",
    format: "mp3",
    url: UNDUHAN_MP3,
  });
});

test("normalize: bentuk SaveTube memakai `thumb`, bukan `thumbnail`", () => {
  // SaveTube (src/scraper/youtube.js:86) menamai thumbnail `thumb` dan
  // menamai format sebagai nomor kualitas ("360"), bukan "mp4"/"mp3".
  const keluar = kapyoutube.normalize({
    title: "Video Uji",
    rawTitle: "Video Uji Official",
    thumb: THUMB,
    duration: 212,
    format: "360",
    url: UNDUHAN_MP4,
  });
  assert.deepEqual(keluar, {
    title: "Video Uji",
    thumbnail: THUMB,
    format: "mp3",
    url: UNDUHAN_MP4,
  });
  assert.equal(keluar.title, "Video Uji", "rawTitle bukan judul yang dikirim");
});

test("normalize: bentuk izuka mp3 (result.download_url) jadi empat field", () => {
  // plugins/download/ytmp3.js sebelum Phase 1 membaca `res.data.result.download_url`.
  assert.deepEqual(
    kapyoutube.normalize({ status: true, result: { download_url: UNDUHAN_MP3, title: "Lagu Uji" } }),
    { title: "Lagu Uji", thumbnail: "", format: "mp3", url: UNDUHAN_MP3 },
  );
});

test("normalize: bentuk azbry mp3 (result.download) jadi empat field", () => {
  // plugins/search/playch.js sebelum Phase 1 membaca `res.data.result.download`
  // — nama field berbeda dari izuka untuk hal yang sama.
  assert.deepEqual(kapyoutube.normalize({ status: true, result: { download: UNDUHAN_MP3, title: "Lagu Uji" } }), {
    title: "Lagu Uji",
    thumbnail: "",
    format: "mp3",
    url: UNDUHAN_MP3,
  });
});

test("normalize: bentuk izuka mp4 (result.video_normal) memilih mp4 kualitas tertinggi", () => {
  // plugins/download/ytmp4.js dan plugins/search/playvid.js menyaring
  // `ext === "mp4"` lalu mengurutkan `quality` menurun. Urutan itu kontrak host.
  const keluar = kapyoutube.normalize({
    status: true,
    result: {
      title: "Video Uji",
      video_normal: [
        { ext: "mp4", quality: "360", url: "https://cdn.example/360.mp4" },
        { ext: "webm", quality: "720", url: "https://cdn.example/720.webm" },
        { ext: "mp4", quality: "480", url: "https://cdn.example/480.mp4" },
      ],
    },
  });
  assert.equal(keluar.url, "https://cdn.example/480.mp4", "hanya ext mp4, kualitas tertinggi");
  assert.equal(keluar.title, "Video Uji");
});

test("normalize: bentuk buffer-only dari downloadWithFallback ditolak", () => {
  // `downloadWithFallback` (src/scraper/youtube.js:375) mengembalikan
  // `{ ...res, url: null, _buffer: res.buffer }` di jalur fallback QByte: byte-nya
  // ada tapi tidak ada URL, dan `sendMedia(chat, data.url)` tidak punya apa-apa
  // untuk dikirim. Menerima bentuk ini berarti centang hijau tanpa file.
  assert.throws(
    () =>
      kapyoutube.normalize({
        title: "Video Uji",
        thumb: THUMB,
        buffer: Buffer.from("video"),
        format: "mp4",
        url: null,
        _buffer: Buffer.from("video"),
      }),
    /tanpa URL/,
  );
});

test("normalize: bentuk kosong dan bukan-objek melempar", () => {
  assert.throws(() => kapyoutube.normalize({}), /tidak dikenali|tanpa URL/i);
  assert.throws(() => kapyoutube.normalize(null), /tidak dikenali/);
  assert.throws(() => kapyoutube.normalize(undefined), /tidak dikenali/);
  assert.throws(() => kapyoutube.normalize(UNDUHAN_MP3), /tidak dikenali/);
});

test("normalize: `status: false` adalah kegagalan walau URL-nya ada", () => {
  // `ytdl` mengembalikan objek, bukan melempar (src/scraper/ytdl.js:153), dan
  // aggregator membungkus hasil di `{ status, result }`. Tanpa pemeriksaan ini,
  // respons "Request timeout (proses terlalu lama)." lolos sebagai scraping sukses.
  assert.throws(
    () => kapyoutube.normalize({ status: false, mess: "Request timeout (proses terlalu lama).", dl: UNDUHAN_MP3 }),
    /proses terlalu lama/,
  );
  assert.throws(
    () => kapyoutube.normalize({ status: false, result: { download_url: "https://cdn.example/x.mp3" } }),
    /youtube/,
  );
});

test("normalize: `status: true` tanpa `dl` tetap gagal", () => {
  // Objek lengkap tapi tidak menghasilkan apa pun yang bisa dikirim.
  assert.throws(() => kapyoutube.normalize({ status: true, title: "Lagu Uji" }), /tanpa URL/i);
});

test("normalize: URL non-http ditolak, bukan diteruskan ke sendMedia", () => {
  for (const jahat of ["javascript:alert(1)", "/path/lokal.mp3", "ftp://cdn.example/a.mp3", "  "]) {
    assert.throws(() => kapyoutube.normalize({ status: true, dl: jahat }), /tanpa URL/i, `harus menolak: ${jahat}`);
  }
});

// ── Review Focus #1: short link ──────────────────────────────────────────────
//
// Bentuk yang tidak bisa dibaca `YOUTUBE_ID_REGEX` (src/scraper/ytdl.js:8)
// tetap harus sampai ke scraper dalam bentuk kanonik, karena `ytdl` hanya
// mengambil ID dari URL itu — dan setiap polling konversi memakai ID tersebut.

test("short link: setiap bentuk yang diterima user diteruskan sebagai ID video yang sama", async () => {
  const bentuk = [
    `https://youtu.be/${ID}`,
    `https://youtu.be/${ID}?t=42`,
    `https://www.youtube.com/watch?v=${ID}`,
    `https://www.youtube.com/watch?app=desktop&v=${ID}&t=1s`,
    `https://m.youtube.com/watch?v=${ID}`,
    `https://music.youtube.com/watch?v=${ID}`,
    `https://www.youtube.com/shorts/${ID}`,
    `https://m.youtube.com/shorts/${ID}`,
    `https://youtube.com/shorts/${ID}?feature=share`,
    `https://www.youtube.com/live/${ID}`,
    `https://www.youtube.com/embed/${ID}`,
    `https://www.youtube.com/v/${ID}`,
  ];

  for (const masuk of bentuk) {
    PANGGILAN_AXIOS.length = 0;
    ymdcnSukses();

    const keluar = await ytdlLokal.run({ url: masuk, format: "mp3" });

    assert.equal(keluar.status, true, `harus sukses untuk ${masuk}`);
    // Yang menentukan bukan bentuk URL-nya, tapi ID video yang sampai ke host
    // konversi: inilah yang dipakai server untuk mencari videonya.
    const requestKonversi = PANGGILAN_AXIOS.find((c) => String(c.url).includes("/api/v1/convert"));
    assert.ok(requestKonversi, `tidak ada request convert untuk ${masuk}`);
    assert.equal(requestKonversi.opts?.params?.v, ID, `ID harus diteruskan apa adanya untuk ${masuk}`);
    // Dan bentuk yang benar-benar sampai ke scraper harus kanonik, bukan
    // bentuk yang tidak bisa dibaca `YOUTUBE_ID_REGEX` untuk `/shorts/`.
    assert.equal(requestKonversi.opts?.params?.f, "mp3");
  }
});

test("short link: backend mp4 juga menerima bentuk yang sama", async () => {
  for (const masuk of [`https://youtu.be/${ID}`, `https://music.youtube.com/watch?v=${ID}`, `https://www.youtube.com/shorts/${ID}`]) {
    PANGGILAN_AXIOS.length = 0;
    savetubeSukses();
    await fallbackLokal.run({ url: masuk, format: "mp4" });
    const info = PANGGILAN_AXIOS.find((c) => String(c.url).includes("/v2/info"));
    assert.ok(info, `tidak ada request info untuk ${masuk}`);
    assert.deepEqual(
      info.body,
      { url: KANONIK },
      `SaveTube harus menerima URL kanonik untuk ${masuk}`,
    );
  }
});

test("short link: bentuk yang tidak dikenal ditolak tanpa satu pun request", async () => {
  const ditolak = [
    "https://vimeo.com/12345",
    "https://youtube.com.evil.example/watch?v=dQw4w9WgXcQ",
    "https://notyoutube.com/watch?v=dQw4w9WgXcQ",
    "ftp://youtube.com/watch?v=dQw4w9WgXcQ",
    "https://www.youtube.com/watch?v=pendek",
    "https://www.youtube.com/",
    "bukan-url",
  ];

  for (const masuk of ditolak) {
    PANGGILAN_AXIOS.length = 0;
    PANGGILAN_HTTP.length = 0;

    for (const backend of kapyoutube.backends) {
      for (const format of ["mp3", "mp4"]) {
        assert.equal(
          backend.applies({ url: masuk, format }),
          false,
          `applies harus false: ${backend.name} / ${masuk}`,
        );
      }
    }

    await assert.rejects(
      () => ytdlLokal.run({ url: masuk, format: "mp3" }),
      /tidak dikenal|butuh \{ url \}/,
      `harus ditolak: ${masuk}`,
    );
    assert.deepEqual(PANGGILAN_AXIOS, [], `tidak boleh ada request ke ymcdn/savetube: ${masuk}`);
    assert.deepEqual(PANGGILAN_HTTP, [], `tidak boleh ada request ke aggregator: ${masuk}`);
  }
});

test("argumen non-URL tidak pernah menyentuh backend mana pun", async () => {
  for (const args of [{}, { url: "" }, { url: "   " }, { url: 123 }, { format: "mp3" }]) {
    for (const backend of kapyoutube.backends) {
      assert.equal(backend.applies(args), false, `applies harus false: ${backend.name} ${JSON.stringify(args)}`);
    }
  }
  assert.deepEqual(PANGGILAN_AXIOS, []);
  assert.deepEqual(PANGGILAN_HTTP, []);
});

// ── format ───────────────────────────────────────────────────────────────────

test("format: default mp3, mp4 diteruskan apa adanya", async () => {
  assert.equal(ytdlLokal.applies({ url: KANONIK }), true, "tanpa format harus dianggap mp3");
  assert.equal(ytdlLokal.applies({ url: KANONIK, format: "mp4" }), false, "ytdl-native hanya mp3");
  assert.equal(fallbackLokal.applies({ url: KANONIK }), false, "youtube-fallback hanya mp4");
  assert.equal(fallbackLokal.applies({ url: KANONIK, format: "mp4" }), true);
  assert.equal(fallbackLokal.applies({ url: KANONIK, format: "MP4" }), true, "huruf besar tetap dikenali");

  ymdcnSukses();
  await ytdlLokal.run({ url: KANONIK });
  assert.equal(
    PANGGILAN_AXIOS.find((c) => String(c.url).includes("/api/v1/convert"))?.opts?.params?.f,
    "mp3",
    "tanpa format, scraper harus dipanggil sebagai mp3",
  );

  PANGGILAN_AXIOS.length = 0;
  ymdcnSukses();
  await ytdlLokal.run({ url: KANONIK, format: "mp4" });
  assert.equal(
    PANGGILAN_AXIOS.find((c) => String(c.url).includes("/api/v1/convert"))?.opts?.params?.f,
    "mp4",
    "format harus diteruskan apa adanya",
  );
});

// ── backend lokal ────────────────────────────────────────────────────────────

test("mp3: ytdl-native memakai scraper lokal dan mengembalikan judul sungguhan", async () => {
  ymdcnSukses({ judul: "Lagu Uji Official" });

  const keluar = await ytdlLokal.run({ url: `https://youtu.be/${ID}`, format: "mp3" });

  assert.equal(keluar.status, true);
  assert.equal(keluar.title, "Lagu Uji Official");
  assert.equal(keluar.dl, UNDUHAN_MP3);
  assert.deepEqual(kapyoutube.normalize(keluar), {
    title: "Lagu Uji Official",
    thumbnail: "",
    format: "mp3",
    url: UNDUHAN_MP3,
  });
  assert.deepEqual(PANGGILAN_HTTP, [], "backend lokal tidak boleh menghubungi aggregator");
});

test("mp3: kegagalan ytdl yang berstatus tidak pernah lolos sebagai data", async () => {
  // `init` tanpa `convertURL` → src/scraper/ytdl.js:100 mengembalikan objek gagal.
  balasYmdcn = async () => ({ data: {} });

  const keluar = await ytdlLokal.run({ url: KANONIK, format: "mp3" });

  assert.equal(keluar.status, false, "scraper lokal harus melaporkan gagal lewat status");
  assert.throws(() => kapyoutube.normalize(keluar), /gagal|tanpa URL/i);
});

test("mp4: youtube-fallback memakai SaveTube, dan format yang dilaporkan adalah mp4", async () => {
  savetubeSukses({ judul: "Video Uji" });

  const keluar = await fallbackLokal.run({ url: `https://www.youtube.com/shorts/${ID}`, format: "mp4" });

  // SaveTube melaporkan `format` sebagai nomor kualitas ("360"), bukan sebagai
  // format kapabilitas. Kalau yang diteruskan apa adanya, `data.format` untuk
  // permintaan mp4 akan berbohong sebagai mp3.
  assert.equal(keluar.format, "mp4");
  assert.deepEqual(kapyoutube.normalize(keluar), {
    title: "Video Uji",
    thumbnail: THUMB,
    format: "mp4",
    url: UNDUHAN_MP4,
  });

  // Kualitas yang diminta dikunci di 360: itu kualitas terkecil yang masih layak
  // ditonton, jadi unduhannya paling ringan untuk kotak 1GB, dan sama dengan
  // yang dipakai plugins/search/ytplay.js:858. Quality string "mp4" ditolak
  // SaveTube (src/scraper/youtube.js:55) — kalau terpakai, jalur lokal mp4
  // akan selalu gagal diam-diam lalu jatuh ke aggregator.
  const unduh = PANGGILAN_AXIOS.find((c) => String(c.url).includes("/download"));
  assert.equal(unduh?.body?.downloadType, "video");
  assert.equal(unduh?.body?.quality, "360", "kualitas pertama yang diminta harus 360");
});

// ── wiring resolver ──────────────────────────────────────────────────────────

test("stable false: URL unduhan YouTube punya masa berlaku pendek", () => {
  assert.equal(kapyoutube.stable, false, "dari cache tidak boleh ada: URL YouTube bisa mati");
});

test("mp3: lokal dulu, aggregator belakangan", async () => {
  ymdcnSukses();
  hasilAggregator = () => {
    throw new Error("aggregator tidak boleh dihubungi sebelum backend lokal");
  };

  const keluar = await resolverYoutube().resolve("youtube", { url: KANONIK, format: "mp3" });

  assert.equal(keluar.source, "ytdl-native");
  assert.equal(keluar.data.url, UNDUHAN_MP3);
});

test("mp4: lokal dulu, aggregator belakangan", async () => {
  savetubeSukses();
  hasilAggregator = () => {
    throw new Error("aggregator tidak boleh dihubungi sebelum backend lokal");
  };

  const keluar = await resolverYoutube().resolve("youtube", { url: KANONIK, format: "mp4" });

  assert.equal(keluar.source, "youtube-fallback");
  assert.equal(keluar.data.format, "mp4");
});

test("mp3: kegagalan scraper lokal → aggregator dapat giliran, dengan URL kanonik", async () => {
  // Kegagalan tercepat yang sah: `ytdl` menangkap error dan mengembalikan
  // `{ status: false, mess }` (src/scraper/ytdl.js:153).
  balasYmdcn = async () => {
    throw new Error("getaddrinfo ENOTFOUND d.ymcdn.org");
  };
  hasilAggregator = aggregatorSukses({ status: true, result: { download_url: UNDUHAN_MP3, title: "Lagu Uji" } });

  const keluar = await resolverYoutube().resolve("youtube", { url: `https://youtu.be/${ID}`, format: "mp3" });

  assert.equal(keluar.source, "izuka");
  assert.equal(keluar.data.url, UNDUHAN_MP3);
  const req = PANGGILAN_HTTP.map((c) => new URL(String(c.url))).at(-1);
  assert.equal(req.searchParams.get("url"), KANONIK, "aggregator harus menerima URL kanonik, bukan short link");
});

test("mp4: kegagalan SaveTube → aggregator dapat giliran", async () => {
  // `SaveTube.download` mencoba 4 kali dengan jeda 800/1600/2400 ms, jadi backend
  // lokal melewati budget-nya. Yang diuji hanya arahnya: hasilnya tidak boleh
  // pernah berasal dari `youtube-fallback`, dan aggregator tetap jadi sumber.
  balasSavetube = async () => {
    throw new Error("cdn.savetube tidak menjawab");
  };
  hasilAggregator = aggregatorSukses({
    status: true,
    result: { title: "Video Uji", video_normal: [{ ext: "mp4", quality: "720", url: UNDUHAN_MP4 }] },
  });

  const keluar = await resolverYoutube({ budget: { localMs: 40, totalMs: 900 } }).resolve("youtube", {
    url: KANONIK,
    format: "mp4",
  });

  assert.equal(keluar.source, "izuka");
  assert.equal(keluar.data.url, UNDUHAN_MP4);
  assert.equal(keluar.data.format, "mp4", "format yang dilaporkan harus sesuai yang diminta");
});

test("semua backend gagal → CapabilityError, tidak ada URL basi yang dikembalikan", async () => {
  balasYmdcn = async () => {
    throw new Error("getaddrinfo ENOTFOUND d.ymcdn.org");
  };
  hasilAggregator = aggregatorSukses({ status: false, msg: "apikey is not registered." });

  await assert.rejects(
    () => resolverYoutube().resolve("youtube", { url: KANONIK, format: "mp3" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(
        error.tried.map((t) => t.name),
        ["ytdl-native", "izuka", "azbry"],
      );
      return true;
    },
  );
});

test("vimeo lewat resolver → no-applicable-backend, breaker tetap bersih", async () => {
  const resolver = resolverYoutube();

  await assert.rejects(
    () => resolver.resolve("youtube", { url: "https://vimeo.com/12345", format: "mp3" }),
    (error) => {
      assert.equal(error.code, "no-applicable-backend");
      assert.deepEqual(error.tried, []);
      return true;
    },
  );
  assert.deepEqual(resolver.breaker.snapshot(), [], "tidak ada backend yang dihubungi, breaker harus kosong");
  assert.deepEqual(PANGGILAN_AXIOS, []);
  assert.deepEqual(PANGGILAN_HTTP, []);
});

test("hasil resolve tidak pernah masuk cache", async () => {
  ymdcnSukses();
  const resolver = resolverYoutube();

  await resolver.resolve("youtube", { url: KANONIK, format: "mp3" });
  const kedua = await resolver.resolve("youtube", { url: KANONIK, format: "mp3" });

  assert.equal(kedua.meta.cached, false, "stable false berarti tidak boleh ada jalur cache");
  assert.equal(kedua.source, "ytdl-native");
});

// ── aggregator ───────────────────────────────────────────────────────────────

test("aggregator: bentuk endpoint yang meleset adalah kegagalan, bukan format yang berbohong", async () => {
  const izuka = kapyoutube.backends.find((b) => b.name === "izuka");
  const azbry = kapyoutube.backends.find((b) => b.name === "azbry");

  // Permintaan mp3 dijawab bentuk mp4. `normalize` tidak tahu format yang dipesan,
  // jadi tanpa pemeriksaan di backend hasilnya `format: "mp3"` yang berisi URL
  // video — plugin akan mengirim mp4 untuk `.ytmp3`.
  hasilAggregator = aggregatorSukses({
    status: true,
    result: { title: "Video", video_normal: [{ ext: "mp4", quality: "720", url: UNDUHAN_MP4 }] },
  });
  await assert.rejects(() => izuka.run({ url: KANONIK, format: "mp3" }), /tanpa download_url/);

  // Permintaan mp4 dijawab bentuk mp3.
  hasilAggregator = aggregatorSukses({ status: true, result: { download_url: UNDUHAN_MP3 } });
  await assert.rejects(() => izuka.run({ url: KANONIK, format: "mp4" }), /tanpa video_normal/);

  // `result` ada tapi kosong.
  hasilAggregator = aggregatorSukses({ status: true, result: {} });
  await assert.rejects(() => azbry.run({ url: KANONIK, format: "mp3" }), /tanpa download/);

  // Amplop hilang.
  hasilAggregator = aggregatorSukses({});
  await assert.rejects(() => izuka.run({ url: KANONIK, format: "mp3" }), /izuka menandai gagal/);
});

test("status false yang tetap membawa URL → resolver gagal, tidak ada hasil sama sekali", async () => {
  // Bentuk yang paling berbahaya: aggregator menjawab 200 dengan `status: false`
  // tapi `result` yang penuh. Plugin sebelum Phase 1 mensyaratkan `data.status`,
  // dan pemeriksaan itu harus tetap berpengaruh sampai belakang.
  balasYmdcn = async () => {
    throw new Error("getaddrinfo ENOTFOUND d.ymcdn.org");
  };
  hasilAggregator = aggregatorSukses({
    status: false,
    msg: "kunci aggregator bermasalah",
    result: { download_url: UNDUHAN_MP3, title: "Lagu Uji" },
  });

  await assert.rejects(
    () => resolverYoutube().resolve("youtube", { url: KANONIK, format: "mp3" }),
    (error) => {
      assert.ok(error instanceof CapabilityError);
      assert.deepEqual(
        error.tried.map((t) => t.name),
        ["ytdl-native", "izuka", "azbry"],
      );
      assert.ok(
        error.tried.every((t) => !/Lagu Uji|\.mp3$/.test(t.reason)),
        `alasan gagal tidak boleh memuat hasil yang ditolak: ${JSON.stringify(error.tried)}`,
      );
      return true;
    },
  );
});

// Sentinel: berkas ini tidak boleh hijau tanpa isi.
test("sentinel: modul kapabilitas benar-benar terimpor", () => {
  assert.equal(typeof kapyoutube.normalize, "function");
  assert.ok(Array.isArray(kapyoutube.backends) && kapyoutube.backends.length > 0);
  for (const backend of kapyoutube.backends) {
    assert.equal(typeof backend.run, "function", `backend ${backend.name} tidak punya run`);
    assert.equal(typeof backend.applies, "function", `backend ${backend.name} tidak punya applies`);
  }
});