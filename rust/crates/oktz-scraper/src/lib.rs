//! Port `src/scraper/` (JS) ke Rust — dipakai plugin `download/*`, `stalker/*`,
//! `search/*`, dan `ai/*`.
//!
//! Aturan seluruh crate ini:
//!
//! - Satu file JS = satu modul Rust dengan nama snake_case
//!   (`geminiVision.js` → `gemini_vision.rs`, `logic-bell.js` → `logic_bell.rs`).
//! - Fungsi scraper menerima `&dyn HttpGet` sebagai argumen pertama dan
//!   mengembalikan `Result<serde_json::Value, HttpError>` yang bentuknya sama
//!   dengan objek yang di-`return` JS. Tidak ada klien jaringan sendiri, tidak
//!   ada state global.
//! - Acuan tunggal adalah file JS-nya: kalau JS menelan error dan
//!   mengembalikan `{ status: false, error }`, port-nya melakukan hal yang
//!   sama; kalau JS melempar, port-nya `Err`.
//!
//! Daftar modul di bawah ini dikelola orkestrator (lihat
//! `docs/rust-rewrite/SCRAPER-PORTING.md`), bukan agent porting.

pub mod common;
pub mod dailymotion;
