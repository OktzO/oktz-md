//! `plugins/owner/ganti-foto2.js` — ganti gambar foto2.jpg.
//!
//! BLOKIR (gelombang-5 §Catatan tambahan): plugin butuh `fs` (tulis
//! `config.js`), `sharp` (resize ke jpeg), dan `updateAssetUrl` dari
//! `src/lib/uploader.js` yang itself butuh `fs`+`sharp`+`node-upload-images`.
//! Kapabilitas tersebut belum ada di `Ctx`. Jangan port paksa. File ini
//! dibuat agar "semua file ada"; implementasi ditunda sampai kapasitas
//! tulis-berkas tersedia.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - guard: `m.isImage` atau quoted `imageMessage`; kalau tidak → teks pakai.
//!   - unduh buffer dari quoted atau pesan pemicu.
//!   - `updateAssetUrl('foto2', buffer, 'foto2.jpeg')` → URL baru; tulis
//!     `config.assets.foto2` ke `config.js` (regex/replace).
//!   - balas sukses dengan URL; error → `te(...)` atau pesan gagal upload.
//!
//! `ponytail:` saat `fs`/`sharp`/`updateAssetUrl` tersedia, tambahkan
//! `pub mod ganti_foto2;` di `src/owner/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief). Nama berkas `ganti-foto2.rs`
//! dipakai agar tidak bentrok dengan `ganti-foto.js` kalau ada.
