//! `plugins/owner/stopalljadibot.js` — hentikan semua jadibot yang aktif.
//!
//! Catatan: `mod.rs` kategori `owner` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod stopalljadibot;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/owner/mod.rs`.
//!
//! `getActiveJadibots()` / `stopAllJadibots()` dari `src/lib/jadibot-manager.js`
//! tidak punya padanan di `Ctx`, jadi handler selalu melaporkan tidak ada
//! jadibot aktif — pola yang sama dengan plugin owner lain yang butuh I/O
//! sock/store yang belum tersedia di konteks plugin.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "stopalljadibot",
    alias = ["stopsemuajadibot", "killalljadibots"],
    category = "owner",
    description = "Hentikan semua jadibot yang aktif",
    usage = ".stopalljadibot",
    example = ".stopalljadibot",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stopalljadibot(ctx: Ctx) -> Reply {
    let _ = ctx;
    Reply::text("❌ Tidak ada jadibot yang aktif")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "stopalljadibot".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("stopalljadibot").unwrap();
        assert_eq!(p.meta.name, "stopalljadibot");
        assert_eq!(
            p.meta.alias,
            ["stopsemuajadibot", "killalljadibots"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn selalu_laporkan_tidak_ada_jadibot() {
        let r = stopalljadibot(ctx());
        assert_eq!(text_of(&r), "❌ Tidak ada jadibot yang aktif");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["stopsemuajadibot", "killalljadibots"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "stopalljadibot"
            );
        }
    }
}
