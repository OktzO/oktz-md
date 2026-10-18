// Buang plugin yang masih berupa stub (belum benar-benar di-port).
//
// Stub = berkas plugin yang dibalasnya hanya pesan "belum tersedia", bukan
// perilaku JS produksi. Selama masih terdaftar di `mod.rs`, bot Rust membalas
// sesuatu yang tidak pernah dibalas bot JS — itu pelanggaran paritas.
//
// Deteksi: badannya memakai `let _ = ctx;` atau kepalanya memuat `BLOKIRAN`.
//
// Apa yang dilakukan:
//   1. hapus berkas stub
//   2. hapus deklarasi `pub mod`-nya (beserta `#[path = "..."]` di atasnya)
//      dari `mod.rs` kategori yang sama
//   3. salin catatan stubnya ke `docs/rust-rewrite/BLOCKED.md`
//
// Pemakaian:
//   node scripts/buang-stub-plugin.mjs            # lihat saja
//   node scripts/buang-stub-plugin.mjs --terapkan # jalankan
import { readFileSync, writeFileSync, readdirSync, rmSync, existsSync } from "node:fs";
import { join, basename } from "node:path";

const SRC = "rust/plugin/src";
const BLOCKED = "docs/rust-rewrite/BLOCKED.md";
const TERAPKAN = process.argv.includes("--terapkan");

function berkasRs(dir, out = []) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name);
    if (e.isDirectory()) berkasRs(p, out);
    else if (e.name.endsWith(".rs") && e.name !== "mod.rs") out.push(p);
  }
  return out;
}

const stub = [];
for (const f of berkasRs(SRC)) {
  const t = readFileSync(f, "utf8");
  if (!/#\[plugin\(/.test(t)) continue;
  if (/let _ = ctx;/.test(t) || /^\/\/!.*BLOKIRAN/m.test(t)) {
    const nama = (t.match(/name\s*=\s*"([^"]+)"/) || [, basename(f)])[1];
    const catatan = t
      .split("\n")
      .filter((l) => l.startsWith("//!"))
      .join("\n");
    stub.push({ berkas: f, nama, catatan });
  }
}

if (stub.length === 0) {
  console.log("tidak ada stub yang terdaftar");
  process.exit(0);
}

console.log(`${stub.length} stub:`);
for (const s of stub) console.log(`  ${s.berkas} -> ${s.nama}`);

if (!TERAPKAN) {
  console.log("(kering; jalankan dengan --terapkan untuk mengubah)");
  process.exit(0);
}

const catatanBaru = [];
for (const s of stub) {
  const t = readFileSync(s.berkas, "utf8");
  rmSync(s.berkas);
  catatanBaru.push(`### ${s.nama} (\`${s.berkas}\`)`);
  catatanBaru.push("");
  catatanBaru.push(s.catatan.replace(/^\/\/! ?/gm, "").trim());
  catatanBaru.push("");
}

// Hapus deklarasi dari mod.rs kategori.
const kategori = new Set(stub.map((s) => s.berkas.split("/src/")[1].split("/")[0]));
for (const kat of kategori) {
  const mod = join(SRC, kat, "mod.rs");
  if (!existsSync(mod)) continue;
  const baris = readFileSync(mod, "utf8").split("\n");
  const namaModul = new Set(
    stub
      .filter((s) => s.berkas.split("/src/")[1].split("/")[0] === kat)
      .map((s) => basename(s.berkas, ".rs").replace(/-/g, "_")),
  );
  const out = [];
  for (let i = 0; i < baris.length; i++) {
    const m = baris[i].match(/^pub mod (\w+);/);
    if (m && namaModul.has(m[1])) {
      if (out.length > 0 && /^#\[path = /.test(out[out.length - 1])) out.pop();
      continue;
    }
    out.push(baris[i]);
  }
  writeFileSync(mod, out.join("\n"));
}

const lama = readFileSync(BLOCKED, "utf8");
writeFileSync(
  BLOCKED,
  `${lama.trimEnd()}\n\n## 12. Stub yang dibuang pada gelombang agent pertama\n\nPlugin berikut sempat dibuatkan berkas stub oleh agent porting (badannya cuma\npesan "belum tersedia"), lalu dibuang dari registrasi karena bukan paritas JS.\nPort ulang setelah kapabilitasnya ada.\n\n${catatanBaru.join("\n")}\n`,
);
console.log("selesai: berkas dihapus, mod.rs dibersihkan, BLOCKED.md diperbarui");
