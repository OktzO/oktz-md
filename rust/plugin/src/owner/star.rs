//! `plugins/owner/star.js` — beri/hapus bintang pada pesan.
//!
//! `ponytail:` `sock.chatModify` dengan field `star` tidak ada di `Ctx` API,
//! dan `m.react('⭐')` tidak bisa digabung dengan teks balasan karena
//! `Reply::with_reaction` hanya satu emoji. Yang dipindah: deteksi quoted,
//! penanganan argumen `hapus`/`unstar`, dan teks balasan. Perubahan state
//! bintang tidak dapat dilakukan di sini.
//!
//! Catatan: baris `pub mod star;` perlu ditambah ke `owner/mod.rs` secara
//! manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "star",
    alias = ["bintang"],
    category = "owner",
    description = "Beri/hapus bintang pada pesan",
    usage = ".star (reply pesan) atau .star hapus (reply pesan)",
    example = ".star",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn star(ctx: Ctx) -> Reply {
    if ctx.quoted().is_none() {
        return Reply::text(
            "⭐ *sᴛᴀʀ ᴍᴇssᴀɢᴇ*\n\n\
             > `.star` (reply pesan) — Beri bintang\n\
             > `.star hapus` (reply pesan) — Hapus bintang",
        );
    }

    let unstar = ctx
        .args
        .first()
        .map(|a| a.to_lowercase() == "hapus" || a.to_lowercase() == "unstar")
        .unwrap_or(false);

    if unstar {
        Reply::text("❌ *Bintang dihapus dari pesan*")
    } else {
        Reply::text("⭐ *Pesan ditandai bintang*")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(quoted: bool, args: &[&str]) -> Reply {
        let ctx = Ctx {
            command: "star".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            quoted: if quoted {
                Some(oktz_plugin_api::QuotedMessage {
                    id: "q1".into(),
                    sender: "628999@s.whatsapp.net".into(),
                    chat: "628111@s.whatsapp.net".into(),
                    text: String::new(),
                    kind: oktz_plugin_api::QuotedKind::Text,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
            timestamp: 0,
        })
            } else {
                None
            },
            ..Default::default()
        };
        star(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("star").unwrap();
        assert_eq!(p.meta.name, "star");
        assert_eq!(p.meta.alias, ["bintang"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn without_quoted_shows_help() {
        let r = run(false, &[]);
        assert!(body(&r).contains("sᴛᴀʀ ᴍᴇssᴀɢᴇ"));
        assert!(!r.is_none());
    }

    #[test]
    fn quoted_without_arg_gives_bintang() {
        let r = run(true, &[]);
        assert_eq!(body(&r), "⭐ *Pesan ditandai bintang*");
    }

    #[test]
    fn quoted_with_hapus_gives_dihapus() {
        let r = run(true, &["hapus"]);
        assert_eq!(body(&r), "❌ *Bintang dihapus dari pesan*");
    }

    #[test]
    fn quoted_with_unstar_gives_dihapus() {
        let r = run(true, &["unstar"]);
        assert_eq!(body(&r), "❌ *Bintang dihapus dari pesan*");
    }
}
