// Sinkronkan `mod.rs` tiap kategori dengan file `.rs` yang ada di dalamnya.
//
// Hanya untuk orkestrator: agent porting tidak boleh menyentuh `mod.rs`.
// Baris non-`pub mod` yang sudah ada di `mod.rs` (helper, re-export) tetap
// dipertahankan dan diletakkan setelah daftar modul.

import fs from "node:fs";
import path from "node:path";

const ROOT =
  process.argv[2] ||
  path.resolve(import.meta.dirname, "../rust/crates/oktz-plugins/src");

// Nama file plugin bisa mengandung `-` (`jtukowi-ai.rs`) atau keyword Rust
// (`use.rs`), sedangkan nama modul harus identifier. Nama modulnya di-slash ke
// `_`; nama plugin di dalam file tetap nama JS aslinya.
function namaModul(file) {
  const ganti = file.replace(/-/g, "_");
  return /^\d/.test(ganti) ? `_${ganti}` : ganti;
}

for (const dir of fs.readdirSync(ROOT, { withFileTypes: true })) {
  if (!dir.isDirectory()) continue;
  const full = path.join(ROOT, dir.name);
  const mods = fs
    .readdirSync(full)
    .filter((f) => f.endsWith(".rs") && f !== "mod.rs")
    .map((f) => f.slice(0, -3))
    .sort();

  if (mods.length === 0) continue;

  const modPath = path.join(full, "mod.rs");
  const lama = fs.existsSync(modPath) ? fs.readFileSync(modPath, "utf8") : "";

  // `#[path = "use.rs"] pub mod use_item;` — nama modul tidak selalu sama
  // dengan nama file, jadi pasangan yang sudah ada dipakai ulang apa adanya.
  // Tanpa ini, sinkron akan menulis `pub mod use;` yang keyword.
  const namaDariPath = new Map();
  const lain = [];
  // Blok `//!` harus tetap paling atas: `//!` hanya sah sebelum item pertama,
  // jadi kalau ikut tersapu ke bawah, `mod.rs` gagal dikompilasi.
  let doc = [];
  const barisLama = lama.split("\n");
  for (let i = 0; i < barisLama.length; i += 1) {
    const l = barisLama[i];
    if (i === 0 && /^(\/\/!|\/\/|$)/.test(l)) {
      doc.push(l);
      continue;
    }
    if (/^\/\/!/.test(l)) {
      doc.push(l);
      continue;
    }
    if (l.trim() === "" || /^pub mod \w+;\s*$/.test(l)) {
      const p = barisLama[i - 1]?.match(/^#\[path = "([^"]+)"\]$/);
      const d = l.match(/^pub mod (\w+);$/);
      if (p && d) namaDariPath.set(p[1].slice(0, -3), d[1]);
      continue;
    }
    // Atribut `#[path]` selalu ditulis ulang inline di bawah, jadi jangan
    // ikut ditulis ulang di bawah — kalau tidak, makanya jadi dua kali.
    if (/^#\[path = "[^"]+"\]$/.test(l)) continue;
    lain.push(l);
  }

  const deklarasi = mods.map((m) => {
    const nama = namaDariPath.get(m) ?? namaModul(m);
    const attr = nama === m ? "" : `#[path = "${m}.rs"]\n`;
    return `${attr}pub mod ${nama};`;
  });

  const isi =
    (doc.length ? doc.join("\n").replace(/\n+$/, "") + "\n\n" : "") +
    deklarasi.join("\n") +
    (lain.length ? "\n\n" + lain.join("\n") + "\n" : "\n");

  if (isi !== lama) {
    fs.writeFileSync(modPath, isi);
    console.log(`sinkron ${dir.name}: ${mods.length} modul`);
  }
}