//! `plugins/owner/colongpp.js` — ambil & pakai foto profil target sebagai PP bot.
//! `pub mod colongpp;` perlu ditambah manual ke `src/owner/mod.rs`.
//!
//! //! BLOKIRAN:
//! - `sock.updateProfilePicture(botJid, processed)` membutuhkan JID bot
//!   sendiri (`sock.user?.id`), yang tidak tersedia baik di `Ctx` maupun di
//!   trait `Sock` (tidak ada metode pengembalian JID bot).
//! - Resize canvas 640x640 (`@napi-rs/canvas drawImage`) tidak ada pada
//!   `gambar::sharp` (hanya resize/encode, tidak ada combinasi teks-gambar).
//! Akibatnya pengaturan PP bot tidak dapat di-port apa adanya.
