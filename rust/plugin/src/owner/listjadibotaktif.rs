//! `plugins/owner/listjadibotaktif.js` — daftar jadibot yang aktif.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `getActiveJadibots()` dari `src/lib/jadibot-manager.js`,
//! `Date.now()` untuk uptime, dan `sock.sendMessage(..., { text, mentions,
//! interactiveButtons })` dengan tombol `🛑 Stop Semua`. Tidak ada satupun di
//! `Ctx`.
//!
//! `ponytail:` kalau entry point kedua jadibot dan `sock` tersedia di `Ctx`
//! nanti, port ulang dari `plugins/owner/listjadibotaktif.js` (render
//! `@id`, uptime, owner, daftar mention, dan tombol interaktif).
//!
//! `format_uptime` disalin dari JS untuk dijaga dan diuji.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `listjadibotaktif.js:18-25` — format uptime milidetik ke teks.
pub(crate) fn format_uptime(ms: i64) -> String {
    let seconds = (ms / 1000).max(0);
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes % 60)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds % 60)
    } else {
        format!("{}s", seconds)
    }
}

#[plugin(
    name = "listjadibotaktif",
    alias = ["jadibotaktif", "activejadibots"],
    category = "owner",
    description = "Lihat jadibot yang sedang aktif dengan detail (DIBLOKIR: butuh jadibot-manager + sock)",
    usage = ".listjadibotaktif",
    example = ".listjadibotaktif",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn listjadibotaktif(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listjadibotaktif").unwrap();
        assert_eq!(p.meta.name, "listjadibotaktif");
        assert_eq!(p.meta.alias, ["jadibotaktif", "activejadibots"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn format_uptime_mirip_js() {
        assert_eq!(format_uptime(0), "0s");
        assert_eq!(format_uptime(45_000), "45s");
        assert_eq!(format_uptime(61_000), "1m 1s");
        assert_eq!(format_uptime(60_000), "1m 0s");
        assert_eq!(format_uptime(3_661_000), "1h 1m");
        assert_eq!(format_uptime(3_600_000), "1h 0m");
    }
}
