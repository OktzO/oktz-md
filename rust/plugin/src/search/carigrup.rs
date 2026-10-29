//! `plugins/search/carigrup.js` — cari grup WhatsApp lewat `api.cuki.biz.id`.
//!
//! `ponytail:` kunci API ada di `config.APIkey?.cuki`, dan `Ctx` tidak membawa
//! config, jadi nilainya ikut `config.js` yang di-hardcode — persis
//! `search/android1.rs`. Effect-nya cabang "API key belum dikonfigurasi" jadi
//! jalur yang benar-benar terpakai sampai router menyediakan config.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.cuki || ''` (`config.js`) = `process.env.APIKEY_CUKI || ""`.
const CUKI_APIKEY: &str = "";

/// `carigrup.js:39` — endpoint pencarian grup.
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://api.cuki.biz.id/api/search/whatsapp-group?apikey={CUKI_APIKEY}&query={}",
        encode_uri_component(text)
    )
}

/// `carigrup.js:25-27` — kartu saat keyword kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔍 *ᴄᴀʀɪ ɢʀᴜᴘ ᴡᴀ*\n\n> Masukkan keyword pencarian\n\n`Contoh: {prefix}carigrup gb isian`"
    )
}

/// `carigrup.js:35` — guard kunci.
pub(crate) const NO_APIKEY: &str = "❌ API key cuki belum dikonfigurasi (APIKEY_CUKI)";

/// `carigrup.js:44` — tidak ada grup.
pub(crate) fn not_found(text: &str) -> String {
    format!("❌ Tidak ditemukan grup untuk keyword *{text}*")
}

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
///
/// Sama dengan `search/apkmod.rs` dan `search/android1.rs`; kandidat
/// `common.rs`.
pub(crate) fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `carigrup.js:59` — deskripsi dipotong 150 karakter UTF-16 lalu `...`.
///
/// `ponytail:` batas JS dihitung per kode unit UTF-16; di sini per `char`, jadi
/// emoji di dekat batas bisa berbeda satu karakter dari JS.
pub(crate) fn short_description(group: &Value) -> String {
    match group.get("description") {
        Some(Value::String(d)) if !d.is_empty() => {
            if d.chars().count() > 150 {
                format!("{}...", d.chars().take(150).collect::<String>())
            } else {
                d.clone()
            }
        }
        _ => "-".to_string(),
    }
}

/// `!data.status || !data.data?.groups?.length` (`carigrup.js:42`) —
/// `(data.data, groups)` kalau ada hasil, `None` untuk cabang "tidak
/// ditemukan".
///
/// Dipisah dari handler karena kunci API belum terbawa `Ctx`: tanpa fungsi ini
/// jalur HTTP-nya mustahil diuji.
pub(crate) fn found_groups(data: &Value) -> Option<(&Value, &[Value])> {
    if !js_truthy(data.get("status")) {
        return None;
    }
    let head = data.get("data")?;
    let groups = head
        .get("groups")
        .and_then(Value::as_array)
        .filter(|g| !g.is_empty())?;
    Some((head, groups))
}

/// `carigrup.js:48-61` — hasil pencarian.
pub(crate) fn format_result(head: &Value, groups: &[Value]) -> String {
    let mut result = String::from("🔍 *ᴄᴀʀɪ ɢʀᴜᴘ ᴡᴀ*\n\n");
    result.push_str(&format!(
        "📌 Keyword: *{}*\n",
        interp(head.get("query"))
    ));
    result.push_str(&format!(
        "📊 Total: *{}* grup ditemukan\n",
        interp(head.get("total"))
    ));

    for (i, g) in groups.iter().enumerate() {
        result.push_str("\n━━━━━━━━━━━━━━━\n");
        result.push_str(&format!("*{}. {}*\n", i + 1, interp(g.get("title"))));
        result.push_str(&format!("📅 {}\n", interp(g.get("date"))));
        result.push_str(&format!("🏷️ {}\n", interp(g.get("category"))));
        result.push_str(&format!("📝 {}\n", short_description(g)));
        result.push_str(&format!("🔗 {}", interp(g.get("group_link"))));
    }
    result
}

#[plugin(
    name = "carigrup",
    alias = ["searchgrup", "findgrup", "grupwa"],
    category = "search",
    description = "Cari grup WhatsApp berdasarkan keyword",
    usage = ".carigrup <keyword>",
    example = ".carigrup gb isian",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn carigrup(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");

    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if CUKI_APIKEY.is_empty() {
        return Reply::text(NO_APIKEY).with_reaction("❌");
    }

    let Some(resp) = ctx.http.get(&api_url(&text)).ok() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    let Some((head, groups)) = found_groups(&data) else {
        return Reply::text(not_found(&text)).with_reaction("❌");
    };

    Reply::text(format_result(head, groups)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = self.body.clone().ok_or_else(|| oktz_plugin_api::HttpError {
                message: "offline".into(),
            })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "carigrup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("carigrup").unwrap().handler)(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("carigrup").unwrap();
        assert_eq!(p.meta.name, "carigrup");
        assert_eq!(p.meta.alias, ["searchgrup", "findgrup", "grupwa"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari grup WhatsApp berdasarkan keyword");
        assert_eq!(p.meta.usage, ".carigrup <keyword>");
        assert_eq!(p.meta.example, ".carigrup gb isian");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["searchgrup", "findgrup", "grupwa"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "carigrup",
                "alias {a}"
            );
        }
    }

    #[test]
    fn a_blank_keyword_shows_the_usage_card_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   "] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn an_empty_key_stops_before_the_request() {
        // `config.APIkey.cuki` tidak terbawa `Ctx`, jadi cabang ini yang aktif
        // selama router belum menyediakan config.
        let http = Stub::new(r#"{"status": true, "data": {"groups": [{"title": "X"}]}}"#);
        let r = run(http.clone(), "gb isian");
        assert_eq!(body_of(&r), NO_APIKEY);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_url_keeps_the_cuki_shape_of_the_js_endpoint() {
        assert_eq!(
            api_url("gb isian"),
            "https://api.cuki.biz.id/api/search/whatsapp-group?apikey=&query=gb%20isian"
        );
        assert_eq!(
            api_url("a&b'c"),
            "https://api.cuki.biz.id/api/search/whatsapp-group?apikey=&query=a%26b'c"
        );
    }

    #[test]
    fn two_groups_render_the_header_and_the_numbered_blocks() {
        let head = serde_json::json!({
            "query": "gb isian",
            "total": 2,
            "groups": [
                {"title": "Grup A", "date": "2024-01-01", "category": "Edukasi", "description": "Deskripsi pendek", "group_link": "https://chat.whatsapp.com/a"},
                {"title": "Grup B", "date": "2024-02-02", "category": "Game", "description": "", "group_link": "https://chat.whatsapp.com/b"},
            ]
        });
        let groups = head["groups"].as_array().unwrap();
        let out = format_result(&head, groups);
        assert!(out.starts_with("🔍 *ᴄᴀʀɪ ɢʀᴜᴘ ᴡᴀ*\n\n📌 Keyword: *gb isian*\n📊 Total: *2* grup ditemukan\n"), "{out}");
        assert!(out.contains("\n━━━━━━━━━━━━━━━\n*1. Grup A*\n📅 2024-01-01\n🏷️ Edukasi\n📝 Deskripsi pendek\n🔗 https://chat.whatsapp.com/a"), "{out}");
        assert!(out.contains("*2. Grup B*\n📅 2024-02-02\n🏷️ Game\n📝 -\n🔗 https://chat.whatsapp.com/b"), "{out}");
    }

    #[test]
    fn a_description_of_exactly_one_hundred_fifty_characters_is_not_truncated() {
        let group = serde_json::json!({"description": "x".repeat(150)});
        assert_eq!(short_description(&group), "x".repeat(150));

        let group = serde_json::json!({"description": "x".repeat(151)});
        assert_eq!(short_description(&group), format!("{}...", "x".repeat(150)));

        let group = serde_json::json!({"description": "  "});
        assert_eq!(
            short_description(&group),
            "  ",
            "hanya string kosong yang jadi `-`, bukan spasi"
        );
    }

    #[test]
    fn missing_fields_are_interpolated_as_undefined_like_js() {
        let group = serde_json::json!({});
        assert_eq!(short_description(&group), "-");
        let out = format_result(&serde_json::json!({}), std::slice::from_ref(&group));
        assert!(out.contains("📌 Keyword: *undefined*"), "{out}");
        assert!(out.contains("📊 Total: *undefined* grup ditemukan"), "{out}");
        assert!(
            out.contains("*1. undefined*\n📅 undefined\n🏷️ undefined\n📝 -\n🔗 undefined"),
            "{out}"
        );
    }

    #[test]
    fn a_falsy_status_or_an_empty_group_list_is_not_found() {
        // Jalur ini tidak bisa diuji lewat handler: kunci API belum terbawa
        // `Ctx`, jadi handler selalu berhenti lebih dulu.
        let body = r#"{"status": true, "data": {"query": "q", "total": 0, "groups": []}}"#;
        let data: Value = serde_json::from_str(body).unwrap();
        assert!(found_groups(&data).is_none(), "{body}");

        for body in [
            r#"{"status": false, "data": {"groups": [{"title": "A"}]}}"#,
            r#"{"status": true, "data": {}}"#,
            r#"{"status": true}"#,
            r#"{"status": true, "data": null}"#,
        ] {
            let data: Value = serde_json::from_str(body).unwrap_or(Value::Null);
            assert!(found_groups(&data).is_none(), "{body}");
        }
    }

    #[test]
    fn a_filled_group_list_passes_the_guard() {
        let data = serde_json::json!({"status": true, "data": {"query": "q", "groups": [{"title": "A"}]}});
        let (head, groups) = found_groups(&data).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(head.get("query").and_then(Value::as_str), Some("q"));
    }

    #[test]
    fn the_renderer_never_panics_on_strange_values() {
        let groups = [serde_json::json!("bukan objek"), serde_json::json!(null), serde_json::json!({"title": 7})];
        let out = format_result(&serde_json::json!({"query": "🚀 \u{0}", "total": 3}), &groups);
        assert!(out.starts_with("🔍 *ᴄᴀʀɪ ɢʀᴜᴘ ᴡᴀ*\n\n📌 Keyword: *🚀 \u{0}*"), "{out}");
        assert!(out.contains("*3. 7*\n"), "{out}");
    }
}