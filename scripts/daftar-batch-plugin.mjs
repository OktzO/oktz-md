// Daftar plugin JS yang belum punya padanan Rust + bagi menjadi batch.
//
// Pemakaian:
//   node scripts/daftar-batch-plugin.mjs            -> tulis /tmp/opencode/batches/
//   node scripts/daftar-batch-plugin.mjs <folder>   -> tulis ke folder itu
//
// Batch dibagi per kategori supaya satu agent = satu folder mod.rs.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseJsConfig, parseRsMeta, semuaFileRs, semuaFileJs, JS_ROOT, REPO }
  from './plugin-meta.mjs';

const KELUARAN = process.argv[2]
  || '/tmp/opencode/batches';

const rsNames = new Set();
for (const f of semuaFileRs()) {
  const m = parseRsMeta(fs.readFileSync(f, 'utf8'));
  if (m?.name) rsNames.add(m.name);
}

const belum = [];
for (const f of semuaFileJs()) {
  const cfg = parseJsConfig(fs.readFileSync(f, 'utf8'));
  if (!cfg?.name) continue;
  if (rsNames.has(cfg.name)) continue;
  belum.push([
    path.relative(JS_ROOT, f).replace(/\.js$/, ''),
    cfg.name,
    cfg.category || '',
    cfg.isOwner === 'true' ? 'owner' : '',
  ]);
}
belum.sort();

console.log(`sudah ter-port: ${rsNames.size} plugin Rust`);
console.log(`belum: ${belum.length}`);

const blocked = fs.readFileSync(path.join(REPO, 'docs/rust-rewrite/BLOCKED.md'), 'utf8');
const namaBlocked = new Set();
const polaBlocked = [];
for (const m of blocked.matchAll(/([a-z0-9_.*/-]+)\.(js|rs)/g)) {
  const base = m[1];
  if (!base.includes('*')) { namaBlocked.add(base.split('/').pop()); continue; }
  if (base === '*' || base.endsWith('/*')) continue;
  polaBlocked.push(new RegExp(
    '^' + base.split('*').map(s => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('.*') + '$'
  ));
}
for (const m of blocked.matchAll(/`([a-z0-9_.*-]+)\/([a-z0-9_.*-]+)`/g)) {
  namaBlocked.add(m[2]);
}
const namaPolos = new Set();
for (const m of blocked.matchAll(/`([a-z0-9-]+)`/g)) namaPolos.add(m[1]);

const tetap = belum.filter(([rel, name]) => {
  const base = rel.split('/').pop();
  return !namaBlocked.has(base) && !namaBlocked.has(name)
    && !polaBlocked.some((p) => p.test(rel) || p.test(base) || p.test(name));
});
for (const nama of namaPolos) {
  for (let i = tetap.length - 1; i >= 0; i--) {
    if (tetap[i][1] === nama || tetap[i][0].split('/').pop() === nama) tetap.splice(i, 1);
  }
}

const per = {};
for (const [rel] of tetap) { const k = rel.split('/')[0]; per[k] = (per[k] || 0) + 1; }
console.log(`kandidat: ${tetap.length} (dari ${belum.length}; ${belum.length - tetap.length} terblokir)`);
for (const [k, n] of Object.entries(per).sort((a, b) => b[1] - a[1])) console.log(`${String(n).padStart(4)} ${k}`);

const urut = Object.keys(per).sort();
const batch = [];
let cur = [];
for (const k of urut) {
  for (const row of tetap.filter(r => r[0].split('/')[0] === k)) {
    cur.push(row);
    if (cur.length === 15) { batch.push(cur); cur = []; }
  }
}
if (cur.length) batch.push(cur);

fs.mkdirSync(KELUARAN, { recursive: true });
let n = 12;
const index = [];
for (const b of batch) {
  const tag = `B${String(n).padStart(2, '0')}`;
  fs.writeFileSync(path.join(KELUARAN, `${tag}.txt`), b.map(r => r[0]).join('\n') + '\n');
  fs.writeFileSync(path.join(KELUARAN, `${tag}.tsv`), b.map(r => r.join('\t')).join('\n') + '\n');
  index.push(`${tag}  ${b.length} file  ${[...new Set(b.map(r => r[0].split('/')[0]))].join(', ')}`);
  n++;
}
fs.writeFileSync(path.join(KELUARAN, 'belum-ported.tsv'), belum.map(r => r.join('\t')).join('\n') + '\n');
console.log(index.join('\n'));
console.log(`total: ${batch.length} batch (B12..B${String(n - 1).padStart(2, '0')})`);