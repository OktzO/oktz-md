// Sinkronkan `lib.rs` crate dengan folder yang benar-benar ada.
//
// Hanya untuk orkestrator: agent porting tidak boleh menyentuh `lib.rs`.
// Yang dipertahankan apa adanya: blok `//!`/`#![...]` paling atas, dan tiap
// item `pub mod` yang sudah punya atribut (`#[path]`, `#[allow]`) atau
// komentar sendiri. Baris lain (helper, re-export) tetap di bawah daftar.
//
// Pemakaian:
//   node scripts/sinkron-lib-plugin.mjs                      # oktz-plugins
//   node scripts/sinkron-lib-plugin.mjs rust/oktz-scraper/src
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = path.resolve(fileURLToPath(import.meta.url), '../..');
const ROOT = process.argv[2]
  ? path.resolve(REPO, process.argv[2])
  : path.join(REPO, 'rust/plugin/src');

const libPath = path.join(ROOT, 'lib.rs');
const lama = fs.existsSync(libPath) ? fs.readFileSync(libPath, 'utf8') : '';

function namaModul(nama) {
  const ganti = nama.replace(/-/g, '_');
  return /^\d/.test(ganti) ? `_${ganti}` : ganti;
}

const items = fs
  .readdirSync(ROOT, { withFileTypes: true })
  .filter((e) => e.isDirectory() || (e.isFile() && e.name.endsWith('.rs')))
  .map((e) => e.name.replace(/\.rs$/, ''))
  .filter((n) => n !== 'lib')
  .sort();

const barisLama = lama.split('\n');
const doc = [];
const itemLama = new Map();
const lain = [];

let pending = [];
for (const l of barisLama) {
  // Inner attribute (`#![...]`) hanya sah di awal crate, jadi ikut blok paling atas.
  if (/^(\/\/!|#!\[)/.test(l)) { doc.push(l); continue; }
  if (/^pub mod \w+;\s*$/.test(l)) {
    const d = l.match(/^pub mod (\w+);$/);
    const pathAttr = pending.find((x) => /^#\[path = "(.+)"\]$/.test(x));
    const file = pathAttr ? pathAttr.match(/"(.+)"/)[1].slice(0, -3) : d[1];
    itemLama.set(file, {
      deklarasi: l,
      tambahan: pending.filter((x) => !/^#\[path = /.test(x)),
    });
    pending = [];
    continue;
  }
  if (/^(#\[|\/\/)/.test(l)) { pending.push(l); continue; }
  if (l.trim() === '') { continue; }
  lain.push(l);
  pending = [];
}

const deklarasi = items.map((m) => {
  const bawaan = itemLama.get(m);
  const nama = bawaan ? bawaan.deklarasi.match(/^pub mod (\w+);$/)[1] : namaModul(m);
  const attrPath = nama === m ? '' : `#[path = "${m}.rs"]`;
  const atas = [...(bawaan?.tambahan ?? [])];
  const attrAllow = atas.filter((x) => /^#\[(?!path)/.test(x));
  const komentar = atas.filter((x) => /^\/\//.test(x));
  return [...komentar, attrAllow, attrPath].filter(Boolean).join('\n')
    + (attrPath || attrAllow.length ? '\n' : '')
    + `pub mod ${nama};`;
});

const isi =
  (doc.length ? doc.join('\n').replace(/\n+$/, '') + '\n\n' : '') +
  deklarasi.join('\n') +
  (lain.length ? '\n\n' + lain.join('\n') + '\n' : '\n');

if (isi !== lama) {
  fs.writeFileSync(libPath, isi);
  console.log(`sinkron lib.rs: ${items.length} modul`);
} else {
  console.log(`lib.rs sudah sinkron: ${items.length} modul`);
}