//! `plugins/owner/schedule.js` — command untuk mengelola scheduled messages.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: mengimpor enam fungsi dari `../../src/lib/scheduler.js`
//! (`scheduleMessage`, `cancelScheduledMessage`, `getScheduledMessages`,
//! `getSchedulerStatus`, `formatTimeRemaining`, `getMsUntilTime`), yang bukan
//! bagian dari `Ctx` API. Tidak ada kapasitas jadwal/pengingat di handler
//! plugin, jadi tidak dapat meniru perilaku JS apa adanya.
//!
//! `ponytail:` kalau `scheduler` diekspose lewat `Ctx` nanti, port ulang seluruh
//! logika `add`/`edit`/`list`/`kategori`/`preset`/`detail`/`del`/`status` dari
//! `plugins/owner/schedule.js`.
//!
//! NOTE: baris `pub mod schedule;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs dilarang disentuh karena sudah milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "schedule",
    alias = ["sched", "jadwal", "timer"],
    category = "owner",
    description = "Bikin reminder atau jadwal bebas dengan text custom (DIBLOKIR: scheduler.js belum tersedia di Rust)",
    usage = ".schedule <add/edit/list/kategori/preset/detail/del/status> [options]",
    example = ".schedule preset sekolah 06:30",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn schedule(_ctx: Ctx) -> Reply {
    Reply::None
}
