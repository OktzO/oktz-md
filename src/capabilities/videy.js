import { idDari, videyShare } from "../scraper/videy.js";
import { aggregator } from "../lib/aggregator.js";

// Tautan berbagi videy bisa dihapus kapan saja dan CDN videy tidak memberi
// tanda kedaluwarsa, jadi hasil scraping tidak boleh keluar dari cache meski
// tautan yang sama masih hidup.
export const stable = false;

// Host dicocokkan dari batas label, bukan dengan `includes`: `videy.co.evil.example`
// adalah domain lain yang tetap memuat "videy.co", dan meneruskannya ke scraper
// berarti satu request sia-sia plus satu kegagalan di breaker untuk semua orang.
// Sub-domain ikut diterima karena halaman videy dilayani dari `www.videy.co`.
function urlVidey(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  return host === "videy.co" || host.endsWith(".videy.co");
}

async function lewatCdnVidey({ url } = {}, ctx = {}) {
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("videy butuh { url }: link berbagi dari videy.co");
  }
  const bersih = url.trim();
  // Guard di dalam backend, sebelum scraper: input salah harus berhenti di
  // sini, bukan jadi satu request ke CDN yang pasti ditolak upstream.
  if (!urlVidey(bersih)) {
    throw new Error(
      `format link Videy tidak dikenali: ${bersih} — yang diterima https://videy.co/v?id=...`,
    );
  }
  // `applies` sudah menyaring ini, tapi `run` tidak bergantung pada kenyataan
  // bahwa ia dipanggil dari resolver: backend ini juga dipanggil langsung, dan
  // id yang tidak terbaca berarti tidak ada pathname yang bisa diprobe.
  const id = idDari(bersih);
  if (!id) {
    throw new Error(`link videy.co tanpa id video yang bisa dibaca: ${bersih}`);
  }
  return await videyShare(bersih, { signal: ctx?.signal });
}

async function lewatNeoxr({ url } = {}, ctx = {}) {
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("videy butuh { url }: link berbagi dari videy.co");
  }
  // `AGGREGATORS.neoxr.base` hanya berisi host dan `aggregator.hit` menempelkan
  // path apa adanya. Semua panggilan neoxr di repo ini memakai `/api` lebih
  // dulu (lihat plugins/ai/txt2img.js dan plugins/sticker/attp.js), jadi path
  // yang lupa `/api` akan dijawab 404.
  //
  // Body lengkap dikembalikan apa adanya, bukan `body.data`, karena `status`
  // ada satu level di atas `data`. Plugin sebelum Phase 1 menolak `!data.status`,
  // dan kalau amplop itu dibuang di sini, respons `{ status: false, data: { url } }`
  // akan lolos ke `sendMedia` sebagai video yang sah.
  return await aggregator.hit("neoxr", "/api/videy", {
    params: { url: url.trim() },
    signal: ctx?.signal,
  });
}

const punyaUrl = (args) => typeof args?.url === "string" && args.url.trim() !== "";

// ── normalisasi ──────────────────────────────────────────────────────────────
//
// Dua backend menamai field yang sama dengan dua bentuk berbeda. Bentuk lokal
// (dari `videyShare`) menyodorkan `{ id, ext, url }` rata; bentuk aggregator
// menyimpan URL satu level di bawah `data`, dan status-nya di `status`.
// Plugin ini hanya mengonsumsi satu field — `url` video — jadi itu yang dinormalisasi.

function urlVideo(nilai) {
  const url = String(nilai ?? "").trim();
  return /^https?:\/\//i.test(url) ? url : "";
}

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons videy tidak dikenali");
  }

  // Bentuk aggregator hanya punya `status`; bentuk lokal tidak punya field itu
  // sama sekali. Karena itu pemeriksaan ditulis eksplisit — `!raw.status` akan
  // salah memblokir bentuk lokal.
  if (raw.status === false) {
    throw new Error(`videy: backend aggregator menandai gagal — ${String(raw.error ?? raw.msg ?? "tanpa alasan")}`);
  }

  // Amplop aggregator dibuka di sini, bukan di backend, supaya dua bentuk punya
  // satu tempat yang menyamakan. Array tidak ikut dibuka: `data: []` adalah
  // kegagalan bentuk, bukan daftar format yang sah.
  const isi =
    raw.data && typeof raw.data === "object" && !Array.isArray(raw.data) ? raw.data : raw;

  const url = urlVideo(isi.url ?? raw.url);
  // Sama seperti brief Task 7 menuntut untuk `formats: []`: bentuk yang tidak
  // menghasilkan apa pun yang bisa dikirim harus menjadi kegagalan, bukan
  // record kosong. Plugin yang menerimanya akan menjalankan `sendMedia` dengan
  // tidak ada file lalu tetap memberi centang hijau ke user.
  if (!url) {
    throw new Error(
      `videy: respons tanpa URL video yang bisa diunduh (${Object.keys(raw).join(", ") || "kosong"})`,
    );
  }

  return { url };
}

// Kedua backend melayani bentuk argumen yang sama (`{ url }`), jadi `applies`
// di sini bukan untuk membedakan tier melainkan untuk memastikan aggregator
// tidak dihubungi untuk link yang sudah ditolak guard, dan backend lokal tidak
// menghabiskan satu slot breaker untuk tautan beranda `videy.co` yang memang
// tidak punya id untuk diprobe.
export const backends = [
  {
    name: "cdn-videy",
    kind: "local",
    applies: (args) => punyaUrl(args) && urlVidey(args.url) && idDari(args.url) !== null,
    run: lewatCdnVidey,
  },
  {
    name: "neoxr",
    kind: "api",
    applies: (args) => punyaUrl(args) && urlVidey(args.url),
    run: lewatNeoxr,
  },
];
