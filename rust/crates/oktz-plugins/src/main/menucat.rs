//! `plugins/main/menucat.js` — menu commands per kategori.
//!
//! Catatan: `pub mod menucat;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/main/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `getCommandsByCategory()` / `getPlugin()` / `getCasesByCategory()`
//!   (`src/lib/plugins.js`): registry plugin Rust tersedia via
//!   `oktz_plugin_api::all_plugins()`, tapi "cases" (case-based commands) dan
//!   simbol permission (`ⓄⓟⓁⒶⒼⓅ`) bukan bagian dari `PluginMeta`/`Reply`.
//! - `onigis` + `generateWAMessageFromContent` + `wrapInteractive` untuk
//!   interactive message dengan tombol quick_reply / bottom_sheet / location:
//!   `Reply::Interactive` hanya punya `body` (tidak ada header media,
//!   nativeFlowMessage, tombol).
//! - `sock.relayMessage` + `viewOnceMessage` + `contextInfo` newsletter:
//!   tidak ada di `Reply`.
//! - `getAssetBuffer("foto2")` / `safeThumbnail` + cuaca via open-meteo:
//!   HTTP tersedia, tapi aset buffer + komposisi interaktif tidak.
