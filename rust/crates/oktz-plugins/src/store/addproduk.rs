//! `plugins/store/addproduk.js` — tambah produk ke toko.
//!
//! Catatan: `pub mod addproduk;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/store/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `uploadImage()` (`src/lib/uploader.js`) untuk unggah thumbnail gambar/video
//!   ke CDN: tidak ada padanan di API plugin (`Ctx` tidak mengekspos uploader,
//!   dan `Reply::Image` hanya menyisipkan `MediaSource` yang sudah ada, bukan
//!   mengunggah byte). Tanpa ini `imageUrl`/`videoUrl` tidak bisa terisi, dan
//!   `products.push` + `db.setting('storeProducts', ...)` tetap bisa direplikasi
//!   lewat `Mutation::SetSetting`, tapi bagian unggah thumbnail dikosongkan.
//!   Jalan naik: tambah kemampuan unggah media ke API plugin lalu sesuaikan.
//! - `m.quoted.download()` / `m.download()` tersedia lewat
//!   `ctx.download_quoted_media()` / `ctx.download_media()`.
//! - `db.setting(...)` baca lewat `ctx.setting("storeProducts")`; tulis lewat
//!   `Mutation::SetSetting { key: "storeProducts", data }`.
