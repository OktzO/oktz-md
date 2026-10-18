//! `plugins/main/sc.js` — link script bot (interactive card).
//!
//! Catatan: `pub mod sc;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/main/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `onigis` + `generateWAMessageFromContent` + `wrapInteractive` untuk
//!   interactive message dengan `cta_url` tombol (Kunjungi GitHub):
//!   `Reply::Interactive` hanya punya `body`, tidak ada native flow / tombol.
//! - `sock.relayMessage` + `{ quoted: m, userJid: sock.user?.id }`:
//!   tidak ada di `Reply`.
//! - `config.bot.name/version/developer` tersedia via `ctx.config().bot`;
//!   `config.assets` tidak dipakai di plugin ini (hanya teks statis).
