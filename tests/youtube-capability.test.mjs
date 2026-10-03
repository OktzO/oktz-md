import { test, beforeEach, mock } from "node:test";
import assert from "node:assert";
import { createCipheriv } from "node:crypto";
import { CapabilityError, createResolver } from "../src/lib/resolve.js";
import {
  FORMAT_DEFAULT,
  extensionFor,
  formatFromMime,
  formatFromToken,
  mimetypeFor,
} from "../src/lib/media-format.js";

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
    // Yang mengunci bentuk kanonik yang benar-benar sampai ke scraper ada di test
    // mp4 di bawah (body SaveTube) dan di tests/capabilities-phase1.test.mjs
    // (query string aggregator): keduanya meneruskan `url` penuh, sedangkan
    // `ytdl` hanya menerima ID. Yang dipin di sini cukup formatnya.
    assert.equal(requestKonversi.opts?.params?.f, "mp3");
  }
});

test("short link: URL tanpa skema dan URL di dalam kalimat tetap dibaca", async () => {
  // Ketiga bentuk ini bekerja sebelum Phase 1: guard plugin lama hanya
  // `url.includes("youtu.be")` dan `YOUTUBE_ID_REGEX` tidak dianchor, jadi
  // menarik ID dari tengah string. Setelah `hostYoutube` memperketat parsing jadi
  // `new URL`, ketiganya jadi `❌ URL harus YouTube` untuk link yang tadinya
  // mengunduh — jadi bentuk-bentuk ini harus dikunci, bukan diserahkan ke Wayback.
  const tanpaSkema = [`youtu.be/${ID}`, `youtube.com/watch?v=${ID}`, `www.youtube.com/watch?v=${ID}`];
  const dalamTeks = [
    `putar https://youtu.be/${ID} dong`,
    `tolong unduh youtube.com/watch?v=${ID} ya`,
    `.ytmp3 https://www.youtube.com/watch?v=${ID}`,
  ];

  for (const masuk of [...tanpaSkema, ...dalamTeks]) {
    // Setidaknya satu backend harus mau mencoba; kalau semua `applies` false,
    // resolver melempar `no-applicable-backend` tanpa satu pun request.
    const yangMau = kapyoutube.backends.filter((b) => b.applies({ url: masuk, format: "mp3" }));
    assert.ok(yangMau.length > 0, `tidak ada backend yang mau mencoba: ${masuk}`);

    PANGGILAN_AXIOS.length = 0;
    ymdcnSukses();
    const keluar = await ytdlLokal.run({ url: masuk, format: "mp3" });
    assert.equal(keluar.status, true, `harus sukses untuk ${masuk}`);
    const req = PANGGILAN_AXIOS.find((c) => String(c.url).includes("/api/v1/convert"));
    assert.equal(req?.opts?.params?.v, ID, `ID harus diteruskan apa adanya untuk ${masuk}`);

    // Backend mp4 dan aggregator harus melihat bentuk kanonik yang sama, kalau
    // tidak maka normalisasi hanya berlaku di satu jalur dan user mendapat
    // mp3/video yang berbeda tergantung backend mana yang menang.
    PANGGILAN_AXIOS.length = 0;
    savetubeSukses();
    await fallbackLokal.run({ url: masuk, format: "mp4" });
    const info = PANGGILAN_AXIOS.find((c) => String(c.url).includes("/v2/info"));
    assert.deepEqual(info?.body, { url: KANONIK }, `SaveTube harus menerima kanonik untuk ${masuk}`);

    PANGGILAN_HTTP.length = 0;
    hasilAggregator = aggregatorSukses({ status: true, result: { download_url: UNDUHAN_MP3 } });
    await kapyoutube.backends.find((b) => b.name === "izuka").run({ url: masuk, format: "mp3" });
    const reqApi = PANGGILAN_HTTP.map((c) => new URL(String(c.url))).at(-1);
    assert.equal(reqApi?.searchParams.get("url"), KANONIK, `aggregator harus menerima kanonik untuk ${masuk}`);
  }
});

test("short link: URL yang dipalsukan di dalam URL lain tetap ditolak", async () => {
  // Pemotongan tautan dari dalam kalimat harus ambil kemunculan pertama yang
  // lengkap, bukan memungut `youtu.be` dari mana saja di string. Kalau tidak,
  // `https://evil.example/redirect?to=https://youtu.be/<id>` akan lolos hanya
  // karena contains "youtu.be" — persis mode yang membuat `includes` tidak bisa
  // dipakai sebagai guard.
  for (const masuk of [
    `https://evil.example/redirect?to=https://youtu.be/${ID}`,
    `https://youtube.com.evil.example/watch?v=${ID}`,
    // Dua tautan dalam satu pesan ikut masuk daftar penolakan, dan itu keputusan
    // yang disengaja: hanya tautan pertama yang dibaca, jadi kalau tautan pertama
    // bukan YouTube pesan ini dijawab "❌ URL harus YouTube" — bukan dipungut
    // `youtu.be` dari tautan kedua diam-diam. Alasannya ada di komentar
    // `kandidatTautan` (src/capabilities/youtube.js): memindai semua token
    // whitespace akan membuat `https://evil.example/redirect?to=https://youtu.be/<id>`
    // (satu tautan, host evil) dan dua tautan diperlakukan berbeda, padahal
    // keduanya ambigu. Kalau suatu saat ini perlu didukung, jawabannya "mana yang
    // kamu mau?" — bukan memilih sendiri.
    `https://evil.example https://youtu.be/${ID}`,
  ]) {
    for (const backend of kapyoutube.backends) {
      assert.equal(backend.applies({ url: masuk, format: "mp3" }), false, `harus ditolak: ${backend.name} / ${masuk}`);
      assert.equal(backend.applies({ url: masuk, format: "mp4" }), false, `harus ditolak: ${backend.name} / ${masuk}`);
    }
  }
});

test("short link: dua tautan memakai tautan pertama, bukan tautan paling mirip YouTube", async () => {
  // Pasangan yang urutannya dibalik tidak ditolak: aturan baca "tautan pertama
  // yang lengkap", jadi `youtu.be` di depan tetap dibaca dan `evil.example`
  // di belakang diabaikan. Yang dikunci di sini bukan penolakannya, tapi ID mana
  // yang dipakai — kalau suatu saat pemindaian semua token jadi mengizinkan
  // `youtu.be` dari tautan kedua, test ini yang menangkapnya.
  const masuk = `https://youtu.be/${ID} https://evil.example`;
  // `youtube-fallback` hanya melayani mp4, jadi backend itu ditanya dengan format
  // yang memang dilayaninya — kalau tidak, test ini menguji gate format, bukan
  // aturan tautan pertama.
  for (const backend of kapyoutube.backends) {
    const format = backend.name === "youtube-fallback" ? "mp4" : "mp3";
    assert.equal(backend.applies({ url: masuk, format }), true, `harus mau mencoba: ${backend.name}`);
  }

  PANGGILAN_AXIOS.length = 0;
  ymdcnSukses();
  await ytdlLokal.run({ url: masuk, format: "mp3" });
  const req = PANGGILAN_AXIOS.find((c) => String(c.url).includes("/api/v1/convert"));
  assert.equal(req?.opts?.params?.v, ID, "ID harus dari tautan pertama");
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

// ── gate format: `applies` adalah satu-satunya penjaga ───────────────────────
//
// `lewatAzbry` (src/capabilities/youtube.js:181) menulis path `/api/download/ytmp3`
// dan `pastikanBentuk` mengharapkan field `download` — keduanya mp3, jadi keduanya
// saling menguatkan dan `pastikanBentuk` tidak akan pernah mendeteksi kalau
// `applies` melebar ke mp4. Akibatnya `.ytmp4` akan dapat `{format:"mp3", url:<mp3>}`
// lalu dikirim `sendMedia(..., {type:"video"})` — persis "format yang berbohong"
// yang `pastikanBentuk` ada untuk mencegahnya. Jadi matriks `applies` di bawah
// yang menjaganya, dan matriks itu harus diuji.

test("format: azbry hanya mp3, dan hanya `applies` yang menjaganya", () => {
  const izuka = kapyoutube.backends.find((b) => b.name === "izuka");
  const azbry = kapyoutube.backends.find((b) => b.name === "azbry");

  assert.equal(azbry.applies({ url: KANONIK, format: "mp3" }), true);
  assert.equal(azbry.applies({ url: KANONIK }), true, "tanpa format berarti mp3");
  assert.equal(
    azbry.applies({ url: KANONIK, format: "mp4" }),
    false,
    "azbry tidak punya endpoint mp4: request mp4 akan dapat mp3 berlabel mp4",
  );
  assert.equal(
    azbry.applies({ url: KANONIK, format: "MP4" }),
    false,
    "huruf besar tidak boleh melewati gate: formatDiminta sudah menurunkannya ke mp4",
  );
  // `izuka` memang melayani dua format lewat dua path berbeda, jadi tidak punya
  // gate — itu yang membuat matriks ini harus diuji per backend, bukan per kapabilitas.
  assert.equal(izuka.applies({ url: KANONIK, format: "mp3" }), true);
  assert.equal(izuka.applies({ url: KANONIK, format: "mp4" }), true);
});

test("format: tidak ada backend mp4 yang reachable lewat aggregator selain izuka", () => {
  for (const backend of kapyoutube.backends.filter((b) => b.kind !== "local")) {
    assert.equal(
      backend.applies({ url: KANONIK, format: "mp4" }),
      backend.name === "izuka",
      `${backend.name} reachable untuk mp4?`,
    );
  }
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

// ── format yang benar-benar diterima ─────────────────────────────────────────
//
// Probe langsung pada 2026-10-03 ke `my.izuka-api.xyz/api/downloader/ytmp3`
// (lihat task-8-report.md §8.4) menjawab:
//
//   "format":"webm" ,  "download_url":"…&mime=audio%2Fwebm&itag=251…"
//
// Jadi endpoint yang namanya `ytmp3` itu menjawab WebM/Opus. `normalize` lama
// menulis `format: "mp3"` untuk hasil itu, dan plugin mengirim byte WebM sebagai
// `audio/mpeg` dengan nama `.mp3` — kelas "format yang berbohong" yang sama
// dengan yang `pastikanBentuk` ada untuk mencegahnya, hanya di sisi label.
//
// Yang diuji di sini: label mengikuti bukti. Yang tidak diuji, dan memang tidak
// boleh: menolak webm (izuka bisa selalu menjawab webm untuk endpoint ini, jadi
// menolak menghapus satu-satunya cadangan aggregator `.ytmp3`) dan mengonversi
// dengan ffmpeg (kotak 1GB, dan Phase 1 soal kecepatan).

const UNDUHAN_WEBM =
  "https://rr1---sn-cx5o4aqj5-tt1l.googlevideo.com/videoplayback?expire=1791028720&itag=251&mime=audio%2Fwebm&dur=213.061";

test("normalize: respons aggregator yang melaporkan webm keluar sebagai webm", () => {
  // Bentuk persis hasil probe: `format` di level `result`.
  const keluar = kapyoutube.normalize({
    status: true,
    result: { title: "Rick Astley", format: "webm", download_url: UNDUHAN_WEBM },
  });
  assert.equal(keluar.format, "webm", "format yang dilaporkan host harus jadi label, bukan mp3");
  assert.equal(keluar.url, UNDUHAN_WEBM);
});

test("normalize: webm bisa dibaca dari `mime` payload atau dari query URL unduhan", () => {
  // Host yang tidak menulis `format` tapi menulis MIME di level result.
  assert.equal(
    kapyoutube.normalize({ status: true, result: { download_url: UNDUHAN_MP3, mime: "audio/webm" } }).format,
    "webm",
  );
  // Host yang menulisannya sama sekali tidak ada: `googlevideo` yang dipakai
  // aggregator membawa `mime=audio%2Fwebm` di query URL-nya, dan itu bukti
  // langsung byte apa yang akan dilayani.
  assert.equal(
    kapyoutube.normalize({ status: true, result: { download_url: UNDUHAN_WEBM } }).format,
    "webm",
  );
  // Parameter codec tidak boleh menggeser token: `audio/ogg; codecs=opus` tetap ogg.
  assert.equal(
    kapyoutube.normalize({ status: true, result: { download_url: UNDUHAN_MP3, mime: "audio/ogg; codecs=opus" } })
      .format,
    "ogg",
  );
});

test("normalize: respons aggregator yang melaporkan mp3 tetap mp3", () => {
  // Penjaga arah sebaliknya: memperbaiki webm tidak boleh mengubah jalur mp3
  // yang memang mp3.
  for (const isi of [
    { download_url: UNDUHAN_MP3, format: "mp3" },
    { download_url: UNDUHAN_MP3, mime: "audio/mpeg" },
    { download_url: `${UNDUHAN_MP3}?mime=audio%2Fmpeg` },
  ]) {
    assert.equal(
      kapyoutube.normalize({ status: true, result: isi }).format,
      "mp3",
      `mp3 harus tetap mp3 untuk ${JSON.stringify(isi)}`,
    );
  }
});

test("normalize: host yang tidak melaporkan format apa pun → default mp3 yang terdokumentasi", () => {
  // Default bukan tebakan: setiap jalur audio di kapabilitas ini menghasilkan mp3
  // (`ytdl` dengan `format: "mp3"`), dan jalur video dikunci lewat filter
  // `ext: "mp4"` — bukan lewat default ini. Dipin ke `FORMAT_DEFAULT` supaya
  // kalau defaultnya diganti, test ini ikut loudly gagal.
  assert.equal(FORMAT_DEFAULT, "mp3");
  assert.equal(kapyoutube.normalize({ status: true, result: { download_url: UNDUHAN_MP3 } }).format, FORMAT_DEFAULT);
  // Bentuk lokal tanpa `format` juga tidak boleh berubah jadi apa-apa lain.
  assert.equal(kapyoutube.normalize({ status: true, title: "Lagu", dl: UNDUHAN_MP3 }).format, FORMAT_DEFAULT);
  // `360` milik SaveTube adalah nomor kualitas, bukan format: menerimanya sebagai
  // label akan membuat `<judul>.360` dan `audio/mpeg` untuk video mp4.
  assert.equal(
    kapyoutube.normalize({ status: true, title: "Video", format: "360", url: UNDUHAN_MP4 }).format,
    FORMAT_DEFAULT,
  );
});

test("backend aggregator: format yang dilaporkan host ikut ke hasil, bukan format yang dipesan", async () => {
  const izuka = kapyoutube.backends.find((b) => b.name === "izuka");
  hasilAggregator = aggregatorSukses({
    status: true,
    result: { title: "Rick Astley", format: "webm", download_url: UNDUHAN_WEBM },
  });

  const keluar = await izuka.run({ url: KANONIK, format: "mp3" });

  assert.equal(keluar.format, "webm", "backend tidak boleh menimpa format yang dilaporkan host");
  assert.equal(kapyoutube.normalize(keluar).format, "webm");
  // Jalur mp3 tetap dipakai — inilah bedanya dengan menolak: cadangan aggregator
  // tidak hilang, hanya labelnya yang jujur.
  assert.match(String(keluar.download_url), /^https:\/\/.*mime=audio%2Fwebm/);
});

test("resolver: fallback aggregator yang menjawab webm dilabeli webm, bukan mp3", async () => {
  balasYmdcn = async () => {
    throw new Error("getaddrinfo ENOTFOUND d.ymcdn.org");
  };
  hasilAggregator = aggregatorSukses({
    status: true,
    result: { title: "Rick Astley", format: "webm", download_url: UNDUHAN_WEBM },
  });

  const keluar = await resolverYoutube().resolve("youtube", { url: KANONIK, format: "mp3" });

  assert.equal(keluar.source, "izuka", "webm tidak boleh dibuang sebagai kegagalan");
  assert.equal(keluar.data.format, "webm");
  assert.equal(keluar.data.url, UNDUHAN_WEBM);
});

test("mp4: cabang `video_normal` tetap mp4 walau host melaporkan format lain", async () => {
  // `izuka` yang menjawab `ytmp4` pun bisa menulis `format: "webm"` di result-nya.
  // Yang menentukan adalah entri `ext: "mp4"` yang benar-benar dipilih, jadi
  // plugin video tidak boleh mendapat label webm untuk file mp4.
  const keluar = kapyoutube.normalize({
    status: true,
    result: {
      title: "Video Uji",
      format: "webm",
      video_normal: [{ ext: "mp4", quality: "360", url: UNDUHAN_MP4 }],
    },
  });
  assert.equal(keluar.format, "mp4");
  assert.equal(keluar.url, UNDUHAN_MP4);
});

test("media-format: pemetaan format ↔ MIME satu arah, dan defaultnya mp3/audio/mpeg", () => {
  // Ini yang dibaca ketiga plugin mp3, jadi tabelnya diuji langsung: webm harus
  // `audio/webm` (bukan `audio/mpeg`), dan ogg harus `audio/ogg` — pemutar
  // memperlakukannya berbeda, jadi keduanya tidak boleh disatukan.
  assert.equal(mimetypeFor("webm"), "audio/webm");
  assert.equal(mimetypeFor("ogg"), "audio/ogg");
  assert.equal(mimetypeFor("opus"), "audio/ogg");
  assert.equal(mimetypeFor("mp3"), "audio/mpeg");
  assert.equal(mimetypeFor("m4a"), "audio/mp4");
  assert.equal(extensionFor("webm"), "webm");
  assert.equal(extensionFor("mp3"), "mp3");

  // Nilai tak dikenal dan kosong tetap jatuh ke default, supaya plugin yang
  // menerima `format` undefined tidak berubah perilakunya.
  for (const kosong of ["", undefined, null, "360", "bukan-format"]) {
    assert.equal(mimetypeFor(kosong), "audio/mpeg", `mimetype default untuk ${JSON.stringify(kosong)}`);
    assert.equal(extensionFor(kosong), FORMAT_DEFAULT, `ekstensi default untuk ${JSON.stringify(kosong)}`);
  }

  // Arah baca: parameter MIME dibuang, huruf besar diterimap, dan nama yang bukan
  // format ditolak.
  assert.equal(formatFromMime("audio/webm"), "webm");
  assert.equal(formatFromMime("AUDIO/MPEG"), "mp3");
  assert.equal(formatFromMime("audio/ogg; codecs=opus"), "ogg");
  assert.equal(formatFromMime("application/octet-stream"), "");
  assert.equal(formatFromToken("WEBM"), "webm");
  assert.equal(formatFromToken(".mp4"), "mp4");
  assert.equal(formatFromToken("360"), "");
  assert.equal(formatFromToken(undefined), "");
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