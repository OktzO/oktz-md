//! `plugins/tools/delpp.js` — menghapus foto profil bot.
//!
//! `ponytail:` handler JS tidak punya guard sama sekali: langsung
//! `sock.user?.id` lalu `sock.removeProfilePicture(botJid)`. Dua-duanya
//! tidak ada di `Ctx`, jadi satu-satunya cabang yang bisa dicapai adalah
//! pelaporan kemampuan yang hilang. Balasan sukses JS tidak dipakai karena
//! menyatakan foto bot terhapus, dan tidak ada yang terjadi.

use oktz_plugin_api::{Ctx, Reply, plugin};

fn unavailable() -> &'static str {
    "⚠️ *Hapus foto profil bot tidak tersedia di port Rust*\n\n\
     > Ctx tidak punya `sock`, jadi `sock.user.id` dan `removeProfilePicture`\n\
     > tidak bisa diakses."
}

#[plugin(
    name = "delpp",
    alias = ["delprofilebot", "delppbot", "hapusppbot"],
    category = "tools",
    description = "Menghapus foto profil bot",
    usage = ".delpp",
    example = ".delpp",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn delpp(_ctx: Ctx) -> Reply {
    Reply::text(unavailable())
}

#[cfg(test)]
mod tests {
    use oktz_plugin_api::Ctx;

    fn run() -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delpp".into(),
            sender: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find("delpp").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delpp").unwrap();
        assert_eq!(p.meta.name, "delpp");
        assert_eq!(
            p.meta.alias,
            ["delprofilebot", "delppbot", "hapusppbot"]
        );
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Menghapus foto profil bot");
        assert_eq!(p.meta.usage, ".delpp");
        assert_eq!(p.meta.example, ".delpp");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["delprofilebot", "delppbot", "hapusppbot"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("delpp")
            );
        }
    }

    #[test]
    fn the_only_reachable_branch_reports_the_missing_capability() {
        let t = run();
        assert!(t.contains("`removeProfilePicture`"), "{t}");
        assert!(
            !t.contains("berhasil dihapus"),
            "teks sukses JS menyatakan penghapusan yang tidak terjadi: {t}"
        );
    }
}
