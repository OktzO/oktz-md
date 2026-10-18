//! `plugins/sticker/ttp.js` — teks ke gambar/stiker.
//!
//! Catatan: `pub mod ttp;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/sticker/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `@napi-rs/canvas` (`createCanvas`, `ctx.fillText`, `measureText`,
//!   `Path2D`, font) untuk rendering teks ke PNG: `oktz_plugin_api::gambar`
//!   hanya mendukung resize/crop/grayscale/encode, tidak ada rendering teks
//!   atau vektor. Tidak ada padanan canvas di API plugin.
//! - `sock.sendImageAsSticker(m.chat, buffer, ...)`: `Reply::Sticker`
//!   tersedia, tapi input buffer hasil canvas tidak ada.
//! - `config.sticker.packname` / `author` tersedia via `ctx.config().sticker`.
