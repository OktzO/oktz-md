//! `plugins/search/playch.js` — putar musik ke saluran (convert opus).
//!
//! Catatan: `pub mod playch;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/search/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `yt-search` (`yts`) untuk pencarian YouTube: paket npm, tidak ada di
//!   API plugin, dan tidak ada padanan pencarian YouTube.
//! - `../../src/lib/resolve.js` (`resolver.resolve("youtube", ...)`): library
//!   internal bot, tidak ada di API plugin.
//! - `ffmpeg` konversi ke opus/ogg: `oktz_plugin_api::ffmpeg` tersedia, tapi
//!   tanpa sumber audio (yt-search + resolver) tidak ada input.
//! - `sock.sendMessage(chId, { audio, ptt, waveform })` ke saluran:
//!   `Reply::Audio` tersedia, tapi pengiriman ke `chId` selain `m.chat` dan
//!   waveform kustom bukan bagian dari `Reply`.
//! - `config.saluran.id` tersedia via `ctx.config().saluran.id`.
