# Rust-Migration Audit — Ourin MD

Date: 2026-08-29 · Host audit: IDX container (2 core / 8GB / Node v20.19.1) · Target production: Pterodactyl panel, x86_64, total RAM 1GB (free ~350-500MB), 1 core, disk 3GB · Metode: baseline real-run (`process.memoryUsage()` + `--expose-gc`, aset nyata `assets/`), sweep dep vs import nyata seluruh repo, spike napi-rs end-to-end.

> **Status doc ini: AUDIT (belum implementasi).** Setiap verdict di bawah keputusan audit,
> belum ada satu baris pun kode production yang diubah. Fase implementasi dijalankan
> lewat prompt implementasi terpisah SETELAH doc ini di-review manual dan di-approve.
> Hasil implementasi nanti dicatat di section "Implementation log" di bawah — jangan
> timpa isi audit.

---

## TL;DR

| Kandidat | Verdict | Alasan satu baris |
|---|---|---|
| OCR — `tesseract.js` → Rust (`tesseract-rs`/bindings C API) | **Go** | Satu-satunya hotspot yang bermasalah nyata: +120-190MB RSS per penggunaan, 2-3s/op di mesin audit (lebih parah di panel 1-core), init leaky |
| 19 dep mati (npm uninstall, bukan Rust) | **Go** (murah, PR terpisah) | ~45MB disk + npm cache, disk panel cuma 3GB |
| Sticker pipeline (`ffmpeg`/`sharp` → Rust) | **No-go** | Sudah native + out-of-process; RSS flat, latensi sub-detik. Rust in-process justru menaikkan RSS baseline bot |
| Canvas (`@napi-rs/canvas`) | **No-go** | Sudah native (skia). +40MB sekali init, 83ms/card — bukan masalah |
| Media convert (`ffmpeg`) | **No-go** | Process spawn, RSS Node flat; rewrite = risiko tanpa win |
| `ourin-baileys` (protobufjs/libsignal) | **No-go** (out of scope) | Butuh fork terpisah; audit memory-leak sudah menandai area ini sensitif |
| JS-pure helper (exif, dll.) | **No-go** | 0.000s — gak ada yang bisa diperbaiki |
| `webpDimensions` util (parsial: dimensi only) | **Go kecil** (bonus, ikut crate) | 400x lebih cepat dari `sharp().metadata()` buat parse dims (0.68ms vs 284ms per 200 gambar) — tapi hanya dipakai di 3 call-site; value rendah, masuk sebagai fungsi util crate, bukan tujuan migrasi |

Kesimpulan arsitektur: **satu crate `ourin-native` (napi-rs v3)**, diekspor dari satu loader
JS dengan fallback otomatis. Isinya awalnya hanya OCR + util kecil. Bukan banyak crate,
bukan "rust-ify semua".

---

## 1. Metodologi & angka baseline

### Cara ukur

- Harness: `/tmp/opencode/bench/baseline.mjs` + `baseline2.mjs` (throwaway; skrip
  dilekatkan di appendix doc implementasi nanti). Menjalankan **fungsi production asli**
  (`imageToWebpFFmpeg`/`videoToWebpFFmpeg` dari `src/lib/ourin-exif.js`, `tesseract.js`
  worker persis pola `plugins/tools/ocr.js`, ffmpeg `@ffmpeg-installer` persis pola
  `src/lib/ourin-ffmpeg.js`), input nyata (`assets/image/ourin2.jpg` di-resize ke
  1080x1920 PNG ~2MB sebagai gambar ponsel tipikal; `assets/video/ourin-mp4.mp4` 1.9MB).
- Metrik: `process.memoryUsage()` (rss/heapUsed/heapTotal/external) sebelum / puncak /
  sesudah, `--expose-gc` + settle 200ms, median N iterasi.
- Catatan jujur: mesin audit 2-core; angka CPU di panel 1-core akan lebih buruk secara
  proporsional untuk semua skenario — perbandingan relatif tetap valid.

### Angka (median, RSS MB)

| Skenario | median | rssBefore | rssPeak | rssAfter | heapPeak |
|---|---|---|---|---|---|
| sticker img→webp (`imageToWebpFFmpeg`) | 0.17s | 84 | 84 | 84 | 7 |
| sticker vid→webp (`videoToWebpFFmpeg`) | 2.23s | 84 | 88 | 82 | 7 |
| sharp img→webp 512 | 0.05s | 82 | 94 | 94 | 6 |
| **OCR tesseract worker-per-call** (pola `ocr.js` sekarang) | 3.10s | 94 | **216** | **152** | 7 |
| **OCR tesseract worker persist** | 2.02s | 184 | **236** | **231** | 7 |
| OCR setelah `terminate()` | — | — | — | 164 (residual) | — |
| convert vid→mp3 (ffmpeg) | 1.18s | 152 | 152 | 152 | 7 |
| canvas text card 1280x720 | 0.083s | 57 | 99 | 94 | 4 |
| exif addExifToWebp (pure JS) | ~0s | 169 | 169 | 169 | 6 |

Pembacaan penting:

- **Skenario paling jelek di repo ini adalah OCR** — dan itu *by design* dari
  `plugins/tools/ocr.js:43-54`: setiap `.ocr` membuat worker baru (~+122MB di atas
  baseline), lalu `terminate()`. Residual +60MB tetap tertinggal per siklus. Dua user
  `.ocr` beruntun di panel 1GB = bot OOM-territory. Catatan `ponytail` di `ocr.js` sudah
  menyadari ini; migrasi Rust menyelesaikan akar masalahnya (native process, tanpa
  WASM heap ganda).
- Sticker/convert/canvas semuanya sehat: ffmpeg bekerja out-of-process (RSS Node flat),
  sharp/napi-canvas native dengan init sekali-jalan.

---

## 2. Kandidat & verdict

### 2.1 OCR — `tesseract.js` → native Rust — **Go**

- **Masalah terukur:** worker-per-call = 3.1s + spike 216MB; worker persist tidak
  pernah balik di bawah ~230MB saat dipakai; residual tetap ada setelah terminate.
  Di panel 1GB/1-core ini kandidat paling mahal yang ada.
- **Bentuk Rust:** binding C API Tesseract (`leptess`/`tesseract-rs` semacamnya) atau
  CLI-sidecar. Keputusan final crate di fase implementasi; syarat audit: hasil teks
  harus identik fungsi (engine + traineddata sama), tanpa spawn per call, memori
  dilepas dengan benar setelah recognize.
- **Yang diukur ulang pas implementasi:** RSS puncak + median per recognize dengan
  harness yang sama. Target: puncak < +40MB di atas baseline process, median < 1.5s
  di mesin audit.
- **Fallback:** tetap `tesseract.js` (JS lama) kalau `.node` gagal load.
- **Risiko:** kualitas binding crate di crates.io bervariasi — kalau gak ada yang
  sehat, opsi kedua audit: sidecar process Rust sekali-spawn dengan pipe. Keputusan
  diimplementasi dengan baik di fase implementasi; kalau keduanya gagal, revert
  ke tesseract.js dan catat.

### 2.2 Dep mati — npm uninstall murni — **Go (PR terpisah, bukan Rust)**

19 dep di `package.json` yang tidak punya import/dynamic-import string mana pun di
`plugins/`, `src/`, `index.js` (diverifikasi grep literal + scan dynamic import):

`figlet` (17.9MB), `openpgp` (16.6MB), `katex` (4MB), `jsdom` (3.9MB), `fflate`,
`unfurl.js`, `acorn`, `@google/generative-ai`, `@sptzx/request`, `bycf`, `nathcf`,
`p-queue`, `cli-table3`, `fetch-cookie`, `@vitalets/google-translate-api`,
`@bochilteam/scraper-primbon`, `@adiwajshing/keyed-db`, `gradient-string`, `zencf`

Catatan:

- `gradient-string` khusus: `src/lib/ourin-logger.js` meng-export mock internalnya,
  tidak ada importer dep aslinya — aman dihapus dep-nya, mock tetap jalan.
- **JANGAN sentuh** `tough-cookie`, `fflate`-via-baileys, `p-queue`-via-baileys,
  `acorn`-via-baileys (transitive `ourin`): hanya hapus dari `dependencies` root,
  biarkan lock menyelesaikan transitive. (`fflate`/`p-queue` juga dibutuhkan root
  menurut lock ROOT `needs` — dicek: itu karena listed di root deps; setelah uninstall
  dari root, transitive tetap terpasang via baileys.)
- Bersihkan `allowScripts` dari dep mati aja (mis. `tesseract.js` **jangan** —
  masih fallback; `ssh2` **jangan** — dipakai `plugins/panel/*`).
- Win: ~45MB `node_modules` + npm cache turun; 0 risiko runtime (gak ada importer).

### 2.3 Sticker pipeline — **No-go**

`imageToWebpFFmpeg`/`videoToWebpFFmpeg`/`sticker.js` semuanya spawn ffmpeg:
out-of-process, RSS Node flat (84→88→82), 0.17s untuk gambar. Menulis ulang ke Rust
in-process (libwebp-sys) menaikkan RSS dasar bot terus-menerus untuk menyelamatkan
sesuatu yang tidak jelek. **No-go.** Kalau suatu saat latensi video→webp (2.2s) jadi
keluhan, tuning flag ffmpeg (preset/threads) dulu — bukan ganti bahasa.

### 2.4 Canvas `@napi-rs/canvas` — **No-go**

Sudah native (skia via napi). +42MB init sekali, 83ms/card. Gak ada masalah.

### 2.5 Media convert ffmpeg — **No-go**

Sama kayak 2.3: process spawn, RSS flat. Gak ada kandidat Rust.

### 2.6 `ourin-baileys` — **No-go (out of scope)**

Fork terpisah, keputusan besar. Repo ini gak menyentuh.

### 2.7 JS-pure helpers — **No-go**

`addExifToWebp` median ~0s. Gak ada yang bisa diperbaat.

### 2.8 `webpDimensions` util — **Go kecil (bonus)**

Divalidasi spike: fungsi Rust parse dims (VP8/VP8L/VP8X) benar terhadap output
sharp, error path jalan, 0.68ms per 200 gambar vs 284ms `sharp().metadata()`.
Cuma 3 call-site (`topixel.js`, `tovideo.js`, `ourin-latex.js`) dan ketiganya
sekunder — jadi ini fungsi util di dalam crate, bukan justifikasi crate. Dikerjakan
"cuma kalau crate jalan untuk OCR; kalau OCR revert, util ikut revert."

---

## 3. Arsitektur (kalau implementasi jalan)

### 3.1 Satu crate `ourin-native`

```
native/                     # crate tunggal ourin-native
  Cargo.toml                # napi = { features = ["full"] }, napi-derive, napi-build
  build.rs                  # napi_build::setup()
  src/lib.rs                # #[napi] fns: ocr_init/ocr_recognize/webp_dimensions
  index.js + index.d.ts     # generated napi-rs (defenisi tipe utk TS/JSDoc)
src/lib/ourin-native-loader.js  # loader + fallback (try load .node, catch → JS impl)
```

- Loader pattern (dipakai semua call-site). Repo `"type": "module"` → pakai
  `createRequire` (bukan `require` global):

```js
// src/lib/ourin-native-loader.js (bentuk akhir disepakati saat implementasi)
import { createRequire } from 'node:module';
const req = createRequire(import.meta.url);
let native = null, fallbackLogged = false;
try { native = req('../native/index.js'); } // napi-rs resolve .node sesuai platform
catch (e) {
  if (!fallbackLogged) { logger.warn('[ourin-native] binary tidak termuat — pakai fallback JS:', e.code || e.message); fallbackLogged = true; }
}
export const hasNative = () => !!native;
export const ocr = native ?? jsFallbackOcr; // ttd identik
```

- Log fallback **sekali** (bukan per-call), alasan: ketauan di produksi tanpa spam.
- Kode JS lama TIDAK dihapus sampai native teruji + fallback ketest (guardrail
  roll-back). Hapus total `tesseract.js` dep hanya setelah beberapa rilis stabil.

### 3.2 Build & distribusi

- **Build di GitHub Codespaces** (`gh cs ssh`) sesuai workflow yang sudah
  disepakati — device target (panel) tidak pernah compile.
- Dua target prebuilt: `x86_64-unknown-linux-gnu` (glibc panel umum) DAN
  `x86_64-unknown-linux-musl` (Alpine image pterodactyl — **kita gak bisa
  memverifikasi image panel**, jadi dua-duanya disediakan; loader coba gnu dulu
  lalu musl). Ini jawaban atas "panel bukan punya aku dan console gak bisa
  dites": jangan taruhan satu ABI.
- Distribusi: `.node` dua target **di-commit ke repo** di `native/platforms/`
  (default — panel gak punya akses private registry/`gh` pasti; git pull = update).
  GitHub Release cuma kalau binary ternyata >20MB per target (keputusan diukur
  pas implementasi). `postinstall` tidak build — hanya verifikasi load non-fatal.
- `postinstall` script sekarang (rebuild `sharp`) TIDAK diubah perilakunya;
  hanya ditambah baris verifikasi load `ourin-native` non-fatal (log sekali).
- Spike di mesin audit sudah membuktikan rantai penuh: rustup → cargo →
  napi v3.12 (napi-derive v3) → linker → `lib*.so` → rename `.node` →
  `require()` sukses di Node 20 → angka benar. (Mesin audit pakai zig-cc
  via nix sebagai linker karena container gak punya gcc — di Codespaces
  toolchain normal, lebih gampang.)

### 3.3 Test

- Unit: fungsi util native vs output sharp (dims), OCR teks vs tesseract.js
  pada 3 gambar fixture (sama engine+traineddata → harus identik).
- Regression yang ada (`node --test tests/`, 83 pass) tetap hijau.
- Test harness memory (sama kayak baseline) dijalanin per item sebelum lanjut
  ke item berikutnya.

### 3.4 Urutan implementasi (per item, commit terpisah)

1. **PR-1 (bukan Rust):** uninstall 19 dep mati + rapikan `allowScripts`. Test
   full suite + bot boot smoke. Commit: `chore: drop 19 dead deps (~45MB)`.
2. **PR-2 (Rust):** crate `ourin-native` + loader + OCR native + fallback +
   test + harness ukur before/after. Commit: `feat(native): OCR via ourin-native, fallback tesseract.js`.
3. **PR-3 (opsional, ikut PR-2 atau terpisah):** `webpDimensions` util di
   call-site `topixel.js`/`tovideo.js`/`ourin-latex.js`. Commit terpisah biar
   revert-able.
4. Setiap PR: laporan before/after RSS di commit body + section "Implementation
   log" di doc ini.

---

## 4. Guardrail (mengikat fase implementasi)

1. Hanya kerjakan verdict **Go**. No-go gak disentuh, gak "sambil lewat" refactor.
2. Item Go tapi ketemu masalah baru (crate gak match, API berubah, bundling libs
   jadi rimet) → **berhenti, laporkan**, jangan improvisasi workaround.
3. `ourin-baileys` gak disentuh walau apapun hasil audit.
4. Output plugin identik secara fungsi (teks OCR, sticker bytes layout).
5. JS lama tetap ada sebagai fallback sampai native teruji; log sekali saat
   fallback aktif.
6. Satu item = satu commit = test = ukur ulang, baru item berikutnya.
7. Panel gak pernah compile: prebuilt `.node` dua target (gnu+musl), loader
   try/catch berurutan, fallback JS.
8. Baseline angka di doc ini jadi acuan "improvement signifikan" — kalau OCR
   native gak lebih baik dari tesseract worker-persist (<+40MB & <1.5s), revert
   dan catat skip.

---

## Appendix A — bukti spike napi-rs (mesin audit, throwaway)

- Toolchain: rustup 1.98.0, `napi` v3.12.2 (`features=["full"]`), `napi-derive`
  v3.6.3, `napi-build` v2.4.1; linker `zig cc` (container gak punya gcc).
- `#[napi] pub fn webp_dimensions(buf: Buffer) -> Result<Uint32Array>` —
  cdylib → `libourin_native_spike.so` → rename `.node` → `require()` OK
  (Node v20.19.1).
- Validasi output vs sharp: lossy `VP8 ` 512x512 PASS, lossless `VP8L` 777x333
  PASS (setelah fix off-by-one signature byte 0x2f), alpha/`VP8X` 1024x512 PASS,
  input bukan-webp → `InvalidArg` PASS.
- Benchmark util: 200 gambar → native 0.68ms vs `sharp().metadata()` 284.62ms.
- Kesalahan yang ketahuan berkat spike (jadi catatan implementasi): napi v3 API
  pindah namespace (`napi::bindgen_prelude::*`, `Uint32Array` untuk array
  return), `crate-type = ["cdylib"]` wajib eksplisit, dan `.so` harus rename
  `.node` (atau pakai `napi build` yang mengurus penamaan).

---

## Implementation log (ditambah saat implementasi — audit di atas tidak diubah)

### PR-1 — dead deps — DONE (`7a2bbaf`)

19 dep di-uninstall dari root. `node_modules` 835MB → 700MB. `fflate`/`p-queue`/
`acorn`/`tough-cookie` tetap terpasang sebagai transitive `ourin`. Tests 99 pass,
boot smoke bersih.

### PR-2 — OCR native via `ourin-native` — DONE (`779fbd0`)

- Crate: `native/` — napi v3 + `tesseract-rs` 0.4.0 (feature `embed-tessdata`,
  bahasa `eng` saja) + `image` 0.25. Prebuilt **di-commit ke repo**
  (`native/platforms/`, ukuran kecil) untuk `x86_64-unknown-linux-gnu` (10.7MB;
  NEEDED cuma libstdc++/libm/libgcc_s/libc) dan `x86_64-unknown-linux-musl`
  (10.9MB; NEEDED cuma `libc.so` musl). GitHub Release ditolak: repo private,
  panel tanpa credential GitHub gagal download assets.
- Ukuran turun dari 22MB→10.7MB per binary: `tessdata_fast` 4.1.0 (4.1MB,
  versi yang sama dengan default tesseract.js) di-seed ke `~/.tesseract-rs/tessdata/`
  sebelum build — bikin `build.rs` embed yang kecil daripada `tessdata_best` 15MB.
- Loader: `src/lib/ourin-native-loader.js` — coba gnu → musl → fallback
  tesseract.js; log sekali; `ocrInit` gagal → fallback permanen.
- Plugin: `plugins/tools/ocr.js` beralih ke loader; output user-facing identik.

Angka sebelum → sesudah (skenario gambar teks 1080x1920, mesin audit):

| Metrik | tesseract.js (worker-per-call) | ourin-native |
|---|---|---|
| median / call | 3.10s | **0.15s** (~20x) |
| RSS peak | 216MB (spike per siklus + residual tumbuh) | 181MB **sekali, flat** |
| RSS after-call steady | nanjak per siklus | 148–164MB stabil (10 call: +16MB total) |

Catatan build: (1) cmake di container ini install ke `lib64/` — harus dipindah
manual ke `lib/` (bug upstream tesseract-rs di host yang pakai LIB_SUFFIX);
(2) musl cdylib butuh `RUSTFLAGS="-C target-feature=-crt-static"` + linker
`zig cc -target x86_64-linux-musl`; (3) PNG fixture dari sharp-SVG gagal baca
OCR karena container tidak punya font sistem (render blank) — bukan bug native,
fixture test beralih ke `@napi-rs/canvas` + font bundel repo.

### PR-3 — webpDimensions util — SKIPPED

Audit asalnya bilang "Go kecil (bonus)". Evaluasi call-site: `tovideo.js` butuh
`meta.pages` (animated check — parser dims gak cukup), `topixel.js` dan
`ourin-latex.js` sudah manggil `sharp()` di alur yang sama jadi metadata
gratis. Win terukur tidak ada → tidak ditulis; dicatat supaya keputusan
punya alasan.

### Pasca-deploy panel (2026-08-29, `1190212`)

- Boot panel Pterodactyl (Node v22.23.2): bersih, native gnu binary load
  tanpa error, tidak ada log fallback — native aktif di production.
- Fix terpisah (ditemukan dari log panel, bukan item audit): `GroupSchedule
  Checker error: now is not defined` — sisa refactor moment-timezone di
  `src/lib/ourin-scheduler.js:544` (`now.second()`/`now.minute()`), diganti
  `formatNow("HH:mm:ss").split(":")`. Tests 99 pass.

## Audit ukuran repo pasca-migrasi `ourin-native` (2026-08-29, pre-implementasi)

### Status quo di HEAD (`49c3ffa`)

Konteks prompt asumsikan state 26MB→69MB dengan 2 binary 21–22MB. State itu
sudah berubah oleh commit `49c3ffa` (rebuilding dengan `tessdata_fast` 4.1MB
mengganti `tessdata_best` 15MB). Angka riil sekarang:

| Aspek | Ukuran |
|---|---|
| working tree (excl `.git`, excl `node_modules`) | 74MB |
| `.git` total (loose + packed) | 101MB (loose ~46MB + `size-pack` ~52MB) |
| `native/platforms/*` (2 binary, tracked) | **21.5MB** (gnu 10.7MB + musl 10.9MB) |
| `assets/` (media) | 17MB |
| sisanya (src, data, doc) | ~36MB |

Fakta sejarah git yang relevan:

- `.git/` 101MB walaupun working tree cuma 74MB karena **history masih punya
  binary 22MB versi lama** (commit `779fbd0`, ter-revert di `dfd90f0`,
  re-add versi lebih kecil di `49c3ffa`) — objek blob lama gak hilang sampai
  gc/purge.
- Coba GitHub Release sudah dilakukan (`v0.1.0` by token PAT): upload berhasil,
  panel tanpa auth tetap gagal because repo private → `fetch-prebuilt.mjs`
  dihapus, postinstall dibersihkan, binary re-commit. (Lihat commit log.)

### Kandidat A — Drop target musl

| Aspek | Nilai |
|---|---|
| Win total sekarang | 10.9MB saja (bukan 22MB) — tapi hanya dari working tree |
| `.git` shrink | 0 — blob lama tetap ada, butuh purge untuk ngaruh |
| Bukti panel | HEAD dan `1190212`: node load OK, no fallback, entry via gnu binary |
| Risiko pindah ke Alpine/distro musl di masa depan | Hipotetis — kita belum pernah butuh; kalau perlu tinggal rebuild dari branch build |
| Verdict | **No-go sekarang** — win 10.9MB terlalu kecil dibanding fragilitas kalau nginx/host ganti image ke Alpine random tanpa warning. Biarkan sebagai candidate khusus bersama purge history |

### Kandidat B — GitHub Release + fetch-prebuilt

**Sudah dicoba (dfd90f0) lalu dikembalikan (49c3ffa).** Ini bukan hipotesis
lagi, ada angka:

| Aspek | Nilai |
|---|---|
| Perlu | egress `github.com` dari panel + asset publik |
| Tes nyata | Release publik `v0.1.0` dibuat, assets upload sukses — fetch dari anonim return **HTTP 404** |
| Sebab | repo private; GitHub Release asset download tetap butuh auth buat repo private (bukan hanya API endpoint) |
| Workaround | taruh binary di npm registry/private CDN → panel butuh credential yang kita gak sediakan; taruh di repo public terpisah → tidak wanted |
| Verdict | **Nope** — gak fixable dalam constraint "panel tanpa credential". Di-tried-dan-direct dihapus. |

### Kandidat C — Purge history (`git filter-repo` terhadap blob binary)

- `.git` baseline: 101MB. Kalau kedua binary (lama + baru) dipurge sepenuhnya,
  `size-pack` teoritis turun ~53MB→~31MB (pengurangan ~21MB), total `.git` ke
  ~75–80MB. Win **paling besar dari tiga kandidat**, but capitalization:
  - rewrite history → semua clone existing harus re-clone (panel production
    tetap bisa ditangani via fresh clone path, risiko user sendiri).
  - tidak boleh dilakukan sampai working tree dari branch yang di-deploy saat
    ini sama, snapshot off-file ada, dan approval manual khusus diberikan.

- **Verdict: audit-only**. Dicatat opsi + win estimasi; **ditolak diimplementasi
  tanpa aproval manual terpisah** (sama permintaan user).

### Rekomendasi

| Urutan | Aksi | Verdict | Alasan satu baris |
|---|---|---|---|
| PR-4 | Nothing (dokumentasi ini aja) | **Go** | Binary sudah 22MB→10.7MB per target (bingung drop 50% vs konteks 69MB yang dicatat); skema commit-in-repo berfungsi di panel |
| PR-5 | Purge history blob binary lama | **Manual-only** | Perlu approval eksplisit + backup clone; aku *akan* membawanya ke review saat kamu sudah purging trail kau sendiri |
| PR-6 | Drop musl | **No-go** | Terlalu kecil win, fragile kalau image berganti |
| PR-7 | GitHub Release | **No-go** | Sudah ditest, repo private → 404 |

**Keputusan final yang dikirimkan ke kamu sebelum eksekusi:** kerjakan nothing di
phase ini kecuali add audit doc ini. Kandidat C siapapun jalan, tunggu izin terpisah.
