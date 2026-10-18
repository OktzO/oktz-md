//! `plugins/main/carifitur.js` — mencari fitur berdasarkan keyword.
//! `pub mod carifitur;` perlu ditambah manual ke `src/main/mod.rs`.
//!
//! //! BLOKIRAN:
//! - Membaca berkas plugin secara dinamis (`fs.readdirSync` + `import()`)
//!   dari folder `plugins/`, yang tidak tersedia di Rust (tidak ada dynamic
//!   import); registry Rust sudah menguasai semua plugin yang terdaftar.
//! - `getCaseCommands` dari `../../case/foto.js` tidak ter-port.
//! - `sock.sendButton` dengan gambar (`getAssetBuffer("foto")`) dan
//!   tombol interaktif tidak tersedia pada `Reply`.
//! Akibatnya pencarian fitur lengkap tidak dapat di-port apa adanya.
