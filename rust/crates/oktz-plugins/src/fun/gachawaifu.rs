//! `plugins/fun/gachawaifu.js` — gacha waifu, jaga perasaan, nikahi.
//! `pub mod gachawaifu;` perlu ditambah manual ke `src/fun/mod.rs`.
//!
//! //! BLOKIRAN:
//! - Akses database penuh (`getDatabase`, `db.setUser`, `db.updateEnergi`,
//!   `db.updateKoin`) tidak tersedia lewat `Ctx` (hanya `user_data`,
//!   `group_data`, `settings` + `Mutation::SetUser/SetGroup/SetSetting`).
//! - Lib `waifu.js`, `romance.js`, `level.js` (`rollWaifu`, `applyAction`,
//!   `applyNeglect`, `angerEffMood`, ...) tidak ter-port.
//! - `onigis` (`prepareWAMessageMedia`, `generateWAMessageFromContent`,
//!   `sock.relayMessage`, `sock.waUploadToServer`) dan axios Pinterest API
//!   tidak tersedia pada kontrak API.
//! Akibatnya sistem waifu lengkap tidak dapat di-port apa adanya.
