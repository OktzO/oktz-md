//! `plugins/search/android1-get.js` — unduh satu APK dari URL halaman an1.com.
//!
//! `ponytail:` JS mengirim dokumen lewat `sock.sendMessage` dengan
//! `contextInfo: { forwardingScore: 99, isForwarded: true }`. `Reply::Document`
//! tidak punya context info, jadi dokumen terkirim tanpa penanda forwarded —
//! nama berkas, mimetype, dan URL tetap sama.
//!
//! `ponytail:` cabang "URL tidak tersedia" mengirim
//! `forwardedNewsletterMessageInfo` (butuh `config.saluran`/`config.bot.name`)
//! plus `interactiveButtons` cta_url. `Reply::Interactive` hanya membawa teksnya,
//! jadi yang terkirim cuma `> ⚠️ Download URL tidak tersedia`.
//!
//! `ponytail:` reaksi pertama (`🕕`) tidak ikut: `Reply::Reaction` hanya satu
//! emoji, dan reaksi terakhir yang menandai hasil akhir — sama seperti
//! `search/android1.rs`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.neoxr || ''` (`config.js`) = `process.env.APIKEY_NEOXR || ""`.
///
/// `ponytail:` `Ctx` tidak membawa config, jadi kunci ikut `config.js` yang
/// di-hardcode — sama seperti `search/android1.rs`.
const NEOXR_APIKEY: &str = "";

/// `android1-get.js:33` — endpoint detail + unduhan.
pub(crate) fn api_url(url: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/an1-get?url={}&apikey={NEOXR_APIKEY}",
        encode_uri_component(url)
    )
}

/// `android1-get.js:26` — guard URL.
pub(crate) const INVALID_URL: &str = "❌ URL tidak valid! Harus URL dari an1.com";

/// `android1-get.js:63` — caption saat `app.url` tidak ada.
pub(crate) const NO_DOWNLOAD_URL: &str = "> ⚠️ Download URL tidak tersedia";

/// `app.url` yang lolos `if (app.url)` DAN bisa dipakai sebagai sumber media.
///
/// Nilai selain string yang truthy (mis. angka) tidak bisa jadi URL, jadi
/// jatuh ke cabang "tidak tersedia" — sedikit lebih longgar dari JS yang
/// meneruskannya apa adanya ke Baileys.
pub(crate) fn app_url(app: &Value) -> Option<&str> {
    app.get("url")
        .filter(|v| js_truthy(Some(*v)))
        .and_then(Value::as_str)
}

#[plugin(
    name = "android1-get",
    alias = ["an1get", "an1dl"],
    category = "search",
    description = "Download APK dari Android1",
    usage = ".android1-get <url>",
    example = ".android1-get https://an1.com/xxx",
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
pub fn android1_get(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();

    if url.is_empty() || !url.contains("an1.com") {
        return Reply::text(INVALID_URL);
    }

    // `throw new Error("Gagal mengambil detail APK")` dilempar ke `catch`, jadi
    // hasilnya template error yang sama seperti di `info/jadwalbola.rs`.
    let Some(resp) = ctx.http.get(&api_url(url)).ok() else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let data = resp.json().unwrap_or(Value::Null);

    if !js_truthy(data.get("status")) || !js_truthy(data.get("data")) {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    }
    let app = data.get("data").unwrap_or(&Value::Null);

    match app_url(app) {
        Some(source) => Reply::document(
            MediaSource::url(source).with_mimetype("application/vnd.android.package-archive"),
            // `fileName: app.name` yang `undefined` berarti Baileys memakai
            // nama bawaannya, jadi string kosong — bukan teks "undefined".
            app.get("name").and_then(Value::as_str).unwrap_or(""),
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
            command: "android1-get".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("android1-get").unwrap().handler)(ctx)
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

    const ADA_URL: &str = r#"{"status": true, "data": {"name": "Subway Surfers", "url": "https://an1.com/a/1.apk"}}"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("android1-get").unwrap();
        assert_eq!(p.meta.name, "android1-get");
        assert_eq!(p.meta.alias, ["an1get", "an1dl"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Download APK dari Android1");
        assert_eq!(p.meta.usage, ".android1-get <url>");
        assert_eq!(p.meta.example, ".android1-get https://an1.com/xxx");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["an1get", "an1dl"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "android1-get",
                "alias {a}"
            );
        }
    }

    #[test]
    fn a_blank_or_foreign_url_is_refused_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   ", "https://apkpure.com/x", "an1 com", "😀😀😀"] {
            assert_eq!(body_of(&run(http.clone(), text)), INVALID_URL, "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn an_app_with_a_url_becomes_a_document_named_after_the_app() {
        let r = run(Stub::new(ADA_URL), "https://an1.com/abc");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(r.has_media());
        let src = r.media_source().expect("harus ada media");
        assert_eq!(
            src,
            &MediaSource::url("https://an1.com/a/1.apk")
                .with_mimetype("application/vnd.android.package-archive")
        );
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Document { file_name, .. } => assert_eq!(file_name, "Subway Surfers"),
                other => panic!("harus Document, dapat {other:?}"),
            },
            other => panic!("harus dibungkus reaksi, dapat {other:?}"),
        }
    }

    #[test]
    fn the_request_goes_to_the_an1_get_endpoint_with_an_encoded_url() {
        let http = Stub::new(ADA_URL);
        let _ = run(http.clone(), "https://an1.com/a b?x=1&y='z'");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/an1-get?url=https%3A%2F%2Fan1.com%2Fa%20b%3Fx%3D1%26y%3D'z'&apikey="
        );
    }

    #[test]
    fn an_app_without_a_url_warns_and_offers_nothing_else() {
        for body in [
            r#"{"status": true, "data": {"name": "Tanpa URL"}}"#,
            r#"{"status": true, "data": {"name": "Kosong", "url": ""}}"#,
        ] {
            let r = run(Stub::new(body), "https://an1.com/a");
            assert_eq!(body_of(&r), NO_DOWNLOAD_URL, "{body}");
            assert_eq!(reaction_of(&r), Some("⚠️"), "{body}");
            assert!(matches!(r.without_mutations().tanpa_reaksi(), Reply::Interactive { .. }), "{body}");
        }
    }

    #[test]
    fn a_falsy_status_or_missing_data_hits_the_catch_branch() {
        for body in [
            r#"{"status": false, "data": {"url": "https://an1.com/a.apk"}}"#,
            r#"{"status": true}"#,
            r#"{"status": true, "data": null}"#,
            "<html>bukan json</html>",
        ] {
            let r = run(Stub::new(body), "https://an1.com/a");
            assert_eq!(body_of(&r), crate::common::error_template_bawaan(".", "android1-get"), "{body}");
            assert_eq!(reaction_of(&r), Some("☢"), "{body}");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "https://an1.com/a");
        assert!(body_of(&r).contains("command `.android1-get`"), "{}", body_of(&r));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn a_missing_name_falls_back_to_the_default_file_name() {
        let body = r#"{"status": true, "data": {"url": "https://an1.com/a.apk"}}"#;
        let r = run(Stub::new(body), "https://an1.com/a");
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Document { file_name, .. } => assert_eq!(file_name, ""),
                other => panic!("harus Document, dapat {other:?}"),
            },
            other => panic!("harus dibungkus reaksi, dapat {other:?}"),
        }
    }

    #[test]
    fn a_very_long_url_is_sent_whole() {
        let url = format!("https://an1.com/{}", "a".repeat(4_000));
        let http = Stub::new(ADA_URL);
        let _ = run(http.clone(), &url);
        let seen = http.seen.lock().unwrap();
        assert!(
            seen[0].contains(&"a".repeat(4_000)),
            "URL tidak boleh dipotong: {}",
            seen[0].len()
        );
    }
}