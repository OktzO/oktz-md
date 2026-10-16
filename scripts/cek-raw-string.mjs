#!/usr/bin/env node
// Cari raw string Rust yang isinya bukan data, tapi kode.
//
// Gejalanya sangat menyesatkan: `r#"` yang ditutup dengan `".` (bukan `"#`)
// membuat seluruh isi string — termasuk fungsi dan test — ditelan lexer. Kode
// di dalamnya tidak pernah dikompilasi: test tidak terdaftar, type error di
// dalamnya tidak muncul, dan test lain terlihat memakai body yang salah.
// Satu kasus nyata: `sfiledl.rs:741` (gelombang scraper pertama).
//
// Yang dicek: setiap raw string, apakah isinya mengandung jejak Rust.
// Data HTML/JSON hampir tidak pernah memuat `fn `, `#[test]`, `assert`,
// `let `, atau `};` di awal baris — kode selalu punya.
//
// Pemakaian: node scripts/cek-raw-string.mjs [akar...]
import fs from "node:fs";
import path from "node:path";

const REPO = path.resolve(new URL("..", import.meta.url).pathname);
const AKAR = process.argv.slice(2);
const DAFKAJ = AKAR.length ? AKAR : ["rust/crates"];

// Hanya jejak kuat. `}` sendirian dan `=> {` dipakai juga oleh JSON/HTML,
// jadi tidak boleh jadi bukti.
const JEJAK = [
  /\bfn\s+[a-z_]\w*\s*\(/,
  /#\[(test|cfg|derive|allow)\b/,
  /\bassert(_eq|_ne)?!/,
  /^\s*(let|impl|struct|enum|mod|use)\s/m,
];

let temuan = 0;
let diperiksa = 0;

function periksaFile(p) {
  const src = fs.readFileSync(p, "utf8");
  let i = 0;
  while (i < src.length) {
    const m = /^r(#+)"/.exec(src.slice(i, i + 40));
    if (!m) {
      i += 1;
      continue;
    }
    const hash = m[1].length;
    const tutup = `"${"#".repeat(hash)}`;
    const akhir = src.indexOf(tutup, i + m[0].length);
    if (akhir === -1) {
      const baris = src.slice(0, i).split("\n").length;
      console.log(`DITEMUKAN  ${p}:${baris}  raw string r${hash}" tanpa penutup`);
      temuan += 1;
      return;
    }
    const isi = src.slice(i + m[0].length, akhir);
    const kena = JEJAK.filter((re) => re.test(isi));
    if (kena.length > 0) {
      const baris = src.slice(0, i).split("\n").length;
      const cuplikan = isi.split("\n").find((l) => kena[0].test(l)) ?? "";
      console.log(
        `DITEMUKAN  ${p}:${baris}  isi raw string(${isi.length} char) berisi kode: ` +
          `${kena.length} jejak, contoh: ${cuplikan.trim().slice(0, 60)}`,
      );
      temuan += 1;
    }
    i = akhir + tutup.length;
  }
}

function jalan(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    if (e.name === "target" || e.name.startsWith(".")) continue;
    const p = path.join(dir, e.name);
    if (e.isDirectory()) jalan(p);
    else if (e.name.endsWith(".rs")) {
      diperiksa += 1;
      periksaFile(p);
    }
  }
}

for (const a of DAFKAJ) {
  const penuh = path.resolve(REPO, a);
  if (!fs.existsSync(penuh)) {
    console.log(`lewat: ${a} tidak ada`);
    continue;
  }
  jalan(penuh);
}

console.log(
  temuan === 0
    ? `bersih: ${diperiksa} file .rs, tidak ada raw string berisi kode`
    : `${temuan} masalah pada ${diperiksa} file .rs`,
);
process.exit(temuan === 0 ? 0 : 1);