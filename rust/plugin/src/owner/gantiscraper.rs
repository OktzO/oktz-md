//! `plugins/owner/gantiscraper.js` — ganti code scraper di `src/scraper`.
//!
//! Catatan: `pub mod gantiscraper;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/owner/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `fs.readdirSync`/`readFileSync`/`writeFileSync` pada `process.cwd()`
//!   (`src/scraper`, `plugins`, `backup/scraper`): `ctx.berkas()` dibatasi ke
//!   akar aset dan tidak mengekspos entire cwd.
//! - `import(...)` dinamis dari file `.js` (`pathToFileURL` + hot reload
//!   modul scraper): Rust tidak mengekspos evaluasi/kode JS pada runtime.
//! - `hotReloadPlugin` dari `src/lib/plugins.js`: spesifik implementasi Node
//!   bot, tidak ada di API plugin.
//! - Deteksi nama scraper dari regex `export default function` pada isi file:
//!   bisa direplikasi teksnya, tapi tanpa tulis berkas + reload tidak berarti.
