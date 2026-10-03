import { ytdl } from "../scraper/ytdl.js";
import { getYoutubeDirectUrl } from "../scraper/youtube.js";
import { aggregator } from "../lib/aggregator.js";

// Tautan unduhan YouTube berumur pendek dan videonya bisa dihapus kapan saja,
// jadi hasil scraping tidak boleh keluar dari cache meski tautannya sama.
export const stable = false;

// ── bentuk URL ────────────────────────────────────────────────────────────────
//
// User WhatsApp hampir selalu mengirim short link: `youtu.be/…`, `m.youtube.com`,
// `music.youtube.com`, atau `youtube.com/shorts/…`. Semuanya adalah video yang
// sama, jadi semua dinormalkan ke satu bentuk kanonik sebelum masuk scraper atau
// aggregator. `SaveTube.download` (src/scraper/youtube.js:52) sudah menyodorkan
// bentuk kanonik ke hostnya, tapi `extractVideoId` di src/scraper/ytdl.js:6 hanya
// bisa membaca sebagian dari bentuk itu — `/shorts/<id>` dan `/live/<id>` tidak
// terbaca sama sekali, dan `ytdl` menjawab `{ status: false }` untuk keduanya
// tanpa satu pun request. `m.` dan `music.` kebetulan ikut terbaca karena regex-nya
// tidak dianchor, bukan karena sengaja keduanya. Menormalkan di sini juga berarti
// aggregator menerima URL yang pasti bisa dibaca, bukan bentuk yang harus ditebak
// ulang di host lain.
//
// Host dicocokkan dari batas label, bukan dengan `includes`: `youtube.com.evil.example`
// adalah domain lain yang tetap memuat "youtube.com", dan meneruskannya ke scraper
// berarti satu polling konversi yang sia-sia plus satu kegagalan breaker untuk semua
// orang. `music.youtube.com` ikut diterima karena itu sub-domain youtube.com.
const POLA_ID = /^[A-Za-z0-9_-]{11}$/;
const JALUR_LANGKAS = new Set(["shorts", "live", "embed", "v"]);

// `new URL` menolak dua bentuk yang justru paling sering diketik user: tautan
// tanpa skema (`youtu.be/…`) dan tautan yang disisipkan di dalam kalimat
// ("putar https://youtu.be/… dong"). Keduanya bekerja sebelum Phase 1 — guard
// plugin lama hanya `url.includes("youtu.be")` dan `YOUTUBE_ID_REGEX` tidak
// dianchor, jadi menarik ID dari tengah string — jadi menolaknya sekarang adalah
// penyempitan yang tidak diminta dan tidak diinginkan.
const TAUTAN_LENGKAP = /\bhttps?:\/\/[^\s<>"']+/i;
const ADA_SKEMA = /^[a-z][a-z0-9+.-]*:/i;

/**
 * Bentuk yang mungkin tautan, urut dari paling mungkin sampai fallback.
 *
 * Pemotongan dari dalam kalimat hanya dilakukan kalau user menulis skema yang
 * lengkap, dan potongan pertamanya dirakim sampai whitespace: itu yang membuat
 * `https://evil.example/redirect?to=https://youtu.be/<id>` terambil utuh dan
 * ditolak sebagai host `evil.example`, bukan dipungut `youtu.be` dari query
 * string-nya. Tanpa skema, kandidat diambil dari token yang memuat "youtu" —
 * polanya sama dengan `YOUTUBE_ID_REGEX` yang lama.
 */
function kandidatTautan(url) {
  const mentah = String(url ?? "").trim();
  const lengkap = TAUTAN_LENGKAP.exec(mentah)?.[0];
  const dasar = lengkap ?? mentah.split(/\s+/).find((token) => /youtu/i.test(token)) ?? mentah;
  // Skema hanya dipulihkan kalau memang tidak ada token skema sama sekali, jadi
  // `ftp://youtube.com/…` dan `javascript:…` tetap sampai ke pemeriksaan protocol
  // dan ditolak di sana — bukan diam-diam jadi request https.
  return [dasar, mentah]
    .filter((nilai, i, semua) => nilai && semua.indexOf(nilai) === i)
    .map((nilai) => (ADA_SKEMA.test(nilai) ? nilai : `https://${nilai}`));
}

function parseTautan(nilai) {
  try {
    const parsed = new URL(nilai);
    return parsed.protocol === "http:" || parsed.protocol === "https:" ? parsed : null;
  } catch {
    return null;
  }
}

function hostYoutube(parsed) {
  const host = parsed.hostname.toLowerCase();
  return host === "youtu.be" || host === "youtube.com" || host.endsWith(".youtube.com");
}

/** ID video dari satu URL yang sudah pasti host-nya YouTube. */
function idDariTautan(parsed) {
  // Short link resmi: seluruh path adalah ID-nya.
  if (parsed.hostname.toLowerCase() === "youtu.be") {
    const kandidat = parsed.pathname.replace(/^\/+/, "").split("/")[0];
    return POLA_ID.test(kandidat) ? kandidat : null;
  }

  // `watch?v=…` dengan parameter lain di sekitarnya: `?app=desktop&v=…&t=1s`.
  const dariQuery = parsed.searchParams.get("v");
  if (dariQuery && POLA_ID.test(dariQuery)) return dariQuery;

  // Bentuk berbagi yang lain. Semuanya dipetakan ke `watch?v=<id>`, dan itu
  // bukan sekadar formalitas: `SaveTube` hanya menerima daftar kualitas
  // tertentu, jadi quality string bukan tautan yang bisa dipakai.
  const segmen = parsed.pathname.replace(/^\/+|\/+$/g, "").split("/");
  if (segmen.length >= 2 && JALUR_LANGKAS.has(segmen[0]) && POLA_ID.test(segmen[1])) {
    return segmen[1];
  }
  return null;
}

/**
 * ID video dari URL YouTube, atau `null` kalau URL itu bukan tautan video.
 *
 * `null` berarti tidak ada yang bisa diprobe — jadi pemanggil boleh berhenti
 * sebelum satu pun request, bukan menunggu upstream menolak.
 */
function idVideo(url) {
  for (const nilai of kandidatTautan(url)) {
    const parsed = parseTautan(nilai);
    if (!parsed || !hostYoutube(parsed)) continue;
    const id = idDariTautan(parsed);
    if (id) return id;
  }
  return null;
}

function kanonikDari(id) {
  return `https://www.youtube.com/watch?v=${id}`;
}

// `applies` dan `run` memakai helper yang sama supaya guard di dalam backend
// tidak bergantung pada kenyataan bahwa ia hanya dipanggil dari resolver.
function target(args) {
  if (typeof args?.url !== "string" || args.url.trim() === "") {
    throw new Error("youtube butuh { url }: link video dari youtube.com atau youtu.be");
  }
  const id = idVideo(args.url);
  if (!id) {
    throw new Error(
      `youtube: format link YouTube tidak dikenali: ${args.url.trim()} — yang diterima https://www.youtube.com/watch?v=<id>, https://youtu.be/<id>, atau https://www.youtube.com/shorts/<id>`,
    );
  }
  return kanonikDari(id);
}

// Dua format, dan hanya dua: plugin menyebut `format`, dan scraper hanya menerima
// mp3 atau mp4. Aturan yang sama dipakai `ytdl` sendiri di
// src/scraper/ytdl.js:32, jadi bentuk lain tidak pernah diam-diam jadi mp3.
function formatDiminta(args) {
  return String(args?.format ?? "").toLowerCase() === "mp4" ? "mp4" : "mp3";
}

// ── backend ──────────────────────────────────────────────────────────────────

async function lewatYtdlNative(args = {}, _) {
  const kanonik = target(args);
  // `ytdl` tidak melempar: semua kegagalan dikembalikan sebagai
  // `{ status: false, mess }` (src/scraper/ytdl.js:106), termasuk timeout polling
  // di :97. Bentuk itu diteruskan apa adanya supaya `normalize` yang memeriksa
  // `status`. Parameter kedua dibiarkan kosong: `ytdl` hanya menerima `(url, format)`,
  // jadi `ctx.signal` tidak punya tempat masuk — lihat catatan di
  // `lewatYoutubeFallback` untuk konsekuensinya ke budget resolver.
  const format = formatDiminta(args);
  return { ...(await ytdl(kanonik, format)), format };
}

async function lewatYoutubeFallback(args = {}, _) {
  const kanonik = target(args);
  // `getYoutubeDirectUrl` dan bukan `downloadWithFallback`: `SaveTube.download`
  // menolak format apa pun di luar daftar kualitasnya
  // (src/scraper/youtube.js:55, dan daftar itu tidak memuat "mp4"), sehingga
  // `downloadWithFallback(url, "mp4")` selalu jatuh ke QByte — yang mengunduh
  // video dan audio ke file sementara, menyatukannya dengan ffmpeg, lalu
  // membaca seluruh hasilnya ke satu Buffer di memori tanpa pernah mengembalikan
  // URL. Pada kotak 1GB itu berisiko OOM, dan `sendMedia(chat, data.url)` tetap
  // tidak punya apa-apa untuk dikirim. `getYoutubeDirectUrl` mencoba beberapa
  // kualitas di jalur yang sama dan selalu mengembalikan URL.
  //
  // `ctx.signal` juga tidak bisa diteruskan ke `st.download` (tanda tangannya
  // `(url, format)`), jadi budget resolver menghentikan resolver, bukan request
  // yang sudah jalan. Request orphan-nya dibatasi timeout 15-30 detik milik
  // SaveTube sendiri.
  const hasil = await getYoutubeDirectUrl(kanonik, "360");
  // `SaveTube` melaporkan `format` sebagai nomor kualitas ("360"), bukan sebagai
  // format kapabilitas, jadi yang diteruskan adalah format yang dipesan.
  return { ...hasil, format: "mp4" };
}

/**
 * Bentuk respons host tidak pernah menyebut format yang diminta: `izuka` menjawab
 * `result.download_url` untuk mp3 dan `result.video_normal` untuk mp4, dan
 * `azbry` menjawab `result.download` untuk mp3. `normalize` hanya melihat respons
 * dan tidak tahu apa yang dipesan, jadi backend yang memeriksa kesesuaiannya —
 * bentuk yang meleset harus jadi kegagalan, bukan `format: "mp4"` yang isinya
 * audio.
 */
function pastikanBentuk(result, format, namaField) {
  if (!result || typeof result !== "object") {
    throw new Error(`youtube: aggregator membalas tanpa result (${namaField} tidak terbaca)`);
  }
  const ada = Array.isArray(result[namaField]) ? result[namaField].length > 0 : Boolean(result[namaField]);
  if (!ada) {
    throw new Error(
      `youtube: aggregator menjawab tanpa ${namaField} untuk permintaan ${format} — bentuk endpoint tidak cocok`,
    );
  }
  return { ...result, format };
}

async function lewatIzuka(args = {}, ctx = {}) {
  const kanonik = target(args);
  const format = formatDiminta(args);
  // `AGGREGATORS.izuka.base` hanya berisi host dan `aggregator.hit` menempelkan
  // path apa adanya, jadi `/api` ditulis eksplisit: tanpa itu host menjawab 404
  // dan cadangan ini justru jadi sumber utama kegagalan.
  const body = await aggregator.hit("izuka", format === "mp4" ? "/api/downloader/ytmp4" : "/api/downloader/ytmp3", {
    params: { url: kanonik },
    signal: ctx?.signal,
  });
  // Dua jalur lama tidak sama-sama memeriksa `status`, jadi bedanya harus jujur:
  // `ytmp4` lama menuntut `data.status` sebelum membaca `result.video_normal`,
  // tapi `ytmp3` lama sama sekali tidak memeriksa `status` — ia langsung memakai
  // `result.download_url` apa adanya. Untuk mp3 jalur ini jadi lebih ketat dari
  // sebelumnya, dan itu memang disengaja: amplop yang menandai gagal tidak boleh
  // diperlakukan sebagai unduhan siap kirim.
  //
  // `!== true`, bukan "status yang tidak kosong": probe langsung pada 2026-10-03
  // ke `my.izuka-api.xyz/api/downloader/ytmp3` menjawab `"status": true` sebagai
  // boolean, jadi bentuk `1` atau `"true"` tidak pernah muncul di endpoint ini.
  // Bentuk yang paling berbahaya justru yang paling ketat lolosnya: host yang
  // hidup tapi tidak mengirim apa pun.
  if (body?.status !== true) {
    throw new Error(`youtube: aggregator izuka menandai gagal — ${String(body?.msg ?? body?.error ?? "tanpa alasan")}`);
  }
  return pastikanBentuk(body.result, format, format === "mp4" ? "video_normal" : "download_url");
}

async function lewatAzbry(args = {}, ctx = {}) {
  const kanonik = target(args);
  const body = await aggregator.hit("azbry", "/api/download/ytmp3", {
    params: { url: kanonik },
    signal: ctx?.signal,
  });
  // Bentuk `!== true` di sini lebih ketat dari `play`/`playch` lama yang hanya
  // menuntut `status` tidak kosong, dan tidak ada satu pun respons azbry yang
  // tersimpan di repo ini untuk membuktikannya — jadi ini belum diuji terhadap
  // hostnya, hanya disamakan dengan izuka yang sudah diprobe.
  if (body?.status !== true) {
    throw new Error(`youtube: aggregator azbry menandai gagal — ${String(body?.msg ?? body?.error ?? "tanpa alasan")}`);
  }
  return pastikanBentuk(body.result, "mp3", "download");
}

// ── normalisasi ───────────────────────────────────────────────────────────────
//
// Empat backend, lima nama field untuk hal yang sama: `dl` (ytdl), `url` (SaveTube),
// `download_url` (izuka mp3), `download` (azbry), dan `video_normal[].url` (izuka
// mp4). Thumbnail punya masalah ketiga: hanya SaveTube yang mengirimnya, jadi field
// itu akan kosong untuk tiga backend lain. `normalize` adalah satu-satunya tempat
// yang boleh menyamakannya.

function urlUnduhan(isi) {
  for (const nama of ["dl", "download_url", "download"]) {
    const kandidat = String(isi?.[nama] ?? "").trim();
    if (/^https?:\/\//i.test(kandidat)) return kandidat;
  }
  if (Array.isArray(isi?.video_normal)) {
    // Kontrak host yang sudah dipakai plugin sebelum Phase 1: hanya `ext: "mp4"`,
    // kualitas tertinggi lebih dulu.
    const terpilih = isi.video_normal
      .filter((v) => v?.ext === "mp4")
      .sort((a, b) => (parseInt(b?.quality) || 0) - (parseInt(a?.quality) || 0))[0];
    const kandidat = String(terpilih?.url ?? "").trim();
    if (/^https?:\/\//i.test(kandidat)) return kandidat;
  }
  const langsung = String(isi?.url ?? "").trim();
  return /^https?:\/\//i.test(langsung) ? langsung : "";
}

function urlGambar(nilai) {
  const url = String(nilai ?? "").trim();
  return /^https?:\/\//i.test(url) ? url : "";
}

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons youtube tidak dikenali");
  }

  // Bentuk lokal dari `ytdl` membawa `status`-nya sendiri, dan kedua aggregator
  // membungkus hasil di `{ status, result }`. Pemeriksaan ditulis eksplisit —
  // `!raw.status` akan salah memblokir bentuk SaveTube yang tidak punya field itu.
  if ("status" in raw && raw.status !== true) {
    throw new Error(
      `youtube: backend menandai gagal — ${String(raw.mess ?? raw.msg ?? raw.error ?? "tanpa alasan")}`,
    );
  }

  // Amplop aggregator dibuka di sini, bukan di backend, supaya semua bentuk punya
  // satu tempat yang menyamakan. `result: []` adalah kegagalan bentuk, bukan
  // daftar format yang sah.
  const isi =
    raw.result && typeof raw.result === "object" && !Array.isArray(raw.result) ? raw.result : raw;

  const url = urlUnduhan(isi);
  // Sama seperti bentuk videy/sfile: backend yang tidak menghasilkan apa pun yang
  // bisa dikirim harus jadi kegagalan. Kalau diteruskan, plugin menjalankan
  // `sendMedia` tanpa file lalu tetap memberi centang hijau ke user.
  if (!url) {
    throw new Error(
      `youtube: respons tanpa URL unduhan yang bisa dikirim (${Object.keys(isi).join(", ") || "kosong"})`,
    );
  }

  return {
    title: String(isi.title ?? "").trim(),
    // Tidak ada respons aggregator yang tercatat di repo ini yang membawa
    // thumbnail, jadi hanya bentuk lokal (SaveTube, `thumb`) yang bisa mengisi.
    // Tidak ada plugin yang mengonsumsi field ini; ia ada karena kontrak
    // kapabilitas menyebutkan empat field.
    thumbnail: urlGambar(isi.thumbnail ?? isi.thumb),
    // Aturan yang sama dengan `ytdl` (src/scraper/ytdl.js:32): apa pun yang bukan
    // mp4 dianggap mp3. Backend sudah menandai format yang dipesan.
    format: String(isi.format ?? "").toLowerCase() === "mp4" ? "mp4" : "mp3",
    url,
  };
}

// ── backend ──────────────────────────────────────────────────────────────────
//
// Kapabilitas ini melayani satu bentuk argumen saja (`{ url, format }`), jadi
// `applies` di sini bukan untuk membedakan tier. Gunanya: aggregator tidak boleh
// dihubungi untuk tautan yang sudah ditolak guard, dan backend lokal tidak
// menghabiskan satu slot breaker untuk `vimeo.com/12345` yang memang tidak punya
// ID video untuk diprobe. Hasil `false` tidak dihitung sebagai kegagalan dan
// tidak menyentuh breaker.
//
// `ytdl-native` dan `youtube-fallback` terpisah karena format yang mereka proses
// benar-benar berbeda: scraper pertama mengonversi ke mp3, scraper kedua meminta URL
// video. Satu backend yang bercabang di dalam `run` akan memakai satu slot breaker
// untuk dua kegagalan yang tidak berkaitan.
export const backends = [
  {
    name: "ytdl-native",
    kind: "local",
    applies: (args) => idVideo(args?.url) !== null && formatDiminta(args) === "mp3",
    run: lewatYtdlNative,
  },
  {
    name: "youtube-fallback",
    kind: "local",
    applies: (args) => idVideo(args?.url) !== null && formatDiminta(args) === "mp4",
    run: lewatYoutubeFallback,
  },
  {
    name: "izuka",
    kind: "api",
    applies: (args) => idVideo(args?.url) !== null,
    run: lewatIzuka,
  },
  {
    name: "azbry",
    kind: "api",
    applies: (args) => idVideo(args?.url) !== null && formatDiminta(args) === "mp3",
    run: lewatAzbry,
  },
];