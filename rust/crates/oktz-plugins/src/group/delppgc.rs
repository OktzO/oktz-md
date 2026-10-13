//! `plugins/group/delppgc.js` — `sock.removeProfilePicture(m.chat)`.
//!
//! `ponytail:` seluruh badan plugin adalah satu panggilan `sock` yang tidak
//! ada di API plugin; tidak ada state tersimpan dan tidak ada guard yang bisa
//! dipindah. Menulis "✅ PP Grup sekarang sudah botak" tanpa menghapus
//! apa pun adalah dusta, jadi plugin diam sampai API avatar grup tersedia di
//! router.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "delppgc",
    alias = ["delprofilegc", "delppgroup", "hapusppgc"],
    category = "group",
    description = "Menghapus foto profil grup",
    usage = ".delppgc",
    example = ".delppgc",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn delppgc(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delppgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        (oktz_plugin_api::find("delppgc").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delppgc").unwrap();
        assert_eq!(p.meta.name, "delppgc");
        assert_eq!(p.meta.alias, ["delprofilegc", "delppgroup", "hapusppgc"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Menghapus foto profil grup");
        assert_eq!(p.meta.usage, ".delppgc");
        assert_eq!(p.meta.example, ".delppgc");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["delprofilegc", "delppgroup", "hapusppgc"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("delppgc")
            );
        }
    }

    #[test]
    fn it_says_nothing_instead_of_faking_an_avatar_removal() {
        let r = run();
        assert!(r.is_none());
        assert!(r.mutations().is_empty());
        assert!(!r.has_media());
    }
}