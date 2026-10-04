import YTMusic from "ytmusic-api";
import { aggregator } from "../lib/aggregator.js";

// Hasil pencarian YT Music berubah beberapa kali dalam sehari dan judul lagu
// yang sama bisa kembali dengan `videoId` berbeda, jadi tidak ada yang boleh
// keluar dari cache.
export const stable = false;

// ── instance YT Music ─────────────────────────────────────────────────────────
//
// `ytmusic-api` butuh `initialize()` setelah `new YTMusic()` sebelum `search()`
// boleh dipakai, dan inisialisasi itu menembak jaringan. Karena itu instance
// dibuat malas: mengimpor modul ini — yang `resolve.js` lakukan lewat import()
// dinamis — tidak boleh membayar koneksi yang mungkin tidak pernah dipakai.
//
// Promise inisialisasi disimpan terpisah dari instance supaya dua pemanggil yang
// datang bersamaan menunggu inisialisasi yang sama. Pola `play2.js:221-229`
// menaruh instance sebelum `await initialize()`, sehingga pemanggil kedua bisa
// menerima instance yang belum siap dan langsung gagal; di sini `instance` baru
// diisi setelah `initialize()` benar-benar selesai.
let instance = null;
let inisialisasi = null;

function getYTMusic() {
  if (instance) return Promise.resolve(instance);
  if (!inisialisasi) {
    const baru = new YTMusic();
    inisialisasi = Promise.resolve(baru.initialize()).then(
      () => {
        instance = baru;
        return baru;
      },
      (error) => {
        // Instance yang gagal di-inisialisasi tidak disimpan: menyimpannya berarti
        // pemanggilan berikutnya menerima instance yang belum siap selamanya.
        inisialisasi = null;
        throw error;
      },
    );
  }
  return inisialisasi;
}

async function lewatYtmusic({ q } = {}) {
  const query = typeof q === "string" ? q.trim() : "";
  if (!query) {
    throw new Error("ytmusic butuh { q }: kata kunci pencarian");
  }
  const ytm = await getYTMusic();
  // Daftar mentahnya diteruskan apa adanya. Penyaringan entri ARTIST/PLAYLIST
  // dilakukan di `normalize`, bukan di sini, supaya penyaringan itu ikut ditebak
  // test dan berlaku untuk bentuk aggregator juga.
  return await ytm.search(query);
}

async function lewatNexray({ q } = {}, ctx = {}) {
  if (typeof q !== "string" || q.trim() === "") {
    throw new Error("ytmusic butuh { q }: kata kunci pencarian");
  }
  // `AGGREGATORS.nexray.base` hanya berisi host dan `aggregator.hit` menempelkan
  // path apa adanya. nexray satu-satunya host di sana yang tidak memakai prefix
  // `/api`: path lama di plugin ini juga `/search/applemusic` tanpa `/api`, dan
  // probe langsung pada 2026-10-03 ke `api.nexray.eu.cc/search/applemusic`
  // menjawab 200 dengan `result` berisi link — menambahkan `/api` akan mengubahnya
  // jadi 404 dan membuat cadangan ini selalu gagal.
  //
  // Body lengkap dikembalikan apa adanya, bukan `body.result`: probe yang sama
  // membuktikan amplopnya `{ status, author, result }`, dan `status` satu level di
  // atas `result`. Kalau amplop dibuang di sini, respons
  // `{ status: false, result: [ … ] }` akan lolos ke `normalize` sebagai daftar
  // lagu yang sah.
  return await aggregator.hit("nexray", "/search/applemusic", {
    params: { q: q.trim() },
    signal: ctx?.signal,
  });
}

const punyaQuery = (args) => typeof args?.q === "string" && args.q.trim() !== "";

// ── normalisasi ──────────────────────────────────────────────────────────────
//
// Dua backend menamai lima hal yang sama dengan dua kosakula berbeda, jadi
// `normalize` adalah satu-satunya tempat yang boleh menyamakannya:
//
//   bentuk lokal (ytmusic-api)          bentuk agregator (nexray)
//   name | title             → title    title
//   artists[] | artist.name  → artist   subtitle   ("Song · Rex Orange County")
//   duration (detik)         → durationSec  — tidak ada, jadi 0
//   thumbnails[last].url     → cover     image
//   videoId                  → url       link
//
// `subtitle` diteruskan apa adanya karena itulah yang dicetak plugin sebelum
// Phase 1 (`t.subtitle || 'Unknown'` di plugins/search/applemusic.js). Mengubahnya
// jadi nama artis saja akan mengubah kalimat yang dilihat user tanpa diminta.

function urlLagu(entri) {
  // `videoId` di-encode karena isinya masuk ke query string: tanpa itu, id yang
  // membawa `&` atau `#` akan menyuntikkan parameter lain ke tautan youtube.
  const id = String(entri.videoId ?? "").trim();
  if (id) return `https://www.youtube.com/watch?v=${encodeURIComponent(id)}`;
  const langsung = String(entri.link ?? entri.url ?? "").trim();
  return /^https?:\/\//i.test(langsung) ? langsung : "";
}

function namaArtis(entri) {
  // `artists` adalah array dan `artist` adalah objek — keduanya dipakai
  // ytmusic-api untuk tipe entri berbeda, jadi salah satu yang dibaca saja akan
  // menghasilkan `Unknown` untuk separuh hasil.
  const dariArray = Array.isArray(entri.artists)
    ? entri.artists.map((a) => String(a?.name ?? "").trim()).filter(Boolean).join(", ")
    : "";
  return dariArray || String(entri.artist?.name ?? "").trim() || String(entri.subtitle ?? "").trim();
}

function gambarSampul(entri) {
  // Thumbnail ytmusic diurutkan dari yang paling kecil ke yang paling besar, jadi
  // yang dipakai adalah entri terakhir.
  const thumbs = Array.isArray(entri.thumbnails) ? entri.thumbnails : [];
  for (const kandidat of [thumbs.at(-1)?.url, entri.image]) {
    const url = String(kandidat ?? "").trim();
    if (/^https?:\/\//i.test(url)) return url;
  }
  return "";
}

function detik(entri) {
  const angka = Number(entri.duration);
  return Number.isFinite(angka) && angka > 0 ? Math.floor(angka) : 0;
}

function track(entri) {
  if (!entri || typeof entri !== "object") return null;
  const judul = String(entri.name ?? entri.title ?? "").trim();
  const url = urlLagu(entri);
  // Entri `ARTIST` dan `PLAYLIST` tidak punya `videoId` maupun `link`, dan
  // `search()` menaruh keduanya DI AWAL daftar. Tanpa penyaringan di sini,
  // hasil pertama yang dipakai adalah artist, bukan lagu — jadi yang disaring
  // adalah "tidak punya tautan yang bisa dibuka", bukan `type`, karena
  // agregator mengirim entri `Artist` dan `Album` yang sah-sah saja punya
  // `link`.
  if (!judul || !url) return null;
  return {
    title: judul,
    artist: namaArtis(entri),
    // Nol berarti "tidak dilaporkan", bukan durasi yang salah. Plugin tidak
    // memakainya, dan menebak format apa pun lebih buruk daripada tidak punya.
    durationSec: detik(entri),
    cover: gambarSampul(entri),
    url,
  };
}

export function normalize(raw) {
  if (raw === null || raw === undefined || typeof raw !== "object") {
    throw new Error("bentuk respons ytmusic tidak dikenali");
  }

  // Bentuk lokal tidak punya field `status` sama sekali, jadi pemeriksaan ditulis
  // eksplisit — `!raw.status` akan salah memblokir hasil pencarian lokal.
  if (raw.status === false) {
    throw new Error(`ytmusic: aggregator menandai gagal — ${String(raw.msg ?? raw.error ?? "tanpa alasan")}`);
  }

  const daftar = Array.isArray(raw) ? raw : raw.result;
  if (!Array.isArray(daftar)) {
    throw new Error(
      `ytmusic: respons bukan daftar lagu (${Array.isArray(raw) ? "array tanpa isi" : Object.keys(raw).join(", ") || "kosong"})`,
    );
  }

  return { tracks: daftar.map(track).filter(Boolean) };
}

// Kedua backend melayani `{ q }` yang sama. `applies` ada supaya breaker tidak
// menghitung permintaan yang memang tidak dilayani siapa pun — dan supaya
// pencarian dengan `q` kosong berhenti tanpa satu pun request.
export const backends = [
  {
    name: "ytmusic",
    kind: "local",
    applies: punyaQuery,
    run: lewatYtmusic,
  },
  {
    name: "nexray",
    kind: "api",
    applies: punyaQuery,
    run: lewatNexray,
  },
];