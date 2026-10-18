//! `plugins/sticker/bratvid.js` — sticker brat animated.
//!
//! Catatan: `pub mod bratvid;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/sticker/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `brat-canvas/video` (`bratVid`) untuk merender video brat: paket npm ini
//!   tidak memiliki padanan di Rust/API plugin, dan `oktz_plugin_api::gambar`
//!   tidak mendukung rendering teks/animasi video.
//! - `ffmpeg` melalui `ensureFfmpegOnPath()` tersedia lewat
//!   `oktz_plugin_api::ffmpeg`, tapi tanpa `bratVid` tidak ada input video.
//! - `sock.sendVideoAsSticker(tempFile, ...)`: `Reply::Video` tersedia, tapi
//!   input buffer dari brat-vid tidak ada.
//! - `config.sticker.packname` / `author` tersedia via `ctx.config().sticker`.
