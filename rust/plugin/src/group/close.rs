//! `plugins/group/close.js` — `sock.groupSettingUpdate(m.chat,
//! 'announcement')`.
//!
//! `ponytail:` TIDAK ada padanan di `Ctx` untuk dua hal yang jadi inti plugin
//! ini. `m.groupMetadata.announce` tidak pernah ditulis ke `groups[chat]` —
//! `handler.js` menyimpannya di `groupSettingsCache`, `Map` in-memory di luar
//! store — dan `sock.groupSettingUpdate` tidak ada di API plugin. Tanpa keduanya
//! port ini tidak bisa menjalankan closures grup; ia sengaja diam dan
//! dicatat di laporan, bukan melaporkan "✅ telah menutup grup" yang tidak
//! pernah terjadi.
//!
//! Naik ke `Reply::GroupSetting` + `announce` di `Ctx` begitu router punya
//! keduanya; sampai itu, file ini hanya menjaga nama/alias/meta tetap
//! terdaftar.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "close",
    alias = ["tutup", "closegroup", "tutupgroup"],
    category = "group",
    description = "Menutup grup agar hanya admin yang bisa chat",
    usage = ".close",
    example = ".close",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn close(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "close".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        (oktz_plugin_api::find("close").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("close").unwrap();
        assert_eq!(p.meta.name, "close");
        assert_eq!(p.meta.alias, ["tutup", "closegroup", "tutupgroup"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Menutup grup agar hanya admin yang bisa chat");
        assert_eq!(p.meta.usage, ".close");
        assert_eq!(p.meta.example, ".close");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["tutup", "closegroup", "tutupgroup"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("close")
            );
        }
    }

    #[test]
    fn it_says_nothing_instead_of_faking_a_group_setting_change() {
        let r = run();
        assert!(r.is_none(), "tidak boleh ada teks yang menyesatkan");
        assert!(r.mutations().is_empty(), "tidak ada store yang boleh ditulis");
        assert!(!r.has_media());
    }
}