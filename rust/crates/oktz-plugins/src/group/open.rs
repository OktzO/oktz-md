//! `plugins/group/open.js` — `sock.groupSettingUpdate(m.chat,
//! 'not_announcement')`.
//!
//! `ponytail:` kembar dari [`crate::group::close`], blocker-nya sama.
//! `m.groupMetadata.announce` hanya hidup di `groupSettingsCache`
//! (`handler.js:2260`), bukan di store plugin, dan `sock.groupSettingUpdate`
//! tidak ada di API plugin. Plugin diam, bukan melapor "✅ telah membuka
//! grup" yang tidak pernah terjadi.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "open",
    alias = ["buka", "opengroup", "bukagroup"],
    category = "group",
    description = "Membuka grup agar semua member bisa chat",
    usage = ".open",
    example = ".open",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn open(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "open".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        (oktz_plugin_api::find("open").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("open").unwrap();
        assert_eq!(p.meta.name, "open");
        assert_eq!(p.meta.alias, ["buka", "opengroup", "bukagroup"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Membuka grup agar semua member bisa chat");
        assert_eq!(p.meta.usage, ".open");
        assert_eq!(p.meta.example, ".open");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["buka", "opengroup", "bukagroup"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("open")
            );
        }
    }

    #[test]
    fn it_says_nothing_instead_of_faking_a_group_setting_change() {
        let r = run();
        assert!(r.is_none());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn close_and_open_do_not_collide_on_any_command() {
        for c in ["close", "tutup", "closegroup", "tutupgroup"] {
            assert_eq!(oktz_plugin_api::find(c).unwrap().meta.name, "close");
        }
        for c in ["open", "buka", "opengroup", "bukagroup"] {
            assert_eq!(oktz_plugin_api::find(c).unwrap().meta.name, "open");
        }
    }
}