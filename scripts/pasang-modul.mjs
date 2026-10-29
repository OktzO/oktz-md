// Pasang deklarasi `pub mod` untuk berkas plugin yang belum terdaftar.
//
// Ditulis setelah `sync-plugin-mods.mjs` terbukti berbahaya: skrip lama itu
// menulis ulang mod.rs dan ikut mengaktifkan berkas yang sengaja tidak
// didaftarkan (28 modul `search` yang tidak pernah dikompilasi). Skrip ini
// HANYA menambah, tidak pernah menghapus, dan menolak berkas stub.
//
// Aturan:
//   - berkas dianggap plugin kalau memuat `#[plugin(` atau `#[answer_handler(`
//   - berkas stub (`let _ = ctx;` atau kepala `BLOKIRAN`) DILAPORKAN, bukan didaftarkan
//   - nama berkas dengan `-` atau `.` didaftarkan lewat `#[path = "..."]`
//
// Pemakaian:
//   node scripts/pasang-modul.mjs           # lihat saja
//   node scripts/pasang-modul.mjs --terapkan
import { readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { join, basename } from "node:path";

// Kata kunci Rust yang tidak boleh jadi nama modul telanjang.
const KATA_KUNCI = new Set([
  "self", "use", "mod", "fn", "type", "match", "move", "ref", "box", "in", "as", "if", "else",
  "for", "while", "loop", "return", "break", "continue", "let", "const", "static", "struct",
  "enum", "trait", "impl", "pub", "crate", "super", "where", "unsafe", "extern", "dyn", "async",
  "await", "try", "yield", "macro", "union", "do", "final", "become", "abstract", "override",
  "priv", "typeof", "unsized", "virtual",
]);

const SRC = process.argv.find((a) => a.startsWith("--src="))?.slice(6) ?? "rust/plugin/src";
const TERAPKAN = process.argv.includes("--terapkan");
// `--hanya=owner/addowner.rs,owner/akses.rs` membatasi ke berkas itu saja —
// supaya pemasangan bisa dicoba per batch kecil, bukan seluruhnya sekaligus.
const HANYA = process.argv
  .find((a) => a.startsWith("--hanya="))
  ?.slice(8)
  .split(",")
  .map((s) => s.trim())
  .filter(Boolean);

const semuaKategori = readdirSync(SRC, { withFileTypes: true })
  .filter((e) => e.isDirectory())
  .map((e) => e.name)
  .sort();

const kurang = [];
const stub = [];
for (const kat of semuaKategori) {
  const modPath = join(SRC, kat, "mod.rs");
  if (!existsSync(modPath)) continue;
  const isiMod = readFileSync(modPath, "utf8");
  const terdaftar = new Set([...isiMod.matchAll(/^pub mod (?:r#)?(\w+);/gm)].map((m) => m[1]));
  // Berkas yang sudah didaftarkan lewat `#[path = "..."]` juga sudah terpakai;
  // tanpa ini modul yang sama terdaftar dua kali dan plugin-nya dobel.
  const berkasTerdaftar = new Set(
    [...isiMod.matchAll(/^#\[path = "([^"]+)"\]/gm)].map((m) => m[1]),
  );
  const berkas = readdirSync(join(SRC, kat)).filter((f) => f.endsWith(".rs") && f !== "mod.rs");
  for (const f of berkas) {
    const teks = readFileSync(join(SRC, kat, f), "utf8");
    const namaModul = basename(f, ".rs").replace(/[^A-Za-z0-9_]/g, "_");
    if (terdaftar.has(namaModul) || berkasTerdaftar.has(f)) continue;
    if (!/#\[(plugin|answer_handler)\(/.test(teks)) continue;
    if (/let _ = ctx;/.test(teks) || /\bBLOKIRAN\b/.test(teks) || /belum tersedia/.test(teks)) {
      stub.push(`${kat}/${f}`);
      continue;
    }
    kurang.push({ kat, berkas: f, namaModul });
  }
}

console.log(`kategori: ${semuaKategori.length} | perlu didaftarkan: ${kurang.length} | stub terlewat: ${stub.length}`);
const dipilih = HANYA ? kurang.filter((k) => HANYA.includes(`${k.kat}/${k.berkas}`)) : kurang;
for (const s of stub) console.log(`  stub (tidak didaftarkan): ${s}`);
for (const k of dipilih) console.log(`  + ${k.kat}/${k.berkas} -> ${k.namaModul}`);

if (!TERAPKAN || dipilih.length === 0) {
  if (!TERAPKAN) console.log("(kering; jalankan dengan --terapkan)");
  process.exit(0);
}

const perKategori = new Map();
for (const k of dipilih) {
  if (!perKategori.has(k.kat)) perKategori.set(k.kat, []);
  perKategori.get(k.kat).push(k);
}

for (const [kat, daftar] of perKategori) {
  const modPath = join(SRC, kat, "mod.rs");
  const lama = readFileSync(modPath, "utf8");
  const tambah = daftar
    .map((k) => {
      // `self.rs` dan `use.rs` bentrok dengan kata kunci Rust. `self`, `super`,
      // dan `crate` TIDAK boleh jadi raw identifier, jadi pakai akhiran `_`.
      const nama = k.namaModul === "self" || k.namaModul === "super" || k.namaModul === "crate"
        ? `${k.namaModul}_`
        : KATA_KUNCI.has(k.namaModul)
          ? `r#${k.namaModul}`
          : k.namaModul;
      return k.berkas === `${k.namaModul}.rs`
        ? `pub mod ${nama};`
        : `#[path = "${k.berkas}"]\npub mod ${nama};`;
    })
    .join("\n");
  writeFileSync(modPath, `${lama.trimEnd()}\n\n// Ditambahkan scripts/pasang-modul.mjs.\n${tambah}\n`);
}
console.log(`selesai: ${dipilih.length} modul didaftarkan di ${perKategori.size} kategori`);
