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

pub mod aio;
pub mod claudehaiku;
pub mod common;
pub mod dafont;
pub mod dailymotion;
pub mod deepseek;
pub mod douyin;
pub mod dramabox;
pub mod fbdown;
pub mod feeb;
pub mod gemini;
pub mod gemini_vision;
pub mod gpt5;
pub mod gsmarena;
pub mod hd;
pub mod gpt52;
pub mod hdvid;
pub mod hdvid2;
pub mod hokinfo;
pub mod ig;
pub mod img2img;
pub mod img2prompt;
pub mod imgdrop;
pub mod imglarger;
pub mod konachan;
pub mod kusonime;
pub mod likee;
pub mod logic_bell;
pub mod lufemboy;
pub mod mconverter;
pub mod mediafire;
pub mod pindl;
pub mod qwen3;
pub mod reddit;
pub mod rednote;
pub mod reelsvideo;
pub mod removebackground;
pub mod seaart;
pub mod sfiledl;
pub mod soundcloud;
pub mod soundclouddl;
pub mod spotify;
pub mod tempmail;
pub mod terabox;
pub mod tiktok;
pub mod tiktoksearch;
pub mod topmedia;
pub mod twitter;
pub mod txt2img;
pub mod txt2img2;
pub mod unlimitedai;
pub mod upscaler;
pub mod videy;
pub mod wallpapersearch;
pub mod wink;
pub mod wwchar;
pub mod youtube;
pub mod yt;
pub mod ytdl;
