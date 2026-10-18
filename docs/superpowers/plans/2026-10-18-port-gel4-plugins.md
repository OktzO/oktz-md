# Port Plugin JS ke Rust (gel4-8) — Implementation Plan

> **For agentic workers:** implement via superpowers:executing-plans (native, one session). Steps use checkbox syntax.

**Goal:** Port 20 plugin JS dari `plugins/<kategori>/<nama>.js` ke `rust/crates/oktz-plugins/src/<kategori>/<nama>.rs` sesuai `docs/rust-rewrite/PORT-BRIEF.md`, dengan daftar output: berkas yang dibuat, jumlah test per berkas, dan berkas yang tidak jadi di-port beserta alasannya. Tidak menyentuh `mod.rs` mana pun; tidak menjalankan `cargo`; tidak `git commit/add`.

**Architecture:** Setiap plugin yang ter-portabilitas menjadi satu file Rust mandiri dengan `#[plugin(...)]` + `pub fn <nama>(ctx: Ctx) -> Reply`, logika bantu di dalam file itu (duplikasi sengaja dengan catatan kepala), dan `#[cfg(test)] mod tests` yang stub `HttpGet` lokal. Plugin yang butuh API yang tidak ada di `Ctx`/`Reply`/`Sok`/`Berkas` dicatat sebagai tidak ter-portabilitas dengan alasan.

**Tech Stack:** Rust, crate `oktz-plugin-api` (Ctx, Reply, HttpGet, HttpCall, MediaSource, Mutation, plugin macro), `oktz-scraper` (img2prompt), `serde_json`, `rand`, `linkme`.

**Spec:** `docs/rust-rewrite/PORT-BRIEF.md` (mengikat), `docs/rust-rewrite/PORTING-PLAN.md` (referensi daftar tidak portabel), `gel4-8.md` (daftar 20 berkas milik saya).

## Global Constraints

- JS produksi adalah acuan mutlak. Rust dan JS berbeda → JS yang benar, jangan memperbaiki JS. Format angka tidak diseragamkan.
- File scope eksklusif: hanya file Rust milik plugin itu. Dilarang sentuh `Cargo.toml`, `lib.rs`, `common.rs`, `pipeline.rs`, `registry.rs`, plugin lain, `mod.rs`. Helper bersama disalin ke file miliknya dengan catatan kepala bahwa itu duplikat.
- Dilarang menjalankan `cargo` (build/test/check). Tidak boleh mengklaim test hijau.
- Dilarang `git commit`, `git add`, atau mengubah branch.
- Komentar, doc comment, dan nama identifier bahasa Indonesia. Huruf CJK dilarang kecuali glyph yang memang ada di teks tampilan JS.
- Satu plugin = satu file Rust di `src/<kategori>/`. Tidak mkdir kategori baru (semua kategori target sudah ada).
- Jika `mod.rs` bukan milik saya dan sudah memuat plugin lain, jangan sentuh; tulis catatan di kepala file bahwa baris `pub mod` perlu ditambah manual.
- `#[plugin]` default `is_private = true` → harus diisi `is_private = false` secara eksplisit untuk plugin bukan private-only.

## Review Focus (input yang belum dicakup tes karena API tidak ada)

- Plugin butuh `getDatabase()` / store arbitrer (`delsewa`, `resetdb`, `listdaftar`, `profile`, `gachahusbu`) → tidak ada di `Ctx`.
- Plugin butuh `sock.relayMessage`/`sendMessage`/`sendButton`/`newsletterCreate` (`allmenu`, `gachahusbu`, `mywaifu` album, `profile`, `setallmenu`, `sprem`, `tourl`, `buatsaluran`) → `Reply` tidak punya tombol interaktif/native flow; `Sock` tidak punya kirim pesan.
- Plugin butuh `m.isPremium`/`m.pushName`/`crypto.randomBytes`/`responseUrl` redirect → tidak ada di `Ctx`.
- `Berkas` tidak punya pemeriksaan direktori + hapus rekursif (`clearsessions` `fs.rmSync recursive`).
- Cheerio parser besar + `sendButton` (`tiktokdl`, `tourl`).

---
### Task 1: Port `canvas/fakecall.js` → `canvas/fakecall.rs`

**Files:**
- Create: `rust/crates/oktz-plugins/src/canvas/fakecall.rs`

**Interfaces:** `pub fn fakecall(ctx: Ctx) -> Reply` via `#[plugin(name="fakecall", alias=["fakecall-android","fakecall-ios"], category="canvas", ...)]`. Pakai `ctx.http` (HttpCall multipart), `ctx.sock().foto_profil`, `Reply::image`.

- [ ] Step: tulis `fakecall.rs` + `#[cfg(test)] mod tests` (stub HttpGet multipart, cek URL endpoint + parts avatar/name/duration, foto profil fallback, gambar keluar, error template).

### Task 2: Port `tools/imgtoprompt.js` → `tools/imgtoprompt.rs`

**Files:**
- Create: `rust/crates/oktz-plugins/src/tools/imgtoprompt.rs`

**Interfaces:** `pub fn imgtoprompt(ctx: Ctx) -> Reply` via `#[plugin(...)]`. Pakai `oktz_scraper::img2prompt(&ctx.http, &bytes)`, `ctx.download_media()`/`download_quoted_media()`, `Reply::text`.

- [ ] Step: tulis `imgtoprompt.rs` + tes (bukan gambar → tolak, unduh gagal → teks, scraper mengembalikan prompt/generatedAt, status eror → teks gagal, body bukan JSON → objek kosong).

### Task 3: Port `tts/ttselon.js` → `tts/ttselon.rs`

**Files:**
- Create: `rust/crates/oktz-plugins/src/tts/ttselon.rs`

**Interfaces:** `pub fn ttselon(ctx: Ctx) -> Reply` via `#[plugin(...)]`. Pakai `ctx.http` GET API Emilia, `oktz_plugin_api::ffmpeg::jalankan_ke_berkas`, `ctx.berkas` (tulis/baca/hapus, buat_folder "temp"), `Reply::voice_note`. Ponytail: `saluranCtx()` (contextInfo) diabaikan; objek config live tidak di-mutasi.

- [ ] Step: tulis `ttselon.rs` + tes (teks kosong → bantuan, API tanpa suara elon → teks gagal, susunan argumen ffmpeg, URL API, voice_note keluar dari bytes opus).

### Task 4: Port `anime/mywaifu.js` → `anime/mywaifu.rs`

**Files:**
- Create: `rust/crates/oktz-plugins/src/anime/mywaifu.rs`

**Interfaces:** `pub fn mywaifu(ctx: Ctx) -> Reply` via `#[plugin(...)]`. Pakai `ctx.http` GET `api.waifu.im/images`, `Reply::sequence` (teks caption + 10 image `MediaSource::Url`). Ponytail: `crypto.randomBytes` (messageSecret) diabaikan; `sock.waUploadToServer`/`relayMessage` diganti pipeline media Url.

- [ ] Step: tulis `mywaifu.rs` + tes (mode bukan sfw/nsfw → bantuan, HTTP tidak ada → error template + reaksi ❌, items kosong → teks tidak ditemukan + reaksi ❌, berhasil → Sequence 11 elemen dengan caption + 10 image url, reaksi ✅, urutan query params).

### Task 5: Port `owner/ganti-namadev.js` → `owner/ganti_namadev.rs`

**Files:**
- Create: `rust/crates/oktz-plugins/src/owner/ganti_namadev.rs`

**Interfaces:** `pub fn ganti_namadev(ctx: Ctx) -> Reply` via `#[plugin(name="ganti-namadev", ...)]`. Pakai `ctx.berkas()` baca_teks/tulis_teks `config.js`, regex ganti `developer: '...'`, `error_template`. Ponytail: objek config runtime live tidak dapat di-mutasi (hanya berkas ditulis).

- [ ] Step: tulis `ganti_namadev.rs` + tes (nama kosong → kartu pakai, nama baru → regex replace + tulis + konfirmasi, file tidak ada → error template, developer tanpa tanda kutip tidak terganti seperti JS).

### Task 6: Catat berkas yang tidak jadi di-port + alasan

**Files:** Tidak ada berkas Rust baru. Output dalam respons akhir.

- [ ] Step: susun daftar 15 berkas tidak ter-portabilitas dengan alasan masing-masing yang merujuk API yang tidak tersedia (`getDatabase`, `sock.relayMessage`/`sendButton`/`newsletterCreate`, `m.isPremium`, `Berkas` tanpa direktori rekursif, cheerio besar, `uploadImage`, `runUploadFanout`).

## Self-review (selesai)

- Spec coverage: semua 20 berkas dari gel4-8.md baik di-port (5) maupun dicatat tidak port (15), masing-masing ada tugas atau catat alasan.
- Semua file target satu per kategori yang sudah ada (canvas, tools, tts, anime, owner) → tidak perlu mkdir kategori baru.
- Tidak ada sentuhan `mod.rs`; catatan `pub mod` manual akan ada di kepala setiap file baru.
- Test per file ditulis dengan stub HttpGet; tidak ada klaim test hijau.

Output akhir (respons ke user): daftar berkas Rust yang dibuat, jumlah test per berkas, dan 15 berkas tidak jadi di-port + alasan.
