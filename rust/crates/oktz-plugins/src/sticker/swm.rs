//! `plugins/sticker/swm.js` — ganti packname/author sticker.
//!
//! Semua guard (reply, sticker, packname) dan pemecahan `packname|author`
//! di-port apa adanya.Langkah media tidak bisa: `QuotedMessage` di
//! `oktz-plugin-api` bacaan saja (`quoted.download()` butuh I/O Baileys),
//! `addExifToWebp` butuh encoder WebP + EXIF, dan `sendImageAsSticker` /
//! `sendVideoAsSticker` tidak punya padanan di `Reply`. Karena itu jalur
//! setelah `m.react('🕕')` berakhir di cabang `catch` JS.
//!
//! `ponytail:` tambah `Ctx::download_quoted()` + `Reply::Sticker` di
//! `oktz-plugin-api` untuk menghidupkan lagi langkah `swm.js:57-79`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::search::applemusic::error_reply;

/// `swm.js:22-30` — kartu cara pakai saat tidak ada pesan yang di-reply.
pub(crate) fn usage_card(prefix: &str) -> String {
    format!(
        "🖼️ *sᴛɪᴄᴋᴇʀ ᴡᴀᴛᴇʀᴍᴀʀᴋ*\n\n\
         > Reply sticker dengan caption:\n\
         > `{prefix}swm packname`\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > `{prefix}swm Foto-AI`\n\
         > `{prefix}swm Foto-AI|LuckyArchz` _(packname + author)_"
    )
}

/// `swm.js:39-47` — `input.includes('|') ? [parts[0], parts[1]] : [input, '']`,
/// tiap bagian `.trim()` dan yang kosong jadi `""`.
pub(crate) fn split_input(input: &str) -> (String, String) {
    let mut parts = input.split('|');
    let packname = parts.next().unwrap_or("").trim().to_string();
    match parts.next() {
        // `parts[1]` saja — pipa ketiga diabaikan, bukan masuk author.
        Some(second) => {
            let author = second.trim().to_string();
            (packname, author)
        }
        None => (packname, String::new()),
    }
}

#[plugin(
    name = "swm",
    alias = ["wm", "stickerwm", "stickermark", "colong"],
    category = "sticker",
    description = "Mengganti packname dan author pada sticker",
    usage = ".swm <packname> atau .swm <packname>|<author>",
    example = ".swm BotName",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn swm(ctx: Ctx) -> Reply {
    let Some(quoted) = ctx.quoted() else {
        return Reply::text(usage_card(&ctx.prefix));
    };

    // `swm.js:33` — `quoted.type === 'stickerMessage' || quoted.isSticker`.
    if !quoted.kind.is_sticker() {
        // `.replace('Message', '')` lalu `|| 'media lain'` — jadi hanya string
        // kosong (atau `Other("")`) yang jatuh ke fallback.
        let label = quoted.kind.wire_type().replace("Message", "");
        let label = if label.is_empty() {
            "media lain".to_string()
        } else {
            label
        };
        return Reply::text(format!("❌ *ɢᴀɢᴀʟ*\n\n> Reply pesan sticker, bukan {label}"));
    }

    let input = ctx.text.trim();
    if input.is_empty() {
        let prefix = &ctx.prefix;
        return Reply::text(format!(
            "❌ *ɢᴀɢᴀʟ*\n\n\
             > Masukkan packname\n\n\
             *ᴄᴏɴᴛᴏʜ:*\n\
             > `{prefix}swm Foto-AI`\n\
             > `{prefix}swm Foto-AI|LuckyArchz` _(+ author)_"
        ));
    }

    let (_packname, _author) = split_input(input);

    // `swm.js:53-79` butuh buffer sticker; lihat catatan `ponytail:` modul.
    Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn quoted(kind: QuotedKind) -> Option<QuotedMessage> {
        Some(QuotedMessage {
            id: "MSG1".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: String::new(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
        })
    }

    fn run(text: &str, q: Option<QuotedMessage>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "swm".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: text.into(),
            args: vec![text.into()],
            quoted: q,
            ..Default::default()
        };
        (oktz_plugin_api::find("swm").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("swm").unwrap();
        assert_eq!(p.meta.name, "swm");
        assert_eq!(p.meta.alias, ["wm", "stickerwm", "stickermark", "colong"]);
        assert_eq!(p.meta.category, "sticker");
        assert_eq!(
            p.meta.description,
            "Mengganti packname dan author pada sticker"
        );
        assert_eq!(p.meta.usage, ".swm <packname> atau .swm <packname>|<author>");
        assert_eq!(p.meta.example, ".swm BotName");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["wm", "stickerwm", "stickermark", "colong"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "swm");
        }
    }

    #[test]
    fn without_a_quote_it_prints_the_usage_card() {
        let r = run("Foto-AI", None);
        assert_eq!(text_of(&r), usage_card("."));
        assert!(text_of(&r).contains("`.swm Foto-AI|LuckyArchz`"));
        assert!(matches!(r, Reply::Text { .. }), "tanpa mutasi apa pun");
    }

    #[test]
    fn the_usage_card_follows_the_active_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "swm".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: "Foto-AI".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("swm").unwrap().handler)(ctx);
        let body = text_of(&r);
        assert!(body.contains("`!swm packname`"), "{body}");
        assert!(!body.contains("`.swm"), "prefix lama tidak boleh bocor: {body}");
    }

    #[test]
    fn a_quoted_non_sticker_is_refused_by_its_wire_type() {
        for (kind, label) in [
            (QuotedKind::Image, "image"),
            (QuotedKind::Video, "video"),
            (QuotedKind::Audio, "audio"),
            (QuotedKind::Document, "document"),
            (QuotedKind::Text, "conversation"),
            (QuotedKind::Other(String::new()), "media lain"),
        ] {
            let r = run("Foto-AI", quoted(kind));
            assert_eq!(
                text_of(&r),
                format!("❌ *ɢᴀɢᴀʟ*\n\n> Reply pesan sticker, bukan {label}"),
                "{label}"
            );
        }
    }

    #[test]
    fn a_quote_without_a_packname_asks_for_one() {
        let r = run("   ", quoted(QuotedKind::Sticker));
        let body = text_of(&r);
        assert!(body.starts_with("❌ *ɢᴀɢᴀʟ*\n\n> Masukkan packname"), "{body}");
        assert!(body.contains("> `.swm Foto-AI`"), "{body}");
        assert!(body.contains("> `.swm Foto-AI|LuckyArchz` _(+ author)_"), "{body}");
        assert!(matches!(r, Reply::Text { .. }), "tanpa mutasi apa pun");
    }

    #[test]
    fn packname_and_author_are_split_on_the_first_pipe() {
        assert_eq!(
            split_input("Foto-AI|LuckyArchz"),
            ("Foto-AI".to_string(), "LuckyArchz".to_string())
        );
        assert_eq!(
            split_input("  Foto-AI  |  Lucky  Archz  "),
            ("Foto-AI".to_string(), "Lucky  Archz".to_string())
        );
        assert_eq!(
            split_input("Foto-AI|Lucky|Army"),
            ("Foto-AI".to_string(), "Lucky".to_string())
        );
        assert_eq!(
            split_input("Foto-AI|"),
            ("Foto-AI".to_string(), String::new())
        );
        assert_eq!(
            split_input("Foto-AI|   "),
            ("Foto-AI".to_string(), String::new())
        );
        assert_eq!(split_input("Foto-AI"), ("Foto-AI".to_string(), String::new()));
    }

    #[test]
    fn every_reachable_packname_falls_into_the_error_template() {
        // Guard lolos, tapi langkah media belum bisa di-port (lihat
        // `ponytail:` modul) — cabang `catch` JS yang dipakai.
        for text in ["Foto-AI", "Foto-AI|LuckyArchz", "  Foto-AI  |  Lucky  "] {
            let r = run(text, quoted(QuotedKind::Sticker));
            assert_eq!(text_of(&r), error_reply(".", "swm"), "input {text:?}");
            assert!(
                matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "☢"),
                "harus bereaksi ☢"
            );
        }
    }
}