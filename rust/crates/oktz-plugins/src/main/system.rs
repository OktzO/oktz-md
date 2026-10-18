//! `plugins/main/system.js` — informasi sistem (RAM, CPU, Disk, Latency).
//!
//! Catatan: `pub mod system;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/main/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `os.totalmem()` / `os.freemem()` / `os.cpus()` / `os.platform()` /
//!   `os.arch()` / `os.uptime()`: tidak ada di `Ctx` (hanya
//!   `ctx.uptime_secs`). Tidak ada API informasi sistem/host di plugin API.
//! - `child_process.exec` (`wmic`, `df -h /`): tidak ada padanan eksekusi
//!   perintah sistem selain `oktz_plugin_api::ffmpeg` (khusus ffmpeg).
//! - `m.messageTimestamp` untuk latency: tidak ada di `Ctx`.
//! - `config.command?.prefix` tersedia lewat `ctx.prefix`;
//!   `config.bot?.name` via `ctx.config().bot.name`.
