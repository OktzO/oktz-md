//! `plugins/owner/ganti-mp3.js` — ganti audio `assets/audio/foto.mp3`.
//!
//! Catatan: `pub mod ganti_mp3;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/owner/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `updateAssetUrl('mp3', buffer, 'foto.mp3')` (`src/lib/uploader.js`)
//!   untuk unggah audio ke CDN dan update config realtime: tidak ada padanan
//!   uploader di API plugin. `ctx.berkas().tulis` hanya menulis ke akar aset,
//!   bukan mengganti `config.js`/`assets/audio/foto.mp3` melalui upload.
//! - `m.type === 'audioMessage'` deteksi media tersedia via `ctx.media`, tapi
//!   unggah hasilnya adalah bagian yang tidak bisa direplikasi.
