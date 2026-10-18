//! `plugins/download/shopeedl.js` — unduh video dari Shopee.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMessage` hanya
//! terkirim sebagai caption teks; URL videonya tidak ikut.

use oktz_plugin_api::{Ctx, HttpBody, HttpGet, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy_value;

/// `shopeedl.js:17` — host scraper pihak ketiga.
pub(crate) const BASE_URL: &str = "https://shopeenowatermark.com";

/// `shopeedl.js:35` — urutan kualitas pilihan.
const ORDER: [&str; 8] = [
    "V1080P",
    "V720P",
    "V540P",
    "V360P",
    "V1080P_H265",
    "V720P_H265",
    "V540P_H265",
    "V360P_H265",
];

/// `shopeedl.js:47` — link tidak ada atau tidak memuat kata "shopee".
pub(crate) const BAD_URL: &str =
    "❌ Masukkan link video Shopee yang valid.\n\nContoh: `.shopeedl https://shopee.co.id/...`";

/// `shopeedl.js:56` — `streams_array` kosong atau tidak ada.
pub(crate) const NO_STREAM: &str =
    "⚠️ Gagal mengekstrak video. Pastikan link video Shopee sudah benar dan bersifat publik.";

/// `shopeedl.js:77` — cabang `catch`.
pub(crate) const SEND_FAILED: &str = "😔 Gagal mengunduh video dari Shopee.";

/// `` `${value}` `` di JS — field yang hilang tercetak `undefined`, `null`
/// tercetak `null`.
fn template_value(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `shopeedl.js:34-41` — `bestStream`.
pub(crate) fn best_stream(streams: &[Value]) -> Option<&Value> {
    for quality in ORDER {
        if let Some(found) = streams
            .iter()
            .find(|s| s.get("quality").and_then(Value::as_str) == Some(quality))
        {
            return Some(found);
        }
    }
    streams.first()
}

/// `shopeedl.js:62-65`.
pub(crate) fn caption(data: &Value, best: &Value) -> String {
    let mut out = String::from("🛍️ *SHOPEE VIDEO DOWNLOADER* 🛍️\n\n");
    if let Some(username) = data.get("username").filter(|v| js_truthy_value(v)) {
        out.push_str(&format!("*Username:* {}\n", template_value(Some(username))));
    }
    out.push_str(&format!(
        "*Kualitas:* {}\n",
        template_value(best.get("quality"))
    ));
    out.push_str("\n> Dibuat oleh bot kesayanganmu");
    out
}

/// `extract(url)` — `Err` = lempar, jadi cabang `catch` plugin yang menyala.
pub(crate) fn extract(url: &str, http: &dyn HttpGet) -> Result<Value, ()> {
    let target = format!("{BASE_URL}/api/extract");
    let response = http
        .post_multipart(&target, &[("url", HttpBody::Form(url.into()))])
        .map_err(|_| ())?;
    // `if (!res.ok) throw new Error(...)` sudah ditangani trait: status di atas
    // 399 jadi `Err`.
    let data = response.json().ok_or(())?;
    if !data.get("success").is_some_and(js_truthy_value) {
        return Err(());
    }
    Ok(data)
}

#[plugin(
    name = "shopeedl",
    alias = ["shopeevideo", "shopeevid"],
    category = "download",
    description = "Download video dari Shopee",
    usage = ".shopeedl <url>",
    example = ".shopeedl https://shopee.co.id/universal-link/video/...",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn shopeedl(ctx: Ctx) -> Reply {
    // `m.args[0] || m.text?.trim()`
    let url = ctx
        .args
        .first()
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| ctx.text.trim().to_string());

    if url.is_empty() || !url.contains("shopee") {
        return Reply::text(BAD_URL);
    }

    let Ok(data) = extract(&url, ctx.http.as_ref()) else {
        return Reply::text(SEND_FAILED).with_reaction("☢");
    };

    let streams = data
        .get("streams_array")
        .filter(|s| js_truthy_value(s))
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice);
    if streams.is_empty() {
        return Reply::text(NO_STREAM).with_reaction("❌");
    }

    let Some(best) = best_stream(streams) else {
        return Reply::text(NO_STREAM).with_reaction("❌");
    };

    // `shopeedl.js:67-72` — `sendMessage` video lalu centang hijau.
    Reply::text(caption(&data, best)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST multipart")
        }
        fn post_multipart(
            &self,
            url: &str,
            parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(format!(
                "{url}::{:?}",
                parts
                    .iter()
                    .map(|(k, v)| (*k, String::from_utf8_lossy(v.bytes()).into_owned()))
                    .collect::<Vec<_>>()
            ));
            let text = self
                .body
                .clone()
                .map_err(|()| oktz_plugin_api::HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "shopeedl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("shopeedl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    fn stream(quality: &str) -> Value {
        serde_json::json!({ "quality": quality, "stream_url": format!("https://cdn/{quality}.mp4") })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("shopeedl").unwrap();
        assert_eq!(p.meta.name, "shopeedl");
        assert_eq!(p.meta.alias, ["shopeevideo", "shopeevid"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video dari Shopee");
        assert_eq!(p.meta.usage, ".shopeedl <url>");
        assert_eq!(
            p.meta.example,
            ".shopeedl https://shopee.co.id/universal-link/video/..."
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["shopeevideo", "shopeevid"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "shopeedl");
        }
    }

    #[test]
    fn a_missing_or_foreign_link_is_refused_without_any_request() {
        let http = Stub::body(serde_json::json!({ "success": true }));
        for text in ["", "   ", "https://tokopedia.com/x"] {
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), BAD_URL, "input {text:?}");
            assert!(reaction_of(&r).is_none(), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_link_is_posted_as_a_multipart_form_field() {
        let http = Stub::body(serde_json::json!({ "success": true, "streams_array": [stream("V1080P")] }));
        let _ = run(http.clone(), "https://shopee.co.id/universal-link/video/1");
        assert_eq!(
            http.seen.lock().unwrap().clone(),
            vec![
                "https://shopeenowatermark.com/api/extract\
                 ::[(\"url\", \"https://shopee.co.id/universal-link/video/1\")]"
                    .to_string()
            ]
        );
    }

    #[test]
    fn the_quality_order_of_the_js_is_kept() {
        let semua = vec![
            stream("V360P_H265"),
            stream("V1080P"),
            stream("V720P"),
            stream("V360P"),
            stream("V540P"),
        ];
        assert_eq!(best_stream(&semua).unwrap()["quality"], "V1080P");

        let tanpa_hd = vec![stream("V360P"), stream("V720P"), stream("V540P")];
        assert_eq!(best_stream(&tanpa_hd).unwrap()["quality"], "V720P");

        let h265_saja = vec![stream("V1080P_H265"), stream("V360P")];
        // `shopeedl.js:35` menaruh `V360P` sebelum `V1080P_H265`, jadi JS
        // memilih yang lebih kecil lebih dulu kalau `V1080P` tidak ada.
        assert_eq!(best_stream(&h265_saja).unwrap()["quality"], "V360P");

        // Tidak ada yang cocok → item pertama, bukan qualities[0] yang undefin.
        let lain = vec![stream("V999P"), stream("V1081P")];
        assert_eq!(best_stream(&lain).unwrap()["quality"], "V999P");

        assert!(best_stream(&[]).is_none());
    }

    #[test]
    fn a_resolved_video_replies_with_the_caption_and_a_green_tick() {
        let r = run(
            Stub::body(serde_json::json!({
                "success": true,
                "username": "toko_saya",
                "streams_array": [stream("V360P"), stream("V720P")]
            })),
            "https://shopee.co.id/universal-link/video/1",
        );
        assert_eq!(
            text_of(&r),
            "🛍️ *SHOPEE VIDEO DOWNLOADER* 🛍️\n\n\
             *Username:* toko_saya\n\
             *Kualitas:* V720P\n\
             \n> Dibuat oleh bot kesayanganmu"
        );
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn the_username_line_is_skipped_when_it_is_falsy() {
        for body in [
            serde_json::json!({ "success": true, "streams_array": [stream("V720P")] }),
            serde_json::json!({ "success": true, "username": "", "streams_array": [stream("V720P")] }),
            serde_json::json!({ "success": true, "username": null, "streams_array": [stream("V720P")] }),
        ] {
            let r = run(Stub::body(body.clone()), "https://shopee.co.id/x");
            assert!(!text_of(&r).contains("Username"), "body {body}: {}", text_of(&r));
        }
    }

    #[test]
    fn an_empty_stream_list_gets_the_extraction_warning() {
        for body in [
            serde_json::json!({ "success": true, "streams_array": [] }),
            serde_json::json!({ "success": true }),
            serde_json::json!({ "success": true, "streams_array": null }),
        ] {
            let r = run(Stub::body(body.clone()), "https://shopee.co.id/x");
            assert_eq!(text_of(&r), NO_STREAM, "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn a_failed_extraction_reports_the_send_failure() {
        for http in [
            Stub::body(serde_json::json!({ "success": false, "error": "Extraction failed" })),
            Stub::body(serde_json::json!({})),
            Stub::offline(),
        ] {
            let r = run(http, "https://shopee.co.id/x");
            assert_eq!(text_of(&r), SEND_FAILED);
            assert_eq!(reaction_of(&r), Some("☢"));
        }
    }
}