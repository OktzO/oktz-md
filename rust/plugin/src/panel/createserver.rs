//! `plugins/panel/createserver.js` — buat server Pterodactyl via panel.
//!
//! Catatan: `pub mod createserver;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/panel/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `config.pterodactyl` (server1..5 dengan domain/apikey/nestid/egg/location):
//!   field ini tidak ada di `oktz_core::config::Runtime`, jadi tidak bisa
//!   dibaca lewat `ctx.config()`.
//! - `axios` POST/GET ke Pterodactyl API (`/api/application/users`, nests,
//!   servers): `ctx.http.get`/`post_json` tersedia, tapi tanpa config panel
//!   tidak ada endpoint yang bisa dipanggil.
//! - `onigis` (`prepareWAMessageMedia`, `generateWAMessageFromContent`,
//!   `proto.InteractiveMessage`, `sock.relayMessage`) untuk interactive
//!   message dengan tombol copy/CTA: `Reply::Interactive` hanya punya `body`,
//!   tanpa header media/native flow/tombol.
//! - `sock.onWhatsApp(targetNumber)` untuk validasi nomor tersedia via
//!   `ctx.sock().ada_whatsapp(...)`, tapi tanpa hasilnya (panel config) tidak
//!   bisa melanjutkan.
//! - `crypto.randomBytes(3)` untuk password: bisa direplikasi, tapi tanpa
//!   integrasi di atas tidak berfungsi.
