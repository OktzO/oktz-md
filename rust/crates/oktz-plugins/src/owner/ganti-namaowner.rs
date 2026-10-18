//! `plugins/owner/ganti-namaowner.js` — ganti nama owner (utama atau tambahan).
//! `pub mod ganti_namaowner;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - Menulis `config.js` (jalur absolut `process.cwd()/config.js`) dengan
//!   regex diganti oleh `berkas` yang menolak jalur absolut dan `..`.
//! - `getDatabase` / `db.setting` untuk `ownerNames` hanya sebagian tersedia
//!   (`ctx.settings` + `Mutation::SetSetting`), namun cabang "ganti nama owner
//!   utama" yang menulis berkas `config.js` intinya tidak dapat di-port.
//! Akibatnya penggantian nama owner tidak dapat di-port apa adanya.
