//! `plugins/owner/backupsc.js` — backup script bot dalam bentuk zip.
//!
//! Catatan: `pub mod backupsc;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/owner/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `archiver` (zip) untuk membuat berkas `.zip`: tidak ada crate kompresi
//!   di API plugin, dan `oktz_plugin_api` tidak mengekspos pembuatan zip.
//! - `fs.readdirSync`/`statSync` rekursif seluruh `process.cwd()`: `ctx.berkas()`
//!   hanya melayani jalur di dalam akar aset, bukan entire cwd, dan tidak ada
//!   daftar rekursif pohon penuh.
//! - `sock.sendMessage({ document, fileName, ... })` + `contextInfo` newsletter
//!   forwarding: `Reply::Document` tersedia, tapi `contextInfo` dengan
//!   `forwardedNewsletterMessageInfo` tidak ada di `Reply`.
//! - `config.bot.name`, `config.saluran` tersedia; `fs.createWriteStream`/`unlink`
//!   untuk berkas besar di luar akar aset tidak tersedia.
