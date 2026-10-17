// Bandingkan nama plugin byte-per-byte antara pluginConfig JS dan
// `#[plugin(name = ...)]` Rust.
//
// JS dibaca dengan cara yang sama untuk semua file: objek literal
// `pluginConfig`, lalu kunci `name`. Nilai bisa string tunggal atau array
// (mis. `['pakustad', 'pak-ustad', 'tanyaustad']`) — yang pertama jadi `name`,
// sisanya jadi alias, sama seperti `#[plugin]`.
//
// Keluaran: daftar nama yang hanya ada di JS (belum di-port) dan yang hanya
// ada di Rust (nama tak dikenal di produksi). Exit 1 kalau ada beda nama pada
// plugin yang sama-sama ada di kedua sisi.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname;

function walk(dir, ekstensi, out = []) {
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) walk(p, ekstensi, out);
    else if (e.endsWith(ekstensi)) out.push(p);
  }
  return out;
}

// `name: "x"` atau `name: ['a', 'b']` di dalam `pluginConfig`.
const POLA_NAMA = /\bname:\s*("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|\[[^\]]*\])/;

function namaDariJs(path) {
  const teks = readFileSync(path, "utf8");
  const m = teks.match(POLA_NAMA);
  if (!m) return null;
  let v = m[1].trim();
  if (v.startsWith("[")) {
    const pertama = v.match(/"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/);
    return pertama ? pertama[0].slice(1, -1) : null;
  }
  return v.slice(1, -1);
}

const js = new Map();
for (const f of walk(join(ROOT, "plugins"), ".js")) {
  const n = namaDariJs(f);
  if (n) js.set(n, f.replace(ROOT, ""));
}

const rs = new Map();
for (const f of walk(join(ROOT, "rust/crates/oktz-plugins/src"), ".rs")) {
  const t = readFileSync(f, "utf8");
  for (const m of t.matchAll(/#\[plugin\(([\s\S]*?)\)\]/g)) {
    const nm = m[1].match(/name\s*=\s*"((?:[^"\\]|\\.)*)"/);
    if (nm) rs.set(nm[1], f.replace(ROOT, ""));
  }
}

const hanyaJs = [...js.keys()].filter((n) => !rs.has(n)).sort();
const hanyaRs = [...rs.keys()].filter((n) => !js.has(n)).sort();

console.log("plugin di JS (nama literal terbaca):", js.size);
console.log("terdaftar di Rust:", rs.size);
console.log("ter-port (nama cocok byte-per-byte):", [...js.keys()].filter((n) => rs.has(n)).length);
console.log("hanya di JS (belum di-port):", hanyaJs.length);
for (const n of hanyaJs) console.log("  -", n, "|", js.get(n));

// `panel/rolemanager.js` membangun `pluginConfig.name` dari ROLES x
// VALID_SERVERS saat modul dimuat, jadi tidak ada literal untuk dibaca. Satu
//-satunya plugin dengan nama non-literal, dan `addownerv1` yang terdaftar di Rust
// adalah anggota generator itu — bukan nama yang tidak dikenal produksi.
const DINAMIS = new Map([["addownerv1", "rust/crates/oktz-plugins/src/panel/rolemanager.rs"]]);
const hanyaRsRiil = hanyaRs.filter((n) => !DINAMIS.has(n));

console.log("hanya di Rust (tidak dikenal di produksi):", hanyaRsRiil.length);
for (const n of hanyaRsRiil) console.log("  +", n, "|", rs.get(n));
for (const [n, f] of DINAMIS) {
  if (hanyaRs.includes(n)) console.log("  ~ nama dinamis:", n, "|", f);
}

process.exit(hanyaRsRiil.length === 0 ? 0 : 1);
