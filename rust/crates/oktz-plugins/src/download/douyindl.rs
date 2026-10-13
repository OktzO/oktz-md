//! `plugins/download/douyindl.js` — unduh video/audio Douyin.
//!
//! `ponytail:` resolver berlapis (`src/lib/resolve.js`) belum di-port; yang
//! dipakai di sini backend agregator yang SUDAH jadi cadangan kedua di JS
//! (`nexrayWeb` → `/api/downloader/douyin`, `src/capabilities/douyin.js:51`).
//! Scraper lokal `snapvideotools` tidak dipindah — butuh scraping HTML.
//!
//! `ponytail:` `Reply` tidak punya variant video/audio, jadi caption yang
//! sudah dirakit tetap terkirim sebagai teks sementara media-nya tidak ikut.
//! Tambahkan
//! `Reply::Video`/`Reply::Audio` di `oktz-plugin-api` kalau pipeline sudah
//! bisa mengunggah keduanya.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

/// `nexrayWeb` di `src/lib/aggregator.js:24`.
pub(crate) const ENDPOINT: &str = "https://api.nexray.web.id/api/downloader/douyin";

/// `URLSearchParams.set` meng-encode sebagai
/// `application/x-www-form-urlencoded` — spasi jadi `+`, byte lain jadi `%XX`
/// uppercase. Dipakai juga `spotifydl` dan `instagramdl` di direktori ini.
pub(crate) fn query_encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(*b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Host Douyin yang sah — `douyindl.js:27-42`.
///
/// Guard host, bukan `includes`: `douyin.com.evil.example` dan `notdouyin.com`
/// lolos pemeriksaan substring; meneruskannya berarti satu request ke host yang
/// tidak sengaja PLUS satu kegagalan di breaker kapabilitas untuk semua orang.
pub(crate) fn host_douyin(url: &str) -> bool {
    let url = url.trim();
    let Some((scheme, rest)) = url.split_once("://") else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return false;
    }
    // `new URL()` reading hostAuthority: userinfo dibuang, port dibuang, dan
    // host berhenti di `/`, `?`, atau `#`.
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('@')
        .next()
        .unwrap_or("");
    let host = match authority.split_once(':') {
        Some((h, _port)) => h,
        None => authority,
    }
    .to_lowercase();
    host == "douyin.com"
        || host.ends_with(".douyin.com")
        || host == "iesdouyin.com"
        || host.ends_with(".iesdouyin.com")
}

/// `douyindl.js:49-55` — kartu penggunaan.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎵 *Douyin Downloader*\n\n\
         Download video atau audio dari Douyin (TikTok China).\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}douyindl <link>*\n\n\
         *CONTOH:*\n\
         > *{prefix}douyindl https://v.douyin.com/xxx*"
    )
}

/// `douyindl.js:62` — host ditolak.
pub(crate) const BAD_HOST: &str =
    "❌ URL tidak valid. Gunakan link dari douyin.com atau v.douyin.com";

/// `douyindl.js:91` — cabang `catch`.
pub(crate) const FAILED: &str = "❌ Gagal mengambil data Douyin, coba lagi nanti";

/// `urlMedia` (`src/capabilities/douyin.js:84`) — hanya `http(s)` yang lolos.
fn url_media(value: Option<&Value>) -> String {
    let s = value
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if s.starts_with("http://") || s.starts_with("https://") {
        s
    } else {
        String::new()
    }
}

/// Hasil `normalize` (`src/capabilities/douyin.js:89`) — `Err` = semua backend
/// menjawab tanpa URL video yang bisa diunduh.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DouyinData {
    pub title: String,
    pub platform: String,
    pub video: String,
    pub audio: String,
}

pub(crate) fn normalize(body: &Value) -> Result<DouyinData, ()> {
    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return Err(());
    }
    let result = body.get("result").ok_or(())?;
    let media = result
        .get("media")
        .and_then(Value::as_array)
        .ok_or(())?;
    let pick = |kind: &str| {
        media
            .iter()
            .find(|m| m.get("type").and_then(Value::as_str) == Some(kind))
            .map(|m| url_media(m.get("url")))
            .unwrap_or_default()
    };
    let video = pick("video");
    if video.is_empty() {
        return Err(());
    }
    Ok(DouyinData {
        title: result
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string(),
        platform: result
            .get("platform")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string(),
        video,
        audio: pick("audio"),
    })
}

/// `douyindl.js:73` — `data.platform || "Douyin"` dan `data.title || ""`.
pub(crate) fn caption(data: &DouyinData) -> String {
    let platform = if data.platform.is_empty() {
        "Douyin"
    } else {
        &data.platform
    };
    format!("🎵 *{platform}*\n\n{}", data.title)
}

#[plugin(
    name = "douyindl",
    alias = ["douyin", "dydl"],
    category = "download",
    description = "Download video/audio dari Douyin (TikTok China)",
    usage = ".douyindl <url>",
    example = ".douyindl https://v.douyin.com/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn douyindl(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    // Tanpa reaksi di sini, sama seperti guard plugin `sfiledl`/`videy`/
    // `pindl`: input salah bukan kegagalan proses, jadi tidak layak memakai
    // centang ❌ yang di command ini dipakai untuk "tidak ada argumen".
    if !host_douyin(text) {
        return Reply::text(BAD_HOST);
    }

    let url = format!("{ENDPOINT}?url={}", query_encode(text));
    let data = ctx
        .http
        .get(&url)
        .ok()
        .and_then(|r| r.json())
        .and_then(|b| normalize(&b).ok());

    match data {
        Some(data) => Reply::text(caption(&data)),
        None => Reply::text(FAILED).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
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
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
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
            command: "douyindl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("douyindl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn ok_body() -> Value {
        json!({
            "status": true,
            "result": {
                "title": "Contoh Video",
                "platform": "Douyin",
                "media": [
                    { "type": "audio", "url": "https://cdn.example/a.mp3" },
                    { "type": "video", "url": "https://cdn.example/v.mp4" }
                ]
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("douyindl").unwrap();
        assert_eq!(p.meta.name, "douyindl");
        assert_eq!(p.meta.alias, ["douyin", "dydl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(
            p.meta.description,
            "Download video/audio dari Douyin (TikTok China)"
        );
        assert_eq!(p.meta.usage, ".douyindl <url>");
        assert_eq!(p.meta.example, ".douyindl https://v.douyin.com/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["douyin", "dydl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "douyindl");
        }
    }

    #[test]
    fn only_douyin_hosts_pass_the_guard() {
        for good in [
            "https://v.douyin.com/xxx",
            "https://douyin.com/video/1",
            "https://www.douyin.com/video/1",
            "http://iesdouyin.com/share/video/1",
            "https://www.iesdouyin.com/share/video/1",
            "  https://v.douyin.com/xxx  ",
        ] {
            assert!(host_douyin(good), "harus lolos: {good}");
        }
        // Yang ini semuanya memuat "douyin.com" sebagai substring tapi host
        //-nya milik orang lain — persis kasus yang dijaga `hostDouyin`.
        for bad in [
            "https://douyin.com.evil.example/x",
            "https://notdouyin.com/x",
            "https://evil.example/?r=douyin.com",
            "ftp://v.douyin.com/x",
            "v.douyin.com/x",
            "https://iesdouyin.com.evil.example/x",
            "",
        ] {
            assert!(!host_douyin(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card_with_a_red_cross() {
        let http = Stub::body(ok_body());
        for text in ["", "   "] {
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
        }
        assert!(
            http.seen.lock().unwrap().is_empty(),
            "tanpa link tidak boleh ada request"
        );
    }

    #[test]
    fn a_foreign_host_is_rejected_without_any_reaction() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://douyin.com.evil.example/x");
        assert_eq!(text_of(&r), BAD_HOST);
        // `douyindl.js:58-60`: input salah bukan kegagalan proses, jadi tidak
        // ada reaksi sama sekali di cabang ini.
        assert!(
            !matches!(r.without_mutations(), Reply::Reaction { .. }),
            "guard host tidak boleh bereaksi"
        );
        assert!(
            http.seen.lock().unwrap().is_empty(),
            "host ditolak tidak boleh dihubungi"
        );
    }

    #[test]
    fn a_resolved_post_replies_with_the_platform_caption() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://v.douyin.com/xxx");
        assert_eq!(text_of(&r), "🎵 *Douyin*\n\nContoh Video");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0],
            "https://api.nexray.web.id/api/downloader/douyin?url=https%3A%2F%2Fv.douyin.com%2Fxxx"
        );
    }

    #[test]
    fn a_missing_platform_falls_back_to_douyin() {
        let body = json!({
            "status": true,
            "result": { "media": [{ "type": "video", "url": "https://cdn.example/v.mp4" }] }
        });
        let r = run(Stub::body(body), "https://v.douyin.com/x");
        assert_eq!(text_of(&r), "🎵 *Douyin*\n\n");
    }

    #[test]
    fn a_response_without_a_video_url_is_a_failure_not_a_green_tick() {
        // `normalize` menolak respons tanpa URL video (`capabilities/douyin.js:106`)
        // supaya plugin tidak mengirim apa-apa lalu tetap memberi centang hijau.
        for body in [
            json!({ "status": false, "msg": "gagal" }),
            json!({ "status": true, "result": { "media": [] } }),
            json!({ "status": true, "result": { "media": [{ "type": "video", "url": "ftp://x" }] } }),
            json!({ "status": true, "result": {} }),
            json!({}),
        ] {
            let r = run(Stub::body(body.clone()), "https://v.douyin.com/x");
            assert_eq!(text_of(&r), FAILED, "body {body}");
            assert!(
                matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "☢"),
                "kegagalan proses harus bereaksi ☢"
            );
        }
    }

    #[test]
    fn a_transport_failure_reports_the_same_message() {
        let r = run(Stub::offline(), "https://v.douyin.com/x");
        assert_eq!(text_of(&r), FAILED);
    }

    #[test]
    fn the_download_url_is_encoded_the_way_url_search_params_does_it() {
        assert_eq!(query_encode("https://v.douyin.com/x"), "https%3A%2F%2Fv.douyin.com%2Fx");
        assert_eq!(query_encode("a b"), "a+b");
        assert_eq!(query_encode("safe-._*"), "safe-._*", "unreserved tak di-encode");
    }
}