//! `plugins/owner/setreply.js` — mengatur variant tampilan reply.
//! `pub mod setreply;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `sock.sendButton` dengan `getAssetBuffer("foto")` dan tombol interaktif
//!   tidak tersedia pada `Reply` (hanya `Reply::Interactive { body }` tanpa
//!   tombol).
//! - `db.setting(...)` + `db.save()` untuk `replyVariant` sebagian tersedia,
//!   namun menu interaktif intinya tidak dapat di-port apa adanya.
