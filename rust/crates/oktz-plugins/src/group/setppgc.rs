//! `plugins/group/setppgc.js` — `sock.updateProfilePicture(m.chat, buffer)`.
//!
//! `ponytail:` dua cabang bisa dipindah, dua tidak. Yang dipindah: guard
//! "tidak ada gambar" (`m.quoted?.isImage` → `QuotedKind::is_image`) dan teks
//! errors. Yang tidak: `m.quoted.download()`/`m.download()` (butuh
//! `downloadContentMessage`) dan `sock.updateProfilePicture`. `Ctx` juga tidak
//! membawa flag "pesan yang dipicu adalah gambar", jadi cabang `m.isImage`
//! tidak punya sumber.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const NO_IMAGE: &str = "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
     > Reply gambar + `{}setppgc`\n\
     > Kirim gambar + caption `{}setppgc`";

pub(crate) fn no_image(prefix: &str) -> String {
    NO_IMAGE.replace("{}", prefix)
}

#[plugin(
    name = "setppgc",
    alias = ["setprofilegc", "setppgroup", "setppgrup"],
    category = "group",
    description = "Mengubah foto profil grup",
    usage = ".setppgc (reply gambar)",
    example = ".setppgc",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn setppgc(ctx: Ctx) -> Reply {
    match ctx.quoted() {
        // Unduh + unggah tidak ada di `Ctx` (lihat `ponytail:` modul).
        Some(_) => Reply::None,
        None => Reply::text(no_image(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn quoted(kind: QuotedKind) -> QuotedMessage {
        QuotedMessage {
            id: "MSG1".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: String::new(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn run(q: Option<QuotedMessage>) -> Reply {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "setppgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            quoted: q,
            ..Default::default()
        };
        (oktz_plugin_api::find("setppgc").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setppgc").unwrap();
        assert_eq!(p.meta.name, "setppgc");
        assert_eq!(p.meta.alias, ["setprofilegc", "setppgroup", "setppgrup"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Mengubah foto profil grup");
        assert_eq!(p.meta.usage, ".setppgc (reply gambar)");
        assert_eq!(p.meta.example, ".setppgc");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["setprofilegc", "setppgroup", "setppgrup"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("setppgc")
            );
        }
    }

    #[test]
    fn without_a_reply_it_prints_the_prefixed_usage() {
        let r = run(None.clone());
        assert_eq!(
            body(&r),
            "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
             > Reply gambar + `!setppgc`\n\
             > Kirim gambar + caption `!setppgc`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_reply_of_any_kind_stops_at_the_unavailable_upload() {
        for kind in [
            QuotedKind::Image,
            QuotedKind::Sticker,
            QuotedKind::Text,
            QuotedKind::Document,
        ] {
            let r = run(Some(quoted(kind.clone())));
            assert!(r.is_none(), "jenis {} tidak boleh replied", kind.wire_type());
            assert!(r.mutations().is_empty(), "jenis {}", kind.wire_type());
        }
    }
}