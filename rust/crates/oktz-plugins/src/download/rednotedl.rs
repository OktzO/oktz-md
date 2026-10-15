//! `plugins/download/rednotedl.js` — unduh video/foto dari RedNote.
//!
//! Scraper `RedNoteDL` (`src/scraper/rednote.js`) ikut dipindah: satu POST JSON
//! ke `savevideodown.com` lalu pemetaan field, tanpa parser HTML.
//!
//! `ponytail:` scraper memasang `X-Forwarded-For`/`X-Real-IP`/`Client-IP` dengan
//! IP acak, tapi trait `HttpGet` tidak punya kontrol header, jadi ketiga header
//! itu hilang.
//!
//! `ponytail:` `Reply` hanya bisa membawa satu media. JS mengirim maksimal 5
//! foto dalam satu album; di sini hanya foto pertama yang ikut, dan teks
//! "Masih ada N foto lagi" pindah ke caption-nya. `Reply` juga tidak punya
//! variant video, jadi video terkirim sebagai judulnya saja.

use oktz_plugin_api::{Ctx, HttpGet, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy_value;

/// `rednote.js:10` — endpoint scraper.
pub(crate) const ENDPOINT: &str = "https://rednote.savevideodown.com/api/download";

/// `rednotedl.js:23-29` — kartu pakai, delimiter markdown-nya asterisk.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📕 *RedNote Downloader*\n\n\
         Download video atau foto dari XiaoHongShu (RedNote).\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}rednotedl <link>*\n\n\
         *CONTOH:*\n\
         > *{prefix}rednotedl https://www.xiaohongshu.com/xxx*"
    )
}

/// `rednotedl.js:40` — `result.status` falsy.
pub(crate) fn rednote_failed(error: &str) -> String {
    format!("❌ *RedNote Gagal*\n\n> {error}")
}

/// `rednotedl.js:64` — cabang `catch`.
pub(crate) const TRANSPORT_FAILED: &str = "❌ Gagal mengambil data RedNote, coba lagi nanti";

/// `rednotedl.js:42` — `resData.error` kosong.
pub(crate) const DEFAULT_ERROR: &str = "Gagal mengambil data";

/// Bentuk `status: true` dari `RedNoteDL` (`src/scraper/rednote.js:35-44`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Rednote {
    pub title: String,
    /// `resData.type` — `"video"` berarti `results` cuma satu URL.
    pub kind: String,
    pub results: Vec<String>,
}

/// `src/scraper/rednote.js:28-44`. `Err` = `success` falsy, `String` = pesan
/// yang sudah siap tampil.
pub(crate) fn normalize(body: &Value) -> Result<Rednote, String> {
    if !body.get("success").is_some_and(js_truthy_value) {
        let error = body
            .get("error")
            .filter(|e| js_truthy_value(e))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_ERROR)
            .to_string();
        return Err(error);
    }
    let kind = body
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let results = if kind == "video" {
        // `[resData.directVideoUrl || resData.downloadUrl]` — arr-nya selalu
        // panjang satu walau elemennya `undefined`.
        let url = body
            .get("directVideoUrl")
            .filter(|v| js_truthy_value(v))
            .or_else(|| body.get("downloadUrl"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        vec![url]
    } else {
        body.get("directImages")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|i| i.as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    Ok(Rednote {
        title: body
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        kind,
        results,
    })
}

/// `rednotedl.js:53-57` — catatan sisa foto, hanya kalau lebih dari lima.
pub(crate) fn more_note(count: usize) -> String {
    if count > 5 {
        format!("_Masih ada {} foto lagi, maksimal 5_", count - 5)
    } else {
        String::new()
    }
}

/// `RedNoteDL(url)` — `Err` = lempar (transport atau body bukan JSON).
pub(crate) fn fetch_rednote(url: &str, http: &dyn HttpGet) -> Result<Result<Rednote, String>, ()> {
    let payload = serde_json::json!({ "url": url });
    let body = http
        .post_json(ENDPOINT, &payload)
        .map_err(|_| ())?
        .json()
        .ok_or(())?;
    Ok(normalize(&body))
}

#[plugin(
    name = "rednotedl",
    alias = ["rednote", "xhsdl", "xiaohongshu"],
    category = "download",
    description = "Download video/foto dari RedNote (XiaoHongShu)",
    usage = ".rednotedl <url>",
    example = ".rednotedl https://www.xiaohongshu.com/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn rednotedl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    let data = match fetch_rednote(url, ctx.http.as_ref()) {
        Err(()) => return Reply::text(TRANSPORT_FAILED).with_reaction("☢"),
        Ok(Err(error)) => return Reply::text(rednote_failed(&error)).with_reaction("☢"),
        Ok(Ok(data)) => data,
    };

    if data.kind == "video" {
        // `results[0]` bisa `undefined` walau array-nya panjang satu; di JS
        // `sendMedia(undefined)` melempar dan masuk ke `catch`.
        if data.results.first().is_none_or(String::is_empty) {
            return Reply::text(TRANSPORT_FAILED).with_reaction("☢");
        }
        // `rednotedl.js:44` — `sendMedia` video dengan caption `result.title`.
        return Reply::text(data.title.clone()).with_reaction("✅");
    }

    if data.results.is_empty() {
        // `rednotedl.js:43-58` tidak terkirim apa pun; hanya reaksi centang.
        return Reply::None.with_reaction("✅");
    }

    // `ponytail:` hanya foto pertama yang ikut — `Reply` cuma bisa satu media.
    Reply::image(
        MediaSource::url(data.results[0].clone()),
        more_note(data.results.len()),
    )
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use serde_json::json;
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
            unimplemented!("cuma POST")
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(format!("{url}::{body}"));
            let text = self
                .body
                .clone()
                .map_err(|()| HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "rednotedl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("rednotedl").unwrap().handler)(ctx)
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("rednotedl").unwrap();
        assert_eq!(p.meta.name, "rednotedl");
        assert_eq!(p.meta.alias, ["rednote", "xhsdl", "xiaohongshu"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(
            p.meta.description,
            "Download video/foto dari RedNote (XiaoHongShu)"
        );
        assert_eq!(p.meta.usage, ".rednotedl <url>");
        assert_eq!(p.meta.example, ".rednotedl https://www.xiaohongshu.com/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["rednote", "xhsdl", "xiaohongshu"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "rednotedl");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card_with_a_red_cross() {
        let http = Stub::body(json!({ "success": true }));
        for text in ["", "   "] {
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaction_of(&r), Some("❌"), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_failed_response_reports_the_backend_message() {
        let r = run(
            Stub::body(json!({ "success": false, "error": "Postingan privat" })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(text_of(&r), "❌ *RedNote Gagal*\n\n> Postingan privat");
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn a_failure_without_a_reason_uses_the_default_text() {
        for body in [json!({ "success": false }), json!({ "success": false, "error": "" })] {
            assert_eq!(normalize(&body).unwrap_err(), DEFAULT_ERROR, "body {body}");
        }
    }

    #[test]
    fn a_video_result_replies_with_the_post_title() {
        let r = run(
            Stub::body(json!({
                "success": true,
                "type": "video",
                "title": "Catatan Umum",
                "directVideoUrl": "https://cdn.example/v.mp4"
            })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(text_of(&r), "Catatan Umum");
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn a_video_without_any_url_falls_into_the_catch_branch() {
        // `results` tetap panjang satu walau nilainya kosong, dan `sendMedia`
        // dengan URL kosong melempar di JS.
        for body in [
            json!({ "success": true, "type": "video", "title": "x" }),
            json!({ "success": true, "type": "video", "title": "x", "directVideoUrl": null }),
        ] {
            let r = run(Stub::body(body.clone()), "https://www.xiaohongshu.com/x");
            assert_eq!(text_of(&r), TRANSPORT_FAILED, "body {body}");
            assert_eq!(reaction_of(&r), Some("☢"));
        }
    }

    #[test]
    fn the_fallback_url_is_used_when_direct_video_is_missing() {
        let data = normalize(&json!({
            "success": true,
            "type": "video",
            "downloadUrl": "https://cdn.example/fallback.mp4"
        }))
        .unwrap();
        assert_eq!(data.results, ["https://cdn.example/fallback.mp4"]);
    }

    #[test]
    fn up_to_five_photos_are_sent_and_the_surplus_is_announced() {
        for count in 1..=5usize {
            let urls: Vec<String> = (0..count).map(|i| format!("https://cdn.example/{i}.jpg")).collect();
            let r = run(
                Stub::body(json!({ "success": true, "type": "image", "directImages": urls })),
                "https://www.xiaohongshu.com/x",
            );
            assert_eq!(text_of(&r), "", "jumlah {count} tidak ada catatan sisa");
            assert_eq!(reaction_of(&r), Some("✅"));
        }

        let urls: Vec<String> = (0..8).map(|i| format!("https://cdn.example/{i}.jpg")).collect();
        let r = run(
            Stub::body(json!({ "success": true, "type": "image", "directImages": urls })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(text_of(&r), "_Masih ada 3 foto lagi, maksimal 5_");
    }

    #[test]
    fn the_surplus_note_only_shows_above_five() {
        assert_eq!(more_note(5), "");
        assert_eq!(more_note(6), "_Masih ada 1 foto lagi, maksimal 5_");
        assert_eq!(more_note(9), "_Masih ada 4 foto lagi, maksimal 5_");
    }

    #[test]
    fn a_post_without_any_media_only_reacts() {
        for body in [
            json!({ "success": true, "type": "image", "directImages": [] }),
            json!({ "success": true, "type": "image" }),
            json!({ "success": true }),
        ] {
            let r = run(Stub::body(body.clone()), "https://www.xiaohongshu.com/x");
            assert!(r.without_mutations().text_body().is_none(), "body {body}");
            assert_eq!(reaction_of(&r), Some("✅"), "body {body}");
        }
    }

    #[test]
    fn the_url_is_sent_as_a_json_body_field() {
        let http = Stub::body(json!({ "success": true, "type": "image", "directImages": [] }));
        let _ = run(http.clone(), "https://www.xiaohongshu.com/x");
        assert_eq!(
            http.seen.lock().unwrap().clone(),
            vec![
                "https://rednote.savevideodown.com/api/download\
                 ::{\"url\":\"https://www.xiaohongshu.com/x\"}"
                    .to_string()
            ]
        );
    }

    #[test]
    fn a_transport_failure_reports_the_catch_message() {
        let r = run(Stub::offline(), "https://www.xiaohongshu.com/x");
        assert_eq!(text_of(&r), TRANSPORT_FAILED);
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}