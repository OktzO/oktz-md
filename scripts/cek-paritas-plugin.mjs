// Cek paritas metadata plugin Rust <-> JS.
// JS adalah acuan. Semua bentuk registrasi Rust dikenali: `#[plugin(..)]`,
// `cek_plugin!`, `tanya_plugin!`, `data_plugin!` (lihat plugin-meta.mjs).
//
// Pemakaian:  node scripts/cek-paritas-plugin.mjs
import fs from 'node:fs';
import path from 'node:path';
import { parseJsConfig, parseRsMeta, semuaFileRs, semuaFileJs, JS_ROOT, REPO }
  from './plugin-meta.mjs';

const FIELD = [
  ['name', 'name'],
  ['__alias', 'alias'],
  ['category', 'category'],
  ['description', 'description'],
  ['usage', 'usage'],
  ['example', 'example'],
  ['cooldown', 'cooldown'],
  ['energi', 'energi'],
  ['isEnabled', 'is_enabled'],
  ['isOwner', 'is_owner'],
  ['isPremium', 'is_premium'],
  ['isGroup', 'is_group'],
  ['isPrivate', 'is_private'],
];

const rsByName = new Map();
let jumlahRs = 0;
const perSumber = {};
for (const f of semuaFileRs()) {
  const meta = parseRsMeta(fs.readFileSync(f, 'utf8'));
  if (!meta) continue;
  jumlahRs++;
  perSumber[meta.__sumber] = (perSumber[meta.__sumber] ?? 0) + 1;
  rsByName.set(meta.name, { file: path.relative(REPO, f), meta });
}

const mismatch = [];
const noRust = [];
let compared = 0;
let totalJs = 0;

for (const f of semuaFileJs()) {
  const cfg = parseJsConfig(fs.readFileSync(f, 'utf8'));
  if (!cfg?.name) continue;
  totalJs++;
  const hit = rsByName.get(cfg.name);
  if (!hit) { noRust.push(`${cfg.name}\t${path.relative(JS_ROOT, f)}`); continue; }
  compared++;
  const diffs = [];
  for (const [jk, rk] of FIELD) {
    const jv = cfg[jk];
    if (jv === undefined) continue;
    if (JSON.stringify(jv) !== JSON.stringify(hit.meta[rk])) {
      diffs.push(`${jk}: JS=${JSON.stringify(jv)} RS=${JSON.stringify(hit.meta[rk])}`);
    }
  }
  if (diffs.length) mismatch.push(`${cfg.name}  ${hit.file}\n    ${diffs.join('\n    ')}`);
}

console.log(`plugin terdaftar di Rust: ${jumlahRs}  ${JSON.stringify(perSumber)}`);
console.log(`plugin di JS (punya pluginConfig.name): ${totalJs}`);
console.log(`terbanding: ${compared}`);
console.log(`--- tidak ada padanan Rust (${noRust.length}) ---`);
console.log(noRust.join('\n'));
console.log(`--- field beda (${mismatch.length}) ---`);
console.log(mismatch.join('\n'));
if (mismatch.length) process.exitCode = 1;