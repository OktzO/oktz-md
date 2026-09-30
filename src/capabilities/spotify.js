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
async function lewatNexray(args = {}) {
  if (typeof args.url === "string" && POLA_URL.test(args.url.trim())) {
    const body = await aggregator.hit("nexray", "/downloader/spotify", {
      params: { url: args.url.trim() },
    });
    return body?.result;
  }
  if (typeof args.q === "string" && args.q.trim() !== "") {
    const body = await aggregator.hit("nexray", "/search/spotify", {
      params: { q: args.q.trim() },
    });
    return body?.result;
  }
  throw new Error("spotify butuh { url } atau { q }, tidak keduanya");
}

// ── normalisasi ──────────────────────────────────────────────────────────────

// Nama field agregator lama berbeda dari scraper lokal, jadi semuanya dibaca di
// sini. Tanpa itu, pindah backend hanya diam-diam jadi "respons tanpa URL".
function namaArtis(entri) {
  if (typeof entri.artist === "string") return entri.artist.trim();
  if (entri.artist?.name) return String(entri.artist.name).trim();
  if (Array.isArray(entri.artists)) {
    return entri.artists
      .map((a) => (typeof a === "string" ? a : a?.name ?? ""))
      .filter(Boolean)
      .join(", ")
      .trim();
  }
  return "";
}

function urlCover(entri) {
  if (typeof entri.cover === "string" && entri.cover) return entri.cover;
  if (typeof entri.thumbnail === "string" && entri.thumbnail) return entri.thumbnail;
  if (Array.isArray(entri.thumbnails)) {
    const terakhir = entri.thumbnails[entri.thumbnails.length - 1];
    if (typeof terakhir === "string" && terakhir) return terakhir;
    if (terakhir?.url) return String(terakhir.url);
  }
  return "";
}

// Plugin mencetak nilai ini apa adanya, jadi durasi yang hilang harus tetap
// string: "undefined" di daftar hasil terlihat seperti bug.
function durasi(entri) {
  if (typeof entri.duration === "string" && entri.duration) return entri.duration;
  if (typeof entri.duration === "number" && entri.duration > 0) {
    const m = Math.floor(entri.duration / 60);
    const s = Math.floor(entri.duration % 60);
    return `${m}:${s < 10 ? `0${s}` : s}`;
  }
  return "0:00";
}

function trackCari(entri) {
  if (!entri || typeof entri !== "object") return null;
  const judul = String(entri.title ?? entri.name ?? "").trim();
  const url = String(entri.url ?? "").trim();

  // Hasil pencarian dicetak sebagai daftar lagu, jadi entri tanpa judul atau
  // tanpa tautan tidak bisa diidentifikasi dan tidak bisa diunduh. Ditolak di
  // sini, bukan ditampilkan: normalize adalah pintu terakhir sebelum respons
  // sampai ke user.
  if (!judul || !url) return null;
  return {
    title: judul,
    artist: namaArtis(entri),
    url,
    cover: urlCover(entri),
    duration: durasi(entri),
  };
}

function trackUnduh(entri) {
  const url = String(entri.url ?? entri.download_url ?? entri.downloadLink ?? "").trim();
  if (!/^https?:\/\//i.test(url)) {
    throw new Error(
      `spotify: respons tidak punya URL unduhan (field: ${Object.keys(entri).join(", ") || "kosong"})`,
    );
  }
  return {
    title: String(entri.title ?? entri.name ?? "").trim(),
    artist: namaArtis(entri),
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
  { name: "spotyloader", kind: "local", run: lewatSpotyloader },
  // `bertingkat` di resolve.js:98 adalah partisi yang stabil — lokal selalu
  // lebih dulu, dan urutan array menentukan siapa yang mencoba duluan di dalam
  // tier itu. Yang dipegang di sini bukan posisinya melainkan nama backend:
  // satu nama per host, tidak pernah diturunkan dari argumen.
  { name: "nexray", kind: "api", run: lewatNexray },
];