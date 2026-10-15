//! `plugins/group/resetlinkgc.js` — cabut link invite grup.
//!
//! `ponytail:` satu-satunya isi plugin adalah `sock.groupRevokeInvite(m.chat)`
//! (`resetlinkgc.js:24`). Link lama tidak dicabut dan tidak ada balasan sukses
//! yang dikirim — plugin diam. `m.react('🔄')`/`'✅'` ikut hilang karena
//! menempel pada percakapan yang sama.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "resetlinkgc",
    alias = ["resetlink", "revokelink", "newlink"],
    category = "group",
    description = "Reset link invite grup",
    usage = ".resetlinkgc",
    example = ".resetlinkgc",
    cooldown = 60,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn resetlinkgc(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run() -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resetlinkgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        (find("resetlinkgc").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("resetlinkgc").unwrap();
        assert_eq!(e.meta.name, "resetlinkgc");
        assert_eq!(e.meta.alias, ["resetlink", "revokelink", "newlink"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Reset link invite grup");
        assert_eq!(e.meta.usage, ".resetlinkgc");
        assert_eq!(e.meta.example, ".resetlinkgc");
        assert_eq!(e.meta.cooldown, 60);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["resetlink", "revokelink", "newlink"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("resetlinkgc"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn it_says_nothing_instead_of_claiming_the_link_was_reset() {
        let r = run();
        assert!(r.is_none());
        assert!(r.mutations().is_empty());
        assert!(
            r.text_body().is_none(),
            "balasan 'link grup lama tidak berlaku' adalah dusta"
        );
    }
}
