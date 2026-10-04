import scrapeSfile from "../scraper/sfiledl.js";
import { aggregator } from "../lib/aggregator.js";

// URL gate sfile.mobi memuat token sekali pakai: halaman yang sama Tomorrow
// bisa mengembalikan `download_url: null` sementara URL yang kemarin masih
// hidup. Karena itu hasil scraping tidak boleh keluar dari cache.
export const stable = false;

// sfile.co adalah domain lama yang masih dipakai halaman unduhan
// (regex di src/scraper/sfiledl.js menerima `downloadNNN.sfile.co` dan
// `downloadNNN.sfile.mobi`), jadi keduanya diterima. Dicocokkan dari batas
// label host: `sfile.mobi.evil.example` adalah domain lain yang tetap memuat
// "sfile.mobi", dan meneruskannya ke scraper berarti satu request sia-sia.
function urlSfile(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  return host === "sfile.mobi" || host.endsWith(".sfile.mobi") || host === "sfile.co" || host.endsWith(".sfile.co");
}

async function lewatSfileMobi({ url } = {}, ctx = {}) {
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("sfile butuh { url }: link file dari sfile.mobi");
  }
  const bersih = url.trim();
  // Guard di dalam backend, sebelum scraper: input salah harus berhenti di
  // sini, bukan jadi tiga request yang pasti ditolak upstream.
  if (!urlSfile(bersih)) {
    throw new Error(
      `format link Sfile tidak dikenali: ${bersih} — yang diterima https://sfile.mobi/... atau https://sfile.co/...`,
    );
  }
  return await scrapeSfile(bersih, { signal: ctx?.signal });
}

async function lewatNeoxr({ url } = {}, ctx = {}) {
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("sfile butuh { url }: link file dari sfile.mobi");
  }
  // `AGGREGATORS.neoxr.base` hanya berisi host dan `aggregator.hit` menempelkan
  // path apa adanya. Semua panggilan neoxr di repo ini memakai `/api` lebih
  // dulu (lihat plugins/ai/txt2img.js dan plugins/sticker/attp.js), jadi path
  // yang lupa `/api` akan dijawab 404.
  const body = await aggregator.hit("neoxr", "/api/sfile", {
    params: { url: url.trim() },
    signal: ctx?.signal,
  });
  // Bentuk neoxr adalah amplop `{ data: { url, filename, mime } }` dengan
  // `status` di luar `data`, dan amplop itu dikembalikan UTUH: backend yang
  // mengembalikan `body.data` membuang `status` sebelum `normalize` sempat
  // membacanya. Plugin sebelum Phase 1 (plugins/download/sfiledl.js) memang
  // tidak pernah memeriksa `status`, tapi tanpa amplop `{ status: false, data:
  // { url } }` akan lolos sebagai file yang siap dikirim. Bentuk neoxr yang
  // sama ditangani di src/capabilities/videy.js.
  return body;
}

const punyaUrl = (args) => typeof args?.url === "string" && args.url.trim() !== "";

// ── normalisasi ──────────────────────────────────────────────────────────────
//
// Dua backend menamai field yang sama dengan empat nama berbeda: scraper lokal
// memakai `file_name`, `size_from_text`, dan `download_url`; aggregator memakai
// `filename`, `size`, dan `url`. `normalize` adalah satu-satunya tempat yang
// boleh menyamakannya.
//
// Perhatikan bahwa bentuk lokal tidak pernah mengirim `size` numerik maupun
// `mime`: `size_from_text` sudah berupa teks siap tampil ("100.42 MB") dari
// halaman sfile, dan mime tidak ada di sana sama sekali. Field yang tidak ada
// menjadi string kosong, bukan `undefined` yang akan tercetak sebagai "undefined"
// di nama file.

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons sfile tidak dikenali");
  }

  // Bentuk lokal tidak punya `status`, jadi pemeriksaan ditulis eksplisit —
  // `!raw.status` akan salah memblokir hasil scraping lokal.
  if (raw.status === false) {
    throw new Error(
      `sfile: aggregator menandai gagal — ${String(raw.msg ?? raw.error ?? "tanpa alasan")}`,
    );
  }

  // Amplop neoxr dibuka di sini, bukan di backend: `data` satu level di bawah
  // `status`, dan amplop yang sudah terbuang tidak bisa diperiksa lagi. Array
  // tidak ikut dibuka — `data: []` adalah kegagalan bentuk, bukan file.
  const isi =
    raw.data && typeof raw.data === "object" && !Array.isArray(raw.data) ? raw.data : raw;

  const url = String(isi.download_url ?? isi.url ?? "").trim();
  // `sfile()` mengembalikan `download_url: null` pada tiga jalur keluar yang
  // berbeda — tanpa `og:url`, tanpa `#download`, dan regex gate yang tidak
  // cocok. Semua itu kegagalan, bukan file yang sah tanpa tautan: plugin akan
  // menjalankan `sendMedia(undefined)` lalu memberi centang hijau ke user yang
  // tidak menerima apa-apa.
  if (!/^https?:\/\//i.test(url)) {
    throw new Error(
      `sfile: respons tanpa URL unduhan (field: ${Object.keys(isi).join(", ") || "kosong"})`,
    );
  }

  return {
    filename: String(isi.file_name ?? isi.filename ?? "").trim(),
    url,
    // PERMUKAAN YANG TIDAK DIPAKAI. `plugins/download/sfiledl.js` hanya
    // mengonsumsi `filename`, `url`, dan `mime` — `size` tidak pernah dibaca
    // plugin mana pun. Dibiarkan karena normalize adalah tempat satu-satunya
    // yang menyamakan `size_from_text` (teks siap tampil dari halaman sfile)
    // dengan `size` numerik dari aggregator, jadi membuangnya di sini berarti
    // kedua bentuk kehilangan satu-satunya tempat di mana nama field itu
    // diterjemahkan. Kalau suatu saat plugin butuh, ia ada di sini; jangan
    // tambah field baru tanpa konsumen.
    size: String(isi.size_from_text ?? isi.size ?? "").trim(),
    mime: String(isi.mime ?? "").trim(),
  };
}

// Kedua backend melayani bentuk argumen yang sama (`{ url }`); `applies` di
// backend aggregator memastikan aggregator tidak dihubungi untuk link yang
// sudah ditolak guard, jadi penolakan input tidak dihitung sebagai kegagalan
// host dan tidak bisa membuka breaker host yang sehat.
export const backends = [
  {
    name: "sfile-mobi",
    kind: "local",
    applies: punyaUrl,
    run: lewatSfileMobi,
  },
  {
    name: "neoxr",
    kind: "api",
    applies: (args) => punyaUrl(args) && urlSfile(args.url),
    run: lewatNeoxr,
  },
];