//! `plugins/owner/chatclear.js` — auto-hapus semua chat grup & private tiap
//! tengah malam.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: mengimpor `isAutoClearEnabled` dan `setAutoClearEnabled` dari
//! `../../src/lib/chat-cleaner.js`, yang bukan bagian dari `Ctx` API
//! (`oktz_plugin_api::Ctx`). Tidak ada kapasitas state auto-clear di handler
//! plugin, jadi tidak dapat meniru perilaku JS apa adanya.
//!
//! `ponytail:` kalau `chat-cleaner` diekspose lewat `Ctx` nanti, port ulang
//! dari `plugins/owner/chatclear.js` (guard `on`/`off`/`status`, reaksi
//! `✅`, dan teks respons yang sama).
//!
//! NOTE: baris `pub mod chatclear;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs dilarang disentuh karena sudah milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "autoclear",
    alias = [],
    category = "owner",
    description = "Auto-hapus semua chat grup & private tiap tengah malam (DIBLOKIR: chat-cleaner.js belum tersedia di Rust)",
    usage = ".autoclear on|off|status",
    example = ".autoclear on",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn chatclear(_ctx: Ctx) -> Reply {
    Reply::None
}
