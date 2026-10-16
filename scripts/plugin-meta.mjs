// Parser metadata plugin bersama untuk Rust dan JS.
//
// Rust: Besides `#[plugin(name = "..", ..)]`, crate ini memakai tiga makro
// posisional di `common.rs` (`cek_plugin!`, `tanya_plugin!`, `data_plugin!`),
// jadi hanya membaca `#[plugin]` akan menghitung plugin sebagai belum ada.
//
// Jebakan yang sudah kejadian: baris `//!` boleh menyebut `#[plugin(...)]`
// di dalam penjelasan; `indexOf` pertama akan salah mendarat ke sana dan
// scan kurungnya kacau, jadi baris komentar dibuang dulu saat mencari.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const REPO = path.resolve(fileURLToPath(import.meta.url), '../..');
export const JS_ROOT = path.join(REPO, 'plugins');
export const RS_ROOT = path.join(REPO, 'rust/crates/oktz-plugins/src');

export function parseJsConfig(src) {
  const i = src.indexOf('pluginConfig');
  if (i < 0) return null;
  const eq = src.indexOf('=', i);
  if (eq < 0) return null;
  let j = eq + 1;
  while (j < src.length && /\s/.test(src[j])) j++;
  if (src[j] !== '{') return null;
  let depth = 0, k = j;
  for (; k < src.length; k++) {
    const c = src[k];
    if (c === '{') depth++;
    else if (c === '}') { depth--; if (depth === 0) break; }
    else if (c === '`') { k++; while (k < src.length && src[k] !== '`') { if (src[k] === '\\') k++; k++; } }
    else if (c === '"' || c === "'") { const q = c; k++; while (k < src.length && src[k] !== q) { if (src[k] === '\\') k++; k++; } }
  }
  const body = src.slice(j + 1, k);
  const out = {};
  const re = /(?:^|[,\n])\s*(\w+)\s*:\s*('(?:[^'\\]|\\.)*'|"(?:[^"\\]|\\.)*"|`(?:[^`\\]|\\.)*`)/g;
  let m;
  while ((m = re.exec(body))) out[m[1]] = m[2].slice(1, -1).replace(/\\(.)/g, '$1');
  for (const key of ['alias']) {
    const km = new RegExp(`(?:^|[,\\n])\\s*${key}\\s*:\\s*\\[([^\\]]*)\\]`).exec(body);
    if (km) {
      out[`__${key}`] = [...km[1].matchAll(/'((?:[^'\\]|\\.)*)'|"((?:[^"\\]|\\.)*)"/g)]
        .map(x => (x[1] ?? x[2]).replace(/\\(.)/g, '$1'));
    }
  }
  return out;
}

function pemanggilan(src, nama) {
  const out = [];
  const re = new RegExp(`\\b${nama}!\\s*\\(`, 'g');
  let m;
  while ((m = re.exec(src))) {
    let depth = 1, k = m.index + m[0].length;
    for (; k < src.length && depth > 0; k++) {
      const c = src[k];
      if (c === '(') depth++;
      else if (c === ')') { depth--; if (depth === 0) break; }
      else if (c === '"') { k++; while (k < src.length && src[k] !== '"') { if (src[k] === '\\') k++; k++; } }
    }
    out.push(src.slice(m.index + m[0].length, k));
  }
  return out;
}

function stringArg(argumen) {
  const s = argumen.trim();
  if (!s.startsWith('"')) return null;
  let k = 1;
  while (k < s.length) {
    if (s[k] === '\\') { k += 2; continue; }
    if (s[k] === '"') break;
    k++;
  }
  return s.slice(1, k).replace(/\\(.)/g, '$1');
}

function aliasArg(argumen) {
  const m = /^\s*\[([^\]]*)\]/.exec(argumen);
  if (!m) return [];
  return [...m[1].matchAll(/"((?:[^"\\]|\\.)*)"/g)].map(x => x[1].replace(/\\(.)/g, '$1'));
}

const MAKRO = { cek_plugin: 'cek', tanya_plugin: 'fun', data_plugin: 'fun' };

export function parseRsMeta(src) {
  const cari = src
    .split("\n")
    .filter((l) => !/^\s*\/\//.test(l))
    .join("\n");
  const i = cari.indexOf("#[plugin(");
  if (i >= 0) {
    let depth = 1, k = i + "#[plugin(".length;
    for (; k < cari.length; k++) {
      const c = cari[k];
      if (c === '(') depth++;
      else if (c === ')') { depth--; if (depth === 0) break; }
      else if (c === '"') { k++; while (k < cari.length && cari[k] !== '"') { if (cari[k] === '\\') k++; k++; } }
    }
    const out = { __sumber: '#[plugin]' };
    const re = /(\w+)\s*=\s*("(?:[^"\\]|\\.)*"|\[[^\]]*\]|true|false|-?\d+(?:\.\d+)?)/g;
    let m;
    while ((m = re.exec(cari.slice(i, k)))) {
      let v = m[2];
      if (v.startsWith('"')) v = v.slice(1, -1).replace(/\\(.)/g, '$1');
      else if (v.startsWith('[')) v = [...v.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map(x => x[1].replace(/\\(.)/g, '$1'));
      else if (v === 'true') v = true;
      else if (v === 'false') v = false;
      else v = Number(v);
      out[m[1]] = v;
    }
    if (out.name) return out;
  }
  for (const [makro, kategori] of Object.entries(MAKRO)) {
    const panggil = pemanggilan(cari, makro);
    if (!panggil.length) continue;
    const arg = [];
    let depth = 0, cur = '';
    for (const c of panggil[0]) {
      if (c === '(' || c === '[' || c === '{') depth++;
      else if (c === ')' || c === ']' || c === '}') depth--;
      if (c === ',' && depth === 0) { arg.push(cur); cur = ''; continue; }
      cur += c;
    }
    arg.push(cur);
    const name = stringArg(arg[1] ?? '');
    if (!name) continue;
    return {
      __sumber: `${makro}!`,
      name,
      alias: aliasArg(arg[2] ?? ''),
      category: kategori,
      description: stringArg(arg[3] ?? ''),
      usage: stringArg(arg[4] ?? ''),
      example: stringArg(arg[5] ?? ''),
      cooldown: makro === 'cek_plugin' ? 5 : 3,
      energi: 0,
      is_private: false,
    };
  }
  return null;
}

export function semuaFileRs() {
  const out = [];
  (function jalan(dir) {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) { jalan(p); continue; }
      if (e.name.endsWith('.rs')) out.push(p);
    }
  })(RS_ROOT);
  return out;
}

export function semuaFileJs() {
  const out = [];
  (function jalan(dir) {
    for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) { jalan(p); continue; }
      if (e.name.endsWith('.js')) out.push(p);
    }
  })(JS_ROOT);
  return out;
}