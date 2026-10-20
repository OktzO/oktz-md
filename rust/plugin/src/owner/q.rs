//! `plugins/owner/q.js` — balas JSON pesan yang sedang di-reply.
//!
//! Catatan meta: `q.js` menulis `category: 'tools'` padahal berkasnya ada di
//! `plugins/owner/`. Nilai `pluginConfig` tetap disalin apa adanya (JS acuan),
//! jadi kategorinya `"tools"` — bukan ikut direktori berkas.

use oktz_plugin_api::{Ctx, QuotedMessage, Reply, plugin};
use serde_json::{Map, Value};

pub(crate) const NO_QUOTE: &str = "❌ *Reply pesan yang ingin di-inspect*";

/// `JSON.stringify(m.quoted, null, 2)`.
///
/// `QuotedMessage` adalah subset terstruktur dari `m.quoted` Baileys, jadi
/// stringify harus dibangun ulang dari field yang benar-benar ada. Kunci
/// memakai nama protobuf yang dipakai plugin lain di repo (`participant`,
/// `remoteJid`, `fileName`) supaya bentuknya masih recognizable.
pub(crate) fn quoted_json(q: &QuotedMessage) -> String {
    let mut map = Map::new();
    map.insert("id".into(), serde_json::json!(q.id));
    map.insert("participant".into(), serde_json::json!(q.sender));
    map.insert("remoteJid".into(), serde_json::json!(q.chat));
    map.insert("type".into(), serde_json::json!(q.kind.wire_type()));
    map.insert("text".into(), serde_json::json!(q.text));
    map.insert("mimetype".into(), serde_json::json!(q.mimetype));
    map.insert("fileName".into(), serde_json::json!(q.file_name));
    map.insert("fromMe".into(), serde_json::json!(q.from_me));
    // `JSON.stringify` dengan indent 2 — pretty printer yang sama.
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_default()
}

#[plugin(
    name = "q",
    alias = ["quoted", "inspect"],
    category = "tools",
    description = "Ambil JSON message dari pesan yang direply",
    usage = ".q (reply pesan)",
    example = "",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn q(ctx: Ctx) -> Reply {
    match ctx.quoted() {
        Some(quoted) => Reply::text(quoted_json(quoted)),
        None => Reply::text(NO_QUOTE),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, find};

    fn ctx_with(quoted: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "q".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            quoted,
            ..Default::default()
        }
    }

    fn text_of(quoted: Option<QuotedMessage>) -> String {
        (find("q").unwrap().handler)(ctx_with(quoted))
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn image_quote() -> QuotedMessage {
        QuotedMessage {
            id: "ABC123".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: "foto".into(),
            kind: QuotedKind::Image,
            mimetype: Some("image/jpeg".into()),
            file_name: Some("pic.jpg".into()),
            from_me: false,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("q").unwrap();
        assert_eq!(p.meta.name, "q");
        assert_eq!(p.meta.alias, ["quoted", "inspect"]);
        assert_eq!(p.meta.category, "tools", "nilai JS menang atas nama folder");
        assert_eq!(p.meta.description, "Ambil JSON message dari pesan yang direply");
        assert_eq!(p.meta.usage, ".q (reply pesan)");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        // `inspect` juga nama `plugins/utility/inspect.js`; nama didahulukan
        // (`plugins.js:539`), jadi alias `inspect` bukan milik q.
        assert_eq!(find("quoted").unwrap().meta.name, "q");
        assert_eq!(find("inspect").unwrap().meta.name, "inspect");
    }

    #[test]
    fn without_a_quote_it_asks_for_one() {
        assert_eq!(text_of(None), NO_QUOTE);
    }

    #[test]
    fn a_quoted_image_is_rendered_as_pretty_json() {
        let out = text_of(Some(image_quote()));
        let v: Value = serde_json::from_str(&out).expect("harus JSON valid");
        assert_eq!(v["type"], "imageMessage", "wire_type dari QuotedKind");
        assert_eq!(v["participant"], "628999@s.whatsapp.net");
        assert_eq!(v["mimetype"], "image/jpeg");
        assert_eq!(v["fileName"], "pic.jpg");
        assert_eq!(v["fromMe"], false);
        assert!(out.contains("\n  "), "indent 2 ala JSON.stringify: {out}");
    }

    #[test]
    fn text_quotes_and_absent_optional_fields_survive_the_round_trip() {
        let q = QuotedMessage {
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            ..image_quote()
        };
        let v: Value = serde_json::from_str(&text_of(Some(q))).unwrap();
        assert_eq!(v["type"], "conversation");
        assert_eq!(v["mimetype"], Value::Null, "None -> null, bukan hilang");
        assert_eq!(v["fileName"], Value::Null);
    }
}
