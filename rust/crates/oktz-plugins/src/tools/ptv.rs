//! `plugins/tools/ptv.js` — kirim video sebagai PTV (circle video).
//!
//! `ponytail:` seluruh pengiriman tidak bisa di-port: `sock.sendMessage` dengan
//! `ptv: true` butuh buffer video dan tidak ada variant `Reply` untuk video
//! (`Reply::Image`/`Sticker` saja). Unduh media juga butuh `download()`.
//! Yang dipindah: pengenalan quoted video (kartu usage) dan reaksi `🕕` yang
//! di JS dikirim sebelum mencoba.

use oktz_plugin_api::{Ctx, Reply, plugin};

fn usage(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > Kirim *video* atau *balas video* lalu ketik:\n\
         > `{prefix}ptv`"
    )
}

fn unavailable() -> &'static str {
    "⚠️ *Kirim PTV tidak tersedia di port Rust*\n\n\
     > `Reply` tidak punya variant video dan `Ctx` tidak punya `download()`,\n\
     > jadi `sock.sendMessage(..., { ptv: true })` tidak bisa dijalankan."
}

#[plugin(
    name = "ptv",
    alias = ["pvideo", "circlevideo"],
    category = "tools",
    description = "Kirim video sebagai PTV (circle video)",
    usage = ".ptv (reply video)",
    example = ".ptv",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn ptv(ctx: Ctx) -> Reply {
    match ctx.quoted() {
        Some(q) if q.kind.is_video() => Reply::text(unavailable()).with_reaction("🕕"),
        // JS `m.isVideo` (video pada pesan pemicu) tidak punya padanan di `Ctx`,
        // jadi quoted non-video selalu jatuh ke `!video` seperti di sana.
        _ => Reply::text(usage(&ctx.prefix)),
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
            mimetype: Some("video/mp4".into()),
            file_name: None,
            from_me: false,
        }
    }

    fn run(quoted: Option<QuotedMessage>) -> R {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ptv".into(),
            sender: "628111@s.whatsapp.net".into(),
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("ptv").unwrap().handler)(ctx)
    }

    fn body(r: &R) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ptv").unwrap();
        assert_eq!(p.meta.name, "ptv");
        assert_eq!(p.meta.alias, ["pvideo", "circlevideo"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Kirim video sebagai PTV (circle video)");
        assert_eq!(p.meta.usage, ".ptv (reply video)");
        assert_eq!(p.meta.example, ".ptv");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_back_to_this_plugin() {
        for a in ["pvideo", "circlevideo"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("ptv")
            );
        }
    }

    #[test]
    fn without_a_video_quote_it_shows_the_usage_card() {
        assert_eq!(body(&run(None.clone())), usage("."));
        for kind in [
            QuotedKind::Text,
            QuotedKind::Image,
            QuotedKind::Audio,
            QuotedKind::Sticker,
        ] {
            assert_eq!(body(&run(Some(quoted(kind.clone())))), usage("."), "{kind:?}");
        }
    }

    #[test]
    fn a_video_quote_reacts_then_reports_the_missing_capability() {
        let r = run(Some(quoted(QuotedKind::Video)));
        match r.without_mutations() {
            R::Reaction { emoji, .. } => assert_eq!(emoji, "🕕", "reaksi JS sebelum mencoba"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(body(&r).contains("ptv: true"), "{}", body(&r));
        assert!(!r.has_media(), "tidak ada media yang benar-benar terkirim");
    }
}
