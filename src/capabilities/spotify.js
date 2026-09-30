import { downloadSpotify } from "../scraper/spotify.js";
import { aggregator } from "../lib/aggregator.js";

// Judul dan metadata Spotify berubah, dan URL unduhan spotyloader kedaluwarsa
// dalam hitungan menit. Cache resolver hanya hidup untuk `stable: true`, jadi
// kapabilitas ini selalu menembak backend sungguhan.
export const stable = false;

// Album dan playlist ikut diterima: spotyloader mengambil track pertama dari
// keduanya, dan menolak di sini membuang permintaan yang mungkin berhasil.
const POLA_URL = /^https?:\/\/open\.spotify\.com\/(track|album|playlist)\//i;

async function lewatSpotyloader({ url } = {}, ctx = {}) {
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("spotify butuh { url }: link track/album/playlist dari open.spotify.com");
  }
  const bersih = url.trim();
  // Guard di dalam backend, sebelum scraper: input salah harus berhenti di
  // sini, bukan jadi satu request HTTP yang pasti ditolak upstream.
  if (!POLA_URL.test(bersih)) {
    throw new Error(
      `format link Spotify tidak dikenali: ${bersih} — yang diterima https://open.spotify.com/track/... , /album/... , atau /playlist/...`,
    );
  }

  try {
    return await downloadSpotify(bersih, { signal: ctx?.signal });
  } catch (error) {
    // Tiga kelas kegagalan lokal tidak sama akibatnya untuk backend cadangan.
    // `status` terisi berarti host menjawab dan permintaannya yang ditolak;
    // `cause` tanpa `status` berarti tidak ada respons sama sekali (DNS atau
    // tunnel mati, bukan upstream 500); tanpa keduanya berarti host sehat tetapi
    // track-nya tidak ada di sana — kelas terakhir justru yang paling mungkin
    // punya jalan keluar di host lain. Bedakan dengan `typeof status === "number"`
    // persis seperti src/lib/serialize.js:560.
    const kelas =
      typeof error.status === "number"
        ? "ditolak upstream"
        : error.cause
          ? "tidak terjangkau"
          : "track tidak ada di sana";
    const gagal = new Error(`spotyloader ${kelas}: ${error.message}`, { cause: error });
    if (typeof error.status === "number") gagal.status = error.status;
    throw gagal;
  }
}

// Pencarian tidak punya backend lokal: spotyloader menerima URL, bukan kata
// kunci, dan tidak ada scraper lokal lain di repo ini yang bisa mengubah judul
// lagu menjadi ID track Spotify. Jadi `{ q }` hanya dilayani aggregator.
async function lewatNexray(args = {}, ctx = {}) {
  const signal = ctx?.signal;
  if (typeof args.url === "string" && POLA_URL.test(args.url.trim())) {
    const body = await aggregator.hit("nexray", "/downloader/spotify", {
      params: { url: args.url.trim() },
      signal,
    });
    return body?.result;
  }
  if (typeof args.q === "string" && args.q.trim() !== "") {
    const body = await aggregator.hit("nexray", "/search/spotify", {
      params: { q: args.q.trim() },
      signal,
    });
    return body?.result;
  }
  throw new Error("spotify butuh { url } atau { q }, tidak keduanya");
}

// Dua backend ini hanya melayani bentuk argumen yang berbeda, jadi "tidak
// berlaku" adalah jawaban yang benar — bukan kegagalan host.
const punyaUrl = (args) => typeof args?.url === "string" && args.url.trim() !== "";
const punyaQuery = (args) => typeof args?.q === "string" && args.q.trim() !== "";

// ── normalisasi ──────────────────────────────────────────────────────────────

// Nama field yang dipakai aggregator tidak sama dengan yang dipakai scraper
// lokal, dan `normalize` adalah satu-satunya tempat yang boleh menyamakan
// keduanya: kalau tidak, pindah backend hanya diam-diam jadi "respons kosong".
// Bentuk yang dibaca di sini adalah hasil yang benar-benar dikirim host:
// `artist`, `thumbnail`, dan `duration` selalu string. Backend `ytmusic` yang
// pernah mengembalikan bentuk lain sudah dicabut, jadi tidak ada lagi pembacaan
// `artists: [{name}]`, `thumbnails: [{url}]`, atau durasi numerik.

function trackCari(entri) {
  if (!entri || typeof entri !== "object") return null;
  const judul = String(entri.title ?? "").trim();
  const url = String(entri.url ?? "").trim();

  // Hasil pencarian dicetak sebagai daftar lagu, jadi entri tanpa judul atau
  // tanpa tautan tidak bisa diidentifikasi dan tidak bisa diunduh. Ditolak di
  // sini, bukan ditampilkan: normalize adalah pintu terakhir sebelum respons
  // sampai ke user.
  if (!judul || !url) return null;
  return {
    title: judul,
    artist: String(entri.artist ?? "").trim(),
    url,
    cover: String(entri.thumbnail ?? "").trim(),
    // Plugin mencetak nilai ini apa adanya, jadi durasi yang hilang harus tetap
    // string: "undefined" di daftar hasil terlihat seperti bug.
    duration: typeof entri.duration === "string" && entri.duration ? entri.duration : "0:00",
  };
}

function trackUnduh(entri) {
  // `download_url` dipakai agregator yang menamai field unduhannya sendiri,
  // `url`/`mime` dipakai spotyloader dan nexray.
  const url = String(entri.url ?? entri.download_url ?? "").trim();
  if (!/^https?:\/\//i.test(url)) {
    throw new Error(
      `spotify: respons tidak punya URL unduhan (field: ${Object.keys(entri).join(", ") || "kosong"})`,
    );
  }
  return {
    title: String(entri.title ?? "").trim(),
    artist: String(entri.artist ?? "").trim(),
    url,
    mime: String(entri.mime ?? entri.mimetype ?? "").trim(),
  };
}

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons spotify tidak dikenali");
  }

  const daftar = Array.isArray(raw) ? raw : raw.tracks;
  if (Array.isArray(daftar)) {
    return { tracks: daftar.map(trackCari).filter(Boolean) };
  }

  return trackUnduh(raw);
}

export const backends = [
  {
    name: "spotyloader",
    kind: "local",
    // Tanpa `applies` di sini, setiap `.spotify` dihitung sebagai kegagalan
    // spotyloader oleh breaker, dan tiga pencarian sudah cukup untuk mengeluarkan
    // host yang sehat dari rotasi padahal tidak pernah dihubungi.
    applies: punyaUrl,
    run: lewatSpotyloader,
  },
  {
    name: "nexray",
    kind: "api",
    applies: (args) => punyaUrl(args) || punyaQuery(args),
    run: lewatNexray,
  },
];