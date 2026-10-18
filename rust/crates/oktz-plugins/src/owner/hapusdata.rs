//! `plugins/owner/hapusdata.js` — reset semua data database ke default.
//! `pub mod hapusdata;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `db.resetToDefaults()` (reset semua tabel database) tidak tersedia lewat
//!   `Ctx` (hanya tiga mutasi store: `SetUser`, `SetGroup`, `SetSetting`).
//! - `pendingReset` (state antarm pesan dengan `setTimeout` 60 detik) dan
//!   `sock.sendMessage` dengan `interactiveButtons` tidak tersedia.
//! Akibatnya reset data tidak dapat di-port apa adanya.
