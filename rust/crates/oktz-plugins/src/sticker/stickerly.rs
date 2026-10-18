//! `plugins/sticker/stickerly.js` — cari/download sticker pack Sticker.ly.
//!
//! Catatan: `pub mod stickerly;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/sticker/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `axios` GET ke `my.izuka-api.xyz/api/search/stickerly-search` + pack:
//!   HTTP tersedia, jadi bagian pencarian bisa direplikasi.
//! - `sock.sendStickerPack(m.chat, urlsToProcess, ...)` untuk mengirim sekaligus
//!   banyak sticker pack: tidak ada di `Reply` (hanya `Reply::Sticker` satu
//!   sumber media).
//! - `sock.sendImageAsSticker(m.chat, sUrl, ...)` dari URL: `Reply::Sticker`
//!   dengan `MediaSource::Url` tersedia, jadi pengiriman per-sticker bisa
//!   direplikasi.
//! - `db.getUser(m.sender)` / `db.save()` untuk `stickerly_session`: store
//!   user tidak diekspos lewat `Ctx` untuk tulis bebas; `session_*` tersedia
//!   tapi berbentuk `Value` global, bukan `db.getUser` objek penuh.
//! - `stickerlyAnswerHandler` (answerHandler): tersedia via
//!   `#[answer_handler]`, tapi butuh store user yang tidak ada.
//! - `m.body` / `m.isCommand` tidak ada di `Ctx` (hanya `text`/`command`).
