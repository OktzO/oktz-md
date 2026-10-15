//! `plugins/download/ytmp4.js` — unduh video YouTube.
//!
//! `ponytail:` resolver berlapis (`src/lib/resolve.js`) belum di-port; yang
//! dipakai di sini backend agregator yang sudah jadi tier kedua di JS (`izuka`
//! → `/api/downloader/ytmp4`, `src/capabilities/youtube.js:257`). `izuka`
//! punya `key: null` di `AGGREGATORS`, jadi tidak butuh `config.APIkey.*`.
//!
//! `ponytail:` kanonikalisasi `kanonikDari(idVideo(url))` di dalam resolver
//! tidak ikut — aggregator menerima URL persis seperti plugin JS
//! mengirimnya.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi file videonya tidak ikut
//! terkirim. JS tidak punya caption maupun nama berkas di jalur ini
//! (`sock.sendMedia(..., null, m, { type: "video" })`), jadi yang tersisa memang
//! hanya reaksi — sama seperti `douyindl.rs` untuk video.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use super::douyindl::query_encode;
use super::ytmp3::{host_youtube, is_http_url, js_to_string, url_field, url_nama};

/// `AGGREGATORS.izuka.base` + path di `src/capabilities/youtube.js:257`.
pub(crate) const ENDPOINT: &str = "https://my.izuka-api.xyz/api/downloader/ytmp4";

/// `ytmp4.js:50` — link hilang.
pub(crate) fn contoh(prefix: &str) -> String {
    format!("Contoh: {prefix}ytmp4 https://youtube.com/watch?v=xxx")
}

/// `ytmp4.js:51` — bukan host YouTube.
pub(crate) const BUKAN_YOUTUBE: &str = "❌ URL harus YouTube";

/// `ytmp4.js:71` — cabang `catch`.
pub(crate) const GAGAL: &str = "Gagal mengunduh video.";

/// Cabang `video_normal` `urlUnduhan` (`youtube.js:317`): hanya `ext: "mp4"`,
/// kualitas tertinggi lebih dulu, dan `sort` JS stabil jadi kualitas seri
/// butuh yang pertama.
fn url_video_normal(isi: &Value) -> String {
    let Some(daftar) = isi.get("video_normal").and_then(Value::as_array) else {
        return String::new();
    };
    let mut terpilih: Option<&Value> = None;
    let mut kualitas_terpilih: i64 = 0;
    for item in daftar {
        if item.get("ext").and_then(Value::as_str) != Some("mp4") {
            continue;
        }
        let kualitas = item
            .get("quality")
            .map(js_to_string)
            .and_then(|raw| crate::common::parse_int(&raw))
            .unwrap_or(0);
        if terpilih.is_none() || kualitas > kualitas_terpilih {
            terpilih = Some(item);
            kualitas_terpilih = kualitas;
        }
    }
    let kandidat = terpilih
        .and_then(|item| item.get("url"))
        .map(js_to_string)
        .unwrap_or_default();
    let kandidat = kandidat.trim().to_string();
    if is_http_url(&kandidat) {
        kandidat
    } else {
        String::new()
    }
}

/// `normalize` (`youtube.js:331`) untuk jalur video. `format` dihasilkan JS
/// tapi tidak dipakai `ytmp4`, jadi yang dikembalikan hanya URL-nya.
pub(crate) fn normalize(isi: &Value) -> Result<String, ()> {
    let mut url = url_nama(isi);
    if url.is_empty() {
        url = url_video_normal(isi);
    }
    if url.is_empty() {
        url = url_field(isi);
    }
    if url.is_empty() {
        return Err(());
    }
    Ok(url)
}

/// `pastikanBentuk` + `lewatIzuka` (`youtube.js:238,257`) digabung: pemeriksaan
/// bentuk aggregator diletakkan di sini juga karena `izuka` tidak butuh key.
fn unduh_video(ctx: &Ctx, url: &str) -> Result<String, ()> {
    let target = format!("{ENDPOINT}?url={}", query_encode(url));
    let respons = ctx.http.get(&target).map_err(|_| ())?;
    let body = respons.json().ok_or(())?;
    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return Err(());
    }
    let result = body.get("result").filter(|r| r.is_object()).ok_or(())?;
    let ada = match result.get("video_normal") {
        Some(Value::Array(daftar)) => !daftar.is_empty(),
        Some(lain) => crate::common::js_truthy_value(lain),
        None => false,
    };
    if !ada {
        return Err(());
    }
    normalize(result)
}

#[plugin(
    name = "ytmp4",
    alias = ["youtubemp4", "ytvideo"],
    category = "download",
    description = "Download video YouTube",
    usage = ".ytmp4 <url>",
    example = ".ytmp4 https://youtube.com/watch?v=xxx",
    cooldown = 20,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ytmp4(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(contoh(&ctx.prefix));
    }
    if !host_youtube(url) {
        return Reply::text(BUKAN_YOUTUBE);
    }

    match unduh_video(&ctx, url) {
        // JS `ytmp4.js:61-63`: hanya `sendMedia` tanpa caption/nama berkas,
        // jadi sisa satu-satunya setelah media hilang adalah centang hijau.
        Ok(_download) => Reply::None.with_reaction("✅"),
        Err(()) => Reply::text(GAGAL).with_reaction("❌"),
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
        body: Result<Value, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Ok(v),
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
                .map_err(|()| HttpError { message: "offline".into() })?
                .to_string();
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
            command: "ytmp4".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("ytmp4").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn ok_body() -> Value {
        json!({
            "status": true,
            "result": {
                "title": "Video Contoh",
                "video_normal": [
                    { "ext": "webm", "quality": "720", "url": "https://cdn.example/v.webm" },
                    { "ext": "mp4", "quality": "360", "url": "https://cdn.example/v-360.mp4" },
                    { "ext": "mp4", "quality": "1080", "url": "https://cdn.example/v-1080.mp4" }
                ]
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ytmp4").unwrap();
        assert_eq!(p.meta.name, "ytmp4");
        assert_eq!(p.meta.alias, ["youtubemp4", "ytvideo"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video YouTube");
        assert_eq!(p.meta.usage, ".ytmp4 <url>");
        assert_eq!(p.meta.example, ".ytmp4 https://youtube.com/watch?v=xxx");
        assert_eq!(p.meta.cooldown, 20);
        assert_eq!(p.meta.energi, 2);
        // JS `ytmp4.js:3-12` tidak menyebut satu pun flag permission.
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["youtubemp4", "ytvideo"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "ytmp4");
        }
    }

    #[test]
    fn the_mp4_endpoint_is_asked_for_and_the_highest_quality_wins() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://youtube.com/watch?v=xxx");
        assert_eq!(reaksi(&r), Some("✅"));
        assert!(
            r.text_body().is_none(),
            "JS tidak punya caption maupun nama berkas di jalur ini"
        );
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0],
            "https://my.izuka-api.xyz/api/downloader/ytmp4\
             ?url=https%3A%2F%2Fyoutube.com%2Fwatch%3Fv%3Dxxx"
        );
        // `ext: "mp4"` + kualitas tertinggi — bukan entri pertama.
        let body = ok_body();
        let hasil = normalize(body.get("result").unwrap()).unwrap();
        assert_eq!(hasil, "https://cdn.example/v-1080.mp4");
    }

    #[test]
    fn ties_keep_the_first_entry_like_a_stable_javascript_sort() {
        let isi = json!({
            "video_normal": [
                { "ext": "mp4", "quality": "720", "url": "https://a/1.mp4" },
                { "ext": "mp4", "quality": "720", "url": "https://a/2.mp4" }
            ]
        });
        assert_eq!(normalize(&isi).unwrap(), "https://a/1.mp4");
    }

    #[test]
    fn a_missing_link_shows_the_example_without_reacting() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), "Contoh: .ytmp4 https://youtube.com/watch?v=xxx");
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_foreign_host_is_rejected_without_any_request() {
        let http = Stub::body(ok_body());
        let r = run(http.clone(), "https://notyoutube.com/x");
        assert_eq!(text_of(&r), BUKAN_YOUTUBE);
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_response_without_video_normal_reports_the_failure_message() {
        for body in [
            json!({ "status": false, "msg": "gagal" }),
            json!({ "status": true, "result": { "video_normal": [] } }),
            json!({ "status": true, "result": {} }),
            // Hanya webm: kontrak host tetap `ext: "mp4"`.
            json!({ "status": true, "result": { "video_normal": [
                { "ext": "webm", "quality": "720", "url": "https://cdn.example/v.webm" }
            ] } }),
            json!({ "status": true }),
        ] {
            let r = run(Stub::body(body.clone()), "https://youtube.com/watch?v=xxx");
            assert_eq!(text_of(&r), GAGAL, "body {body}");
            assert_eq!(reaksi(&r), Some("❌"));
        }
    }

    #[test]
    fn a_transport_failure_reports_the_same_message() {
        let r = run(Stub::offline(), "https://youtube.com/watch?v=xxx");
        assert_eq!(text_of(&r), GAGAL);
        assert_eq!(reaksi(&r), Some("❌"));
    }
}
