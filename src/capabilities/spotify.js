import { downloadSpotify } from "../scraper/spotify.js";
import { createAggregatorClient } from "../lib/aggregator.js";

// Judul dan metadata Spotify berubah, dan URL unduhan spotyloader kedaluwarsa
// dalam hitungan menit. Cache resolver hanya hidup untuk `stable: true`, jadi
// kapabilitas ini selalu menembak backend sungguhan.
export const stable = false;

// Album dan playlist ikut diterima: spotyloader mengambil track pertama dari
// keduanya, dan menolak di sini membuang permintaan yang mungkin berhasil.
const POLA_URL = /^https?:\/\/open\.spotify\.com\/(track|album|playlist)\//i;

// Nama field aggregator lama berbeda dari scraper lokal, jadi keduanya dibaca.
// Kalau tidak, pindah backend hanya diam-diam jadi "respons tanpa URL".
const POLA_VIDEO_ID = /^[A-Za-z0-9_-]{11}$/;

// Klien tunggal: `hit` tidak menyimpan state per request, jadi satu instance
// cukup dan tidak menambah memori seiring pemakaian. Pintu satu-satunya ke
// domain agregator — plugin tidak boleh punya akses langsung.
const agregator = createAggregatorClient();

// ytmusic-api ditahan sebagai satu instance, bukan dibuat per panggilan:
// initialize() mengambil konteks Innertube dari jaringan dan mengulangi
// panggilan itu untuk setiap `.spotify` akan menambah satu round-trip di depan
// hasil yang sudah ada. Modulnya sendiri dimuat saat pertama dipakai — `.spdl`
// hanya butuh unduhan dan tidak boleh membayar ~40MB YT Music di kotak 1GB.
let ytmusic = null;

async function instanceYTMusic() {
  if (!ytmusic) {
    const { default: YTMusic } = await import("ytmusic-api");
    const instance = new YTMusic();
    await instance.initialize();
    // Baru disimpan setelah initialize() berhasil, kalau tidak instance rusak
    // akan dipakai selamanya tanpa pernah mencoba initialize lagi.
    ytmusic = instance;
  }
  return ytmusic;
}

// search() mengembalikan daftar campur — entri ARTIST dan PLAYLIST datang lebih
// dulu dan tidak punya videoId, jadi tidak bisa dipakai sebagai lagu.
function adalahLagu(entri) {
  return Boolean(entri?.videoId) || Boolean(entri?.url);
}

async function lewatSpotyloader({ url } = {}) {
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
    return await downloadSpotify(bersih);
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

async function lewatYTMusic({ q } = {}) {
  if (typeof q !== "string" || q.trim() === "") {
    throw new Error("spotify butuh { q }: kata kunci pencarian");
  }

  const ytm = await instanceYTMusic();
  const hasil = await ytm.search(q.trim());
  const lagu = Array.isArray(hasil) ? hasil.filter(adalahLagu) : [];
  if (!lagu.length) {
    // Nol hasil itu kegagalan, bukan jawaban kosong. Backend yang hidup tapi
    // tidak punya apa-apa harus melepas ke backend lain, bukan menutup
    // pencarian dengan daftar kosong.
    throw new Error("ytmusic tidak menemukan lagu untuk kueri ini");
  }
  return { tracks: lagu };
}

async function lewatNexray(args = {}) {
  if (typeof args.url === "string" && POLA_URL.test(args.url.trim())) {
    const body = await agregator.hit("nexray", "/downloader/spotify", {
      params: { url: args.url.trim() },
    });
    return body?.result;
  }
  if (typeof args.q === "string" && args.q.trim() !== "") {
    const body = await agregator.hit("nexray", "/search/spotify", {
      params: { q: args.q.trim() },
    });
    return body?.result;
  }
  throw new Error("spotify butuh { url } atau { q }, tidak keduanya");
}

// ── normalisasi ──────────────────────────────────────────────────────────────

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

// YT Music memberi detik, agregator memberi teks "m:ss". Plugin mencetak apa
// adanya ke daftar hasil, jadi kedua bentuk diseragamkan di sini supaya
// "⏱️ undefined" tidak pernah muncul ke user.
function durasi(entri) {
  const detik = Number(entri.duration_seconds ?? entri.seconds);
  if (Number.isFinite(detik) && detik > 0) {
    const m = Math.floor(detik / 60);
    const s = Math.floor(detik % 60);
    return `${m}:${s < 10 ? `0${s}` : s}`;
  }
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
  const videoId = String(entri.videoId ?? "").trim();

  // Hasil YT Music tidak punya ID track Spotify, dan mengarang ID berarti
  // mengirim tautan rusak ke user. Satu-satunya tautan yang bisa dibuka
  // dari sana adalah video YouTube-nya.
  let url = String(entri.url ?? "").trim();
  if (!url && POLA_VIDEO_ID.test(videoId)) url = `https://www.youtube.com/watch?v=${videoId}`;

  // Tanpa tautan, entri tidak bisa dipakai user apa pun: tidak bisa
  // diunduh dan tidak bisa dibuka. Syarat yang sama dipegang backend
  // lewatYTMusic, tapi diulang di sini karena normalize adalah pintu terakhir
  // sebelum respons tampil ke user — backend mana pun bisa mengembalikan
  // daftar campuran.
  if (!url) return null;
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
  // Urutan di dalam tier lokal tidak menentukan apa pun — resolver menyusun
  // ulang sendiri. Yang penting jumlah slot breaker: satu nama per host.
  { name: "spotyloader", kind: "local", run: lewatSpotyloader },
  { name: "ytmusic", kind: "local", run: lewatYTMusic },
  // Aggregator gratis adalah cadangan terakhir: sering mati, dan tidak pernah
  // jadi pilihan pertama walau posisinya di array paling atas.
  { name: "nexray", kind: "api", run: lewatNexray },
];