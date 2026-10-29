//! `plugins/owner/swgc.js` — post Group Status/Story ke grup pilihan.
//! `pub mod swgc;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `sock.sendMessage(jid, { groupStatusMessage: ... })` (border hijau) dan
//!   pembuatan pesan sintetis `groupStatusMessageV2` tidak ada pada `Reply`.
//! - `sock.groupFetchAllParticipating()` tersedia (`semua_grup`), namun
//!   `handleAntiSwGc`, `saluranCtx`, `file-type`, dan berkas temp `temp/`
//!   tidak semua tersedia.
//! Akibatnya group story tidak dapat di-port apa adanya.
