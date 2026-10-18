//! `plugins/canvas/profileig.js` — membuat canvas profile instagram.
//! `pub mod profileig;` perlu ditambah manual ke `src/canvas/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `uploadImage` dari `../../src/lib/uploader.js` (unggah buffer gambar
//!   lalu dapatkan URL) tidak tersedia di kontrak API, sehingga cabang "reply
//!   gambar untuk dijadikan foto profil" tidak dapat menyusun URL ke API
//!   canvas.
//! - `downloadMediaMessage` dari `onigis` juga tidak tersedia.
//! Akibatnya profile IG canvas tidak dapat di-port apa adanya.
