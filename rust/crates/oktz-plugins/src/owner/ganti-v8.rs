//! `plugins/owner/ganti-v8.js` — ganti gambar v8.jpg (thumbnail welcome).
//!
//! BLOKIR (gelombang-5 §Catatan tambahan): plugin butuh `fs` (tulis
//! `config.js`), `sharp` (resize ke jpeg), dan `updateAssetUrl` dari
//! `src/lib/uploader.js` yang butuh `fs`+`sharp`+`node-upload-images`.
//! Kapabilitas tersebut belum ada di `Ctx`. Jangan port paksa. File ini
//! dibuat agar "semua file ada"; implementasi ditunda sampai kapasitas
//! tulis-berkas tersedia.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - guard: `m.isImage` atau quoted `imageMessage`; kalau tidak → teks pakai.
//!   - unduh buffer dari quoted atau pesan pemicu.
//!   - `updateAssetUrl('v8', buffer, 'v8.jpeg')` → path baru; tulis
//!     `config.assets.v8` ke `config.js`.
//!   - balas sukses dengan path; error → pesan gagal upload / `te(...)`.
//!
//! `ponytail:` saat `fs`/`sharp`/`updateAssetUrl` tersedia, tambahkan
//! `pub mod ganti_v8;` di `src/owner/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief).
