//! `plugins/ai/tofigurev2.js` — ubah gambar ke style Figure v2.
//!
//! Catatan: `pub mod tofigurev2;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/ai/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `uploadImage(buffer, 'image.jpg')` (`src/lib/uploader.js`) untuk unggah
//!   gambar ke CDN: tidak ada padanan uploader di API plugin.
//! - `applyImageEffect('tofigurav3', imageUrl)` (`src/lib/effect-api.js`):
//!   library internal bot, tidak ada di API plugin, dan tidak ada padanan
//!   pemrosesan efek gambar selain `oktz_plugin_api::gambar` (resize/grayscale
//!   encode saja, tidak ada style transfer).
//! - `sock.sendMedia(m.chat, res, null, m, { type: 'image' })`:
//!   `Reply::Image` tersedia, tapi input byte dari effect API tidak ada.
