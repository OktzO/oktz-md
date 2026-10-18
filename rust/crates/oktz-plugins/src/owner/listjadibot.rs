//! `plugins/owner/listjadibot.js` — lihat semua session jadibot yang tersimpan.
//!
//! `ponytail:` `getAllJadibotSessions()` / `getActiveJadibots()` berasal dari
//! `src/lib/jadibot-manager.js`, dan `sock.sendMessage` dengan
//! `interactiveButtons` tidak ada di `Ctx` API. Tidak dapat di-port saat
//! ini. Yang dipindah: teks penjelasan ketersediaan.
//!
//! Catatan: baris `pub mod listjadibot;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "listjadibot",
    alias = ["jadibotlist", "alljadibot"],
    category = "owner",
    description = "Lihat semua session jadibot yang tersimpan",
    usage = ".listjadibot",
    example = ".listjadibot",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn listjadibot(ctx: Ctx) -> Reply {
    let _ = ctx;
    Reply::text(
        "⚠️ *Daftar Jadibot tidak tersedia di port Rust*\n\n\
         > `getAllJadibotSessions()` dan `getActiveJadibots()` dari\n\
         > `src/lib/jadibot-manager.js` belum tersedia di `Ctx` API.\n\
         > Tombol interaktif juga tidak dapat dikirim.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        let ctx = Ctx {
            command: "listjadibot".into(),
            ..Default::default()
        };
        listjadibot(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("listjadibot").unwrap();
        assert_eq!(p.meta.name, "listjadibot");
        assert_eq!(p.meta.alias, ["jadibotlist", "alljadibot"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn tampilkan_pesan_keterbatasan() {
        let r = run();
        assert!(body(&r).contains("Daftar Jadibot tidak tersedia"));
        assert!(r.mutations().is_empty());
    }
}
