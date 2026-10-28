//! `plugins/tools/rvo.js` — baca pesan sekali lihat (view once).
//!
//! `ponytail:` inti plugin tidak bisa di-port: `quoted.download()` dan
//! `sock.sendMessage` tidak ada di `Ctx`/`QuotedMessage`. Yang dipindah: kedua
//! guard sebelum unduhan dan reaksi `⏱️`.
//!
//! `isViewOnce` tidak ada di `QuotedMessage`, jadi syarat `!quoted.isViewOnce &&
//! !quoted.isMedia` dipangkas jadi `!quoted.is_media()`. Ini melonggarkan syarat,
//! bukan mengetatkan: quoted yang bukan media apa pun tetap ditolak, sedangkan
//! media biasa ikut lolos ke cabang "tidak tersedia" alih-alih ditolak salah.

use oktz_plugin_api::{Ctx, Reply, plugin};

fn no_quote(prefix: &str) -> String {
    format!(
        "Reply pesan sekali lihat (view once) untuk membukanya.\n\n\
         `Contoh: {prefix}rvo` (reply pesan view once)"
    )
}

fn not_view_once() -> &'static str {
    "❌ Reply pesan view once (sekali lihat) untuk membukanya."
}

fn unavailable() -> &'static str {
    "⚠️ *Membuka view once tidak tersedia di port Rust*\n\n\
     > `QuotedMessage` tidak punya `download()` dan `Ctx` tidak punya `sock`,\n\
     > jadi media view once tidak bisa diunduh maupun dikirim ulang."
}

#[plugin(
    name = "rvo",
    alias = ["readvo", "readviewonce", "readview"],
    category = "tools",
    description = "Baca pesan sekali lihat (view once)",
    usage = ".rvo (reply pesan view once)",
    example = ".rvo",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn rvo(ctx: Ctx) -> Reply {
    let Some(quoted) = ctx.quoted() else {
        return Reply::text(no_quote(&ctx.prefix));
    };
    if !quoted.is_media() {
        return Reply::text(not_view_once());
    }
    Reply::text(unavailable()).with_reaction("⏱️")
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
            timestamp: 0,
        }
    }

    fn run(quoted: Option<QuotedMessage>) -> R {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "rvo".into(),
            sender: "628111@s.whatsapp.net".into(),
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("rvo").unwrap().handler)(ctx)
    }

    fn body(r: &R) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("rvo").unwrap();
        assert_eq!(p.meta.name, "rvo");
        assert_eq!(p.meta.alias, ["readvo", "readviewonce", "readview"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Baca pesan sekali lihat (view once)");
        assert_eq!(p.meta.usage, ".rvo (reply pesan view once)");
        assert_eq!(p.meta.example, ".rvo");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["readvo", "readviewonce", "readview"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("rvo")
            );
        }
    }

    #[test]
    fn without_a_quote_it_asks_for_one() {
        assert_eq!(body(&run(None.clone())), no_quote("."));
    }

    #[test]
    fn a_non_media_quote_is_refused() {
        for kind in [
            QuotedKind::Text,
            QuotedKind::Reaction,
            QuotedKind::Poll,
            QuotedKind::Contact,
            QuotedKind::Location,
        ] {
            assert_eq!(body(&run(Some(quoted(kind.clone())))), not_view_once(), "{kind:?}");
        }
    }

    #[test]
    fn any_media_quote_reacts_then_reports_the_missing_capability() {
        for kind in [
            QuotedKind::Image,
            QuotedKind::Video,
            QuotedKind::Audio,
            QuotedKind::Sticker,
            QuotedKind::Document,
        ] {
            let r = run(Some(quoted(kind.clone())));
            match r.without_mutations() {
                R::Reaction { emoji, .. } => assert_eq!(emoji, "⏱️", "{kind:?}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
            assert!(body(&r).contains("`download()`"), "{kind:?}: {}", body(&r));
            assert!(!r.has_media());
        }
    }
}
