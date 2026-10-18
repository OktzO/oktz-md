//! `plugins/sticker/smeme-animated.js` — stiker meme animasi.
//!
//! Catatan: `pub mod smeme_animated;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/sticker/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `uploadImage(buffer)` (`src/lib/uploader.js`) untuk unggah gambar ke CDN:
//!   tidak ada padanan uploader di API plugin.
//! - `downloadMediaMessage` (`onigis`) untuk download media pesan:
//!   `ctx.download_media()` / `ctx.download_quoted_media()` tersedia, jadi
//!   bagian download bisa direplikasi; unggah hasilnya tidak.
//! - `axios` GET ke `api.neoxr.eu/api/memegenvid`: HTTP tersedia, tapi butuh
//!   `imageUrl` dari uploadImage yang tidak ada.
//! - `sock.sendImageAsSticker` / `sendVideoAsSticker`: `Reply::Sticker` /
//!   `Reply::Video` tersedia, tapi input buffer dari API Neoxr tidak tersedia.
