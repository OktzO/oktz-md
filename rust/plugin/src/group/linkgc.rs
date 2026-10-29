//! `plugins/group/linkgc.js` — kirim link invite grup.
//!
//! `ponytail:` satu-satunya isi plugin adalah `sock.groupInviteCode(m.chat)`
//! (`linkgc.js:24`) lalu `https://chat.whatsapp.com/${code}`. Kode invite hanya
//! ada di Baileys dan tidak ada di API plugin, jadi plugin diam — bukan
//! menempelkan link yang tidak pernah diambil. `m.react('🕕')`/`'✅'` ikut
//! hilang karena menempel pada percakapan yang sama.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "linkgc",
    alias = ["linkgrup", "getlink", "gclink"],
    category = "group",
    description = "Dapatkan link invite grup",
    usage = ".linkgc",
    example = ".linkgc",
    cooldown = 10,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn linkgc(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run() -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "linkgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        (find("linkgc").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("linkgc").unwrap();
        assert_eq!(e.meta.name, "linkgc");
        assert_eq!(e.meta.alias, ["linkgrup", "getlink", "gclink"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Dapatkan link invite grup");
        assert_eq!(e.meta.usage, ".linkgc");
        assert_eq!(e.meta.example, ".linkgc");
        assert_eq!(e.meta.cooldown, 10);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["linkgrup", "getlink", "gclink"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("linkgc"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn it_never_invents_an_invite_link() {
        let r = run();
        assert!(r.is_none());
        assert!(r.mutations().is_empty());
        assert!(
            r.text_body().is_none(),
            "tidak ada kode invite untuk disalin"
        );
    }
}
