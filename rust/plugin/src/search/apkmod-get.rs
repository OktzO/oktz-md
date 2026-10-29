//! `plugins/search/apkmod-get.js` — unduh satu APK MOD dari hasil pencarian.
//!
//! `ponytail:` JS mengirim dokumen lewat `sock.sendMessage` dengan
//! `contextInfo: { forwardingScore: 99, isForwarded: true }`; `Reply::Document`
//! tidak punya context info, jadi penanda forwarded tidak ikut.
//!
//! `ponytail:` cabang "URL tidak tersedia" mengirim
//! `forwardedNewsletterMessageInfo` (`config.saluran?.id`,
//! `config.saluran?.name || config.bot?.name`) — `Ctx` tidak membawa config,
//! dan `Reply::Interactive` tidak punya context info, jadi yang terkirim hanya
//! teksnya.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` — lihat catatan `ponytail:` di
/// `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `apkmod-get.js:35` — endpoint pencarian dengan nomor hasil.
pub(crate) fn api_url(query: &str, no: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/apkmod?q={}&no={no}&apikey={NEOXR_APIKEY}",
        encode_uri_component(query)
    )
}

/// `apkmod-get.js:28` — guard format.
pub(crate) fn bad_format(prefix: &str) -> String {
    format!("❌ Format: `{prefix}apkmod-get <no> <query>`")
}

/// `apkmod-get.js:68` — caption saat `file.url` tidak ada.
pub(crate) const NO_DOWNLOAD_URL: &str = "> ⚠️ Download URL tidak tersedia";

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
///
/// Sama dengan `search/apkmod.rs` dan `search/android1.rs`; ketiganya
/// kandidat untuk `common.rs`.
pub(crate) fn interp(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `apkmod-get.js:56` — `file.filename || `${app.name}.apk``. `app.name` yang
/// hilang jadi teks `undefined`, sama seperti template literal di JS.
pub(crate) fn file_name(file: &Value, app: &Value) -> String {
    match file.get("filename") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(v) if crate::common::js_truthy(Some(v)) => interp(Some(v)),
        _ => format!("{}.apk", interp(app.get("name"))),
    }
}

/// `file.url?.trim()` yang truthy — nilai non-string tidak bisa jadi sumber
/// dokumen, jadi jatuh ke cabang "tidak tersedia".
pub(crate) fn file_url(file: Option<&Value>) -> Option<String> {
    let raw = file?.get("url")?.as_str()?.trim();
    if raw.is_empty() { None } else { Some(raw.to_string()) }
}

#[plugin(
    name = "apkmod-get",
    alias = ["apkmodget", "getapkmod"],
    category = "search",
    description = "Download APK MOD dari hasil pencarian",
    usage = ".apkmod-get <no> <query>",
    example = ".apkmod-get 1 vpn",
    cooldown = 15,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn apkmod_get(ctx: Ctx) -> Reply {
    let no = ctx.args.first().cloned().unwrap_or_default();
    let query = ctx.args.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");

    // `!no` = argumen pertama bukan angka (`parseInt` -> NaN) atau 0.
    let no_ok = crate::common::parse_int(&no).is_some_and(|n| n != 0);
    if !no_ok || query.is_empty() {
        return Reply::text(bad_format(&ctx.prefix));
    }

    let Some(resp) = ctx.http.get(&api_url(&query, &no)).ok() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    // `!data?.status || !data?.data` -> `throw new Error(...)` -> `catch`.
    if !js_truthy(data.get("status")) || !js_truthy(data.get("data")) {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    }
    let app = data.get("data").unwrap_or(&Value::Null);

    let file = data.get("file").unwrap_or(&Value::Null);
    match file_url(Some(file)) {
        Some(url) => Reply::document(
            MediaSource::url(url).with_mimetype("application/vnd.android.package-archive"),
            file_name(file, app),
        )
        .with_reaction("✅"),
        None => Reply::interactive(NO_DOWNLOAD_URL.to_string())
        .with_reaction("⚠️"),
    }
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
            command: "apkmod-get".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("apkmod-get").unwrap().handler)(ctx)
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

    fn doc_of(r: &Reply) -> (Option<String>, String) {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Document {
                    media,
                    file_name,
                    ..
                } => {
                    let url = media
                        .as_ref()
                        .and_then(|m| match m {
                            MediaSource::Url { url, .. } => Some(url.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();
                    (Some(url), file_name.clone())
                }
                other => panic!("harus Document, dapat {other:?}"),
            },
            other => panic!("harus dibungkus reaksi, dapat {other:?}"),
        }
    }

    const ADA_FILE: &str = r#"{
        "status": true,
        "data": {"name": "Shadow Fight 2", "version": "1.121.0", "mod": "Premium"},
        "file": {"url": "  https://cdn.apkmod/app.apk  ", "filename": "SF2-premium.apk"}
    }"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("apkmod-get").unwrap();
        assert_eq!(p.meta.name, "apkmod-get");
        assert_eq!(p.meta.alias, ["apkmodget", "getapkmod"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Download APK MOD dari hasil pencarian");
        assert_eq!(p.meta.usage, ".apkmod-get <no> <query>");
        assert_eq!(p.meta.example, ".apkmod-get 1 vpn");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["apkmodget", "getapkmod"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "apkmod-get",
                "alias {a}"
            );
        }
    }

    #[test]
    fn a_missing_number_or_query_prints_the_format_without_a_request() {
        let http = Stub::empty();
        for text in ["", "1", "vpn", "abc vpn", "0 vpn", "  "] {
            assert_eq!(body_of(&run(http.clone(), text)), bad_format("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_file_with_a_url_becomes_a_document() {
        let http = Stub::new(ADA_FILE);
        let r = run(http.clone(), "2 shadow fight");
        let (url, name) = doc_of(&r);
        assert_eq!(url.as_deref(), Some("https://cdn.apkmod/app.apk"));
        assert_eq!(name, "SF2-premium.apk");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/apkmod?q=shadow%20fight&no=2&apikey="
        );
    }

    #[test]
    fn the_file_name_falls_back_to_the_app_name_plus_apk() {
        let body = r#"{"status": true, "data": {"name": "Vpn Master"}, "file": {"url": "https://cdn/a.apk"}}"#;
        let (_, name) = doc_of(&run(Stub::new(body), "1 vpn"));
        assert_eq!(name, "Vpn Master.apk");

        let body = r#"{"status": true, "data": {}, "file": {"url": "https://cdn/a.apk"}}"#;
        let (_, name) = doc_of(&run(Stub::new(body), "1 vpn"));
        assert_eq!(name, "undefined.apk", "`app.name` hilang jadi undefined");
    }

    #[test]
    fn an_empty_or_absent_file_url_warns() {
        for body in [
            r#"{"status": true, "data": {"name": "A"}}"#,
            r#"{"status": true, "data": {"name": "A"}, "file": {}}"#,
            r#"{"status": true, "data": {"name": "A"}, "file": {"url": "   "}}"#,
            r#"{"status": true, "data": {"name": "A"}, "file": {"url": 7}}"#,
        ] {
            let r = run(Stub::new(body), "1 vpn");
            assert_eq!(body_of(&r), NO_DOWNLOAD_URL, "{body}");
            assert_eq!(reaction_of(&r), Some("⚠️"), "{body}");
        }
    }

    #[test]
    fn a_falsy_status_or_missing_data_hits_the_catch_branch() {
        for body in [
            r#"{"status": false, "data": {"name": "A"}, "file": {"url": "https://cdn/a.apk"}}"#,
            r#"{"status": true, "file": {"url": "https://cdn/a.apk"}}"#,
            r#"{"status": true, "data": null}"#,
            "<html>bukan json</html>",
        ] {
            let r = run(Stub::new(body), "1 vpn");
            assert_eq!(body_of(&r), crate::common::error_template_bawaan(".", "apkmod-get"), "{body}");
            assert_eq!(reaction_of(&r), Some("☢"), "{body}");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "1 vpn");
        assert!(body_of(&r).contains("command `.apkmod-get`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_number_and_query_are_sent_as_typed_by_the_user() {
        // `no` ikut `parseInt` di guard tapi dikirim apa adanya di URL.
        let http = Stub::new(ADA_FILE);
        let _ = run(http.clone(), "007 vpn pro max & co");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/apkmod?q=vpn%20pro%20max%20%26%20co&no=007&apikey="
        );
    }

    #[test]
    fn a_very_long_query_is_sent_whole() {
        let http = Stub::new(ADA_FILE);
        let _ = run(http.clone(), &format!("1 {}", "b".repeat(4_000)));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"b".repeat(4_000)), "{}", seen[0].len());
    }
}