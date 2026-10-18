//! `plugins/vps/delvps.js` — hapus VPS DigitalOcean.
//! `pub mod delvps;` perlu ditambah manual ke `src/vps/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `config.digitalocean.token` / `.sellers` / `.ownerPanels` tidak ada pada
//!   `oktz_core::config::Runtime` (hanya `api_keys`, `owner`, `bot`, `assets`,
//!   `sticker`, `saluran`, ...), jadi token dan daftar penjual tidak dapat
//!   dibaca.
//! Akibatnya penghapusan VPS tidak dapat di-port apa adanya.
