# Rename `ourin` → generic (bangun ulang, lepas dari label ourin)

Tanggal: 2026-09-25
Status: disetujui (scope 8A — native TIDAK di-rename)

## 1. Konteks & tujuan

Proyek ini memakai label lokal `ourin` di tiga tempat berbeda, padahal **tidak ada
package bernama "ourin" di npm**. Bukti faktual:

```
package.json               : "ourin": "npm:onigis@^10.1.0-rc.6"
node_modules/ourin/package.json : "name": "onigis"
package-lock.json          : node_modules/ourin -> registry.npmjs.org/onigis/...tgz
```

Jadi `ourin` hanyalah nama folder/alias. Tujuan rename: **proyek tidak lagi
men privilegiado label "ourin"** di kode, aset, dan konfigurasi, memakai nama
generik, tanpa merusak integritas library pihak ketiga (`onigis`) maupun
mekanisme prebuilt native.

## 2. Keputusan yang sudah disetujui

| # | Keputusan | Alasan |
|---|---|---|
| D1 | Alias npm `ourin` → `onigis` | Alias label lokal; package aslinya sudah `onigis`. |
| D2 | `src/lib/ourin-*.js` → `src/lib/*.js` (strip prefix) | `src/lib/` isinya **hanya** 81 file `ourin-*.js`, nol file lain → **nol collision** (diverifikasi). |
| D3 | Aset `ourin-<x>` → `<x>`; `ourin.jpeg`→`foto.jpeg`, `ourin2/3`→`foto2/3` | Strip prefix; stem bare `ourin` → `foto`. |
| D4 | Plugin `ganti-ourin-<x>.js` → `ganti-<x>.js`; `ganti-ourin.js`→`ganti-foto.js` | Konsisten dengan D2/D3. |
| D5 | **Native (`ourin-native`, `ourin_native.*.node`) TETAP** (opsi 8A) | `fetch-prebuilt.mjs` menarik dari GitHub Releases dengan sha256 di-pinning. Rename = 404 saat fresh deploy. |
| D6 | Import rusak `ourin-sticker-reply.js` **tidak dihapus**, hanya di-rewrite | Sudah di-guard `try/catch`; menghapus = risiko perubahan perilaku. |
| D7 | "Mourinho" di `family100.json` **tidak boleh** ikut berubah | False positive substring. |
| D8 | **Scope extension disetujui**: sweep sisa identifier internal, komentar, dan nama test di luar edge case §6 — dikerjakan sebagai "Task 5b" | I5 menemukan 87 kemunculan di ±35 file, bukan hanya 4 item §6. Owner pilih sweep penuh; whitelist hanya 3 item sah (D5, D7, D10). |
| D9 | Owner memilih stem brand untuk 2 string user-facing di `config.js` = **`Oktz`**, bukan `foto` (`sticker.packname`, `saluran.name`) | Kedua string = identitas produk & channel user, bukan sisa nama modul kode. `packname` juga homoglyph mathematical-bold (`𝗢𝗨𝗥𝗶𝗡`) sehingga tak terlihat oleh pencarian teks biasa. |
| D10 | `https://api.ourin.my.id` (`src/lib/apimanager.js:1014`) **dipertahankan** | Endpoint API pihak ketiga yang hidup (HTTP 200). Mengubahnya = production break. Mengandung substring "ourin", bukan nama modul. |
| D11 | Helper migrasi `scripts/rename-*.mjs` + `rewrite-alias.mjs` ditambahkan dan **sengaja** menyimpan nama lama sebagai peta rename | Sekali eksekusi, nama lama = data. Karena itu `scripts/` dikecualikan dari audit sisa nama. |

## 3. Scope

**Di dalam:**
- `src/lib/*.js` — 81 file
- Import specifier ke `src/lib/ourin-*.js` — ±1.252 referensi di ±735 file
- Alias npm di `package.json` + rename folder `node_modules/ourin`→`onigis` + `package-lock.json`
- Import `from "ourin"` / `import("ourin")` — 73 ekspresi di 58 file
- Aset gambar/audio/video/font + kunci `config.assets` + semua call site `getAssetBuffer` / `config.assets[...]` / path `assets/...`
- 17 file `plugins/owner/ganti-ourin*.js` + teks user-facing di dalamnya
- `case/ourin.js` → `case/foto.js` + 5 import
- Sweep sisa identifier internal, komentar, dan nama test di luar §6 (D8) — termasuk `example:` pada plugin di luar 17 `ganti-*`
- 2 string branding user-facing di `config.js` → stem `Oktz` (D9)
- `scripts/rename-*.mjs` + `scripts/rewrite-alias.mjs` — helper migrasi sekali pakai, map-driven (D11)
- Edge case §6

**Di luar (TIDAK boleh disentuh):**
- Laporan audit lama di `docs/**` (`memory-leak-audit.md`, `plugin-audit-2026-09-20.md`, `profiling-audit.md`, `profiling-harness.md`, `ram-safety-audit-2026-09-21.md`, `rust-migration-audit.md`) dan `README.md`. **Kecuali** `docs/superpowers/**` (spec + plan) dan `.superpowers/**` (ledger SDD): keduanya **dokumen kerja** yang dipelihara selama eksekusi, bukan artefak rename.
- `.opencode/`, `.git/`
- `native/` (Cargo.toml, `ourin_native.*.node`, `fetch-prebuilt.mjs`, `index.cjs`, `platforms/`)
- `node_modules/` — KECUALI rename alias (§4.2)
- `package.json` field `name` (tetap `oktz-md`), `version`, `description`
- Semua `ourin-native` sebagai referensi ke modul native (termasuk tag log)

## 4. Aturan transformasi (deterministik)

### 4.1 Modul internal
- File: `src/lib/ourin-X.js` → `src/lib/X.js`
- Import specifier berisi `ourin-X.js` → `X.js` (path relatif maupun absolut)
- Nama file `ourin-native-loader.js` → `native-loader.js` (file-nya tetap ourin, hanya nama file yang generic)

### 4.2 Alias npm
- `package.json`: hapus entri `"ourin"`, tambahkan `"onigis": "^10.1.0-rc.6"` pada `dependencies` (menjaga posisi/urutan alfabetis).
- `node_modules/ourin` → `node_modules/onigis` (rename folder, **tanpa** `npm install`).
- `package-lock.json`, diedit manual (network npm tidak boleh diandalkan):
  - `packages[""].dependencies`: hapus key `ourin`, tambah key `onigis` = `^10.1.0-rc.6`.
  - Rename key `packages["node_modules/ourin"]` → `packages["node_modules/onigis"]`, **pertahankan** field `name: "onigis"`, `version`, `resolved`, `integrity` apa adanya.
  - Rename semua key `packages["node_modules/ourin/node_modules/..."]` → `packages["node_modules/onigis/node_modules/..."]`.
  - **Jangan** mengubah `resolved`/`integrity` — tarball yang sama persis.
- Semua `from "ourin"` dan `import("ourin")` → `onigis`.

### 4.3 Aset & kunci config
- Nama file: `ourin-X.ext` → `X.ext`; `ourin.jpeg`→`foto.jpeg`; `ourin2.jpeg`→`foto2.jpeg`; `ourin3.jpeg`→`foto3.jpeg`
- Kunci `config.assets`: `ourin-X` → `X`; `ourin`→`foto`; `ourin2`→`foto2`; `ourin3`→`foto3`; `pp-kosong` **tetap**
- Semua referensi string aset ikut berubah (lihat §6.4)

### 4.4 Plugin ganti-* + case
- `ganti-ourin-X.js` → `ganti-X.js`
- `ganti-ourin.js`→`ganti-foto.js`; `ganti-ourin2.js`→`ganti-foto2.js`; `ganti-ourin3.js`→`ganti-foto3.js`
- `case/ourin.js` → `case/foto.js`; update 5 import (`allmenu.js:14`, `carifitur.js:2`, `menu.js:1`, `menucat.js:2`, `handler.js:68`)

### 4.5 User-facing
Teks balasan plugin yang menyebut nama file/kunci aset harus ikut diperbarui agar tidak membocorkan "ourin" ke user. Contoh: "File: assets/image/ourin-rpg.jpeg" → "assets/image/rpg.jpeg".

## 5. Pengecualian mutlak (whitelist — jangan rewrite)

| Lokasi | Alasan |
|---|---|
| `native/**` | D5 — binary + sha256 + GitHub Releases. |
| Tag log `"ourin-native"` di `native-loader.js` | Mengacu modul native yang tetap `ourin-native`. |
| `src/data/family100.json` (2×) | "Mourinho" (D7). |
| `https://api.ourin.my.id` (`src/lib/apimanager.js:1014`) | D10 — endpoint API pihak ketiga yang hidup. |
| `scripts/rename-assets.mjs`, `rename-lib.mjs`, `rename-lock.mjs`, `rewrite-alias.mjs` | D11 — nama lama adalah peta rename; di luar audit sisa nama. |
| Laporan audit lama di `docs/**`, `README.md`, `.opencode/` | Di luar scope. `docs/superpowers/**` = dokumen kerja, lihat §3. |

## 6. Edge case (must-do)

1. **`src/handler.js:242`** — `import("./lib/ourin-sticker-reply.js")` (file tak ada, sudah di-guard `try/catch`). Rewrite specifier → `./lib/sticker-reply.js`. **Jangan** hapus blok; variabel tetap `undefined` & usage guarded di `handler.js:1369`.
2. **`src/scraper/unlimitedai.js:60,63`** — default & fallback `"ourin-ai"` **tidak ada** di `CHARACTERS` (kunci valid: `ai-assistant`, `kobo-ai`, `waguri-ai`, `jokovic-ai`, `prabowo-ai`). Ganti `"ourin-ai"` → `"ai-assistant"` di kedua tempat (side-fix: default yang sebelumnya broken sekarang resolve).
3. **`.ourin-temp`** (`src/lib/auto-backup.js:58`) → `.foto-temp`. Stale dir `.ourin-temp` yang tertinggal tidak ikut dibersihkan (dapat diterima).
4. **Referensi aset non-obvious** yang wajib di-update:
   - `src/lib/context.js:15-17` — `"ourin-games"`, `"ourin-rpg"`, `"ourin-winner"`
   - `src/lib/serialize.js:1044` — `getAssetBuffer("ourin-mp4")`
   - `src/handler.js:1987,1996` — `config.assets["ourin-promote"|"ourin-demote"]`
   - `plugins/panel/createserver.js:333` — `getAssetBuffer("ourin-v8")`
   - `plugins/group/rulesgrup.js:58` — `getAssetBuffer("ourin-rules")`
   - `plugins/group/goodbye.js:303`, `welcome.js:282` — `getAssetBuffer("ourin-mp4")`
   - `src/lib/brat.js:235` — `assets/ourin-font.ttf`
   - `plugins/main/menu.js:387` — brand string `ourin-baileys` (user-visible; ubah ke `onigis-baileys` atau `foto-baileys` — pilih `foto-baileys` demi konsistensi stem)
5. **Konsistensi teks reply `ganti-*`**: beberapa reply menyebut nama file yang **sudah salah sebelum rename** (mis. `ganti-mp3.js` menyebut `assets/audio/ourin.mp3` padahal file `ourin-mp3.mp3`; `ganti-large.js` menyebut `ourin-v10.jpeg` yang tidak ada). Rename **tidak wajib** memperbaiki ketidakkonsistenan ini; cukup samakan penamaan baru secara literal. (Catatan untuk follow-up terpisah.)

## 7. Rencana eksekusi (1 penulis + N reviewer)

Temuan arsitektur: fase di bawah **saling tumpang tindih** pada `package.json`, `config.js`, dan ratusan file import. Menjalankan subagent **penulis** paralel akan bentrok & korup. Maka:

- **Phase 0 — Preflight (penulis tunggal)**: catat baseline (jumlah test, jumlah plugin loaded, status lint, `git status` bersih).
- **Phase 1 — Alias npm**: edit `package.json` + rename `node_modules/ourin`→`onigis` + rewrite `package-lock.json` + rewrite 73 import `ourin`→`onigis`. Verifikasi: `node -e "import('onigis')"` resolve.
- **Phase 2 — Modul internal**: `git mv` 81 file `src/lib/ourin-X.js`→`X.js`; rewrite import specifier `ourin-X.js`→`X.js` di semua file. Verifikasi: `node --check` semua file yang berubah.
- **Phase 3 — Aset**: `git mv` aset; rewrite kunci `config.assets`; rewrite call site aset (§6.4).
- **Phase 4 — Plugin & case**: `git mv` `ganti-*`, `case/ourin.js`; rewrite import; update teks reply.
- **Phase 5 — Edge case**: §6.1–6.2.
- **Phase 6 — Verifikasi**: jalankan subagent **reviewer paralel** atas area berbeda (src, plugins, tests, config) untuk masing-masing mengonfirmasi **nol** sisa referensi yang tidak masuk whitelist. Plus test suite penuh + lint + smoke load.

## 8. Gerbang verifikasi (wajib semua hijau sebelum "selesai")

- `npm test` — 0 fail (baseline saat ini: 304 pass)
- `npm run lint` — 0 error
- `grep -rIl "ourin" src plugins tests data case index.js config.js package.json` — **hanya** whitelisted (`family100.json` 2×, `native-loader.js` tag `ourin-native`, `menu.js` jika memilih `foto-baileys` berarti 0)
- Smoke test boot: bot memuat **plugin 829 / 40 kategori** seperti baseline; tidak ada `MODULE_NOT_FOUND`; pairing tetap berfungsi.
- `node -e "import('onigis')"` — resolve OK (alias benar).

## 9. Rollback

Semua perubahan file di-`git mv`/di-rewrite. `git diff` + `git status` HARUS di-inspect sebelum commit. Rollback = `git reset --hard <baseline-commit>` (node_modules di-reinstall via `npm install`). Tidak ada migrasi data; `config.assets` diubah realtime di config, dan `updateAssetUrl` menunjuk path baru.
