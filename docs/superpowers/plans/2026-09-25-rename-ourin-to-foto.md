# Rename `ourin` → generic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lepas semua label `ourin` dari kode, aset, dan konfigurasi proyek, memakai nama generik, tanpa merusak integritas `onigis` (library pihak ketiga) maupun mekanisme prebuilt native.

**Architecture:** Satu penulis deterministik (skrip migrasi berbasis peta eksplisit, bukan regex tebakan) untuk Phase 1–5, karena fase-fase ini saling tumpang tindih pada `package.json`, `config.js`, dan ratusan file import — subagent penulis paralel akan bentrok dan korup. Subagent dipakai **paralel hanya sebagai reviewer** di Phase 6. Kontak permanen dengan sistem adalah **test invarian** (`tests/rename-invariants.test.mjs`), bukan skrip migrasi sekali pakai.

**Tech Stack:** Node.js ESM, `node --test` (test runner bawaan), npm alias (`npm:` specifier / alias label), `git mv`.

**Spec:** `docs/superpowers/specs/2026-09-25-rename-ourin-to-foto-design.md`

## Global Constraints

- **D5:** `native/` TIDAK boleh disentuh — `ourin-native`, `ourin_native.*.node`, `fetch-prebuilt.mjs` (`REPO="OktzO/ourin-md"` + sha256), `index.cjs`, `platforms/`. Tag log `"ourin-native"` di `native-loader.js` juga TETAP.
- **D7:** `src/data/family100.json` — kemunculan "Mourinho" (2×) TIDAK boleh berubah.
- Di luar scope: `docs/`, `README.md`, `.opencode/`, `.superpowers/`, `.git/`, `node_modules/` (kecuali rename alias di Task 1).
- `package.json` field `name` tetap `oktz-md`; `version`/`description` tidak berubah.
- `pp-kosong` (kunci aset & file) tidak berubah.
- **Tidak boleh menjalankan `npm install`** (network npm tidakandalan). Lockfile diedit manual.
- Setiap task harus berakhir dengan `npm test` (0 fail) + `npm run lint` (0 error).
- Migrasi memakai **peta nama eksplisit** yang dibangun dari `git ls-files`, bukan `sed`/regex tebakan.

---

### Task 0: Baseline & titik rollback

**Files:**
- Create: `tests/rename-invariants.test.mjs`

**Interfaces:**
- Consumes: tidak ada.
- Produces: test invarian yang dipakai Task 1–5; angka baseline untuk membandingkan.

- [ ] **Step 1: Tulis test invarian (fase awal — harus GAGAL)**

Buat `tests/rename-invariants.test.mjs` yang memuat 5 invarian (detail di Task 1–5; di Task 0 cukup kerangka yang meng-collectiahi ketiga kondisi saat ini: alias `ourin` masih ada, `src/lib` masih 81 file `ourin-*`, dan import relatif semua resolve). Rigorously, test #1 (alias) dan #2 (prefix) **akan gagal** setelah Task 1–2, jadi pada Task 0 test harus ditulis agar **lolos dulu** untuk kondisi baseline? Tidak — TDD: tulis test yang **mendifinisikan kondisi akhir**, sehingga sekarang harus GAGAL.

Isi test (kondisi akhir yang ditentukan):
```js
import { describe, it } from "node:test";
import assert from "node:assert";
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const SCAN_DIRS = ["src", "plugins", "tests", "data", "case"];
const SCAN_EXT = [".js", ".mjs", ".cjs"];

function walk(dir, out = []) {
  for (const e of fs.readdirSync(path.join(ROOT, dir), { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (SCAN_EXT.includes(path.extname(e.name))) out.push(p);
  }
  return out;
}
const FILES = [
  ...SCAN_DIRS.flatMap((d) => (fs.existsSync(path.join(ROOT, d)) ? walk(d) : [])),
  ...["index.js", "_test.mjs"].filter((f) => fs.existsSync(path.join(ROOT, f))),
];
const read = (f) => fs.readFileSync(path.join(ROOT, f), "utf8");
const WHITELIST = [
  // "Mourinho" (D7)
  "src/data/family100.json",
  // fetch-prebuilt / native (D5)
  "native/fetch-prebuilt.mjs",
  "native/index.cjs",
  "native/Cargo.toml",
  "native/build.rs",
  "native/build-prebuilt.sh",
];
```

- [ ] **Step 2: Tambahkan 5 invarian (lengkap, bukan placeholder)**

Lanjut file test yang sama dengan 5 `describe`/`it`:

**I1 — semua import relatif resolve:**
```js
it("I1: semua import relatif resolve ke file yang ada", () => {
  const bad = [];
  for (const f of FILES) {
    const src = read(f);
    const re = /(?:from|import)\s*\(?\s*["'](\.[^"']+)["']/g;
    let m;
    while ((m = re.exec(src))) {
      const spec = m[1];
      const base = path.resolve(ROOT, path.dirname(f), spec);
      const cands = [base, base + ".js", base + ".mjs", base + ".cjs", base + "/index.js"];
      if (!cands.some((c) => fs.existsSync(c))) bad.push(`${f} -> ${spec}`);
    }
  }
  assert.deepStrictEqual(bad, [], `import rusak:\n${bad.join("\n")}`);
});
```

**I2 — tidak ada file `src/lib/ourin-*.js` yang tersisa:**
```js
it("I2: tidak ada lagi src/lib/ourin-*.js", () => {
  const left = fs.readdirSync(path.join(ROOT, "src/lib")).filter((f) => f.startsWith("ourin-"));
  assert.deepStrictEqual(left, [], `masih ada: ${left.join(", ")}`);
});
```

**I3 — tidak ada import specifier ke `ourin` (npm alias lama):**
```js
it("I3: tidak ada import dari specifier 'ourin'", () => {
  const bad = [];
  for (const f of FILES) {
    const src = read(f);
    if (/(?:from|import)\s*\(?\s*["']ourin["']/.test(src)) bad.push(f);
  }
  assert.deepStrictEqual(bad, [], `masih import 'ourin': ${bad.join(", ")}`);
});
```

**I4 — semua kunci `config.assets` ada file/URL yang cocok:**
```js
it("I4: setiap kunci config.assets punya file aset yang ada", async () => {
  const cfg = (await import(path.join(ROOT, "config.js"))).default;
  const missing = [];
  for (const [k, v] of Object.entries(cfg.assets ?? {})) {
    if (/^https?:\/\//.test(v)) continue;
    if (!fs.existsSync(path.join(ROOT, v))) missing.push(`${k} -> ${v}`);
  }
  assert.deepStrictEqual(missing, [], `aset hilang: ${missing.join(", ")}`);
});
```

**I5 — tidak ada sisa `ourin` di luar whitelist:**
```js
it("I5: tidak ada sisa referensi 'ourin' di luar whitelist", () => {
  const hits = [];
  for (const f of FILES) {
    if (WHITELIST.includes(f)) continue;
    const lines = read(f).split("\n");
    lines.forEach((ln, i) => {
      if (/ourin/i.test(ln) && !/ourin-native/.test(ln)) hits.push(`${f}:${i + 1}: ${ln.trim().slice(0, 90)}`);
    });
  }
  assert.deepStrictEqual(hits, [], `sisa referensi ourin:\n${hits.slice(0, 40).join("\n")}`);
});
```

- [ ] **Step 3: Run test — verifikasi GAGAL (I2, I3, I5 harus gagal; I1, I4 harus PASS)**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: I1 PASS, I4 PASS, **I2 FAIL** (81 file `ourin-*.js`), **I3 FAIL** (58 file import `ourin`), **I5 FAIL** (banyak sisa).

- [ ] **Step 4: Commit test invarian**

```bash
git add tests/rename-invariants.test.mjs
git commit -m "test(rename): invariant import/asset/aset untuk rename ourin→generic"
```

> **CATATAN COMMIT:** Task 0 dan setiap task berikutnya mengandung `git commit`. Kebijakan repo: **hanya commit bila user memintanya.** Plan ini sudah disetujui user untuk dieksekusi, tetapi commit tetap perlu konfirmasi eksplisit. Jika user menolak, lewati semua langkah `git commit` dan gunakan `git diff` untuk review. **Tanyakan ke user sebelum eksekusi.**

---

### Task 1: Alias npm `ourin` → `onigis`

**Files:**
- Modify: `package.json` (dependencies)
- Modify: `package-lock.json` (keys `packages`)
- Rename dir: `node_modules/ourin` → `node_modules/onigis`
- Modify: 58 file yang mengandung `from "ourin"` / `import("ourin")` (lihat I3)

**Interfaces:**
- Consumes: test I3 dari Task 0.
- Produces: `import ... from "onigis"` dan `import("onigis")` valid; `node_modules/onigis` ada.

- [ ] **Step 1: Tulis test alias (GAGAL)**

Tambahkan ke `tests/rename-invariants.test.mjs`:
```js
it("alias: onigis resolve, ourin tidak ada", async () => {
  await assert.doesNotReject(() => import("onigis"));
  await assert.rejects(() => import("ourin"), (e) => e.code === "ERR_MODULE_NOT_FOUND");
  assert.ok(fs.existsSync(path.join(ROOT, "node_modules/onigis")), "node_modules/onigis harus ada");
  assert.ok(!fs.existsSync(path.join(ROOT, "node_modules/ourin")), "node_modules/ourin harus hilang");
});
```

- [ ] **Step 2: Run — verifikasi GAGAL**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: alias test FAIL (`onigis` `ERR_MODULE_NOT_FOUND`).

- [ ] **Step 3: Edit `package.json`**

Hapus baris `"ourin": "npm:onigis@^10.1.0-rc.6",` dari `dependencies`; tambahkan `"onigis": "^10.1.0-rc.6",` (pertahankan urutan alfabetik — `dependencies` saat ini terurut, `ourin` ada di akhir; `onigis` masuk sesuai posisi). Verifikasi: `node -e "console.log(require('./package.json').dependencies.onigis)"` → `^10.1.0-rc.6`.

- [ ] **Step 4: Rename folder `node_modules`**

```bash
mv node_modules/ourin node_modules/onigis
```

- [ ] **Step 5: Edit `package-lock.json` dengan skrip deterministik**

Buat `scripts/rename-lock.mjs` (commit, karena tool yang berguna):
```js
import fs from "node:fs";
const p = "package-lock.json";
const lock = JSON.parse(fs.readFileSync(p, "utf8"));

// root deps
const rootDeps = lock.packages[""].dependencies;
delete rootDeps.ourin;
rootDeps.onigis = "^10.1.0-rc.6";
rootDeps = Object.fromEntries(Object.entries(rootDeps).sort(([a],[b]) => a.localeCompare(b)));
lock.packages[""].dependencies = rootDeps;

// rename package keys
const next = {};
for (const [k, v] of Object.entries(lock.packages)) {
  const nk = k.replace(/^node_modules\/ourin(\/|$)/, "node_modules/onigis$1");
  next[nk] = v;
}
lock.packages = Object.fromEntries(
  Object.entries(next).sort(([a], [b]) => a.localeCompare(b)),
);
fs.writeFileSync(p, JSON.stringify(lock, null, 2) + "\n");
```
Run: `node scripts/rename-lock.mjs`
Verifikasi: `node -e "const l=require('./package-lock.json');console.log(!!l.packages['node_modules/onigis'], !!l.packages['node_modules/ourin'])"` → `true false`.

- [ ] **Step 6: Rewrite 58 file import `ourin`→`onigis`**

Buat `scripts/rewrite-alias.mjs`:
```js
import fs from "node:fs";
const targets = process.argv.slice(2);
const reFrom = /(\bfrom\s*)(["'])ourin\2/g;
const reImp = /(\bimport\s*\(\s*)(["'])ourin\2/g;
for (const f of targets) {
  let s = fs.readFileSync(f, "utf8");
  const before = s;
  s = s.replace(reFrom, '$1$2onigis$2').replace(reImp, '$1$2onigis$2');
  if (s !== before) { fs.writeFileSync(f, s); console.log("rewrote", f); }
}
```
Jalankan untuk semua file yang I3 tandai. Verifikasi: `grep -rIl "from \"ourin\"\|import(\"ourin\")" src plugins tests` → kosong.

- [ ] **Step 7: Run test — verifikasi PASS**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: alias test + I3 PASS.

- [ ] **Step 8: Lint + commit**

```bash
npm run lint
git add -A package.json package-lock.json scripts src plugins tests
git commit -m "refactor(alias): npm alias ourin→onigis + rewrite import specifier"
```

---

### Task 2: Rename 81 file `src/lib/ourin-*.js` + rewrite import

**Files:**
- Rename: 81 file `src/lib/ourin-X.js` → `src/lib/X.js` (via `git mv`)
- Modify: semua file yang meng-import `ourin-X.js` (±735 file, ±1252 referensi)

**Interfaces:**
- Consumes: I1, I2.
- Produces: `src/lib/` berisi 81 file tanpa prefix `ourin-`.

- [ ] **Step 1: Tulis `scripts/rename-lib.mjs` (peta eksplisit dari `git ls-files`)**

```js
import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const ROOT = process.cwd();
const SCAN_DIRS = ["src", "plugins", "tests", "data", "case"];
const SCAN_EXT = new Set([".js", ".mjs", ".cjs"]);

const libFiles = execSync('git ls-files "src/lib/*.js"', { encoding: "utf8" })
  .trim().split("\n").filter(Boolean);

const map = new Map();
for (const f of libFiles) {
  const base = path.basename(f);
  if (!base.startsWith("ourin-")) continue;
  const next = base.slice("ourin-".length);
  const target = path.join(path.dirname(f), next);
  if (fs.existsSync(target)) {
    console.error(`COLLISION: ${f} -> ${target} sudah ada`);
    process.exit(1);
  }
  map.set(base, next);
}
console.log(`peta: ${map.size} file (collision: 0)`);

function walk(dir, out = []) {
  for (const e of fs.readdirSync(path.join(ROOT, dir), { withFileTypes: true })) {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) walk(p, out);
    else if (SCAN_EXT.has(path.extname(e.name))) out.push(p);
  }
  return out;
}
const files = [
  ...SCAN_DIRS.flatMap((d) => (fs.existsSync(path.join(ROOT, d)) ? walk(d) : [])),
  ...["index.js", "_test.mjs"].filter((f) => fs.existsSync(path.join(ROOT, f))),
];

const dryRun = process.argv.includes("--dry-run");
let moved = 0, rewritten = 0;
for (const [oldBase, newBase] of map) {
  const oldPath = path.join("src/lib", oldBase);
  const newPath = path.join("src/lib", newBase);
  if (!fs.existsSync(path.join(ROOT, oldPath))) continue;
  if (!dryRun) execFileSync("git", ["mv", oldPath, newPath], { cwd: ROOT });
  moved++;
}
for (const f of files) {
  if (dryRun) continue;
  const abs = path.join(ROOT, f);
  let s = fs.readFileSync(abs, "utf8");
  const before = s;
  for (const [oldBase, newBase] of map) {
    if (s.includes(oldBase)) s = s.split(oldBase).join(newBase);
  }
  if (s !== before) { fs.writeFileSync(abs, s); rewritten++; }
}
console.log(`renamed: ${moved}, rewrite import: ${rewritten} file`);
```

- [ ] **Step 2: Dry-run — verifikasi peta benar (0 collision, 81 pasangan)**

Run: `node scripts/rename-lib.mjs --dry-run`
Expected: `peta: 81 file (collision: 0)`, `renamed: 81, rewrite import: 0`.

- [ ] **Step 3: Jalankan migrasi**

Run: `node scripts/rename-lib.mjs`
Expected: `renamed: 81, rewrite import: <N> file` dengan N > 0.
Verifikasi: `ls src/lib/ourin-*` → tidak ada; `grep -rIl "ourin-[a-z-]*\.js" src plugins tests data case` → kosong.

- [ ] **Step 4: Run test — I1 & I2 PASS**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: I1 PASS (semua import resolve), I2 PASS.

- [ ] **Step 5: Full test + lint**

```bash
npm test
npm run lint
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(lib): rename src/lib/ourin-*.js → src/lib/*.js (81 file)"
```

---

### Task 3: Rename aset + kunci `config.assets` + call site

**Files:**
- Rename: 20 file aset (`assets/**/ourin-*`, `ourin.jpeg/2/3.jpeg`, `ourin-font.ttf`, `ourin-mp3.mp3`, `ourin-mp4.mp4`)
- Modify: `config.js` (kunci `config.assets`)
- Modify: call site aset (§6.4 spec): `src/lib/context.js:15-17`, `src/lib/serialize.js:1044`, `src/handler.js:1987,1996`, `plugins/panel/createserver.js:333`, `plugins/group/rulesgrup.js:58`, `plugins/group/goodbye.js:303`, `plugins/group/welcome.js:282`, `src/lib/brat.js:235`

**Interfaces:**
- Consumes: I4, I5.
- Produces: `config.assets` kunci generik; aset terlink.

- [ ] **Step 1: Test I4 sudah ada (GAGAL setelah rename file)**

Setelah file aset di-`git mv` tapi sebelum update `config.assets`/path, I4 akan gagal. Urutan: rename file dulu.

- [ ] **Step 2: Tulis `scripts/rename-assets.mjs`**

```js
import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const ROOT = process.cwd();

const assetFiles = execSync('git ls-files "assets"', { encoding: "utf8" })
  .trim().split("\n").filter((f) => path.basename(f).startsWith("ourin"));

// ourin- prefixed -> strip ; bare ourin / ourin2 / ourin3 -> foto / foto2 / foto3
function newName(base) {
  if (base.startsWith("ourin-")) return base.slice("ourin-".length);
  return base.replace(/^ourin(?=\d*\.)/, "foto");
}
const map = new Map();
for (const f of assetFiles) {
  const base = path.basename(f);
  const next = newName(base);
  const target = path.join(path.dirname(f), next);
  if (fs.existsSync(path.join(ROOT, target))) {
    console.error(`COLLISION: ${f} -> ${target}`);
    process.exit(1);
  }
  map.set(base, next);
}
console.log(`peta aset: ${map.size}`);

// 1) git mv file
for (const [oldB, newB] of map) {
  for (const [dir] of map) void dir;
  const oldPath = assetFiles.find((f) => path.basename(f) === oldB);
  execFileSync("git", ["mv", oldPath, path.join(path.dirname(oldPath), newB)], { cwd: ROOT });
}

// 2) rewrite string aset di SELURUH file (kunci config + call site + reply text)
const oldTokens = new Set();
for (const [oldB] of map) {
  oldTokens.add(oldB);                              // ourin-rpg.jpeg
  oldTokens.add(oldB.replace(/\.[a-z0-9]+$/i, "")); // ourin-rpg
}
const targets = [];
const SKIP_FILES = new Set(["package.json", "package-lock.json", "package-lock.yml"]);
(function walk(d) {
  for (const e of fs.readdirSync(path.join(ROOT, d), { withFileTypes: true })) {
    if (d === "node_modules" || d === ".git" || d === "docs" || d === "native" || d === ".superpowers") continue;
    const p = path.join(d, e.name);
    if (e.isDirectory()) walk(p);
    else if ([".js", ".mjs", ".cjs"].includes(path.extname(e.name))) targets.push(p);
  }
})(".");
let n = 0;
for (const f of targets) {
  // JANGAN sentuh manifest: stem-replace `"ourin"` akan merusak key dependency
  if (SKIP_FILES.has(path.basename(f))) continue;
  const abs = path.join(ROOT, f);
  let s = fs.readFileSync(abs, "utf8");
  const before = s;
  //Basename penuh (paling spesifik, diproses lebih dulu)
  for (const [oldB, newB] of [...map].sort((a, b) => b[0].length - a[0].length)) {
    s = s.split(oldB).join(newB);
  }
  // stem tanpa ekstensi (kunci config.assets)
  for (const [oldB, newB] of map) {
    const oldStem = oldB.replace(/\.[a-z0-9]+$/i, "");
    const newStem = newB.replace(/\.[a-z0-9]+$/i, "");
    s = s.split(`"${oldStem}"`).join(`"${newStem}"`);
    s = s.split(`'${oldStem}'`).join(`'${newStem}'`);
  }
  if (s !== before) { fs.writeFileSync(abs, s); n++; }
}
console.log(`aset di-rewrite di ${n} file`);
```

- [ ] **Step 3: Jalankan migrasi aset**

Run: `node scripts/rename-assets.mjs`
Expected: `peta aset: 20`, `aset di-rewrite di N file`.
Verifikasi: `find assets -name 'ourin*'` → kosong.

- [ ] **Step 3: Update kunci `config.assets` + nilai path**

`config.assets`: setiap kunci `ourin-X`→`X`; `ourin`→`foto`; `ourin2`→`foto2`; `ourin3`→`foto3`; `pp-kosong` tetap. Nilai path string ikut di-map ke nama file baru.

- [ ] **Step 4: Update call site (§6.4)**

Ganti string aset di 9 lokasi. `src/lib/brat.js:235` path `assets/ourin-font.ttf`→`assets/font.ttf`.

- [ ] **Step 5: Run test — I4 & I5 PASS**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: I4 PASS (semua kunci punya file), I5 PASS.

- [ ] **Step 6: Full test + commit**

```bash
npm test
git add -A
git commit -m "refactor(asset): rename aset ourin* → generic + update config.assets & call site"
```

---

### Task 4: Rename 17 plugin `ganti-ourin*.js` + `case/ourin.js` + teks user-facing

**Files:**
- Rename: `plugins/owner/ganti-ourin-*.js`→`ganti-*.js`; `ganti-ourin.js`→`ganti-foto.js`; `ganti-ourin2.js`→`ganti-foto2.js`; `ganti-ourin3.js`→`ganti-foto3.js`
- Rename: `case/ourin.js`→`case/foto.js`
- Modify: import 5 file yang refer `case/ourin.js` (`allmenu.js:14`, `carifitur.js:2`, `menu.js:1`, `menucat.js:2`, `handler.js:68`)
- Modify: teks reply di 17 plugin (nama file/kunci aset → generik)

**Interfaces:**
- Consumes: I1, I5.
- Produces: 17 plugin `ganti-*` terdaftar; `case/foto.js` ter-import.

- [ ] **Step 1: Tulis `scripts/rename-plugins.mjs`**

```js
import { execFileSync, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
const ROOT = process.cwd();

const tracked = execSync("git ls-files", { encoding: "utf8" }).trim().split("\n");

// 1) plugin ganti-ourin* -> ganti-*
const plugins = tracked.filter((f) => /plugins\/owner\/ganti-ourin/.test(f));
const pmap = new Map();
for (const f of plugins) {
  const base = path.basename(f);
  const next = base.startsWith("ganti-ourin-")
    ? base.replace("ganti-ourin-", "ganti-")
    : base.replace(/ganti-ourin(?=\d*\.js$)/, "ganti-foto");
  pmap.set(f, path.join(path.dirname(f), next));
}
// 2) case/ourin.js -> case/foto.js
if (tracked.includes("case/ourin.js")) pmap.set("case/ourin.js", "case/foto.js");

for (const [from, to] of pmap) {
  if (fs.existsSync(path.join(ROOT, to))) { console.error(`COLLISION: ${to}`); process.exit(1); }
}
console.log(`peta plugin/case: ${pmap.size}`);
for (const [from, to] of pmap) execFileSync("git", ["mv", from, to], { cwd: ROOT });

// rewrite import case/ourin.js -> case/foto.js di semua file
const files = tracked.filter((f) => fs.existsSync(path.join(ROOT, f))
  && [".js", ".mjs", ".cjs"].includes(path.extname(f)));
let n = 0;
for (const f of files) {
  const abs = path.join(ROOT, f);
  let s = fs.readFileSync(abs, "utf8");
  const before = s;
  s = s.split("case/ourin.js").join("case/foto.js");
  if (s !== before) { fs.writeFileSync(abs, s); n++; }
}
console.log(`import case/ rewritten di ${n} file`);
```

- [ ] **Step 2: Jalankan migrasi plugin + case**

Run: `node scripts/rename-plugins.mjs`
Expected: `peta plugin/case: 18`, `import case/ rewritten di 5 file`.
Verifikasi: `ls plugins/owner/ganti-ourin*.js` → kosong; `ls plugins/owner/ganti-*.js | wc -l` → 17; `ls case/` → `foto.js`.

- [ ] **Step 2: Rewrite import `case/ourin.js`→`case/foto.js` (sudah dilakukan oleh `rename-plugins.mjs` Step 1)**

Verifikasi: `grep -rn "case/ourin.js" src plugins` → kosong.

- [ ] **Step 3: Update teks reply user-facing**

Di 17 plugin, ganti nama file/kunci aset dalam string reply sesuai peta aset Task 3 (mis. "assets/image/ourin-rpg.jpeg"→"assets/image/rpg.jpeg", "ourin-v8.jpeg"→"v8.jpeg", "ourin.mp3"→"mp3.mp3"). **Jangan** memperbaiki ketidakkonsistenan lama (mis. `ganti-large` sebut `ourin-v10.jpeg` yang tidak ada) — follow-up terpisah.

- [ ] **Step 4: Run test — I1 & I5 PASS + smoke hitung plugin**

Jalankan boot smoke (`node index.js` singkat atau `npm test`); verifikasi tidak ada `MODULE_NOT_FOUND` dan hitung plugin = 829 (baseline).

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(plugin): rename ganti-ourin* + case/ourin.js, update reply text"
```

---

### Task 5b: Sweep residual `ourin` (identifier internal, komentar, nama test)

**Files:** `src/lib/apimanager.js`, `src/handler.js`, 6 plugin yang mengimpor `ourinApi`, `src/scraper/unlimitedai.js`, `src/lib/{serialize,lid,group-protection,emoji-map,exif,plugins,auto-backup}.js`, `plugins/main/{menu,allmenu,sc}.js`, `plugins/owner/getplugin.js`, `plugins/vps/createvps.js`, `plugins/{group/promote,group/demote}.js`, `tests/*.test.mjs` (nama/deskripsi test), `tests/rename-invariants.test.mjs` (whitelist)

**Interfaces:**
- Consumes: I5.
- Produces: 0 kemunculan `ourin` di luar 3 whitelist.

- [ ] **Step 1: Tambahkan whitelist ke I5**

Whitelist ini **wajib** dan tidak boleh longgar:
- `api.ourin.my.id` — API eksternal milik pihak ketiga, HIDUP (HTTP 200). `src/lib/apimanager.js:1014` `baseURL: "https://api.ourin.my.id"`. **TIDAK BOLEH diganti** — production break.
- `ourin-native`, `ourin_native.*.node` — modul native (D5).
- `Mourinho` di `src/data/family100.json` (D7).
Tambahkan sebagai pola yang menghapus kemunculan spesifik tersebut sebelum memeriksa residual, seperti yang sudah dilakukan untuk `ourin-native`. JANGAN whitelist kata `ourin` secara umum.

- [ ] **Step 2: Rename identifier internal**

| Lama | Baru |
|---|---|
| `OurinApiManager` | `FotoApiManager` |
| `OurinApiProvider` | `FotoApiProvider` |
| `OurinMainApiProvider` | `FotoMainApiProvider` |
| `OurinApiRequestOptions` | `FotoApiRequestOptions` |
| `OurinApiAuthShape` | `FotoApiAuthShape` |
| `OurinApiProviderShape` | `FotoApiProviderShape` |
| `OurinApiMultipartFile` | `FotoApiMultipartFile` |
| `ourinApi` (variabel, 6 file importer) | `fotoApi` |
| `ourinGames` (alias di `src/handler.js:71`) | `fotoGames` |

Termasuk semua JSDoc `@param {OurinApi...}` / `@typedef {Object} OurinApi...` / `@returns {OurinApi...}` di `apimanager.js`. JANGAN sentuh string `https://api.ourin.my.id`.

- [ ] **Step 3: Bersihkan komentar & teks usang**

`src/lib/serialize.js`, `lid.js`, `group-protection.js`, `emoji-map.js`, `exif.js`, `plugins.js` (komentar `loadPlugins('./ourin-plugins')` → `'./plugins'`), `plugins/main/menu.js:387` (`ourin-baileys` → `foto-baileys`), `allmenu.js:551` (`ourin-menu-v8`), `plugins/main/sc.js` & `plugins/owner/getplugin.js` (string `ourin/onigis`), `plugins/vps/createvps.js` (`ourin-bot`), `plugins/group/promote.js` & `demote.js` (komentar), `src/lib/auto-backup.js` (`.ourin-temp` → `.foto-temp`).

- [ ] **Step 4: Bersihkan nama/deskripsi test**

`tests/*.test.mjs` menyebut nama module lama di nama test & string sementara: `ourin-premium-db`, `ourin-logger`, `ourin-lid`, `ourin-brat`, `ourin-ffmpeg-tone-`, `ourin-ffmpeg-pwned-`, `ourin/lib/Socket/groups.js`. Ganti agar sesuai nama file saat ini. JANGAN ubah assertion/behavior test.

- [ ] **Step 5: Run test — I5 harus PASS**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: **I1–I5 semua PASS** (Task 5b menutup residual; Task 5 di bawah hanya 4 edge case yang sudah tercakup di sini).
`npm test` harus 309 pass / 0 fail.

- [ ] **Step 6: Commit**

```bash
npm run lint
git add -A
git commit -m "refactor(rename): sweep residual ourin identifiers, comments, test names"
```

---

### Task 5: Edge case (§6.1–6.2 spec)

**Files:**
- Modify: `src/handler.js:242` — `import("./lib/ourin-sticker-reply.js")` → `import("./lib/sticker-reply.js")`. **JANGAN** hapus blok try/catch; file memang tidak ada, sudah di-guard, `handleStickerReply` tetap `undefined` & usage guarded di `handler.js:1369`.
- Modify: `src/scraper/unlimitedai.js:60,63` — `"ourin-ai"` → `"ai-assistant"` (default + fallback; kunci `ai-assistant` ADA di `CHARACTERS`, side-fix default yang sebelumnya broken).
- Modify: `src/lib/auto-backup.js:58` — `".ourin-temp"` → `".foto-temp"`.
- Modify: `plugins/main/menu.js:387` — brand user-visible `ourin-baileys` → `foto-baileys`.

**Interfaces:**
- Consumes: I5.
- Produces: nol sisa `ourin` di luar whitelist; default `UnlimitedAI` resolve ke karakter nyata.

- [ ] **Step 1: Tambah test edge-case (GAGAL)**

Tambah ke `tests/rename-invariants.test.mjs`:
```js
it("edge: UnlimitedAI default resolve ke karakter yang ada", async () => {
  const { UnlimitedAI } = await import(path.join(ROOT, "src/scraper/unlimitedai.js"));
  // panggil dengan prompt kosong & tanpa karakter → harus TIDAK throw "char undefined"
  // (gunakan asersi ringan: fungsi ter-export & punya default; test EINTEGRITI: cek CHARACTERS punya kunci default)
  assert.equal(typeof UnlimitedAI, "function");
});
it("edge: tidak ada import ke modul ourin-sticker-reply yang tak Terdaftar", () => {
  const src = read("src/handler.js");
  assert.ok(!/ourin-sticker-reply/.test(src), "specifier ourin-sticker-reply harus di-rewrite");
  assert.ok(/\.\/lib\/sticker-reply\.js/.test(src), "harus menunjuk ./lib/sticker-reply.js");
});
```

- [ ] **Step 2: Run — verifikasi GAGAL**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: edge tests FAIL (masih ada `ourin-sticker-reply`, I5 masih detect `ourin-ai`/`.ourin-temp`/`ourin-baileys`).

- [ ] **Step 3: Terapkan 4 edit edge-case**

- [ ] **Step 4: Run test — PASS**

Run: `node --test tests/rename-invariants.test.mjs`
Expected: semua PASS (I1–I5 + alias + edge).

- [ ] **Step 5: Full test + lint + commit**

```bash
npm test
npm run lint
git add -A
git commit -m "fix(edge): rewrite import rusak, UnlimitedAI default, .foto-temp, brand string"
```

---

### Task 6: Verifikasi akhir (reviewer paralel)

**Files:** tidak ada perubahan kode; hanya review.

- [ ] **Step 1: Jalankan 4 reviewer subagent PARALEL**

 reviewer A (src/), B (plugins/), C (tests+case+data+index), D (config+package+native-check). Masing-masing: jalankan `grep`/I1–I5, laporkan sisa `ourin` yang TIDAK whitelisted, dan konfirmasi import resolve. Parse output tidak acceptable — harus lihat bukti.

- [ ] **Step 2: Gerbang verifikasi (SEMUA harus hijau)**

```bash
npm test                # 0 fail (baseline 304 pass + test baru)
npm run lint            # 0 error
node -e "import('onigis').then(()=>console.log('OK'))"   # alias resolve
node --test tests/rename-invariants.test.mjs            # I1–I5 + alias + edge PASS
find assets -name 'ourin*'                              # kosong
ls src/lib/ourin-*                                      # tidak ada
```

- [ ] **Step 3: Smoke boot (opsional, parse log)**

`timeout -s TERM 40 node index.js 2>&1 | grep -i "plugin\|MODULE_NOT_FOUND\|Error"`. Verifikasi: "plugin 829 ... loaded successfully" (baseline) dan tidak ada `MODULE_NOT_FOUND` / `ERR_MODULE_NOT_FOUND`.

- [ ] **Step 4: Commit verifikasi (jika ada perbaikan dari reviewer)**

```bash
git add -A
git commit -m "chore(rename): verifikasi akhir reviewer — 0 sisa ourin di luar whitelist"
```

---

## Rollback

Semua Phase 1–5 berupa `git mv` + rewrite → terlihat penuh di `git diff`/`git status`. Karena ada perubahan **belum-commit** dari kerja diagnosability sebelumnya, rollback TIDAK boleh `git reset --hard` (menghapus fix itu). Rollback aman: `git stash push` (simpan working) atau `git revert <commit>` per task. `node_modules` di-pulihkan via `mv node_modules/onigis node_modules/ourin` + revert `package.json`/`package-lock.json`.
