// Cabut registrasi modul untuk berkas yang gagal dikompilasi.
//
// Pemakaian:
//   node scripts/cabut-modul.mjs daftar.txt
//
// `daftar.txt` berisi satu jalur relatif terhadap
// `rust/crates/oktz-plugins/src/` per baris (mis. `tools/ipwho.rs`).
// Skrip menghapus `pub mod ...;` yang menunjuk berkas itu, termasuk bentuk
// `#[path = "..."]`. Tidak pernah menyentuh berkas lain.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, basename, join } from "node:path";

const SRC = new URL("../rust/crates/oktz-plugins/src/", import.meta.url).pathname;
const daftar = readFileSync(process.argv[2], "utf8")
  .split("\n")
  .map((s) => s.trim())
  .filter(Boolean);

let dicabut = 0;
for (const rel of daftar) {
  const modPath = join(SRC, dirname(rel), "mod.rs");
  if (!existsSync(modPath)) {
    console.log("tanpa mod.rs:", rel);
    continue;
  }
  const isi = readFileSync(modPath, "utf8").split("\n");
  const nama = basename(rel, ".rs");
  const keluar = [];
  for (let i = 0; i < isi.length; i++) {
    const b = isi[i];
    // `#[path = "..."]` diikuti `pub mod x;`
    if (b.trim().startsWith("#[path")) {
      const cocok = b.includes(`"${basename(rel)}"`) || b.includes(`"${rel}"`);
      const lanjut = isi[i + 1] ?? "";
      if (cocok && /^\s*pub mod /.test(lanjut)) {
        console.log("cabut", rel, "->", lanjut.trim());
        i++;
        dicabut++;
        continue;
      }
    }
    if (
      new RegExp(`^\\s*pub mod (?:r#)?${nama.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")};`).test(b)
    ) {
      console.log("cabut", rel, "->", b.trim());
      dicabut++;
      continue;
    }
    keluar.push(b);
  }
  writeFileSync(modPath, keluar.join("\n"));
}
console.log("total dicabut:", dicabut);
