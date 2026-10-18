//! Kategori `islamic` — port dari `plugins/islamic/murrotal.js` dan
//! `plugins/islamic/quran.js`.
//!
//! Catatan: baris `pub mod islamic;` perlu ditambahkan ke
//! `rust/crates/oktz-plugins/src/lib.rs` oleh orchestrator (tidak boleh
//! disentuh agent porting per aturan PORT-BRIEF #2).

pub mod murrotal;
pub mod quran;