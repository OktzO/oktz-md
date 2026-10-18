//! `plugins/panel/delpanel.js` — hapus server (+ user) Pterodactyl.
//!
//! BLOKIR (gelombang-5 §Catatan tambahan): plugin butuh `config.pterodactyl`
//! (`config.js:258-299`) yang TIDAK ada di `Runtime`/`Ctx::config()` (
//! `oktz-core/src/config.rs` tidak punya field pterodactyl), dan handler
//! mengirim beberapa `m.reply` berurutan + `axios` GET/DELETE. Jangan port
//! paksa. File ini dibuat agar "semua file ada"; implementasi ditunda sampai
//! konfigurasi panel tersedia di `Ctx::config()`.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - argumen opsional `s1`/`s2`/`s3`, lalu `serverId`, lalu opsi `full`.
//!   - validasi `domain`+`apikey`; daftar server yang tersedia.
//!   - GET /api/application/servers/{id} → `attributes.user`, `attributes.name`.
//!   - GET /api/application/users/{id} → `attributes.root_admin`, `attributes.username`.
//!   - balas "menghapus panel...", lalu DELETE server; kalau `full` && !admin → DELETE user.
//!   - error apa pun → `te(prefix, command, pushName)`.
//!
//! `ponytail:` saat `config.pterodactyl` ditambahkan ke `Runtime`, tambahkan
//! `pub mod delpanel;` di `src/panel/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief).
