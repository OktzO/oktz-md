# CRATES — `oktz-md` Rust Rewrite

Generated: **2026-10-10** · Sumber versi: crates.io API + docs.rs · Tidak ada file lain yang diubah.

> Setiap versi di bawah **terverifikasi**, bukan dari ingatan. Kolom versi berisi tautan
> sumber. Lisensi diambil dari field `license` crates.io **dan** dicek silang ke LICENSE di
> repo upstream. Tanda ⚠ = keputusan berisiko, dijelaskan di bagian akhir.

> **Catatan target deploy.** Pterodactyl 1GB RAM / 1 core / disk 3GB. Binary **tidak pernah
> dikompilasi di panel** — build di CI/Codespaces (lihat `rust-migration-audit.md` §3.2).
> Konsekuensi: pilih crate yang build-nya berat bagi RAM build, bukan yang paling ringan
> saat runtime.

---

## 1. Inti / Async

| crate | versi terverifikasi | lisensi | dipakai untuk | feature penting | status |
|---|---|---|---|---|---|
| `tokio` | [1.53.2](https://crates.io/api/v1/crates/tokio) | MIT | runtime async, fs, subprocess ffmpeg, signal | `rt-multi-thread`, `fs`, `process`, `signal`, `time`, `sync`, `macros`, `net` | aktif (rilis 2026-10-03) |
| `reqwest` | [0.13.5](https://crates.io/api/v1/crates/reqwest) | MIT OR Apache-2.0 | HTTP client (ganti `axios`/`node-fetch`/`undici`) | ⚠ `rustls` — **bukan `rustls-tls`**; `json`, `multipart`, `cookies`, `gzip`, `brotli`, `stream` | aktif (2026-09-08) |
| `serde` | [1.0.229](https://crates.io/api/v1/crates/serde) | MIT OR Apache-2.0 | derive serialize/deserialize | `derive` | aktif |
| `serde_json` | [1.0.151](https://crates.io/api/v1/crates/serde_json) | MIT OR Apache-2.0 | JSON codec | ⚠ `preserve_order` — **wajib** (lihat §7) | aktif |
| `tokio-util` | [0.7.20](https://crates.io/api/v1/crates/tokio-util) | MIT | `CancellationToken` (shutdown bersih) | `rt` | aktif (2026-10-09) |
| `futures` | [0.3.34](https://crates.io/api/v1/crates/futures) | MIT OR Apache-2.0 | `StreamExt`/`TryStreamExt` untuk body streaming | `std`, `async-await` | aktif |

### ⚠ reqwest 0.13 — tiga feature berubah (WAJIB baca)

Briefing menyebut `rustls-tls`. Feature itu **tidak ada lagi** di 0.13. Verified dari
[CHANGELOG v0.13.0](https://raw.githubusercontent.com/seanmonstar/reqwest/master/CHANGELOG.md)
dan feature map crates.io:

| Salah | Benar (0.13.5) |
|---|---|
| `rustls-tls` | **`rustls`** — sudah **default**, dan `default-tls` sekarang *menunjuk ke* `rustls` |
| — | `rustls-no-provider` — pakai provider crypto sendiri (lihat §7) |
| `native-tls` default | native-tls **tidak** default lagi di 0.13 |

`query` dan `form` juga **tidak default** di 0.13 — harus di-enable eksplisit.

Konsekuensi untuk target kita: `rustls` sudah otomatis jadi backend, jadi **tetap tanpa
OpenSSL** (kewajiban terpenuhi). Tapi build-nya memakai **aws-lc-rs** yang butuh cmake +
C compiler. Untuk target `x86_64-unknown-linux-musl` yang sudah dipakai repo, ini wajib
diverifikasi saat spike — lihat §7.

---

## 2. Storage

| crate | versi terverifikasi | lisensi | dipakai untuk | feature penting | status |
|---|---|---|---|---|---|
| `libsql` | [0.9.30](https://crates.io/api/v1/crates/libsql) | MIT | Turso remote **dan** file lokal, satu API | ⚠ `core` + `remote`. **Jangan** `default` | ⚠ slow, 0.10 masih pre |
| `rusqlite` | [0.40.2](https://crates.io/api/v1/crates/rusqlite) | MIT | fallback lokal sinkron | `bundled` | paling aktif (2026-08-08) |
| `sqlx` | [0.9.0](https://crates.io/api/v1/crates/sqlx) | MIT OR Apache-2.0 | ❌ tidak dipakai | — | ditolak, alas di §6 |

### Rekomendasi: `libsql` 0.9.30

Alas parity. `src/lib/turso.js` + `config.js:240` menunjukkan repo memakai **Turso remote**
(`TURSO_URL` + `TURSO_AUTH_TOKEN`) dengan **fallback file JSON lokal** (`lowdb`, dipanggil di
`src/lib/database.js:122`). Kebutuhan itu:

- API async — `libsql` async (`Database::open_with_opts().await`, `conn.query().await`),
  diverifikasi via Context7 `/tursodatabase/libsql`.
- Dialect + format file sama persis dengan `@libsql/client` → file lama langsung kebaca.
- Satu crate menutup kedua mode. `rusqlite` sync-only (butuh `spawn_blocking`),
  `libsql-client` (crate terpisah) tidak punya mode lokal.

**Feature:** `default` = `core+replication+remote+sync+tls` — terlalu berat (replication,
TLS, wasm). Pakai `features = ["core", "remote"]`.

**Risiko:** ritme rilis stabil lambat — `0.9.30` (**2026-03-19**) sudah ~7 bulan, sementara
`0.10.0-pre.4` (2026-06-02) masih prerelease. Tidak archived; repo masih di-push 2026-10-01.
Kalau blocking naik ke 0.10 stable, re-evaluate.

---

## 3. Util

| crate | versi terverifikasi | lisensi | dipakai untuk | feature penting | status |
|---|---|---|---|---|---|
| `anyhow` | [1.0.104](https://crates.io/api/v1/crates/anyhow) | MIT OR Apache-2.0 | error di boundary plugin | `std` | aktif |
| `thiserror` | [2.0.21](https://crates.io/api/v1/crates/thiserror) | MIT OR Apache-2.0 | error typed di lib | `std` | aktif |
| `tracing` | [0.1.44](https://crates.io/api/v1/crates/tracing) | MIT | logging terstruktur (ganti `pino`) | `attributes` | 0.1.x stabil; ritme lambat wajar |
| `tracing-subscriber` | [0.3.23](https://crates.io/api/v1/crates/tracing-subscriber) | MIT | subscriber + filter | `env-filter`, `json` | aktif |
| `moka` | [0.12.16](https://crates.io/api/v1/crates/moka) | (MIT OR Apache-2.0) AND Apache-2.0 | cache (ganti `lru-cache`) | ⚠ `future` — `default` **kosong** | aktif |
| `governor` | [0.10.4](https://crates.io/api/v1/crates/governor) | MIT | rate limit (ganti `rate-limiter-flexible`) | `std`, `jitter` | aktif (repo push 2026-08-24) |
| `dashmap` | [6.2.1](https://crates.io/api/v1/crates/dashmap) | MIT | ganti `global.*` yang race-prone | `inline` | aktif |
| `once_cell` | [1.21.4](https://crates.io/api/v1/crates/once_cell) | MIT OR Apache-2.0 | lazy static | `std` | aktif |
| `arc-swap` | [1.9.2](https://crates.io/api/v1/crates/arc-swap) | MIT OR Apache-2.0 | swap konfigurasi lock-free | — | aktif |
| `regex` | [1.13.1](https://crates.io/api/v1/crates/regex) | MIT OR Apache-2.0 | parsing teks (ganti `he`, `qs`) | `perf` | aktif |
| `dotenvy` | [0.15.7](https://crates.io/api/v1/crates/dotenvy) | MIT | baca `.env` | `cli` (opsional) | ⚠ rilis 2023 — stale tapi ~60 baris, aman |
| `sysinfo` | [0.39.6](https://crates.io/api/v1/crates/sysinfo) | MIT | baca RSS sendiri (memory-monitor) | `system` | aktif (repo push 2026-10-09) |

**`sysinfo` lisensi aman.** Versi lama (0.30-an) LGPL-3.0+; semua 0.38.x/0.39.x terverifikasi
**MIT**. Jangan turunkan ke `<0.38` karena reposisi lisensi.

### Timezone: `chrono` + `chrono-tz` (bukan `time`)

| crate | versi | lisensi | catatan |
|---|---|---|---|
| `chrono` | [0.4.45](https://crates.io/api/v1/crates/chrono) | MIT OR Apache-2.0 | dipakai juga oleh `libsql`, `tokio-cron-scheduler`, `whatsapp-rust` — **sudah transitif** |
| `chrono-tz` | [0.10.4](https://crates.io/api/v1/crates/chrono-tz) | MIT OR Apache-2.0 | database IANA untuk scheduler sholat |
| `time` | [0.3.55](https://crates.io/api/v1/crates/time) | MIT OR Apache-2.0 | ❌ tidak dipakai |

Pakai `chrono`+: sudah jadi dependensi wajib di hampir semua crate WA/DB, jadi menambah
`chrono-tz` benar-benar gratis. `time` crate butuh `local-offset` + thread_local sendiri
dan tidak punya database IANA — tidak ada win. `chrono-tz` rilis terakhir 2025-07-11, tapi
databasenya statis (hasil bake IANA), jadi staleness bukan masalah.

---

## 4. Media

| crate | versi terverifikasi | lisensi | dipakai untuk | feature penting | status |
|---|---|---|---|---|---|
| `image` | [0.25.10](https://crates.io/api/v1/crates/image) | MIT OR Apache-2.0 | ganti `sharp` | ⚠ `default-features = false` + `jpeg`,`png`,`webp` | aktif |
| `tiny-skia` | [0.12.0](https://crates.io/api/v1/crates/tiny-skia) | **BSD-3-Clause** | raster dasar untuk canvas | `std`, `simd` | aktif |
| `cosmic-text` | [0.19.0](https://crates.io/api/v1/crates/cosmic-text) | MIT OR Apache-2.0 | ⚠ shaping teks di atas tiny-skia | `std` | aktif |
| `resvg` | [0.48.1](https://crates.io/api/v1/crates/resvg) | Apache-2.0 OR MIT | render SVG (ganti `brat-canvas`) | `text`, `system-fonts` | aktif |
| `ab_glyph` | [0.2.32](https://crates.io/api/v1/crates/ab_glyph) | Apache-2.0 | ❌ tidak dipakai | — | kalah vs `cosmic-text`, §6 |
| `zip` | [9.0.1](https://crates.io/api/v1/crates/zip) | MIT | sticker pack (ganti `adm-zip`/`archiver`) | ⚠ `default-features = false` + `deflate` | aktif (2026-10-10) |
| `infer` | [0.22.0](https://crates.io/api/v1/crates/infer) | MIT | deteksi tipe by magic byte (ganti `file-type`) | `std` | aktif |
| `mime_guess` | [2.0.5](https://crates.io/api/v1/crates/mime_guess) | MIT | ext→mime (ganti `mime-types`) | — | ⚠ rilis 2024 — stale tapi stabil |
| `phonenumber` | [0.3.10+9.0.33](https://crates.io/api/v1/crates/phonenumber) | Apache-2.0 | ganti `awesome-phonenumber` | — | ⚠ field lisensi crates.io `null`, diverifikasi ke LICENSE upstream |
| `qrcode` | [0.14.1](https://crates.io/api/v1/crates/qrcode) | MIT OR Apache-2.0 | render QR (ganti `qrcode` npm) | `svg` | ⚠ rilis 2024 |
| `base64` | [0.23.1](https://crates.io/api/v1/crates/base64) | MIT OR Apache-2.0 | encode media | `std` | aktif |
| `sha2` / `hmac` / `aes-gcm` | [0.11.0](https://crates.io/api/v1/crates/sha2) / [0.13.0](https://crates.io/api/v1/crates/hmac) / [0.11.1](https://crates.io/api/v1/crates/aes-gcm) | MIT OR Apache-2.0 / Apache-2.0 OR MIT | **bersyarat** — hanya kalau storage dienkripsi | `std` | aktif; **tidak perlu untuk protokol WA** |

### Canvas: `tiny-skia` + `cosmic-text` (+ `resvg` untuk SVG)

Repo punya `fillText` **132×**, `measureText` **57×**, `GlobalFonts` **30×** di 33 file —
teks adalah mayoritas pemakaian canvas, bukan gambar. `tiny-skia` saja **tidak bisa shaping
teks** (dia hanya raster path). Jadi:

- **`cosmic-text` wajib** — shaping, font fallback, `measureText`, layout multi-line.
- `ab_glyph` ❌ — bisa raster glyph tapi **tanpa shaping**, jadi emoji/CJK/ligatur rusak.
  Itu risiko fungsional nyata di bot multilingual, bukan sekadar soal bobot.
- `resvg` perlu karena `brat-canvas` dipakai untuk SVG. Kalau semua jalur SVG bisa diganti
  `image`, `resvg` bisa di-skip dan hemat satu crate.

### ⚠ `image` — jangan pakai `default`

`default` = `rayon + default-formats`, dan `default-formats` menarik **`avif`** → `ravif`
(AVIF *encoder*, lambat dan berat). Repo tidak pernah menghasilkan AVIF — hasil scan
`package.json` + call-site: `jpeg` 14×, `png` 8×, `webp` 7×, `mp4` 1×. Set
`default-features = false`, enable hanya tiga format itu. Hemat build time dan binary size
yang nyata.

### ⚠ `zip` — `default` sangat berat

`default` = `aes-crypto+bzip2+deflate64+deflate+lzma+ppmd+time+zstd+xz`. Untuk sticker pack
cukup `deflate`. **Lisensi 9.0.1 = MIT** — field `non-standard` hanya ada di `0.10.0-alpha.1`
(2023) yang sudah ditinggalkan.

---

## 5. Lain-lain

| crate | versi terverifikasi | lisensi | dipakai untuk | feature penting | status |
|---|---|---|---|---|---|
| `whatsapp-rust` | [0.7.0](https://crates.io/api/v1/crates/whatsapp-rust) | MIT | ⚠ layer WA (pengganti Baileys) | `sqlite-storage`, `tokio-transport`, `tokio-runtime` | ⚠ muda, 895★, push 2026-10-10 |
| `waproto` | [0.7.0](https://crates.io/api/v1/crates/waproto) | MIT | protobuf WA (transitif — **jangan** dipakai langsung) | default | ikut `whatsapp-rust` |
| `cron` | [0.17.0](https://crates.io/api/v1/crates/cron) | MIT OR Apache-2.0 | parse ekspresi jadwal | `serde` (opsional) | ✅ **rekomendasi** — parser, aktif |
| `tokio-cron-scheduler` | [0.15.1](https://crates.io/api/v1/crates/tokio-cron-scheduler) | MIT/Apache-2.0 | scheduler batteries-included | `signal` | ⚠ rilis 2025-10-28 (~1 tahun) |
| `russh` | [0.64.1](https://crates.io/api/v1/crates/russh) | Apache-2.0 | ganti `ssh2` (plugin panel) | `rsa`, `flate2` | ⚠ async/tokio, aktif (push 2026-10-05) |
| `russh-keys` | [0.49.2](https://crates.io/api/v1/crates/russh-keys) | Apache-2.0 | key parsing untuk russh | — | ⚠ 0.50 masih beta |
| `clap` | [4.6.7](https://crates.io/api/v1/crates/clap) | MIT OR Apache-2.0 | CLI args | `derive`, `env` | aktif |
| `scraper` | [0.27.0](https://crates.io/api/v1/crates/scraper) | **ISC** | ganti `cheerio` | `serde` (opsional) | ✅ aktif (push 2026-10-05, 2424★) |
| `kuchikiki` | [0.8.2](https://crates.io/api/v1/crates/kuchikiki) | MIT | ❌ tidak dipakai | — | kalah vs `scraper`, §6 |

### ⚠ Repo moved — URL lama salah

| crates.io bilayer | Repo sebenarnya (verified via GitHub API) |
|---|---|
| `github.com/jlucaso1/whatsapp-rust` | **github.com/oxidezap/whatsapp-rust** |
| `github.com/warp-tech/russh` | **github.com/Eugeny/russh** |

Keduanya balas "Moved Permanently". Kalau URL lama di-bookmark atau dipakai CI checkout,
build akan gagal.

### Cron: pilih `cron`, bukan `tokio-cron-scheduler`

`cron` adalah **pure parser** — docs.rs: "A cron expression parser and schedule explorer".
Tidak ada scheduler, tidak ada timer. `tokio-cron-scheduler` yang menyediakan loop async-nya
(dependensi `croner`, `tokio`, `chrono`, `chrono-tz`), tapi rilis terakhir 2025-10-28 —
~1 tahun stale meski repo masih di-push.

Repo sudah punya scheduler sendiri (`src/lib/ourin-scheduler.js`), jadi cukup pakai `cron`
(parser, aktif, dependency kecil) + loop `tokio::time::sleep` sendiri ~20 baris.
`tokio-cron-scheduler` bawa fitur yang tidak kita pakai: NATS storage, Postgres storage,
`prost`, `english` — semuanya di balik feature, tapi menambah surface area tanpa manfaat.

### `russh` — async, tapi cek pas spike

`russh` 0.64.1 murni async di atas tokio (`russh::client::connect()` mengembalikan future,
`Handle` untuk channel). Tapi dokumentasinya cuma **49.93%** ter-cover dan `russh-keys`
terpisah dengan 0.50 masih beta. Plugin panel (`plugins/panel/installtema*.js`, 5 file)
pakai `ssh2` untuk theme install — port ke russh harus diuji di spike, bukan diasumsikan.

---

## 6. Dilarang pakai

| crate | Versi | Alasan dilarang |
|---|---|---|
| `sqlx` | 0.9.0 | Fiturnya `macros`/`migrate` memaksa `DATABASE_URL` **saat compile** untuk query checking. Build bot tidak boleh bergantung pada DB yang hidup. Butuh async multi-driver yang tidak kita pakai. |
| `ab_glyph` | 0.2.32 | Tanpa text shaping. `fillText` 132× + bot multilingual → emoji/CJK/ligatur akan rusak. Bukan soallisensi (Apache-2.0 aman), tapi soal fungsi. |
| `kuchikiki` | 0.8.2 | `scraper` 0.27.0 lebih aktif (push 2026-10-05, 2424★, 14 issue) dan API-nya stabil. `kuchikiki` versi terbaru bahkan berupa tag(non-semver) `0.8.9-speedreader`. |
| `time` | 0.3.55 | Butuh `local-offset` + thread_local sendiri, tidak punya database IANA, sementara `chrono` sudah jadi dependensi transitif hampir semua crate WA/DB. Nol win, nol penghematan. |
| `native-tls` (reqwest) | — | Butuh OpenSSL dev — persis yang tidak ada di Pterodactyl. 0.13 sudah tidak default; jangan di-enable manual. |
| `ffmpeg-sys` | 4.3.3 | ⚠ **Rilis terakhir 2021-07-10**, lisensi WTFPL, hanya 209K download. Sampah. Tetap `tokio::process::Command` ke binary ffmpeg — sesuai verdict `rust-migration-audit.md` §2.5 (RSS Node flat, rewrite tanpa win). |
| `prost` | 0.14.4 | Tidak perlu memanggil protobuf WA sendiri. `waproto` 0.7.0 (transitif dari `whatsapp-rust`) sudah menangani. Menambah `prost` = dua versi protobuf di satu binary. |
| `tesseract-rs` (baru) | 0.4.0 | **Sudah dipakai** di `native/Cargo.toml:12` (`features = ["embed-tessdata"]`) untuk OCR production. Jangan tambah dependensi kedua; reuse `ourin-native` sesuai §2.1 audit. |
| `dotenvy` (`cli` feature) | 0.15.7 | Mem drags `clap` ke dotenv. Kita sudah pakai `clap` sendiri — feature-nya dobel. |

---

## 7. Tiga keputusan paling berisiko

### 1. `reqwest` 0.13 — aws-lc-rs vs musl

Briefing minta `rustls-tls`. Feature itu **sudah tidak ada** — 0.13.0 me-rename jadi
`rustls` dan menjadikannya default, dengan provider crypto **aws-lc-rs** (bukan `ring`).
Kewajiban "tanpa OpenSSL" tetap terpenuhi, tapi aws-lc-rs butuh **cmake + C compiler**
saat build. Repo sudah punya target `x86_64-unknown-linux-musl` (`rust-migration-audit.md`
§3.2) yang butuh cross-compile — dan aws-lc-rs notorious susah di-cross-compile ke musl.
Opsi: `rustls-no-provider` + pasang provider `ring` sendiri. **Belum diuji — wajib jadi
satu item spike sebelum commit.**

### 2. `libsql` 0.9.30 — stabil 7 bulan, 0.10 masih pre

Ini satu-satunya di daftar yang ril stabilnya tertinggal jauh: `0.9.30` keluar
**2026-03-19**, sementara `0.10.0-pre.4` (2026-06-02) masih prerelease. Dipilih karena
parity — repo benar-benar pakai Turso remote + fallback lokal, dan `libsql` async menutup
keduanya dengan format file yang sama. Tapi kalau versi 0.10 tidak pernah rilis stable,
kita terkunci. Mitigasi: `rusqlite` 0.40.2 (rilis 2026-08-08, paling aktif) sebagai plan B —
hanya butuh `spawn_blocking` untuk await.

### 3. `serde_json` `preserve_order` — wajib, diam-diam menentukan diff

Tanpa feature ini `serde_json::Map` = `BTreeMap` → **urutan key berubah** setiap
serialize. Repo menyimpan 8 file JSON lewat lowdb (`database.js:127-153`) dan mem-`merge`
default di atas data (`{ ...defaults, ...store.data }`). Preserve-order bukan.Jpa cosmetics:
tanpa itu setiap flush menulis ulang file dengan urutan key berbeda → churn diff besar,
`git diff` spam, dan risk regressi kalau ada kode yang bergantung urutan. Feature `preserve_order`
(→ `indexmap`) **ada dan terverifikasi** di 1.0.151.

---

## Ringkasan 10 crate inti

| # | crate | versi | lisensi | alasan inti |
|---|---|---|---|---|
| 1 | `whatsapp-rust` | 0.7.0 | MIT | layer WA — satu-satunya blockbuster |
| 2 | `tokio` | 1.53.2 | MIT | runtime; `full` cukup, tanpa fitur tambahan |
| 3 | `reqwest` | 0.13.5 | MIT OR Apache-2.0 | HTTP; `rustls` + `json`/`multipart`/`cookies`/`gzip`/`brotli`/`stream` |
| 4 | `libsql` | 0.9.30 | MIT | Turso remote + lokal, async, format kompatibel |
| 5 | `serde` + `serde_json` | 1.0.229 / 1.0.151 | MIT OR Apache-2.0 | `derive` + `preserve_order` |
| 6 | `tokio-util` | 0.7.20 | MIT | `CancellationToken` untuk shutdown bersih |
| 7 | `tracing` + `tracing-subscriber` | 0.1.44 / 0.3.23 | MIT | ganti `pino`, `env-filter` |
| 8 | `image` | 0.25.10 | MIT OR Apache-2.0 | ganti `sharp`, trim ke jpeg/png/webp |
| 9 | `tiny-skia` + `cosmic-text` | 0.12.0 / 0.19.0 | BSD-3-Clause / MIT OR Apache-2.0 | ganti `@napi-rs/canvas`, incl. shaping teks |
| 10 | `moka` | 0.12.16 | (MIT OR Apache-2.0) AND Apache-2.0 | cache, `future` (default kosong!) |

Semua versi diverifikasi **2026-10-10** lewat crates.io API + docs.rs + GitHub API.