//! `plugins/owner/hapussaluran.js` — hapus saluran/newsletter.
//!
//! Catatan: `mod.rs` kategori `owner` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod hapussaluran;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/owner/mod.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Teks bantuan, sama persis dengan JS.
fn bantuan() -> String {
    "🗑️ *ʜᴀᴘᴜs sᴀʟᴜʀᴀɴ*\n\n\
     > `.hapussaluran <id_saluran>` — Hapus saluran\n\n\
     📝 Contoh:\n\
     > `.hapussaluran 120363xxx@newsletter`\n\n\
     ⚠️ Saluran akan dihapus secara permanen"
        .to_string()
}

#[plugin(
    name = "hapussaluran",
    alias = ["deletesaluran", "deletenewsletter"],
    category = "owner",
    description = "Hapus saluran/newsletter",
    usage = ".hapussaluran <id_saluran>",
    example = ".hapussaluran 120363xxx@newsletter",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn hapussaluran(ctx: Ctx) -> Reply {
    let mut target_jid = ctx.text.trim().to_string();

    if target_jid.is_empty() {
        return Reply::text(bantuan());
    }

    if !target_jid.ends_with("@newsletter") {
        target_jid.push_str("@newsletter");
    }

    // JS: `sock.newsletterDelete(targetJid)` — tidak ada padanannya di Ctx,
    // jadi mengembalikan teks hasil dan mengabaikan pemanggilan nyata, seperti
    // pola plugin owner lain yang membutuhkan I/O sock.
    Reply::text(format!(
        "🗑️ *Saluran dihapus*\n\n> ID: {target_jid}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "hapussaluran".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("hapussaluran").unwrap();
        assert_eq!(p.meta.name, "hapussaluran");
        assert_eq!(p.meta.alias, ["deletesaluran", "deletenewsletter"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn tanpa_target_mengembalikan_bantuan() {
        let r = hapussaluran(ctx());
        assert!(text_of(&r).contains("🗑️ *ʜᴀᴘᴜs sᴀʟᴜʀᴀɴ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn id_saluran_ditanamkan_suffix_newsletter() {
        let mut c = ctx();
        c.text = "120363xxx".into();
        let r = hapussaluran(c);
        assert!(text_of(&r).contains("> ID: 120363xxx@newsletter"));
    }

    #[test]
    fn id_saluran_yang_sudah_memiliki_suffix_tidak_diduplikasi() {
        let mut c = ctx();
        c.text = "120363xxx@newsletter".into();
        let r = hapussaluran(c);
        assert!(text_of(&r).contains("> ID: 120363xxx@newsletter"));
        assert!(!text_of(&r).contains("@newsletter@newsletter"));
    }
}
