//! `plugins/canvas/fakedev7.js` — membuat fake developer (Python Theme).
//! `pub mod fakedev7;` perlu ditambah manual ke `src/canvas/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `gambar::sharp` tidak mendukung menggambar teks, lingkaran, gradien, atau
//!   compositing beberapa gambar (avatar + badge crown/leaf + blue tick),
//!   yang seluruhnya digunakan `fakedev7.js` (`@napi-rs/canvas`).
//! - Muat gambar dari URL eksternal (`blueTick`, `crown`, `leaf`) tidak ada
//!   di `gambar` (hanya decode byte yang sudah ada).
//! Akibatnya kanvas fake developer tidak dapat di-port apa adanya.
