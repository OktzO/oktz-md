import scrapePinterest from "../scraper/pindl.js";
import { aggregator } from "../lib/aggregator.js";

// Pin bisa dihapus owner-nya kapan saja dan URL dari ilovepin.net sudah diproses
// ulang berkali-kali, jadi hasil scraping tidak boleh keluar dari cache meski
// pin-nya sendiri tidak berubah.
export const stable = false;

// Menerima dua bentuk link yang benar-benar bisa dipindai: host Pinterest
// dengan jalur /pin/, dan pin.it yang merupakan pemendek resmi. Sisanya
// ditolak sebelum ada request, bukan setelah 502.
function urlPin(url) {
  let parsed;
  try {
    parsed = new URL(String(url).trim());
  } catch {
    return false;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") return false;
  const host = parsed.hostname.toLowerCase();
  // Host dicocokkan per label, bukan dengan `includes` di seluruh string:
  // pin.it.evil.example adalah domain lain yang tetap memuat "pin.it".
  if (host === "pin.it") return true;
  // Label `pinterest` dicek di posisi mana pun supaya pinterest.co.uk dan
  // id.pinterest.com sama-sama diterima. Konsekuensinya disadari: host
  // pinterest.example.com lolos, dan biayanya satu request scraper yang sia-sia
  // — tidak ada kredensial yang dikirim, dan guard level plugin sudah lebih
  // longgar dari ini. Penolakan yang salah lebih mahal daripada permintaan sia-sia.
  if (!host.split(".").includes("pinterest")) return false;
  return /^\/pin\//i.test(parsed.pathname);
}

async function lewatIlovepin(args = {}, ctx = {}) {
  const bersih = String(args.url).trim();
  if (!urlPin(bersih)) {
    throw new Error(
      `format link Pinterest tidak dikenali: ${bersih} — yang diterima https://pinterest.com/pin/... , https://id.pinterest.com/pin/... , atau https://pin.it/...`,
    );
  }

  // Scraper ini mengembalikan null untuk semua kegagalan (upstream 5xx, pin
  // dihapus, cookie tidak berlaku). null dihitung kegagalan backend oleh
  // resolve.js, jadi aggregator tetap punya giliran.
  return await scrapePinterest(bersih, { signal: ctx?.signal });
}

async function lewatNexray(args = {}, ctx = {}) {
  const signal = ctx?.signal;
  if (typeof args.url === "string" && args.url.trim() !== "") {
    const body = await aggregator.hit("nexrayWeb", "/api/downloader/pinterest", {
      params: { url: args.url.trim() },
      signal,
    });
    if (body?.status !== true) {
      return body && typeof body === "object" ? body : { status: false, msg: "tanpa alasan" };
    }
    const d = body?.result;
    if (Array.isArray(d) || (d && (d.images || d.videos || d.type))) {
      return { status: true, result: d };
    }
    if (d && d.video) {
      return { status: true, result: { type: "video", videos: [{ url: d.video }] } };
    }
    if (d && (d.image || d.thumbnail)) {
      return { status: true, result: { type: "image", images: [{ name: "orig", url: d.image || d.thumbnail }] } };
    }
    return { status: true, result: d ?? null };
  }
  if (typeof args.q === "string" && args.q.trim() !== "") {
    const body = await aggregator.hit("nexrayWeb", "/api/search/pinterest", {
      params: { q: args.q.trim() },
      signal,
    });
    if (body?.status !== true) {
      return body && typeof body === "object" ? body : { status: false, msg: "tanpa alasan" };
    }
    return { status: true, result: Array.isArray(body?.result) ? body.result : [] };
  }
  throw new Error("pinterest butuh { url } atau { q }, tidak keduanya");
}

const punyaUrl = (args) => typeof args?.url === "string" && args.url.trim() !== "";
const punyaQuery = (args) => typeof args?.q === "string" && args.q.trim() !== "";

// ── normalisasi ──────────────────────────────────────────────────────────────
//
// Tiga backend mengirim tiga nama field berbeda untuk hal yang sama, dan
// `normalize` adalah satu-satunya tempat yang boleh menyamakannya. Kalau tidak,
// pindah backend hanya diam-diam jadi "respons kosong" dan plugin salah baca
// bentuk.

// Bentuk lokal (src/scraper/pindl.js) tidak punya `type` di level teratas:
// `type` ada di tiap entri `media`, dan satu pin hanya punya satu jenis media.
function dariLokal(media) {
  const terbaik = media.find((m) => m?.url && /^https?:\/\//i.test(String(m.url).trim()));
  if (!terbaik) {
    throw new Error("pinterest: respons scraper lokal tanpa media yang bisa diunduh");
  }
  // Hanya satu entri yang diteruskan: plugin mengirim semua yang ada di
  // mediaList, dan satu pin video punya sampai lima varian kualitas. Lima
  // file video ke user bukan hasil yang lebih baik, hanya lima kali kerja.
  const jenis = terbaik.type;
  // Type tidak dikenal tidak boleh ditebak jadi gambar: plugin akan mengirim
  // .mp3 atau .webm sebagai gambar dan user menerima file rusak yang dilaporkan
  // sukses. Scraper lokal sekarang hanya menaruh video dan image di `media`,
  // tapi normalize adalah pintu terakhir dan menolak, bukan menebak.
  if (jenis !== "video" && jenis !== "image") {
    throw new Error(`pinterest: tipe media "${jenis}" dari scraper lokal tidak dikenal`);
  }
  return { type: jenis, media: [{ type: jenis, url: String(terbaik.url).trim() }] };
}

// Bentuk aggregator: `type` di level teratas, URL-nya bertebar di `videos`,
// `images`, atau `download`.
function dariAggregator(raw) {
  const ambil = (nilai) => {
    const url = String(nilai ?? "").trim();
    return /^https?:\/\//i.test(url) ? url : "";
  };

  if (raw.type === "video") {
    // `videos` sudah diurutkan dari kualitas tertinggi, jadi entri pertama yang
    // punya URL adalah yang dikirim user. `download` adalah alias lama.
    const videos = Array.isArray(raw.videos) ? raw.videos : [];
    const url = ambil(videos.find((v) => ambil(v?.url))?.url) || ambil(raw.download);
    if (!url) throw new Error("pinterest: respons video aggregator tanpa URL yang bisa diunduh");
    return { type: "video", media: [{ type: "video", url }] };
  }

  const gambar = Array.isArray(raw.images) ? raw.images : [];
  // Aggregator mengirim beberapa varian ukuran dalam `images`; `orig` adalah yang
  // dicari, dan kalau tidak ada, entri terakhir adalah varian terbesar yang
  // dikirimnya — perilaku yang sama dengan yang dipakai plugin sebelumnya.
  const asli = gambar.find((g) => g?.name === "orig") || gambar[gambar.length - 1];
  const url = ambil(asli?.url) || ambil(raw.download);
  if (!url) throw new Error("pinterest: respons gambar aggregator tanpa URL yang bisa diunduh");
  return { type: "image", media: [{ type: "image", url }] };
}

function pinCari(entri) {
  if (!entri || typeof entri !== "object") return null;
  // `image` dan `images_url` adalah nama yang benar-benar dikirim host; `link`
  // dipakai untuk tahu pin mana asalannya dan tidak ada di semua host. `||`
  // bukan `??`: host yang mengirim `image: ""` menyisakan `images_url` yang
  // justru berisi URL, dan `??` akan memblokirnya.
  const gambar = String(entri.image || entri.images_url || "").trim();
  if (!/^https?:\/\//i.test(gambar)) return null;
  return {
    title: String(entri.title ?? entri.name ?? entri.description ?? "").trim(),
    image: gambar,
    link: String(entri.link ?? entri.pin_link ?? entri.url ?? "").trim(),
  };
}

// Bentuk lokal (src/scraper/pindl.js) tidak punya `status` maupun `result`, jadi
// keberadaan salah satunya sudah cukup untuk tahu bahwa yang sampai ke sini
// adalah amplop. `status` dibaca sebelum `result` dibuka: membukanya belakangan
// membuat `status: false` yang tidak terlihat.
function isiAmplop(raw) {
  if (Array.isArray(raw) || (!("status" in raw) && !("result" in raw))) return raw;
  if (!raw.status) {
    throw new Error(
      `pinterest: aggregator menandai gagal — ${String(raw.msg ?? raw.error ?? "tanpa alasan")}`,
    );
  }
  const isi = raw.result;
  if (!isi || typeof isi !== "object") {
    throw new Error("pinterest: aggregator menjawab tanpa result yang bisa dipakai");
  }
  return isi;
}

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons pinterest tidak dikenali");
  }

  const isi = isiAmplop(raw);

  // Pencarian dicek lebih dulu: hasil kosong di sini adalah jawaban yang sah
  // ("tidak ada yang cocok"), sedangkan respons unduhan tanpa media adalah
  // kegagalan yang harus dilempar supaya backend lain sempat mencoba.
  const daftar = Array.isArray(isi) ? isi : Array.isArray(isi.pins) ? isi.pins : null;
  if (daftar !== null) return { pins: daftar.map(pinCari).filter(Boolean) };

  if (Array.isArray(isi.media)) return dariLokal(isi.media);

  if (isi.type !== "video" && isi.type !== "image") {
    throw new Error(
      `bentuk respons pinterest tidak dikenali: ${Object.keys(isi).join(", ") || "kosong"}`,
    );
  }
  return dariAggregator(isi);
}

// Dua backend ini hanya melayani bentuk argumen yang berbeda: scraper lokal
// menerima link pin, tidak menerima kata kunci. Tanpa `applies` di sini, setiap
// `.pap` dan `.pin` dihitung sebagai kegagalan ilovepin oleh breaker, dan tiga
// pencarian sudah cukup untuk mengeluarkan host yang sehat dari rotasi.
export const backends = [
  {
    name: "ilovepin",
    kind: "local",
    applies: punyaUrl,
    run: lewatIlovepin,
  },
  {
    name: "nexray",
    kind: "api",
    applies: (args) => (punyaUrl(args) && urlPin(args.url)) || punyaQuery(args),
    run: lewatNexray,
  },
];
