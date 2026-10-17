//! `plugins/search/apkmod.js` — cari APK MOD Premium lewat `api.neoxr.eu`.
//!
//! `ponytail:` JS mengirim daftar lewat `sock.sendButton` dengan picker
//! `single_select` plus thumbnail `getAssetBuffer("foto")`. `Reply::Interactive`
//! hanya membawa teksnya, jadi tombol (judul, deskripsi, `id` untuk
//! `apkmod-get`) dan thumbnail tidak ikut — persis teks `apkmod.js:52-58`.
//!
//! `ponytail:` label tombol `${app.name.substring(0, 24)}` bisa melempar
//! TypeError kalau `name` bukan string. Karena tombol tidak bisa dikirim,
//! pemotongan 24 karakter itu tidak ikut, dan `name` yang hilang dirender
//! `undefined` lewat [`interp`] — bukan error.
//!
//! `ponytail:` `ctx.sessions` tidak punya padanan map session tingkat-atas yang
//! dipakai tombol, jadi tidak ada state yang ditulis di sini.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{error_template, js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `apkmod.js:39` — endpoint pencarian.
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/apkmod?q={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(text)
    )
}

/// `apkmod.js:27-32` — kartu saat `m.text` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📱 *ᴀᴘᴋ ᴍᴏᴅ sᴇᴀʀᴄʜ*\n\n\
         > Cari APK MOD Premium\n\n\
         > Contoh:\n\
         \\`{prefix}apkmod vpn\\`"
    )
}

/// `apkmod.js:47` — tidak ada hasil.
pub(crate) fn not_found(text: &str) -> String {
    format!("❌ Tidak ditemukan hasil untuk: `{text}`")
}

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
///
/// Sama dengan `search/android1.rs`; dipindah ke `common.rs` kalau orkestrator
/// setuju.
pub(crate) fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `apkmod.js:52-58` — caption daftar aplikasi. Dipanggil dengan `apps` yang
/// SUDAH di-`slice(0, 15)`, jadi tidak ada pemotongan kedua di sini.
pub(crate) fn format_list(apps: &[Value], text: &str) -> String {
    let mut caption = format!("📱 *Hasil pencarian dari {text}*\n\n");
    for (i, app) in apps.iter().enumerate() {
        caption.push_str(&format!("*{}.* {}\n", i + 1, interp(app.get("name"))));
        caption.push_str(&format!("   ├ 🏷️ {}\n", interp(app.get("version"))));
        caption.push_str(&format!("   └ 🔓 {}\n\n", interp(app.get("mod"))));
    }
    caption
}

/// `data.data` yang lolos `!data?.data?.length` — array/string tak kosong,
/// bentuk lain dianggap gagal (`.length` numeric di JS, bukan array).
pub(crate) fn has_items(data: Option<&Value>) -> bool {
    match data {
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::String(s)) => !s.is_empty(),
        _ => false,
    }
}

#[plugin(
    name = "apkmod",
    alias = ["modapk2", "apkpremium"],
    category = "search",
    description = "Cari dan download APK MOD Premium",
    usage = ".apkmod <query>",
    example = ".apkmod vpn",
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
pub fn apkmod(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(resp) = ctx.http.get(&api_url(text)).ok() else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    if !js_truthy(data.get("status")) || !has_items(data.get("data")) {
        return Reply::text(not_found(text)).with_reaction("❌");
    }
    // String non-kosong lolos guard lalu `apps.forEach` melempar TypeError
    // di JS -> `catch` -> template error.
    let Some(apps) = data.get("data").and_then(Value::as_array) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let apps = &apps[..apps.len().min(15)];

    Reply::Interactive {
        body: format_list(apps, text),
    }
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
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
            let body = self.body.clone().ok_or_else(|| HttpError {
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
            command: "apkmod".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("apkmod").unwrap().handler)(ctx)
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

    const DUA_APPS: &str = r#"{
        "status": true,
        "data": [
            {"name": "Shadow Fight 2", "version": "1.121.0", "mod": "Premium"},
            {"name": "Subway Surfers", "version": "3.0.0", "mod": "Unlimited"}
        ]
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("apkmod").unwrap();
        assert_eq!(p.meta.name, "apkmod");
        assert_eq!(p.meta.alias, ["modapk2", "apkpremium"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari dan download APK MOD Premium");
        assert_eq!(p.meta.usage, ".apkmod <query>");
        assert_eq!(p.meta.example, ".apkmod vpn");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["modapk2", "apkpremium"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "apkmod", "alias {a}");
        }
    }

    #[test]
    fn a_blank_query_shows_the_usage_card_without_a_request() {
        let http = Stub::empty();
        for text in ["", "  \t "] {
            assert_eq!(body_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn two_apps_render_the_numbered_caption() {
        let http = Stub::new(DUA_APPS);
        let r = run(http.clone(), "shadow fight");
        let body = body_of(&r);
        assert!(
            body.starts_with("📱 *Hasil pencarian dari shadow fight*\n\n"),
            "{body}"
        );
        assert!(
            body.contains("*1.* Shadow Fight 2\n   ├ 🏷️ 1.121.0\n   └ 🔓 Premium\n\n"),
            "{body}"
        );
        assert!(
            body.contains("*2.* Subway Surfers\n   ├ 🏷️ 3.0.0\n   └ 🔓 Unlimited\n\n"),
            "{body}"
        );
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(matches!(r.without_mutations(), Reply::Interactive { .. }));

        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/apkmod?q=shadow%20fight&apikey="
        );
    }

    #[test]
    fn only_fifteen_apps_are_listed() {
        let apps: Vec<Value> = (0..20)
            .map(|i| json!({"name": format!("App {i}"), "version": "1", "mod": "M"}))
            .collect();
        let body = format!("{{\"status\": true, \"data\": {}}}", json!(apps));
        let out = body_of(&run(Stub::new(&body), "app"));
        assert!(out.contains("*15.* App 14\n"), "{out}");
        assert!(!out.contains("App 15"), "hasil ke-16 terpotong: {out}");
    }

    #[test]
    fn a_missing_field_is_interpolated_as_undefined_like_js() {
        let body = r#"{"status": true, "data": [{"name": "Cuma Nama"}]}"#;
        let out = body_of(&run(Stub::new(body), "x"));
        assert!(
            out.contains("*1.* Cuma Nama\n   ├ 🏷️ undefined\n   └ 🔓 undefined\n\n"),
            "{out}"
        );
    }

    #[test]
    fn status_false_or_an_empty_list_reports_not_found() {
        for body in [
            r#"{"status": false, "data": [{"name": "A"}]}"#,
            r#"{"status": true, "data": []}"#,
            r#"{"status": true}"#,
            r#"{"status": true, "data": ""}"#,
            "bukan json",
        ] {
            let r = run(Stub::new(body), "vpn & co");
            assert_eq!(body_of(&r), not_found("vpn & co"), "{body}");
            assert_eq!(reaction_of(&r), Some("❌"), "{body}");
        }
    }

    #[test]
    fn a_non_array_payload_hits_the_catch_branch_like_the_foreach_type_error() {
        let r = run(Stub::new(r#"{"status": true, "data": "x"}"#), "q");
        assert_eq!(body_of(&r), error_template(".", "apkmod"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "q");
        assert!(body_of(&r).contains("command `.apkmod`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_query_is_encoded_like_encode_uri_component() {
        let http = Stub::new(DUA_APPS);
        let _ = run(http.clone(), "vpn gratis 100% & co's");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/apkmod?q=vpn%20gratis%20100%25%20%26%20co's&apikey="
        );
    }

    #[test]
    fn a_very_long_query_is_sent_whole() {
        let http = Stub::new(DUA_APPS);
        let _ = run(http.clone(), &"a".repeat(4_000));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "{}", seen[0].len());
    }

    #[test]
    fn the_caption_helper_never_panics_on_strange_values() {
        let apps = [json!("bukan objek"), json!(null), json!({"name": 7})];
        let out = format_list(&apps, "🚀 \u{0} ñ");
        assert!(out.starts_with("📱 *Hasil pencarian dari 🚀 \u{0} ñ*\n\n"), "{out}");
        assert!(out.contains("*3.* bukan objek\n   ├ 🏷️ undefined\n"), "{out}");
        assert!(out.contains("*2.* null\n"), "{out}");
    }
}