//! `plugins/owner/listsewa.js` — lihat daftar grup yang terdaftar sewa.
//! `pub mod listsewa;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `db.db.data.sewa` (tabel terpisah di luar `user_data`/`group_data`/
//!   `settings`) tidak dapat dibaca lewat `Ctx`.
//! - `timeHelper.fromTimestamp` dari `../../src/lib/time.js` tersedia,
//!   namun data sewa itu sendiri tidak terjangkau.
//! Akibatnya daftar sewa tidak dapat di-port apa adanya.
