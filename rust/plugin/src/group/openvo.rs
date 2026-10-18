//! `plugins/group/openvo.js` — buka pesan 1x lihat yang di-reply.
//!
//! Yang dipindah: dua gerbang yang bisa dijawab tanpa jaringan — tidak ada
//! reply, dan tipe yang tidak didukung.
//!
//! `ponytail:` pemeriksaan `content.viewOnce` (`openvo.js:50`) tidak bisa
//! dilakukan karena `QuotedMessage` tidak membawa flag itu, dan pengunduhannya
//! `downloadContentFromMessage` (`:78`) tidak ada di API plugin. Jadi media
//! yang dikenali (image/video/audio) berakhir tanpa balasan apa pun —
//! bukan video yang dikirim.

use oktz_plugin_api::{Ctx, QuotedKind, Reply, plugin};

/// `openvo.js:63-69` — `type.includes('image'|'video'|'audio')` pada kunci
/// pesan, bukan pada nama enum. `wire_type()` mengembalikan nama itu.
pub(crate) fn media_type(kind: &QuotedKind) -> Option<&'static str> {
    let wire = kind.wire_type();
    if wire.contains("image") {
        Some("image")
    } else if wire.contains("video") {
        Some("video")
    } else if wire.contains("audio") {
        Some("audio")
    } else {
        None
    }
}

/// `openvo.js:22-26`.
fn no_quoted(prefix: &str) -> String {
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n\
         > Balas pesan 1x lihat dengan perintah ini!\n\
         > Gunakan: `{prefix}openvo` (reply pesan 1x lihat)"
    )
}

/// `openvo.js:72-74`.
const UNSUPPORTED: &str = "Tipenya gak didukung, hanya support image, video, audio";

#[plugin(
    name = "openvo",
    alias = [],
    category = "group",
    description = "Membuka pesan 1x lihat yang di-reply",
    usage = ".openvo (reply pesan 1x lihat)",
    example = ".openvo",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn openvo(ctx: Ctx) -> Reply {
    let Some(quoted) = ctx.quoted() else {
        return Reply::text(no_quoted(&ctx.prefix));
    };

    let Some(_media_type) = media_type(&quoted.kind) else {
        return Reply::text(UNSUPPORTED);
    };

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedMessage;
    use oktz_plugin_api::find;

    fn ctx_with(kind: Option<QuotedKind>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "openvo".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            quoted: kind.map(|kind| QuotedMessage {
                id: "M1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: String::new(),
                kind,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        }
    }

    fn run(kind: Option<QuotedKind>) -> Reply {
        (find("openvo").unwrap().handler)(ctx_with(kind))
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("openvo").unwrap();
        assert_eq!(e.meta.name, "openvo");
        assert!(e.meta.alias.is_empty(), "alias di JS kosong");
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Membuka pesan 1x lihat yang di-reply");
        assert_eq!(e.meta.usage, ".openvo (reply pesan 1x lihat)");
        assert_eq!(e.meta.example, ".openvo");
        assert_eq!(e.meta.cooldown, 5);
        assert_eq!(
            e.meta.energi, 1,
            "satu-satunya plugin di port ini yang berenergi 1"
        );
        assert!(e.meta.is_enabled);
        assert!(!e.meta.is_group && !e.meta.is_private);
        assert!(!e.meta.is_admin && !e.meta.is_bot_admin);
        assert!(!e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn without_a_reply_it_asks_for_one() {
        let r = run(None);
        assert_eq!(
            body(&r),
            "❌ *ɢᴀɢᴀʟ*\n\n\
             > Balas pesan 1x lihat dengan perintah ini!\n\
             > Gunakan: `.openvo` (reply pesan 1x lihat)"
        );
    }

    #[test]
    fn the_hint_follows_the_live_prefix() {
        let mut ctx = ctx_with(None);
        ctx.prefix = "!".into();
        let r = (find("openvo").unwrap().handler)(ctx);
        assert!(body(&r).contains("`!openvo`"), "{}", body(&r));
    }

    #[test]
    fn media_kinds_map_to_the_three_supported_types() {
        assert_eq!(media_type(&QuotedKind::Image), Some("image"));
        assert_eq!(media_type(&QuotedKind::Video), Some("video"));
        assert_eq!(media_type(&QuotedKind::Audio), Some("audio"));
    }

    #[test]
    fn anything_else_is_refused_by_name() {
        for kind in [
            QuotedKind::Text,
            QuotedKind::Sticker,
            QuotedKind::Document,
            QuotedKind::Contact,
            QuotedKind::Poll,
            QuotedKind::Other("viewOnceMessage".into()),
        ] {
            assert_eq!(media_type(&kind), None, "{kind:?}");
        }
        let r = run(Some(QuotedKind::Text));
        assert_eq!(body(&r), UNSUPPORTED);
    }

    #[test]
    fn a_supported_kind_goes_silent_because_the_download_is_missing() {
        for kind in [QuotedKind::Image, QuotedKind::Video, QuotedKind::Audio] {
            let r = run(Some(kind.clone()));
            assert!(
                r.is_none(),
                "{kind:?}: tidak ada `downloadContentFromMessage`"
            );
            assert!(r.mutations().is_empty());
            assert!(!r.has_media());
        }
    }
}
