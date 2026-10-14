//! `plugins/tools/toimg.js` — mengubah sticker menjadi gambar.
//!
//! `ponytail:` unduh + kirim tidak bisa di-port: `download()` tidak ada di
//! `QuotedMessage`, dan `sock.sendMedia` tidak ada di `Ctx`. Yang dipindah:
//! deteksi sticker pada quoted (kartu "tidak ada sticker") dan reaksi `🕕`.
//!
//! `m.isSticker` (sticker pada pesan pemicu) dan `stickerMessage.isAnimated`
//! tidak punya padanan di `Ctx`, jadi hanya quoted yang diperiksa dan cabang
//! "sticker animasi" tidak bisa dibedakan dari sticker biasa.

use oktz_plugin_api::{Ctx, Reply, plugin};

fn no_sticker(prefix: &str) -> String {
    format!(
        "❌ *ɢᴀɢᴀʟ*\n\n\
         > Tidak ada sticker yang terdeteksi!\n\n\
         *Cara penggunaan:*\n\
         > 1. Kirim sticker + caption `{prefix}toimg`\n\
         > 2. Reply sticker dengan `{prefix}toimg`"
    )
}

fn unavailable() -> &'static str {
    "⚠️ *Sticker ke gambar tidak tersedia di port Rust*\n\n\
     > `QuotedMessage` tidak punya `download()` dan `Ctx` tidak punya `sock`,\n\
     > jadi `sock.sendMedia(..., { type: 'image' })` tidak bisa dijalankan."
}

#[plugin(
    name = "toimg",
    alias = ["toimage", "stickertoimage", "stimg"],
    category = "tools",
    description = "Mengubah sticker menjadi gambar",
    usage = ".toimg (reply/caption sticker)",
    example = ".toimg",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn toimg(ctx: Ctx) -> Reply {
    match ctx.quoted() {
        Some(q) if q.kind.is_sticker() => Reply::text(unavailable()).with_reaction("🕕"),
        _ => Reply::text(no_sticker(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, QuotedKind, QuotedMessage, Reply as R};

    fn quoted(kind: QuotedKind) -> QuotedMessage {
        QuotedMessage {
            id: "q1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn run(quoted: Option<QuotedMessage>) -> R {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "toimg".into(),
            sender: "628111@s.whatsapp.net".into(),
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("toimg").unwrap().handler)(ctx)
    }

    fn body(r: &R) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("toimg").unwrap();
        assert_eq!(p.meta.name, "toimg");
        assert_eq!(p.meta.alias, ["toimage", "stickertoimage", "stimg"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Mengubah sticker menjadi gambar");
        assert_eq!(p.meta.usage, ".toimg (reply/caption sticker)");
        assert_eq!(p.meta.example, ".toimg");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["toimage", "stickertoimage", "stimg"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("toimg")
            );
        }
    }

    #[test]
    fn without_a_sticker_quote_it_explains_both_ways() {
        assert_eq!(body(&run(None.clone())), no_sticker("."));
        for kind in [
            QuotedKind::Text,
            QuotedKind::Image,
            QuotedKind::Video,
            QuotedKind::Document,
        ] {
            assert_eq!(body(&run(Some(quoted(kind.clone())))), no_sticker("."), "{kind:?}");
        }
        assert!(body(&run(None.clone())).contains("> 1. Kirim sticker + caption `.toimg`"));
    }

    #[test]
    fn a_sticker_quote_reacts_then_reports_the_missing_capability() {
        let r = run(Some(quoted(QuotedKind::Sticker)));
        match r.without_mutations() {
            R::Reaction { emoji, .. } => assert_eq!(emoji, "🕕"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(body(&r).contains("sendMedia"), "{}", body(&r));
        assert!(!r.has_media(), "tidak ada gambar yang benar-benar terkirim");
    }
}
