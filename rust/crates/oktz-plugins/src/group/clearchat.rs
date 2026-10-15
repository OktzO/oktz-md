//! `plugins/group/clearchat.js` — bersihkan chat grup.
//!
//! `ponytail:` seluruh badan plugin adalah operasi Baileys:
//! `sock.chatModify({ delete, lastMessages })` (`clearchat.js:26-32`) dan,
//! kalau itu gagal, `sock.chatModify({ clear, messages })` (`:38-46`). Keduanya
//! tidak ada di API plugin, jadi tidak ada pesan yang dihapus dan tidak ada
//! balasan sukses yang dikirim — plugin diam, bukan melapor chat sudah bersih.
//! `m.react('🗑️')`/`'☢'` ikut hilang karena menempel pada percakapan yang sama.

use oktz_plugin_api::{Ctx, Reply, plugin};

// `pluginConfig.name` di JS adalah array 5 nama (`clearchat.js:3`).
// `PluginMeta.name` hanya `String`, jadi elemen pertama jadi `name` dan sisanya
// jadi `alias` — registry `matches()` tetap menerima kelima nama.
#[plugin(
    name = "clearchat",
    alias = ["cc", "cleangc", "deletechat", "delchat"],
    category = "group",
    description = "Membersihkan chat grup",
    usage = ".clearchat",
    example = ".clearchat",
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
pub fn clearchat(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run() -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "clearchat".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        (find("clearchat").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("clearchat").unwrap();
        assert_eq!(p.meta.name, "clearchat");
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Membersihkan chat grup");
        assert_eq!(p.meta.usage, ".clearchat");
        assert_eq!(p.meta.example, ".clearchat");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_enabled);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_name_from_the_js_array_resolves_back_here() {
        // `name: ['clearchat','cc','cleangc','deletechat','delchat']`.
        for n in ["clearchat", "cc", "cleangc", "deletechat", "delchat"] {
            assert_eq!(
                find(n).map(|p| p.meta.name.as_str()),
                Some("clearchat"),
                "nama {n}"
            );
        }
        // JS menaruh kelima nama di `name` (array) dengan `alias` kosong.
        // Rust hanya punya satu string `name`, jadi empat sisanya jadi alias.
        assert_eq!(
            find("clearchat").unwrap().meta.alias,
            ["cc", "cleangc", "deletechat", "delchat"],
            "nama array JS dipetakan ke alias"
        );
    }

    #[test]
    fn it_stays_silent_instead_of_faking_a_cleanup() {
        let r = run();
        assert!(
            r.is_none(),
            "tidak ada `chatModify` jadi tidak ada yang diklaim"
        );
        assert!(r.mutations().is_empty());
        assert!(!r.has_media());
    }
}
