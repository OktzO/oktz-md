//! Plugin `plugins/ai/`.
//!
//! `ai.js` dan `characterai.js` sengaja tidak ada di sini: keduanya butuh
//! kemampuan yang tidak ada di registry plugin (lihat laporan batch E1).

pub mod aiassist;
#[path = "anime-gen.rs"]
pub mod anime_gen;
pub mod bananagen2;
pub mod claudehaiku;
pub mod deepseek;
pub mod dolphin;
pub mod feelbetter;
pub mod gita;
pub mod gpt5;
#[path = "jokowi-ai.rs"]
pub mod jokowi_ai;
#[path = "kobo-ai.rs"]
pub mod kobo_ai;
pub mod matematika;
pub mod muslimai;
#[path = "prabowo-ai.rs"]
pub mod prabowo_ai;
pub mod quillbot;
pub mod qwen3;
pub mod simi;
pub mod sologo;
pub mod text2img;
pub mod text2img2;
pub mod txt2img2;
#[path = "waguri-ai.rs"]
pub mod waguri_ai;
pub mod wormgpt;

// Ditambahkan scripts/pasang-modul.mjs.
