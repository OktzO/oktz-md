#!/usr/bin/env node
// Audit harness Fase 1: mengukur sejauh mana bot sudah mandiri dari agregator.
//
// ── apa yang sebenarnya terukur, dan apa yang tidak ──────────────────────────
//
// Instalasi ini mematikan SELURUH akses jaringan lebih dulu, jadi tidak satu pun
// backend lokal pun bisa menjawab: kedelapan backend `local` adalah scraper
// yang butuh network. Karena itu "berhasil tanpa network" TIDAK bisa diamati di
// sini — bukan karena harness-nya lemah, tapi karena bentuk pertanyaannya mustahil
// dijawab tanpa jaringan.
//
// Yang bisa diamati, dan yang justru merupakan janji Fase 1, adalah URUTAN TIER:
// backend `local` yang pertama kali dijalankan, dan agregator sebagai cadangan
// terakhir. resolve.js:98-101 menyusun ulang antrean begitu (`local` dulu,
// sisanya kemudian), jadi urutan tier itulah persis yang dijanjikan Fase 1 — dan
// memblokir jaringan membuatnya bisa diamati tanpa interferensi host lain.
//
// Batas kedua yang tidak bisa dihapus: memblokir koneksi membuat backend lokal
// gagal dalam milidetik, sedangkan host yang benar-benar mati biasanya diam
// beberapa detik lalu connection-reset. Jadi urutan tier yang terlihat di sini
// BUKAN bukti siapa yang menang balapan latency di produksi. Untuk `hd/unblur`
// perbedaannya menentukan: urutan tiernya lokal (lihat hd.js:124-129) tapi di
// produksi polling render berjalan puluhan detik melawan budget 3000ms, jadi
// agregator yang menang. Harness ini tidak bisa mengukur itu, jadi verdict
// produksi untuk jalur-jalur seperti itu DECLARED di `KASUS` di bawah, dengan
// alasan yang disalin dari komentar kodenya — bukan tebakan, dan bukan diam.
//
// ── kenapa pemblokiran dibuat total ───────────────────────────────────────────
//
// Memblokir satu modul saja (misal `httpAxios`) tidak cukup: scraper yang
// memakai `axios` langsung atau `undici` akan menembak keluar tanpa terlihat,
// dan audit ini akan melaporkan independen padahal sebenarnya bocor. Karena itu
// `fetch`, `http`, `https`, `net`, dan `tls` dipasang SEMUA sebelum modul
// kapabilitas mana pun diimpor.
//
// Menolak, bukan menggantung: menggantung membuat setiap kasus memakan budget
// penuh 3000ms dan menggagalkan audit dengan alasan yang salah.
//
// ── memori ───────────────────────────────────────────────────────────────────
//
// Kotak ini punya RAM 1GB. Harness ini satu proses, satu kasus pada satu waktu,
// tidak menyimpan body respons apa pun (yang diblokir memang tidak pernah
// sampai), dan keluar lewat `process.exit` supaya timer orphan `pollEnhanceTask`
// (batasnya 60 detik, lihat hd.js:86) tidak menahan proses hidup.

import fs from "node:fs";
import http from "node:http";
import https from "node:https";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import tls from "node:tls";

import { resetResolver, resolver } from "../src/lib/resolve.js";

// ── definisi host agregator ───────────────────────────────────────────────────
//
// Harus sama persis dengan `AGGREGATORS` di src/lib/aggregator.js dan dengan
// guard test tests/no-aggregator-in-plugins.test.mjs. Kalau ada host agregator
// baru yang masuk ke aggregator.js tapi tidak ke sini, audit ini akan salah
// melapor panggilan agregator sebagai "mandiri".
const HOST_AGREGATOR = [
  "api.neoxr.eu",
  "api.nexray.eu.cc",
  "my.izuka-api.xyz",
  "api.cuki.biz.id",
  "api.siputzx.my.id",
  "api.azbry.com",
];

function hostAggregator(host) {
  const h = String(host ?? "").toLowerCase();
  return h !== "" && HOST_AGREGATOR.includes(h);
}

// ── pemblokiran jaringan ──────────────────────────────────────────────────────

/** Percobaan koneksi yang tercatat sejak kasus terakhir dimulai. */
let percobaan = [];

function tolak(cara, host) {
  const hs = String(host ?? "(tanpa host)").toLowerCase();
  percobaan.push({ cara, host: hs });
  const error = new Error(`audit-offline: koneksi diblokir (${cara} → ${hs})`);
  error.code = "AUDIT_OFFLINE_BLOCKED";
  return error;
}

// Bentuk argumen `http.request` / `net.Socket#connect` sangat beragam: string
// URL, objek `{ hostname }`, objek `{ host }`, angka port, atau path unix socket.
// Semuanya harus dibaca, karena host agregator bisa muncul lewat jalur mana saja
// yang dipakai axios atau undici.
function hostDari(args) {
  const pertama = args[0];
  if (typeof pertama === "string") {
    try {
      return new URL(pertama).hostname;
    } catch {
      return "";
    }
  }
  if (typeof pertama === "number" || pertama === null || pertama === undefined) {
    return typeof args[1] === "string" ? args[1] : "";
  }
  if (Array.isArray(pertama)) return "";
  if (typeof pertama === "object" && pertama !== null) {
    if (typeof pertama.hostname === "string") return pertama.hostname;
    if (typeof pertama.host === "string") return pertama.host;
    if (typeof pertama.href === "string") {
      try {
        return new URL(pertama.href).hostname;
      } catch {
        return "";
      }
    }
  }
  return "";
}

// Path unix socket bukan jaringan keluar: dipakai lowdb untuk database lokal,
// jadi memblokirnya akan menggagalkan audit dengan error yang tidak berkaitan
// dengan kemandirian kapabilitas.
function pathUnix(args) {
  return typeof args[0] === "string" && !args[0].includes("://");
}

const fetchAsli = globalThis.fetch;
globalThis.fetch = function fetchDiblokir(input) {
  const url = typeof input === "string" ? input : (input?.url ?? input?.href ?? "");
  let host = "";
  try {
    host = url ? new URL(String(url)).hostname : "";
  } catch {
    host = "";
  }
  return Promise.reject(tolak("fetch", host));
};
// Properti turunan fetch asli ikut disalin: Undici masih mutate
// `fetch.preconnect`, dan menghapusnya bisa membuat modul lain gagal import
// dengan error yang tidak ada hubungannya dengan audit ini.
for (const kunci of ["preconnect", "Request", "Response", "Headers"]) {
  if (fetchAsli && kunci in fetchAsli) globalThis.fetch[kunci] = fetchAsli[kunci];
}

for (const [modul, nama] of [
  [http, "http"],
  [https, "https"],
]) {
  modul.request = function requestDiblokir(...args) {
    throw tolak(`${nama}.request`, hostDari(args));
  };
  modul.get = function getDiblokir(...args) {
    throw tolak(`${nama}.get`, hostDari(args));
  };
}

const connectAsli = net.Socket.prototype.connect;
net.Socket.prototype.connect = function connectDiblokir(...args) {
  if (pathUnix(args)) return connectAsli.apply(this, args);
  throw tolak("net.Socket#connect", hostDari(args));
};
net.connect = function netConnectDiblokir(...args) {
  if (pathUnix(args)) return new net.Socket();
  throw tolak("net.connect", hostDari(args));
};
net.createConnection = net.connect;
if (typeof tls.connect === "function") {
  tls.connect = function tlsConnectDiblokir(...args) {
    if (pathUnix(args)) return new tls.TLSSocket(new net.Socket());
    throw tolak("tls.connect", hostDari(args));
  };
}

// ── kasus uji ────────────────────────────────────────────────────────────────
//
// Satu baris per jalur nyata yang dipakai plugin, bukan satu baris per
// kapabilitas: `applies` memecah satu kapabilitas menjadi jalur yang berbeda,
// dan jalur satu bisa mandiri sementara jalur lain tidak. `spotify` unduh
// dilayani `spotyloader`, `spotify` cari tidak punya backend lokal sama sekali —
// menjadikannya satu baris akan menyembunyikan pemisahan yang justru sedang
// diukur.
//
// Argumen sengaja berbentuk nyata (link open.spotify.com, `kind` hd, file gambar
// di disk) supaya guard di dalam backend tidak menolak lebih dulu dan
// pengukuran benar-benar sampai ke percobaan tier.
const GAMBAR = path.join(os.tmpdir(), "audit-offline-1x1.png");
// 1x1 PNG dari byte literal, supaya skrip tidak bergantung pada aset repo dan
// tidak pernah mengambil jalur unduh apa pun.
fs.writeFileSync(
  GAMBAR,
  Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
    "base64",
  ),
);

const KASUS = [
  {
    label: "spotify/unduh",
    args: { url: "https://open.spotify.com/track/4cOdK2wGLETKBW3PvgPWqT" },
  },
  {
    label: "spotify/cari",
    args: { q: "neffex grateful" },
    catatan:
      "tidak ada backend local untuk { q }: spotyloader menerima URL, bukan kata kunci — backend ytmusic dicabut di Task 3 justru karena tidak pernah menghasilkan ID track Spotify",
  },
  { label: "pinterest/pin", args: { url: "https://id.pinterest.com/pin/9988776655/" } },
  {
    label: "pinterest/cari",
    args: { q: "kucing lucu" },
    catatan: "tidak ada backend local untuk { q }: ilovepin menerima link pin, bukan kata kunci",
  },
  { label: "douyin/unduh", args: { url: "https://v.douyin.com/iRNBho7u/" } },
  { label: "sfile/unduh", args: { url: "https://sfile.mobi/9xK2mLpQ" } },
  { label: "videy/unduh", args: { url: "https://videy.co/v?id=abcdef12" } },
  { label: "youtube/mp3", args: { url: "https://youtu.be/dQw4w9WgXcQ", format: "mp3" } },
  { label: "youtube/mp4", args: { url: "https://youtu.be/dQw4w9WgXcQ", format: "mp4" } },
  { label: "ytmusic/cari", args: { q: "neffex grateful" } },
  { label: "hd/imglarger", args: { kind: "imglarger", media: GAMBAR } },
  {
    label: "hd/unblur",
    args: { kind: "unblur", media: GAMBAR },
    // Verdict produksi yang TIDAK bisa diukur oleh harness ini. Alasannya
    // disalin dari hd.js:124-129: tier lokal hanya menang kalau render selesai
    // di dalam budget 3000ms, dan `pollEnhanceTask` adalah loop yang berjalan
    // puluhan detik di luar budget itu. Di sini koneksi ditolak dalam milidetik
    // sehingga urutannya terlihat lokal — itulah batas harness, bukan resultado.
    produksi: {
      mandiri: false,
      alasan: "di produksi lokal kalah balapan polling: budget lokal 3000ms, pollEnhanceTask berjalan puluhan detik (hd.js:124-129)",
    },
  },
];

// Jaring pengaman tambahan. Resolver sudah punya budget 8000ms sendiri; angka
// ini mencegah satu backend yang tidak pernah selesai menggantung audit selamanya.
const BATAS_KASUS_MS = 25_000;

function denganBatas(karya, ms) {
  return new Promise((selesai, gagal) => {
    const timer = setTimeout(() => gagal(new Error(`kasus melebihi ${ms}ms`)), ms);
    Promise.resolve(karya).then(
      (nilai) => {
        clearTimeout(timer);
        selesai(nilai);
      },
      (error) => {
        clearTimeout(timer);
        gagal(error);
      },
    );
  });
}

async function jalankan(kasus) {
  const namaKap = kasus.label.split("/")[0];
  const mod = await import(`../src/capabilities/${namaKap}.js`);
  const cap = mod.default ?? mod;

  // Backend mana yang benar-benar dijalankan resolver, dalam urutan. Ini dibaca
  // dari pemanggilan `run` yang diinstrumentasi, BUKAN dari `error.tried`:
  // `tried` hanya ada kalau semua backend gagal, jadi sebuah backend yang
  // berhasil offline akan hilang dari catatan dan dilaporkan sebagai bukan
  // mandiri — kebalikan dari kebenaran.
  //
  // yang dipatch adalah objek backend-nya, bukan array `backends`. Array dan
  // objek di dalamnya adalah nilai biasa (hanya binding ekspornya yang tidak
  // bisa diubah), jadi menimpa `run` satu backend tidak merusak modul lain.
  const urutan = [];
  const asli = cap.backends.map((b) => {
    const runAsli = b.run;
    return [b, runAsli];
  });
  for (const [b, runAsli] of asli) {
    b.run = (...args) => {
      urutan.push(b.name);
      return runAsli(...args);
    };
  }

  const melayani = cap.backends.filter(
    (b) => typeof b.applies !== "function" || b.applies(kasus.args) !== false,
  );
  const adaLokal = melayani.some((b) => b.kind === "local");
  // Bedakan "tier lokal tidak berlaku untuk jalur ini" dari "tier lokal hilang
  // dari modul ini". Keduanya terlihat sama dari `adaLokal`, tapi yang kedua
  // bocor: satu `kind: "lokal"` yang salah ketik menghapus backend lokal dari
  // modul, lalu setiap request langsung naik ke tier aggregator tanpa pernah
  // mencoba jalur mandiri — persis yang diblokir Phase 1. Yang pertama hanya
  // bentuk capability itu memang begitu sejak awal (lihat `catatan` di KASUS).
  const adaLokalDiModul = cap.backends.some((b) => b.kind === "local");

  percobaan = [];
  resetResolver();

  // Scraper yang gagal menulis ke console sendiri — `pindl.js:100` misalnya
  // `console.error("Error:", …)` lalu `return null`. Kalau dibiarkan, baris itu
  // muncul di tengah progres dan menutupi tabel. Ditangkap di sini dan dijadikan
  // bukti: kalau sebuah baris melaporkan "scraper complained: …", itu bukti
  // tambahan bahwa backend lokal benar-benar dicoba dan gagal karena jaringan
  // diblokir — bukan karena tidak pernah dihubungi.
  const keluhan = [];
  const consoleAsli = { log: console.log, error: console.error, warn: console.warn };
  const tangkap = (...args) => {
    keluhan.push(args.map((a) => (a instanceof Error ? a.message : String(a))).join(" ").slice(0, 160));
  };
  console.log = tangkap;
  console.error = tangkap;
  console.warn = tangkap;

  let hasil;
  let galat = null;
  try {
    hasil = await denganBatas(resolver.resolve(namaKap, kasus.args), BATAS_KASUS_MS);
  } catch (error) {
    galat = error;
  } finally {
    for (const [b, runAsli] of asli) b.run = runAsli;
    console.log = consoleAsli.log;
    console.error = consoleAsli.error;
    console.warn = consoleAsli.warn;
  }

  const backendPertama = cap.backends.find((b) => b.name === urutan[0]) ?? null;
  const terukurMandiri = backendPertama?.kind === "local";
  const mandri = terukurMandiri && kasus.produksi?.mandiri !== false;

  const hosts = [...new Set(percobaan.map((p) => p.host).filter((h) => h !== ""))];
  const agregatorTohok = hosts.filter(hostAggregator);

  return {
    label: kasus.label,
    mandri,
    terukurMandiri,
    adaLokal,
    adaLokalDiModul,
    backendPertama: backendPertama?.name ?? "—",
    // `source` hanya ada kalau resolve() berhasil, yang di harness ini hampir
    // tidak pernah terjadi (lihat catatan di kepala berkas). `tried[0]` dipakai
    // sebagai fallback supaya kolom tidak pernah kosong tanpa penjelasan.
    dipakai: hasil?.source ?? galat?.tried?.[0]?.name ?? backendPertama?.name ?? "—",
    agregatorTohok,
    kontak: percobaan.length,
    hasil: hasil ? "ok" : "gagal",
    keluhan,
    catatan: kasus.catatan ?? null,
    produksi: kasus.produksi ?? null,
  };
}

function pad(teks, lebar) {
  const s = String(teks);
  return s + " ".repeat(Math.max(0, lebar - s.length));
}

const hasil = [];
for (const kasus of KASUS) {
  process.stderr.write(`  audit: ${kasus.label} ...\n`);
  try {
    hasil.push(await jalankan(kasus));
  } catch (error) {
    // Kasus yang meledak di luar resolver (mis. impor modul gagal) tidak boleh
    // menghentikan seluruh audit: satu baris rusak tidak boleh menghilangkan
    // hasil sebelas baris lain.
    hasil.push({
      label: kasus.label,
      mandri: false,
      terukurMandiri: false,
      adaLokal: true,
      adaLokalDiModul: false,
      backendPertama: "—",
      dipakai: "—",
      agregatorTohok: [],
      kontak: 0,
      hasil: "error",
      catatan: `audit gagal menjalankan kasus: ${error?.message ?? error}`,
      produksi: kasus.produksi ?? null,
    });
  }
}

const lebarKap = Math.max(...hasil.map((h) => h.label.length), "KAPABILITAS".length);
const lebarBackend = Math.max(...hasil.map((h) => h.dipakai.length), "BACKEND DIPAKAI".length);
const baris = [
  `${pad("KAPABILITAS", lebarKap)}  ${pad("MANDIRI", 7)}  ${pad("BACKEND DIPAKAI", lebarBackend)}  CATATAN`,
];
for (const h of hasil) {
  const catatan = [
    h.produksi?.mandiri === false ? `tidak mandiri di produksi — ${h.produksi.alasan}` : null,
    h.catatan,
    h.terukurMandiri ? "urutan tier: lokal dulu" : h.adaLokal ? "BOCOR: agregator dihubungi lebih dulu" : "tidak ada backend local yang berlaku",
    h.agregatorTohok.length > 0 ? `host agregator dihubungi: ${h.agregatorTohok.join(", ")}` : null,
    // Bukti bahwa scraper lokal benar-benar dijalankan: log miliknya sendiri
    // yang tertangkap, bukan tebakan harness.
    h.keluhan?.length > 0 ? `scraper lokal gagal seperti seharusnya (${h.keluhan.length} log)` : null,
  ]
    .filter(Boolean)
    .join(" · ");
  baris.push(
    `${pad(h.label, lebarKap)}  ${pad(h.mandri ? "ya" : "tidak", 7)}  ${pad(h.dipakai, lebarBackend)}  ${catatan}`,
  );
}
console.log(baris.join("\n"));

const total = hasil.length;
const mandiri = hasil.filter((h) => h.mandri).length;
const persen = total === 0 ? 0 : Math.round((mandiri / total) * 100);
console.log("");
console.log(`${mandiri}/${total} kapabilitas mandiri tanpa network (${persen}%)`);

// Angka kedua, tidak disembunyikan: berapa yang terlihat lokal HANYA karena
// koneksi ditolak dalam milidetik. Selisih antara dua angka ini adalah ukuran
// kebohongan harness, dan harus menyusut kalau Kapitalnya membesar.
const terukur = hasil.filter((h) => h.terukurMandiri).length;
if (terukur !== mandiri) {
  console.log(
    `catatan harness: urutan tier terlihat lokal untuk ${terukur}/${total} jalur, tetapi ${terukur - mandiri} di antaranya tidak mandiri di produksi`,
  );
}

// Rollup per kapabilitas. Sebuah kapabilitas hanya dihitung mandiri kalau
// SEMUA jalurnya mandiri — memakai "ada satu jalur yang mandiri" akan melaporkan
// spotify sebagai mandiri padahal pencarian Spotify-nya masih lewat agregator.
const perKap = new Map();
for (const h of hasil) {
  const nama = h.label.split("/")[0];
  if (!perKap.has(nama)) perKap.set(nama, { total: 0, mandiri: 0, adaLokal: false });
  const k = perKap.get(nama);
  k.total += 1;
  k.adaLokal ||= h.adaLokal;
  if (h.mandri) k.mandiri += 1;
}
const semuaMandiri = [...perKap.values()].filter((k) => k.mandiri === k.total);
const adaLokalKap = [...perKap.values()].filter((k) => k.adaLokal);
console.log(
  `per kapabilitas: ${semuaMandiri.length}/${perKap.size} mandiri di semua jalurnya, ${adaLokalKap.length} punya backend local`,
);
for (const [nama, k] of perKap) {
  if (k.mandiri !== k.total) {
    process.stderr.write(`  ${nama}: ${k.mandiri}/${k.total} jalur mandiri — sisanya lewat agregator\n`);
  }
}

// Bocor = jalur yang punya backend local yang berlaku tapi resolver
// menghubungi agregator lebih dulu. Jalur tanpa backend local yang berlaku
// (pencarian spotify dan pinterest) BUKAN bocor: tidak ada jalur lokal yang
// bisa dihubungi lebih dulu, jadi exit code tetap berguna untuk dipakai CI.
const bocor = hasil.filter((h) => !h.terukurMandiri && h.adaLokal);
const hilangTierLokal = hasil.filter((h) => !h.adaLokalDiModul);
if (bocor.length > 0) {
  console.log("");
  console.log(`BOCOR: ${bocor.map((h) => h.label).join(", ")} — backend aggregator dihubungi padahal ada backend local yang berlaku`);
}
if (hilangTierLokal.length > 0) {
  console.log("");
  console.log(`BOCOR: ${hilangTierLokal.map((h) => h.label).join(", ")} — modul kapabilitasnya tidak punya backend kind "local" sama sekali`);
}

fs.rmSync(GAMBAR, { force: true });
process.exit(bocor.length > 0 || hilangTierLokal.length > 0 ? 1 : 0);
