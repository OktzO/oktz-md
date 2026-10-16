// Scan huruf non-Latin yang bocor ke file .rs / .md.
// Batas: ideogram CJK, kana, hangul, tanda baca CJK, fullwidth forms.
// Boleh: glyph tampilan yang memang dipakai bot JS (box drawing, 「」〔〕┈⬡㊗, emoji).
//
// Pemakaian:
//   node scripts/scan-nonlatin.mjs [folder] [ekstensi]
//   node scripts/scan-nonlatin.mjs                          -> crates/, .rs
//   node scripts/scan-nonlatin.mjs docs/rust-rewrite .md
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = path.resolve(fileURLToPath(import.meta.url), '../..');
const ROOT = path.resolve(REPO, process.argv[2] || 'rust/crates');
const ext = process.argv[3] || '.rs';

const IZIN = new Set([
  '\u256d', '\u2570', '\u256e', '\u2571', '\u256f', // ╭ ╰ ╮ ╱ ╯
  '\u2502', '\u2500', '\u251c', '\u2514', '\u2503', '\u2551', // │ ─ ├ └ ┣ ║
  '\u2524', '\u2566', '\u2569', '\u255a', '\u2560', '\u2554',
  '\u2563', '\u2567', '\u250c', '\u2510',
  '\u300c', '\u300d', '\u300e', '\u300f', // 「 」 『 』
  '\u3014', '\u3015', // 〔 〕  (dipakai energi.js)
  '\u3010', '\u3011', // 【 】
  '\u2508', '\u254c', // ┈ ╌
  '\u2b21', // ⬡
  '\u3297', '\u3299', // ㊗ ㊙
  '\u2588', '\u2584', '\u2580', '\u25a0', // █ ▄ ▀ ■
  '\u2192', '\u21a9', '\u21b3', '\u21d2', // → ↩ ↳ ⇒
  '\u00b7', '\u2022', '\u2026', // · • …
  '\u2013', '\u2014', '\u2018', '\u2019', '\u201c', '\u201d',
  '\u00a9', '\u00ae', '\u2122',
  '\ufe0f', '\ufe0e',
]);

const RANGES = [
  [0x2e80, 0x2fff], [0x3000, 0x303f], [0x3040, 0x30ff], [0x3100, 0x312f],
  [0x3130, 0x318f], [0x31f0, 0x31ff], [0x3400, 0x4dbf], [0x4e00, 0x9fff],
  [0xa960, 0xa97f], [0xac00, 0xd7af], [0xd7b0, 0xd7ff], [0xf900, 0xfaff],
  [0xfe30, 0xfe4f], [0xff00, 0xffef], [0x20000, 0x2ffff],
];

function nessa(ch) {
  const c = ch.codePointAt(0);
  return RANGES.some(([a, b]) => c >= a && c <= b);
}

const JELEKAH = [];
const DUI = new Map();

function jalan(dir) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) { jalan(p); continue; }
    if (!e.name.endsWith(ext)) continue;
    const lines = fs.readFileSync(p, 'utf8').split('\n');
    lines.forEach((line, i) => {
      for (const ch of line) {
        const c = ch.codePointAt(0);
        if (c === 0x200b || c === 0xfeff) continue;
        if (!nessa(ch)) continue;
        if (IZIN.has(ch)) continue;
        if (!DUI.has(ch)) DUI.set(ch, []);
        const arr = DUI.get(ch);
        if (arr.length < 4) arr.push(`${path.relative(REPO, p)}:${i + 1}`);
        JELEKAH.push(ch);
      }
    });
  }
}

jalan(ROOT);

console.log(`total huruf non-Latin di luar whitelist: ${JELEKAH.length}`);
if (JELEKAH.length === 0) {
  console.log('BERSIH');
} else {
  for (const [ch, arr] of DUI) {
    console.log(`U+${ch.codePointAt(0).toString(16).toUpperCase().padStart(4, '0')} ${JSON.stringify(ch)}  x${arr.length}  ${arr.join(', ')}`);
  }
  process.exitCode = 1;
}