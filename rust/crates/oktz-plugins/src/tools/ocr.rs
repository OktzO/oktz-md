//! `plugins/tools/ocr.js` — extract teks dari gambar (Offline/Local).
//! `pub mod ocr;` perlu ditambah manual ke `src/tools/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `ocr` dari `../../src/lib/native-loader.js` (tesseract.js / ourin-native
//!   `.node`) tidak tersedia di kontrak API.
//! - `sendToolsPreview` dari `../../src/lib/context.js` tidak tersedia.
//! Akibatnya OCR gambar tidak dapat di-port apa adanya.
