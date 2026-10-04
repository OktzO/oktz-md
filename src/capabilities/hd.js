import fs from "node:fs";
import FormData from "form-data";
import config from "../../config.js";
import { upload, get } from "../scraper/hd.js";
import { createEnhanceTask, pollEnhanceTask } from "../scraper/hdvid.js";
import { aggregator } from "../lib/aggregator.js";

// Hasil enhancement adalah file render di host pihak ketiga: bisa dihapus, bisa
// kedaluwarsa, dan tidak pernah kembali sama persis untuk input yang sama. Tidak
// ada yang boleh keluar dari cache.
export const stable = false;

// Dua perintah, dua host, dua jalur render yang sama-sama lambat dan dua-sama
// berakhir di `{ url }`. Yang membedakan bukan `media` — keduanya menerima file
// gambar yang sama — dan bukan nama backend, karena setiap backend hanya
// melayani satu jenis.
const JENIS = new Map([
  ["imglarger", "/api/tools/imglarger"],
  ["unblur", "/api/tools/unblur"],
]);

function jenis(args) {
  return typeof args?.kind === "string" ? args.kind.trim().toLowerCase() : "";
}

// Di backend, bukan hanya di `applies`: backend ini juga dipanggil langsung, dan
// operator yang salah memanggilnya butuh tahu kedua nama yang sah.
function pastikanJenis(args) {
  const ini = jenis(args);
  if (!JENIS.has(ini)) {
    throw new Error(
      `hd butuh { kind: 'imglarger' | 'unblur' } sesuai perintah pemanggil, bukan ${JSON.stringify(args?.kind ?? null)}`,
    );
  }
  return ini;
}

function punyaMedia(args) {
  return typeof args?.media === "string" && args.media.trim() !== "";
}

// Dicek sebelum scraper mana pun dipanggil: `fs.createReadStream` menolak file
// yang tidak ada dengan pesan yang jauh lebih sulit dibaca daripada yang di sini.
function pathMedia(args) {
  if (!punyaMedia(args)) {
    throw new Error("hd butuh { media }: path file gambar di disk");
  }
  const media = args.media.trim();
  if (!fs.existsSync(media)) {
    throw new Error(`hd: file gambar tidak ada: ${media}`);
  }
  return media;
}

const untukJenis = (yangDijual) => (args) => jenis(args) === yangDijual && punyaMedia(args);
const untukSemuaJenis = (args) => JENIS.has(jenis(args)) && punyaMedia(args);

// ── backend lokal: imglarger.com ──────────────────────────────────────────────

async function lewatPhotoai(args) {
  const ini = pastikanJenis(args);
  if (ini !== "imglarger") {
    throw new Error(`backend imglarger tidak melayani kind "${ini}"`);
  }
  const media = pathMedia(args);
  const unggahan = await upload(media);
  // `code` dibaca dari level teratas: `upload()` di src/scraper/hd.js sudah
  // membuka satu amplop `data.data`. Endpoint sekeluarga dibaca
  // `upload?.data?.code` di src/scraper/imglarger.js:84, jadi satu lapis inilah
  // yang tersisa.
  const code = String(unggahan?.code ?? "").trim();
  if (!code) {
    throw new Error(
      `imglarger: host tidak mengembalikan code (${Object.keys(unggahan ?? {}).join(", ") || "kosong"})`,
    );
  }
  return await get(code);
}

// ── backend lokal: fgsi.dpdns.org ─────────────────────────────────────────────

// Batas polling yatim, bukan batas menunggu render. `pollEnhanceTask` bawaan
// 10 menit; angka ini dipakai untuk polling yang sudah kalah race dari budget
// resolver dan tidak lagi awaited siapa pun. Lihat catatan panjang di bawah
// `pollEnhanceTask` untuk kenapa polling ada di luar budget.
const POLL_YATIM_MS = 60_000;

// Dua langkah sengaja dipisah supaya batas budget resolver terlihat jelas di satu
// tempat: pembuatan task ikut budget, polling tidak.
async function lewatFgsi(args) {
  const ini = pastikanJenis(args);
  if (ini !== "unblur") {
    throw new Error(`backend unblur tidak melayani kind "${ini}"`);
  }
  const media = pathMedia(args);

  // `config.APIkey.fgsi` dibaca dari `APIKEY_FGSI` (config.js:328) sedangkan
  // scraper membaca alias lama `FGSI_API_KEY` sendiri. Kalau hanya alias lama
  // yang diisi, config kosong — jadi key hanya diteruskan saat benar-benar ada,
  // dan `undefined` membiarkan scraper memakai default-nya sendiri.
  const key = String(config.APIkey?.fgsi ?? "").trim();
  const task = await createEnhanceTask(media, key === "" ? undefined : key);

  const pollUrl = String(task?.pollUrl ?? "").trim();
  if (!pollUrl) {
    throw new Error(`unblur: host tidak mengembalikan pollUrl (${Object.keys(task ?? {}).join(", ") || "kosong"})`);
  }

  // POLLING SENGAJA DI LUAR BUDGET resolver, dan itu bukan pilihan gaya.
  //
  // Yang terikat budget hanya PEMBUATAN task di atas: `createEnhanceTask` satu
  // request, jadi kegagalan di sana membuat resolver pindah backend dalam
  // hitungan detik. `pollEnhanceTask` berbeda — ia loop yang memang butuh waktu.
  // Kalau polling ikut terikat budget resolver, budget itu habis sambil menunggu
  // render dan backend aggregator TIDAK PERNAH sempat dicoba — persis
  // kebalikan dari tujuan Fase 1, yaitu punya cadangan ketika host lokal mati.
  //
  // `pollEnhanceTask` juga tidak menerima `signal` (tandatangananya hanya
  // `{ apiKey, pollIntervalMs, timeoutMs, maxTransientErrors }`), jadi
  // mengaitkannya ke `ctx.signal` butuh signature baru di scraper milik plugin
  // `hdvid` — perubahan yang bukan urusan kapabilitas ini. Jangan tambahkan
  // `Promise.race` lokal melawan signal sebagai jalan pintasan: begitu polling
  // ikut dibatalkan, backend lokal tidak pernah menghasilkan URL sama sekali.
  //
  // Batasnya jujur dan tidak disembunyikan: resolver hanya memberi tier lokal
  // `localMs` (3000ms bawaan), jadi backend ini menang HANYA kalau render selesai
  // di dalam jendela itu — praktis hanya kalau polling pertama sudah `success`.
  // Selebihnya resolver menyerah sesuai budgetnya lalu tier aggregator menjawab.
  // `.hd3` tidak boleh bergantung pada render yang kebetulan cepat.
  //
  // Karena hasil polling yang telat sudah dibuang resolver, `timeoutMs` di sini
  // bukan "berapa lama kita menunggu render" melainkan "berapa lama polling
  // yatim boleh menggantung". Default scraper 10 menit berasal dari
  // `plugins/tools/hdvid.js`, yang memang dipakai orang untuk video dan boleh
  // menunggu; sebuah socket yang menggantung 10 menit tidak bisa dipakai di
  // kotak 1GB.
  //
  // Key diteruskan ke polling juga wajib: default `pollEnhanceTask` membaca
  // `FGSI_API_KEY`, sedangkan key di atas dibaca dari `APIKEY_FGSI`. Tanpa ini
  // task berhasil dibuat lalu polling ditolak.
  return await pollEnhanceTask(pollUrl, {
    ...(key === "" ? {} : { apiKey: key }),
    timeoutMs: POLL_YATIM_MS,
  });
}

// ── backend agregator: my.izuka-api.xyz ───────────────────────────────────────

async function lewatIzuka(args = {}, ctx = {}) {
  const ini = pastikanJenis(args);
  const media = pathMedia(args);

  const form = new FormData();
  // Dialirkan dari disk, bukan dibaca ke memori: ini file milik user, dan kotak
  // ini punya batas 1GB. `form-data` yang mengalirkan bytes-nya, dan
  // `aggregator.hit` meneruskan `FormData` apa adanya ke axios, yang memasang
  // header multipart beserta boundary-nya sendiri.
  form.append("image", fs.createReadStream(media), { filename: "image.jpg", contentType: "image/jpeg" });
  // Field tambahan hanya untuk imglarger, dan bukan karangan: plugin lama
  // mengirim keduanya ke endpoint yang sama (`git show ff397d2^:plugins/tools/
  // hd2.js:53-54`), dan menghilangkannya membuat "naikkan ukuran" berubah jadi
  // parameter lain.
  if (ini === "imglarger") {
    form.append("type", "upscale");
    form.append("scale", "2");
  }

  // `AGGREGATORS.izuka.base` hanya berisi host dan `aggregator.hit` menempelkan
  // path apa adanya; kedua path sudah berawalan `/api`, sama seperti pada plugin
  // lama.
  //
  // 60 detik bukan angka baru: `timeout: 60000` dipakai kedua plugin lama
  // (`git show ff397d2^:plugins/tools/hd2.js:58` dan `hd3.js:52`). Default aggregator 5 detik akan memotong setiap enhancement
  // yang memang butuh waktu.
  //
  // Body lengkap dikembalikan apa adanya, bukan `body.result`: kedua plugin
  // lama membaca `data.status` sebelum memakai `data.result`, jadi `status` ada
  // SATU level di atas `result`. Kalau amplop dibuang di sini, respons
  // `{ status: false, result: "https://…" }` akan lolos ke `normalize` dan
  // dikirim ke user sebagai hasil yang sah.
  return await aggregator.hit("izuka", JENIS.get(ini), {
    method: "POST",
    data: form,
    timeout: 60_000,
    signal: ctx?.signal,
  });
}

// ── normalisasi ──────────────────────────────────────────────────────────────
//
// Satu field yang dijanjikan — `url` — dengan banyak nama di bentuk respons.
// Baris rujukan di bawah menunjuk pada versi SEBELUM Phase 1 (lihat
// `git show ff397d2^:plugins/tools/hd2.js`), karena setelah rewire nama field
// yang dibaca plugin berubah menjadi `data.url`:
//
//   `get(code)` (src/scraper/hd.js:60)     { downloadUrls: [ … ], status }
//   `pollEnhanceTask()` (hdvid.js:81-85)    { …, res_url }
//   izuka imglarger (hd2 lama:62 dan :74)  { status, result: "https://…" }
//   izuka unblur (hd3 lama:56 dan :63)     { status, result: { output_url: [ … ] } }
//   bentuk paling polos                     { url }
//
// `normalize` adalah satu-satunya tempat yang boleh menyamakannya. Backend tidak
// boleh membuka amplop apa pun: `status` satu level di atas `result`, jadi
// backend yang melepasnya membuat `normalize` tak punya apa pun untuk diperiksa.

function urlHttp(nilai) {
  const url = String(nilai ?? "").trim();
  return /^https?:\/\//i.test(url) ? url : "";
}

function kandidat(isi) {
  if (typeof isi === "string") return [isi];
  if (!isi || typeof isi !== "object") return [];
  const daftar = [];
  if (typeof isi.url === "string") daftar.push(isi.url);
  // `img_url` dan `download_url` dibaca scraper sekeluarga di
  // src/scraper/imglarger.js:66-68 dari `CheckStatus` yang sama, jadi keduanya
  // nama yang benar-benar ada di produk ini, bukan tebakan.
  if (typeof isi.img_url === "string") daftar.push(isi.img_url);
  if (typeof isi.download_url === "string") daftar.push(isi.download_url);
  // Indeks 0 untuk `output_url` dibaca hd3.js lama:63 dan hdvid2.js:211; untuk
  // `downloadUrls`, src/scraper/imglarger.js:67 juga memakai indeks 0. Jadi
  // keduanya satu elemen pertama, bukan "yang paling besar" atau "yang terakhir".
  if (Array.isArray(isi.downloadUrls)) daftar.push(isi.downloadUrls[0]);
  if (Array.isArray(isi.output_url)) daftar.push(isi.output_url[0]);
  if (typeof isi.res_url === "string") daftar.push(isi.res_url);
  return daftar;
}

function urlHasil(bentuk) {
  // Satu level amplop lebih dulu: `result` dari izuka dan `data` dari scraper
  // sekeluarga. Array TIDAK ikut dibuka sebagai amplop — `data: []` adalah
  // kegagalan bentuk, bukan hasil.
  for (const isi of [bentuk, bentuk?.result, bentuk?.data]) {
    for (const mentah of kandidat(isi)) {
      const url = urlHttp(mentah);
      if (url) return url;
    }
  }
  return "";
}

export function normalize(raw) {
  if (!raw || typeof raw !== "object") {
    throw new Error("bentuk respons hd tidak dikenali");
  }

  // Dua `status` yang berbeda hidup berdampingan: boolean dari amplop izuka, dan
  // string dari CheckStatus imglarger (`"waiting"` lalu `"success"`). Keduanya
  // berarti "belum ada hasil yang boleh dikirim", tapi hanya yang kedua yang
  // boleh bernilai apa pun selain `success`.
  if (typeof raw.status === "boolean" && raw.status === false) {
    throw new Error(`hd: aggregator menandai gagal — ${String(raw.msg ?? raw.error ?? "tanpa alasan")}`);
  }
  if (typeof raw.status === "string" && raw.status.trim().toLowerCase() !== "success") {
    // `get(code)` adalah satu panggilan CheckStatus tanpa polling, jadi host
    // yang masih mengolah menjawab `status: "waiting"` — dan `src/scraper/
    // imglarger.js:70` memperlakukan "ada downloadUrls" sebagai cukup, yang
    // berarti bentuk `waiting` bisa saja tetap membawa URL. Menerimanya berarti
    // plugin mengirim sesuatu yang bukan hasil enhance sambil tetap memberi
    // centang hijau. Statusnya yang jadi acuan, bukan isi array-nya.
    throw new Error(`hd: host lokal belum selesai atau gagal — status "${raw.status}"`);
  }

  const url = urlHasil(raw);
  // Bentuk yang tidak menghasilkan apa pun yang bisa dikirim harus jadi
  // kegagalan, bukan `{ url: "" }`: plugin akan menjalankan `sendMessage` dengan
  // tidak ada file lalu tetap memberi centang hijau ke user.
  if (!url) {
    throw new Error(`hd: respons tanpa URL hasil yang bisa dikirim (${Object.keys(raw).join(", ") || "kosong"})`);
  }

  return { url };
}

// `applies` yang memisahkan jenis adalah yang menjaga satu host tidak pernah
// dicatat gagal karena kapabilitas ini dipakai untuk jenis yang memang tidak
// dilayaninya: tanpa itu, satu `.hd2` akan mengeluarkan `fgsi-enchantvideo` dari
// rotasi walaupun host itu sehat dan tidak pernah dihubungi.
export const backends = [
  { name: "photoai-imglarger", kind: "local", applies: untukJenis("imglarger"), run: lewatPhotoai },
  { name: "fgsi-enchantvideo", kind: "local", applies: untukJenis("unblur"), run: lewatFgsi },
  { name: "izuka", kind: "api", applies: untukSemuaJenis, run: lewatIzuka },
];