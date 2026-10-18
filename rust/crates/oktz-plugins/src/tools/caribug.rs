//! `plugins/tools/caribug.js` — cari bug di kode lewat API Cuki.
//!
//! NOTE: baris `pub mod caribug;` perlu ditambah ke `tools/mod.rs` secara
//! manual (mod.rs dilarang disentuh).
//!
//! `ponytail:` JS bereaksi dua kali (`🕕` awal, lalu `✅`/`❌` akhir).
//! `Reply::with_reaction` cuma satu emoji, jadi cuma reaksi akhir yang ikut.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

pub(crate) const ENDPOINT: &str = "https://api.cuki.biz.id/api/aicode/caribug";

/// `explainFailure(new MissingApiKeyError('cuki', 'APIKEY_CUKI'), ..)`
/// (`stalker-fallback.js:29-45`).
pub(crate) const MISSING_KEY: &str =
    "❌ *API Key belum diset!*\n\n> Command `.caribug` butuh `APIKEY_CUKI` di `.env`\n> Saat ini belum ada, jadi tidak bisa dipanggil.";

pub(crate) const SCAN_FAILED: &str = "Gagal menganalisa kode dari server";

/// `urlencode` untuk query param — pakai encode_uri_component (bukan
/// URLSearchParams: spasi jadi `%20`).
pub(crate) fn api_url(code: &str, api_key: &str) -> String {
    format!(
        "{ENDPOINT}?apikey={}&code={}&language=auto",
        encode_uri_component(api_key),
        encode_uri_component(code)
    )
}

fn severity_level(meta: &Value) -> String {
    let level = meta
        .get("severityInfo")
        .and_then(|v| v.get("level"))
        .and_then(Value::as_str)
        .unwrap_or("-");
    let icon = meta
        .get("severityInfo")
        .and_then(|v| v.get("icon"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if icon.is_empty() {
        level.to_string()
    } else {
        format!("{level} {icon}")
    }
}

fn rendered(data: &Value) -> String {
    let meta = data.get("metadata").unwrap_or(&Value::Null);
    let bug_info = data.get("bugsFound").unwrap_or(&Value::Null);

    let total = bug_info
        .get("total")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let mut text = format!(
        "🐛 *HASIL ANALISA BUG*\n\n\
         *Bahasa:* {}\n\
         *Tingkat:* {}\n\
         *Bug Ditemukan:* {}\n\n",
        meta.get("detectedLanguage")
            .and_then(Value::as_str)
            .unwrap_or("-"),
        severity_level(meta),
        total
    );

    if let Some(s) = bug_info.get("summary").and_then(Value::as_str)
        && !s.is_empty()
    {
        text.push_str(&format!("*📝 Ringkasan:*\n{s}\n\n"));
    }

    if let Some(fixed) = data
        .get("codeAnalysis")
        .and_then(|v| v.get("fixed"))
        .and_then(|v| v.get("code"))
        .and_then(Value::as_str)
        && !fixed.is_empty()
    {
            let lang = meta
                .get("detectedLanguage")
                .and_then(Value::as_str)
                .unwrap_or("");
            text.push_str(&format!(
                "*✨ Kode Perbaikan:*\n```{lang}\n{fixed}\n```\n\n"
            ));
    }

    if let Some(details) = bug_info.get("details").and_then(Value::as_array)
        && !details.is_empty()
    {
        text.push_str("*📌 Detail:* \n");
        for d in details {
            let tipe = d.get("type").and_then(Value::as_str);
            let desc = d.get("description").and_then(Value::as_str);
            let label = tipe.or(desc).unwrap_or("-");
            text.push_str(&format!("- {label}\n"));
        }
    }

    text
}

#[plugin(
    name = "caribug",
    alias = ["debug", "findbug"],
    category = "tools",
    description = "Cari bug di kode pemrograman",
    usage = ".caribug [kode] atau reply kode",
    example = ".caribug function test() {}",
    cooldown = 20,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn caribug(ctx: Ctx) -> Reply {
    let code = ctx
        .quoted
        .as_ref()
        .map(|q| q.text.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ctx.text.trim().to_string());

    if code.is_empty() {
        return Reply::text(format!(
            "*🐛 CARI BUG*\n\nKirim kode atau reply pesan kode untuk mencari bug.\n\nContoh:\n`{prefix}caribug function test() {{}}`",
            prefix = ctx.prefix
        ));
    }

    let api_key = ctx.config().api_keys.cuki.clone();
    if api_key.is_empty() {
        return Reply::text(MISSING_KEY).with_reaction("❌");
    }

    let Ok(res) = ctx.http.get(&api_url(&code, &api_key)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);

    let success = body.get("success").is_some_and(crate::common::js_truthy_value);
    let Some(data) = body.get("data").filter(|v| success && !v.is_null()) else {
        return Reply::text(SCAN_FAILED).with_reaction("❌");
    };

    Reply::text(rendered(&data)).with_reaction("✅")
}

#[cfg(test)]
use serde_json::json;

#[cfg(test)]
fn contoh() -> Value {
    serde_json::json!({
        "success": true,
        "data": {
            "metadata": {
                "detectedLanguage": "JavaScript",
                "severityInfo": { "level": "Tinggi", "icon": "🔴" }
            },
            "bugsFound": {
                "total": 2,
                "summary": "Ada potensi crash saat variabel tidak didefinisikan",
                "details": [
                    { "type": "ReferenceError", "description": "x tidak didefinisikan" },
                    { "description": "tidak ada penanganan error" }
                ]
            },
            "codeAnalysis": {
                "fixed": { "code": "function test() { try {} catch {} }" }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
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
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "caribug".into(),
            http,
            ..Ctx::default()
        };
        caribug(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "caribug".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    cuki: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        };
        caribug(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_kode_menampilkan_konto_pakai() {
        let r = run(Stub::offline(), "");
        assert!(text_of(&r).contains("*🐛 CARI BUG*"));
        assert!(text_of(&r).contains("`.caribug function test() {}`"));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_menghentikan_sebelum_request() {
        let http = Stub::offline();
        let r = run(http.clone(), "function x() {}");
        assert_eq!(text_of(&r), MISSING_KEY);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_membawa_kunci_dan_kode_ter_encode() {
        assert_eq!(
            api_url("let x = y", "KUNCI"),
            "https://api.cuki.biz.id/api/aicode/caribug?apikey=KUNCI&code=let%20x%20%3D%20y&language=auto"
        );
    }

    #[test]
    fn jalur_berhasil_mengembalikan_teks_analisa() {
        let http = Stub::body(contoh());
        let r = run_dengan_kunci(http.clone(), "function x() {}");
        assert_eq!(reaction_of(&r), Some("✅"));
        let t = text_of(&r);
        assert!(t.contains("🐛 *HASIL ANALISA BUG*"), "{t}");
        assert!(t.contains("*Bahasa:* JavaScript"));
        assert!(t.contains("*Tingkat:* Tinggi 🔴"));
        assert!(t.contains("*Bug Ditemukan:* 2"));
        assert!(t.contains("*📝 Ringkasan:*"));
        assert!(t.contains("*✨ Kode Perbaikan:*"));
        assert!(t.contains("- ReferenceError"));
        assert!(t.contains("- tidak ada penanganan error"));
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.cuki.biz.id/api/aicode/caribug?apikey=KUNCI&code=function%20x()%20%7B%7D&language=auto"]
        );
    }

    #[test]
    fn success_false_atau_data_kosong_menghasilkan_gagal() {
        for body in [
            serde_json::json!({ "success": false, "data": {} }),
            serde_json::json!({ "success": true }),
            serde_json::json!({ "success": true, "data": null }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "x");
            assert_eq!(text_of(&r), SCAN_FAILED, "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_error_template() {
        let r = run_dengan_kunci(Stub::offline(), "x");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.caribug` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
