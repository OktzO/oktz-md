#!/usr/bin/env node
// Bagi 59 file `src/scraper/*.js` jadi beberapa grup untuk agent paralel.
//
// Aturan:
// - `dailymotion.js` dilewati (sudah diport orkestrator sebagai contoh acuan).
// - File yang saling mengimpor WAJIB satu grup, kalau tidak crate-nya tidak
//   bisa dikompilasi sebelum grup lain selesai (dependensi eksplisit di bawah).
// - Sasaran ukuran ~500 baris JS per grup; berkas >400 baris berdiri sendiri.
import { readFileSync, readdirSync, writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";

const REPO = new URL("..", import.meta.url).pathname;
const SRC = join(REPO, "src/scraper");
const KELUAR = "/tmp/opencode/scraper-batches";
const MAKS = 520;

// Kelompok yang tidak boleh dipisah: impor antar-scraper.
const WAJIB_BERSAMA = [
  ["geminiVision.js", "gemini.js"],
  ["aio.js", "ig.js", "pindl.js", "ytdl.js"],
];

const ukuran = new Map();
for (const f of readdirSync(SRC).filter((f) => f.endsWith(".js"))) {
  ukuran.set(f, readFileSync(join(SRC, f), "utf8").split("\n").length - 1);
}
ukuran.delete("dailymotion.js");

// Satuan packing: satu file, atau satu kelompok wajib-bersama.
const satuan = [];
const dipakai = new Set();
for (const grup of WAJIB_BERSAMA) {
  const isi = grup.filter((f) => ukuran.has(f));
  if (isi.length === 0) continue;
  isi.forEach((f) => dipakai.add(f));
  satuan.push({ isi, baris: isi.reduce((s, f) => s + ukuran.get(f), 0) });
}
for (const [f, baris] of ukuran) {
  if (!dipakai.has(f)) satuan.push({ isi: [f], baris });
}

// Berkas sangat besar berdiri sendiri.
const besar = satuan.filter((s) => s.baris > MAKS);
const kecil = satuan.filter((s) => s.baris <= MAKS);

// First-fit-decreasing: besar dulu supaya sisa ruang terisi rapi.
kecil.sort((a, b) => b.baris - a.baris);
const grup = besar.map((s) => ({ isi: [...s.isi], baris: s.baris }));
for (const s of kecil) {
  let target = null;
  for (const g of grup) {
    if (g.baris + s.baris <= MAKS) {
      target = g;
      break;
    }
  }
  if (!target) {
    target = { isi: [], baris: 0 };
    grup.push(target);
  }
  target.isi.push(...s.isi);
  target.baris += s.baris;
}

mkdirSync(KELUAR, { recursive: true });
grup.forEach((g, i) => {
  const nama = `S${String(i + 1).padStart(2, "0")}`;
  const baris = g.isi
    .map((f) => `${f}\t${ukuran.get(f)}\t${f.replace(/\.js$/, ".rs")}`)
    .join("\n");
  writeFileSync(join(KELUAR, `${nama}.tsv`), baris + "\n");
  console.log(`${nama}: ${g.isi.length} file, ${g.baris} baris — ${g.isi.join(", ")}`);
});
console.log(`total grup: ${grup.length}, total file: ${grup.reduce((s, g) => s + g.isi.length, 0)}`);
