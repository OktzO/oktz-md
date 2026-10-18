//! `plugins/owner/ganti-mp4.js` — ganti video foto.mp4.
//!
//! BLOKIR (gelombang-5 §Catatan tambahan): plugin butuh `fs` (tulis
//! `config.js`), `sharp` (opsional, untuk jpeg; mp4 tidak di-resize), dan
//! `updateAssetUrl` dari `src/lib/uploader.js` yang butuh `fs`+`sharp`.
//! Kapabilitas tersebut belum ada di `Ctx`. Jangan port paksa. File ini
//! dibuat agar "semua file ada"; implementasi ditunda sampai kapasitas
//! tulis-berkas tersedia.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - guard: `m.type === 'videoMessage'` atau quoted `videoMessage`; kalau
//!     tidak → teks pakai.
//!   - unduh buffer dari quoted atau pesan pemicu.
//!   - `updateAssetUrl('mp4', buffer, 'foto.mp4')` → path baru; tulis
//!     `config.assets.mp4` ke `config.js`.
//!   - balas sukses dengan path; error → pesan gagal upload / `te(...)`.
//!
//! `ponytail:` saat `fs`/`sharp`/`updateAssetUrl` tersedia, tambahkan
//! `pub mod ganti_mp4;` di `src/owner/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief).
