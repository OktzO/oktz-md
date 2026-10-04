import { DouyinDL } from "../scraper/douyin.js";
import { aggregator } from "../lib/aggregator.js";

// Tautan media dari snapvideotools berumur pendek dan post Douyin bisa
// dihapus kapan saja, jadi hasil scraping tidak boleh keluar dari cache meski
// post-nya sendiri tidak berubah.
export const stable = false;

// Tiga host yang benar-benar bisa dipindai: halaman video, halaman share, dan
// pemendek resmi v.douyin.com. Host dicocokkan dari ujung string, bukan dengan
// `includes`: douyin.com.evil.example adalah domain lain yang tetap memuat
// "douyin.com", dan meneruskannya ke scraper berarti satu request ke host yang
// tidak sengaja plus satu kegagalan di breaker untuk semua orang.
function urlDouyin(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  return (
    host === "douyin.com" ||
    host.endsWith(".douyin.com") ||
    host === "iesdouyin.com" ||
    host.endsWith(".iesdouyin.com")
  );
}

async function lewatSnapvideotools({ url } = {}, ctx = {}) {
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("douyin butuh { url }: link video dari douyin.com atau v.douyin.com");
  }
  const bersih = url.trim();
  // Guard di dalam backend, sebelum scraper: input salah harus berhenti di
  // sini, bukan jadi satu request yang pasti ditolak upstream.
  if (!urlDouyin(bersih)) {
    throw new Error(
      `format link Douyin tidak dikenali: ${bersih} — yang diterima https://douyin.com/video/... , https://www.iesdouyin.com/share/video/... , atau https://v.douyin.com/...`,
    );
  }
  return await DouyinDL(bersih, { signal: ctx?.signal });
}

async function lewatAzbry({ url } = {}, ctx = {}) {
  const signal = ctx?.signal;
  if (typeof url !== "string" || url.trim() === "") {
    throw new Error("douyin butuh { url }: link video dari douyin.com atau v.douyin.com");
  }
  // `AGGREGATORS.azbry.base` hanya berisi host dan `aggregator.hit` menempelkan
  // path apa adanya, jadi `/api` harus ditulis eksplisit: tanpa itu host
  // menjawab 404 dan fallback ini justru jadi sumber utama kegagalan.
  const body = await aggregator.hit("azbry", "/api/downloader/douyin", {
    params: { url: url.trim() },
    signal,
  });
  // `status` azbry ada SATU level di atas `result`, jadi memeriksa `body.status`
  // di sini adalah satu-satunya tempat yang bisa membacanya sebelum amplopnya
  // hilang. Plugin sebelum Phase 1 persis meminta `res.data?.status &&
  // res.data?.result` (plugins/download/douyindl.js) dan mengulang tiga kali
  // kalau salah satu tidak terpenuhi; kalau amplop dibuang diam-diam, respons
  // 200 dengan `status: false` tapi `result` yang penuh akan lolos sebagai
  // scraping yang sukses.
  //
  // Perhatikan arahnya: ini KEBALIKAN dari `lewatNeoxr` di src/capabilities/videy.js
  // yang sengaja mengembalikan body utuh. Bentuk dua host memang berbeda —
  // azbry menaruh `result` di dalam, videy menaruh `url` di dalam `data` dengan
  // `status` di luar. Yang dijaga di sini dan di sana sama: `status` tidak boleh
  // hilang tanpa satu backend sudah menjadikannya kegagalan lebih dulu.
  if (body?.status !== true) {
    throw new Error(
      `douyin: aggregator menandai gagal — ${String(body?.msg ?? body?.error ?? "tanpa alasan")}`,
    );
  }
  return body?.result;
}

const punyaUrl = (args) => typeof args?.url === "string" && args.url.trim() !== "";

// ── normalisasi ──────────────────────────────────────────────────────────────
//
// Dua backend punya empat nama field yang sama dan tidak punya field yang
// sama, jadi `normalize` adalah tempat tunggal yang boleh menyamakannya.
//
// Bentuk lokal (`DouyinDL` di src/scraper/douyin.js) sudah menandai dirinya di
// `status`, dan `platform` berisi `platformName` dari host. Bentuk aggregator
// tidak membawa `status` ke sini: `lewatAzbry` memeriksa `body.status` dulu dan
// melempar kalau bukan `true`, jadi yang diteruskan ke sini cuma `result`.
// Karena itu pemeriksaan `raw.status === false` di bawah hanya berlaku untuk
// bentuk lokal dan harus ditulis eksplisit — `!raw.status` akan salah memblokir
// aggregator.

function urlMedia(nilai) {
  const url = String(nilai ?? "").trim();
  return /^https?:\/\//i.test(url) ? url : "";
}

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons douyin tidak dikenali");
  }

  // `DouyinDL` mengembalikan objek ini, bukan melempar, jadi tanpa pemeriksaan
  // di sini backend dianggap berhasil dan aggregator tidak pernah diberi
  // giliran.
  if (raw.status === false) {
    throw new Error(`douyin: scraper lokal gagal — ${String(raw.error ?? "tanpa alasan")}`);
  }

  const video = urlMedia(raw.video);
  // Video adalah media yang dijanjikan plugin; audio boleh kosong karena post
  // tanpa trek audio itu nyata. `video: null` berarti host menjawab 200 dengan
  // `mediaUrls` tanpa entri video — menerimanya sebagai "sukses" membuat
  // plugin mengirim tidak ada file lalu tetap memberi centang hijau.
  if (!video) {
    throw new Error(
      `douyin: respons tanpa URL video yang bisa diunduh (${Object.keys(raw).join(", ") || "kosong"})`,
    );
  }

  return {
    title: String(raw.title ?? "").trim(),
    platform: String(raw.platform ?? "").trim(),
    video,
    audio: urlMedia(raw.audio),
  };
}

// Kedua backend melayani bentuk argumen yang sama (`{ url }`), jadi `applies`
// di sini bukan untuk membedakan tier melainkan untuk memastikan aggregator
// tidak Essence dihubungi untuk link yang sudah ditolak guard di backend lokal.
export const backends = [
  {
    name: "snapvideotools",
    kind: "local",
    applies: punyaUrl,
    run: lewatSnapvideotools,
  },
  {
    name: "azbry",
    kind: "api",
    applies: (args) => punyaUrl(args) && urlDouyin(args.url),
    run: lewatAzbry,
  },
];