// Masukkan plugin `is_owner` baru ke allow-list test di
// `oktz-plugins/src/common.rs`.
//
// Test `every_is_owner_plugin_is_known` menolak plugin owner yang belum
// terdaftar, jadi setiap port owner baru harus masuk daftar itu. Daftar itu
// milik orkestrator, bukan agent.
//
// Pemakaian: node scripts/tambah-owner-allowlist.mjs
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const SRC = "rust/plugin/src";
const COMMON = join(SRC, "common.rs");

function namaOwner(file) {
  const t = readFileSync(file, "utf8");
  if (!/is_owner\s*=\s*true/.test(t)) return null;
  const m = t.match(/name\s*=\s*"([^"]+)"/);
  return m ? m[1] : null;
}

const semua = [];
for (const cat of readdirSync(SRC)) {
  const dir = join(SRC, cat);
  try {
    for (const f of readdirSync(dir)) {
      if (!f.endsWith(".rs")) continue;
      const n = namaOwner(join(dir, f));
      if (n) semua.push(n);
    }
  } catch {
    /* bukan folder */
  }
}

const c = readFileSync(COMMON, "utf8");
const m = c.match(/const KNOWN: &\[&str\] = &\[[\s\S]*?\];/);
if (!m) {
  console.error("allow-list `known` tidak ditemukan di common.rs");
  process.exit(1);
}
const blok = m[0];
const isi = blok.slice(blok.indexOf("["), blok.lastIndexOf("]") + 1);
const ada = [...isi.matchAll(/"([^"]+)"/g)].map((x) => x[1]);
const baru = [...new Set(semua)].filter((n) => !ada.includes(n)).sort();

if (baru.length === 0) {
  console.log("allow-list sudah lengkap:", ada.length, "nama");
  process.exit(0);
}

// Sisipkan sebagai baris baru sebelum penutup, dengan kelipatan 10 per baris.
const baris = [];
for (let i = 0; i < baru.length; i += 10) {
  baris.push("            " + baru.slice(i, i + 10).map((n) => `"${n}",`).join(" "));
}
const tambahan = "\n" + baris.join("\n") + "\n        ";
// `blok` berakhir dengan `];` sehingga pola `\]$` tidak pernah cocok dan
// penulisan jadi no-op. Sisipkan tepat sebelum `]` penutup.
const potong = blok.lastIndexOf("]");
const baruBlok = blok.slice(0, potong) + tambahan + blok.slice(potong);
writeFileSync(COMMON, c.replace(blok, baruBlok));
console.log("ditambahkan", baru.length, "nama:", baru.join(", "));