#!/usr/bin/env node
// Bagi plugin yang belum ter-port menjadi gelombang agent paralel.
//
// Pembagian hanya berdasarkan nama file: satu agent satu kelompok file,
// tidak ada file yang muncul di dua kelompok. Jumlah agent = jumlah folder
// brief yang ditulis, bukan jumlah plugin.
//
// Keluaran: `/tmp/opencode/gelombang-<n>.md` (brief per agent) dan daftar
// ringkas di stdout.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";

const JUMLAH = Number(process.argv[2] || 6);
const bebas = readFileSync("/tmp/opencode/bebas.txt", "utf8").trim().split("\n");
if (bebas.length === 0) {
  console.log("tidak ada plugin bebasCapability yang tersisa");
  process.exit(0);
}

mkdirSync("/tmp/opencode", { recursive: true });

// Bagi rata dengan sisanya distributes across the agents.
const kelompok = Array.from({ length: JUMLAH }, () => []);
bebas.forEach((f, i) => kelompok[i % JUMLAH].push(f));

kelompok.forEach((daftar, i) => {
  if (daftar.length === 0) return;
  const n = i + 1;
  const isi = `# Gelombang ${n} dari ${JUMLAH}

Baca \`docs/rust-rewrite/PORT-BRIEF.md\` DULU dan ikuti seluruh aturannya.
Bagian ini supplementing brief; bagian yang bertentangan,
brief yang menang.

## Plugin milikmu (${daftar.length} file)

${daftar.map((f) => `- \`${f}\``).join("\n")}

## Pembagian file

Folder Rust hasil: mengikuti struktur folder JS. Untuk setiap plugin JS di
\`plugins/<kategori>/<file>.js\`, buat
\`rust/plugin/src/<kategori>/<file>.rs\`.

Kalau dua file JS di daftarmu berakhiran nama sama setelah normalisasi
(hanya berbeda ekstensi), pakai nama Rust yang berbeda dan tulis catatan
di kedua file.

## mod.rs

DILARANG menyentuh \`mod.rs\` mana pun. Orkestrator akan menambah baris
\`pub mod\` setelah kamu selesai. Kalau folder kategorimu belum ada sama
sekali, buat folder dan \`mod.rs\` kosong (tanpa isinya) di folder itu saja.

## Catatan tambahan

- Kalau sebuah plugin ternyata butuh kapabilitas yang belum ada (sock, fs,
  sharp, ffmpeg, entry point kedua, AIRich), JANGAN port paksa. Tulis
  di kepala file plugin itu blokirannya dan lanjutkan ke plugin berikutnya.
- Prioritaskan plugin yang paling sederhana lebih dulu supaya semua file ada walau waktu habis.
`;
  writeFileSync(`/tmp/opencode/gelombang-${n}.md`, isi);
  console.log(`gelombang ${n}: ${daftar.length} plugin -> /tmp/opencode/gelombang-${n}.md`);
});

console.log(`total ${bebas.length} plugin dibagi ke ${kelompok.filter((k) => k.length > 0).length} agent`);