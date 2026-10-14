//! `plugins/owner/stop.js` — balas "bot dimatikan", lalu `process.exit(1)`.
//!
//! `ponytail:` langkah kedua TIDAK di-port. `process.exit` butuh menghentikan
//! proses dari dalam handler, dan `Ctx` sengaja tidak punya akses ke itu —
//! plugin pure tidak boleh mematikan host. Yang dipindah adalah gate owner
//! dan teks balasan. Kalau shutdown dari command memang dibutuhkan, itu
//! keputusan router/pipeline, bukan plugin.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const NOT_OWNER: &str = "❌ *Owner Only!*";
pub(crate) const STOPPING: &str =
    "🛑 *Stopping Bot...*\n\n> Bot dimatikan. Harus dinyalakan manual dari terminal.";

#[plugin(
    name = "stop",
    alias = ["shutdown", "kill"],
    category = "owner",
    description = "Stop bot process",
    usage = ".stop",
    example = ".stop",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stop(ctx: Ctx) -> Reply {
    // JS memanggil `config.isOwner(m.sender)` secara eksplisit, terpisah dari
    // flag `m.isOwner`; keduanya sumber yang sama, jadi `ctx.is_owner` dipakai.
    if !ctx.is_owner {
        return Reply::text(NOT_OWNER);
    }
    Reply::text(STOPPING)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(is_owner: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "stop".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_owner,
            ..Default::default()
        };
        (find("stop").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("stop").unwrap();
        assert_eq!(p.meta.name, "stop");
        assert_eq!(p.meta.alias, ["shutdown", "kill"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Stop bot process");
        assert_eq!(p.meta.usage, ".stop");
        assert_eq!(p.meta.example, ".stop");
        assert_eq!(p.meta.cooldown, 0, "JS menaruh 0, bukan default 3");
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["shutdown", "kill"] {
            assert_eq!(find(a).unwrap().meta.name, "stop", "alias {a}");
        }
    }

    #[test]
    fn the_owner_gets_the_shutdown_notice() {
        assert_eq!(run(true.clone()).text_body(), Some(STOPPING));
    }

    #[test]
    fn a_non_owner_is_turned_away_before_any_shutdown_text() {
        assert_eq!(run(false.clone()).text_body(), Some(NOT_OWNER));
    }
}
