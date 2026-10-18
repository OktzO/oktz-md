//! `plugins/jpm/jpm.js` — sistem JPM lengkap (broadcast, hidetag, channel,
//! auto, blacklist, delay, update).
//!
//! Catatan: `pub mod jpm;` sudah tercantum di `jpm/mod.rs` (kategori baru,
//! milik plugin ini).
//!
//! //! BLOKIRAN:
//! - `getDatabase()` + `db.setting(...)` / `db.getUser(...)` untuk `jedaJpm`,
//!   `jpmBlacklist`, `autoJpmBlacklist`, sesi JPM: `Mutation::SetSetting` /
//!   `SetUser` tersedia, tapi state kompleks (global `jpmSessions`,
//!   `statusjpm`, `stopjpm`, scheduler AutoJPM) tidak diekspos di handler
//!   tunggal.
//! - `sock.groupFetchAllParticipating()` / `fetchGroupsSafe` / `onWhatsApp`:
//!   `ctx.sock().ada_whatsapp()` dan `semua_grup()` tersedia, tapi logika
//!   broadcast per grup dengan jeda + hide tag memang bukan `Reply` tunggal.
//! - `onigis` + `generateWAMessageFromContent` + `proto.InteractiveMessage`
//!   + `sock.relayMessage` untuk menu interaktif dengan tombol:
//!   `Reply::Interactive` hanya punya `body`, tidak ada native flow/tombol.
//! - `getBinaryNodeChild` + `sock.query` (newsletter subscribed): tidak ada
//!   di `Sock` trait.
//! - `child_process`/`setTimeout`/`setInterval` untuk jadwal AutoJPM: tidak
//!   ada di API plugin.
//! - `config.pterodactyl`-mirip dan `config.bot.version` sebagian tersedia,
//!   tapi tanpa integrasi di atas tidak berfungsi.
