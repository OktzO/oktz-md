//! `plugins/download/facebookdl.js` — unduh video Facebook.
//!
//! Scraper `fbdown` (`src/scraper/fbdown.js`) cuma satu GET ke agregator
//! nexray dan tidak butuh API key, jadi ikut dipindah utuh di sini.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMedia` hanya
//! terkirim sebagai caption teks; URL-nya tidak ikut — tidak ada padanan
//! `Reply::Video` di `oktz-plugin-api`.

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};
use serde_json::Value;

/// `fbdown.js:5` — agregator `nexrayWeb` (`src/lib/aggregator.js:30`).
pub(crate) const ENDPOINT: &str = "https://api.nexray.web.id/api/downloader/facebook";

/// `encodeURIComponent` versi axios untuk `params` — `axios.cjs:1866`.
/// Berbeda dari `URLSearchParams` (`super::douyindl::query_encode`): axios
/// melepas `%3A` jadi `:` dan menaruh spasi sebagai `+`.
pub(crate) fn axios_query(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(*b as char),
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => out.push(*b as char),
            b':' => out.push(':'),
            b'$' => out.push('$'),
            b',' => out.push(','),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `facebookdl.js:32` — `/facebook\.com|fb\.watch|fb\.com/i`.
pub(crate) fn is_facebook_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("facebook.com") || lower.contains("fb.watch") || lower.contains("fb.com")
}

/// `facebookdl.js:24-29` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *CARA PAKAI*\n\n\
         - `{prefix}facebookdl <url>`\n\n\
         *Contoh:*\n\
         - `{prefix}fbdown https://www.facebook.com/watch?v=xxx`"
    )
}

/// `facebookdl.js:33` — URL bukan Facebook.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link Facebook.";

/// `facebookdl.js:43` — respons tanpa `medias` yang tidak kosong.
pub(crate) const NO_MEDIA: &str = "❌ Gagal mengambil video. Coba link lain atau pastikan postingan bersifat publik.\n\n_Catatan: Sistem saat ini belum mendukung download foto Facebook, hanya video._";

/// `facebookdl.js:53` — `medias` ada tapi tidak ada URL-nya.
pub(crate) const NO_VIDEO: &str = "❌ Video tidak ditemukan di link tersebut.\n\n_Catatan: Sistem saat ini belum mendukung download foto Facebook, hanya video._";

/// Satu entri `result.medias` (`src/scraper/fbdown.js:11-12`). `formatted_size`
/// tidak pernah diisi scraper, tapi barisnya tetap ada di caption (`facebookdl.js:59`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Media {
    pub quality: String,
    pub url: String,
    pub formatted_size: String,
}

/// Bentuk hasil `fbdown` yang dibaca plugin.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Facebook {
    pub title: String,
    pub medias: Vec<Media>,
}

/// `r.title ?? ""` — `null` jadi string kosong, nilai selain string tetap
/// tercetak seperti di JS.
fn nullable_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `fbdown.js:8-15` — hanya `status === true` dengan `result` yang diterima.
pub(crate) fn normalize(body: &Value) -> Option<Facebook> {
    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let result = body.get("result").filter(|r| !r.is_null())?;
    let mut medias = Vec::new();
    // Urutan push di `fbdown.js:11-12` menentukan urutan `medias`.
    for (field, quality) in [("video_hd", "hd"), ("video_sd", "sd")] {
        if let Some(url) = result
            .get(field)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            medias.push(Media {
                quality: quality.to_string(),
                url: url.to_string(),
                formatted_size: String::new(),
            });
        }
    }
    Some(Facebook {
        title: nullable_text(result.get("title")),
        medias,
    })
}

/// `facebookdl.js:47-49` — HD dulu, lalu SD, lalu item pertama.
pub(crate) fn pick(medias: &[Media]) -> Option<&Media> {
    medias
        .iter()
        .find(|m| m.quality == "hd")
        .or_else(|| medias.iter().find(|m| m.quality == "sd"))
        .or_else(|| medias.first())
}

/// `facebookdl.js:56-62`.
pub(crate) fn caption(data: &Facebook, media: &Media) -> String {
    let title = if data.title.is_empty() {
        "Video Facebook"
    } else {
        &data.title
    };
    let quality = if media.quality.is_empty() {
        "Normal"
    } else {
        &media.quality.to_uppercase()
    };
    let mut out = String::from("🎥 *FACEBOOK DOWNLOADER*\n\n");
    out.push_str(&format!("*Judul:* {title}\n"));
    out.push_str(&format!("*Kualitas:* {quality}\n"));
    if !media.formatted_size.is_empty() {
        out.push_str(&format!("*Ukuran:* {}\n", media.formatted_size));
    }
    out.push_str("\n_Catatan: Fitur ini tidak mensupport postingan foto._");
    out
}

/// `fbdown(url)` (`src/scraper/fbdown.js`). Scraper itu sendiri yang menelan
/// kegagalan jaringan dan mengembalikan `{ status: false }`, jadi `catch` di
/// plugin praktis tidak pernah menyala — dan `Err` di sini berarti bentuk itu.
pub(crate) fn fetch(url: &str, http: &dyn HttpGet) -> Option<Facebook> {
    let target = format!("{ENDPOINT}?url={}", axios_query(url));
    http.get(&target).ok().and_then(|r| r.json()).as_ref().and_then(normalize)
}

#[plugin(
    name = "facebookdl",
    alias = ["fbdown", "fb", "facebook", "fbdl"],
    category = "download",
    description = "Download video Facebook",
    usage = ".facebookdl <url>",
    example = ".facebookdl https://www.facebook.com/watch?v=xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn facebookdl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_facebook_url(url) {
        return Reply::text(BAD_URL);
    }

    let Some(data) = fetch(url, ctx.http.as_ref()) else {
        return Reply::text(NO_MEDIA).with_reaction("❌");
    };
    if data.medias.is_empty() {
        return Reply::text(NO_MEDIA).with_reaction("❌");
    }

    let Some(media) = pick(&data.medias) else {
        return Reply::text(NO_VIDEO).with_reaction("❌");
    };
    if media.url.is_empty() {
        return Reply::text(NO_VIDEO).with_reaction("❌");
    }

    // `facebookdl.js:64-72` — `sendMedia` video lalu reaksi centang.
    Reply::text(caption(&data, media)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
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
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
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
            command: "fbdown".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("facebookdl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn ok_body() -> Value {
        serde_json::json!({
            "status": true,
            "result": {
                "title": "Uji Coba",
                "video_hd": "https://cdn.example/hd.mp4",
                "video_sd": "https://cdn.example/sd.mp4"
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("facebookdl").unwrap();
        assert_eq!(p.meta.name, "facebookdl");
        assert_eq!(p.meta.alias, ["fbdown", "fb", "facebook", "fbdl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video Facebook");
        assert_eq!(p.meta.usage, ".facebookdl <url>");
        assert_eq!(p.meta.example, ".facebookdl https://www.facebook.com/watch?v=xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["fbdown", "fb", "facebook", "fbdl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "facebookdl");
        }
    }

    #[test]
    fn the_guard_accepts_the_three_host_patterns() {
        for good in [
            "https://www.facebook.com/watch?v=x",
            "https://fb.watch/abc",
            "https://fb.com/x",
            "https://www.FACEBOOK.com/x",
        ] {
            assert!(is_facebook_url(good), "harus lolos: {good}");
        }
        for bad in ["https://youtube.com/watch?v=x", "", "facebook"] {
            assert!(!is_facebook_url(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card_without_any_request() {
        let http = Stub::body(ok_body());
        for text in ["", "   "] {
            assert_eq!(text_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_foreign_host_is_rejected_without_any_reaction() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://youtube.com/watch?v=x");
        assert_eq!(text_of(&r), BAD_URL);
        assert!(
            !matches!(r.without_mutations(), Reply::Reaction { .. }),
            "guard host tidak boleh bereaksi"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_link_is_encoded_the_way_axios_params_does_it() {
        assert_eq!(
            axios_query("https://www.facebook.com/watch?v=xxx"),
            "https:%2F%2Fwww.facebook.com%2Fwatch%3Fv%3Dxxx"
        );
        assert_eq!(axios_query("a b"), "a+b");
        assert_eq!(axios_query("a:b,c$d"), "a:b,c$d");
    }

    #[test]
    fn a_resolved_video_replies_with_the_caption_and_a_green_tick() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://www.facebook.com/watch?v=xxx");
        assert_eq!(
            text_of(&r),
            "🎥 *FACEBOOK DOWNLOADER*\n\n\
             *Judul:* Uji Coba\n\
             *Kualitas:* HD\n\
             \n_Catatan: Fitur ini tidak mensupport postingan foto._"
        );
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "✅"),
            "sukses harus bereaksi ✅"
        );
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![
                "https://api.nexray.web.id/api/downloader/facebook\
                 ?url=https:%2F%2Fwww.facebook.com%2Fwatch%3Fv%3Dxxx"
                    .to_string()
            ]
        );
    }

    #[test]
    fn hd_is_preferred_over_sd_and_sd_over_the_first_item() {
        let sd_only = normalize(&serde_json::json!({
            "status": true,
            "result": { "video_sd": "https://cdn.example/sd.mp4" }
        }))
        .unwrap();
        assert_eq!(pick(&sd_only.medias).unwrap().quality, "sd");

        let both = normalize(&ok_body()).unwrap();
        assert_eq!(pick(&both.medias).unwrap().quality, "hd");

        let sd_only_quality_empty = Facebook {
            title: String::new(),
            medias: vec![Media {
                quality: String::new(),
                url: "https://cdn.example/a.mp4".into(),
                formatted_size: "12 MB".into(),
            }],
        };
        let only = pick(&sd_only_quality_empty.medias).unwrap();
        assert_eq!(only.quality, "");
    }

    #[test]
    fn the_caption_falls_back_to_the_js_defaults() {
        let data = Facebook::default();
        let media = Media {
            quality: String::new(),
            url: "https://cdn.example/a.mp4".into(),
            formatted_size: String::new(),
        };
        assert_eq!(
            caption(&data, &media),
            "🎥 *FACEBOOK DOWNLOADER*\n\n\
             *Judul:* Video Facebook\n\
             *Kualitas:* Normal\n\
             \n_Catatan: Fitur ini tidak mensupport postingan foto._"
        );
    }

    #[test]
    fn the_size_line_only_appears_when_there_is_a_size() {
        let data = Facebook::default();
        let media = Media {
            quality: "hd".into(),
            url: "u".into(),
            formatted_size: "12 MB".into(),
        };
        assert!(caption(&data, &media).contains("*Ukuran:* 12 MB\n"), "{}", caption(&data, &media));

        let tanpa = Media {
            formatted_size: String::new(),
            ..media.clone()
        };
        assert!(!caption(&data, &tanpa).contains("*Ukuran:*"));
    }

    #[test]
    fn an_empty_medias_list_gets_the_no_media_message() {
        for body in [
            serde_json::json!({ "status": true, "result": { "title": "x" } }),
            serde_json::json!({ "status": true, "result": { "video_hd": "", "video_sd": "" } }),
        ] {
            let data = normalize(&body).unwrap();
            assert!(data.medias.is_empty(), "{body}");
        }
        let r = run(Stub::body(serde_json::json!({ "status": true, "result": {} })), "https://fb.com/x");
        assert_eq!(text_of(&r), NO_MEDIA);
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "❌"),
            "kegagalan harus bereaksi ❌"
        );
    }

    #[test]
    fn a_failed_response_is_reported_as_no_media_not_as_a_system_error() {
        // `fbdown.js:15` menelan respons gagal jadi `{ status: false }`, jadi
        // `catch` plugin tidak pernah menyala untuk request yang gagal.
        for body in [
            serde_json::json!({ "status": false, "message": "Gagal mengambil data dari API" }),
            serde_json::json!({}),
            serde_json::json!({ "status": true }),
        ] {
            let r = run(Stub::body(body.clone()), "https://fb.com/x");
            assert_eq!(text_of(&r), NO_MEDIA, "body {body}");
        }
    }

    #[test]
    fn a_transport_failure_takes_the_same_path_as_a_failed_response() {
        let r = run(Stub::offline(), "https://fb.com/x");
        assert_eq!(text_of(&r), NO_MEDIA);
    }
}