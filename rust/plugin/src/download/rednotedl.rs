//! `plugins/download/rednotedl.js` — unduh video/foto dari RedNote.
//!
//! Scraper `RedNoteDL` (`src/scraper/rednote.js`) sudah ada di crate
//! `oktz-scraper` (`oktz_scraper::rednote::red_note_dl`): satu POST JSON ke
//! `rednote.savevideodown.com` lengkap dengan header `X-Forwarded-For`,
//! `X-Real-IP`, dan `Client-IP` berisi IP acak, lalu pemetaan field. Plugin ini
//! hanya memetakan hasilnya ke balasan.
//!
//! Tiga penyimpangan yang tercatat apa adanya:
//!
//! - **Foto kedua dan seterusnya tidak dikirim.** JS mengirim maksimal 5 foto
//!   satu per satu (`rednotedl.js:48-52`); satu `Reply` hanya satu pesan, jadi
//!   hanya foto pertama yang ikut.
//! - **Catatan sisa foto pindah ke caption.** JS mengirimnya sebagai pesan
//!   terpisah setelah semua foto (`rednotedl.js:53-57`); di sini teks itu
//!   jadi caption foto pertama supaya tidak hilang.
//! - **Reaksi awal `🕕` dibuang** (`rednotedl.js:33`), mengikuti konvensi repo:
//!   `Reply::Reaction` hanya bisa satu, jadi reaksi terakhir yang ikut — `✅`
//!   setelah terkirim, `☢` di cabang gagal.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

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

/// Bentuk `status: true` dari `RedNoteDL` (`src/scraper/rednote.js:35-44`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Rednote {
    /// `title` tidak ikut keluar kalau `undefined`, jadi `None` = `undefined`.
    pub title: Option<String>,
    /// `type` — `"video"` berarti `results` cuma satu URL.
    pub kind: String,
    pub results: Vec<String>,
}

/// `String(v)` untuk nilai JSON — `${result.error}` (`rednotedl.js:40`) sama
/// saja mem-string-kan nilai apa adanya, bukan memaksa jadi teks. Kunci yang
/// tidak ada berarti `undefined`, yang berbeda dari `null`.
fn teks_js(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// Baca hasil scraper. `Err` = `status: false` dengan pesan error siap tampil.
pub(crate) fn hasil_dari(body: &Value) -> Result<Rednote, String> {
    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return Err(teks_js(body.get("error")));
    }
    Ok(Rednote {
        title: body
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string),
        kind: body
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        // `[directVideoUrl || downloadUrl]` bisa berisi `undefined`, dan
        // `directImages` bisa bukan array — keduanya jadi daftar kosong URL.
        results: body
            .get("results")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|i| i.as_str().unwrap_or_default().to_string())
                    .collect()
            })
            .unwrap_or_default(),
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

    let data = match oktz_scraper::rednote::red_note_dl(ctx.http.as_ref(), url) {
        // `RedNoteDL` tidak punya `try`, jadi kegagalan transport melempar dan
        // masuk `catch` (`rednotedl.js:61-65`).
        Err(_) => return Reply::text(TRANSPORT_FAILED).with_reaction("☢"),
        Ok(value) => match hasil_dari(&value) {
            Ok(data) => data,
            Err(error) => return Reply::text(rednote_failed(&error)).with_reaction("☢"),
        },
    };

    if data.kind == "video" {
        // `results[0]` bisa `undefined` walau array-nya panjang satu; di JS
        // `sendMedia(undefined)` melempar dan masuk `catch`.
        match data.results.first() {
            Some(video) if !video.is_empty() => {
                // `rednotedl.js:44` — video dengan caption `result.title`.
                return Reply::video(
                    MediaSource::url(video.clone()),
                    data.title.clone().unwrap_or_default(),
                )
                .with_reaction("✅");
            }
            _ => return Reply::text(TRANSPORT_FAILED).with_reaction("☢"),
        }
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
    use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const SENDER: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        balasan: Result<String, String>,
        seen: Mutex<Vec<HttpCall>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Err("offline".to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.clone());
            let text = self
                .balasan
                .clone()
                .map_err(|message| oktz_plugin_api::HttpError { message })?;
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
            command: "rednotedl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
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

    /// Isi balasan sebenarnya: `without_mutations` hanya membuang `Mutate`,
    /// jadi `Reaction { reply }` masih harus dibuka satu lapis.
    fn isi(r: &Reply) -> &Reply {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            lain => lain,
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
        let http = Stub::body(serde_json::json!({ "success": true }));
        for text in ["", "   "] {
            let r = run(http.clone(), text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaction_of(&r), Some("❌"), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_failed_response_reports_the_backend_message() {
        // Body di sini bentuk responses `rednote.savevideodown.com`, bukan
        // bentuk hasil scraper: `success` yang menentukan cabang gagal.
        let r = run(
            Stub::body(serde_json::json!({ "success": false, "error": "Postingan privat" })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(text_of(&r), "❌ *RedNote Gagal*\n\n> Postingan privat");
        assert_eq!(reaction_of(&r), Some("☢"));
        // `resData.error || "Gagal mengambil data"` juga kena error kosong.
        for body in [serde_json::json!({ "success": false }), serde_json::json!({ "success": false, "error": "" })] {
            let r = run(Stub::body(body.clone()), "https://www.xiaohongshu.com/x");
            assert_eq!(text_of(&r), "❌ *RedNote Gagal*\n\n> Gagal mengambil data");
            assert_eq!(reaction_of(&r), Some("☢"), "body {body}");
        }
    }

    #[test]
    fn the_failure_branch_prints_a_missing_error_verbatim() {
        // `rednotedl.js:40` memakai `${result.error}`; scraper selalu mengisi
        // error dari `resData.error || "Gagal mengambil data"`, jadi di sini
        // yang bisa hilang adalah seluruh kuncinya — tercetak `undefined`.
        for body in [serde_json::json!({ "status": false }), serde_json::json!({})] {
            assert_eq!(
                hasil_dari(&body).unwrap_err(),
                "undefined",
                "body {body}"
            );
        }
        // Nilai non-teks ikut ter-string-kan seperti `String(v)` di JS.
        assert_eq!(
            hasil_dari(&serde_json::json!({ "status": false, "error": 7 })).unwrap_err(),
            "7"
        );
    }

    #[test]
    fn a_video_result_is_sent_as_a_video_carrying_the_post_title() {
        let r = run(
            Stub::body(serde_json::json!({
                "success": true,
                "type": "video",
                "title": "Catatan Umum",
                "directVideoUrl": "https://cdn.example/v.mp4"
            })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(reaction_of(&r), Some("✅"));
        match isi(&r) {
            Reply::Video { media, caption, .. } => {
                assert_eq!(caption, "Catatan Umum");
                assert_eq!(
                    media,
                    &Some(oktz_plugin_api::MediaSource::Url {
                        url: "https://cdn.example/v.mp4".to_string(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
        // `directVideoUrl || downloadUrl` — tanpa yang pertama, yang kedua dipakai.
        let r = run(
            Stub::body(serde_json::json!({
                "success": true,
                "type": "video",
                "directVideoUrl": "",
                "downloadUrl": "https://cdn.example/fallback.mp4"
            })),
            "https://www.xiaohongshu.com/x",
        );
        match isi(&r) {
            Reply::Video { media, .. } => assert_eq!(
                media,
                &Some(oktz_plugin_api::MediaSource::Url {
                    url: "https://cdn.example/fallback.mp4".to_string(),
                    mimetype: None,
                })
            ),
            lain => panic!("harus video, dapat {lain:?}"),
        }
    }

    #[test]
    fn a_video_without_any_url_falls_into_the_catch_branch() {
        // `results` tetap panjang satu walau nilainya `undefined` (`rednote.js:42`),
        // jadi `result.results?.[0]` falsy dan `sendMedia` melempar di JS.
        for body in [
            serde_json::json!({ "success": true, "type": "video", "title": "x" }),
            serde_json::json!({ "success": true, "type": "video", "title": "x", "directVideoUrl": null, "downloadUrl": null }),
        ] {
            let r = run(Stub::body(body.clone()), "https://www.xiaohongshu.com/x");
            assert_eq!(text_of(&r), TRANSPORT_FAILED, "body {body}");
            assert_eq!(reaction_of(&r), Some("☢"), "body {body}");
        }
    }

    #[test]
    fn the_video_title_may_be_absent_and_then_the_caption_is_empty() {
        let r = run(
            Stub::body(serde_json::json!({
                "success": true,
                "type": "video",
                "downloadUrl": "https://cdn.example/v.mp4"
            })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(text_of(&r), "");
    }

    #[test]
    fn hasil_dari_takes_the_single_results_entry_of_a_video_post() {
        // `directVideoUrl || downloadUrl` sudah diselesaikan di dalam scraper.
        let data = hasil_dari(&serde_json::json!({
            "status": true,
            "type": "video",
            "results": ["https://cdn.example/fallback.mp4"]
        }))
        .unwrap();
        assert_eq!(data.results, ["https://cdn.example/fallback.mp4"]);
        assert_eq!(data.title, None);
    }

    #[test]
    fn up_to_five_photos_are_sent_and_the_surplus_is_announced() {
        for count in 1..=5usize {
            let urls: Vec<String> = (0..count).map(|i| format!("https://cdn.example/{i}.jpg")).collect();
            let r = run(
                Stub::body(serde_json::json!({ "success": true, "type": "image", "directImages": urls })),
                "https://www.xiaohongshu.com/x",
            );
            assert_eq!(text_of(&r), "", "jumlah {count} tidak ada catatan sisa");
            assert_eq!(reaction_of(&r), Some("✅"));
        }

        let urls: Vec<String> = (0..8).map(|i| format!("https://cdn.example/{i}.jpg")).collect();
        let r = run(
            Stub::body(serde_json::json!({ "success": true, "type": "image", "directImages": urls })),
            "https://www.xiaohongshu.com/x",
        );
        assert_eq!(text_of(&r), "_Masih ada 3 foto lagi, maksimal 5_");
    }

    #[test]
    fn the_first_photo_is_the_one_that_is_sent() {
        let r = run(
            Stub::body(serde_json::json!({
                "success": true,
                "type": "image",
                "directImages": ["https://cdn.example/0.jpg", "https://cdn.example/1.jpg"]
            })),
            "https://www.xiaohongshu.com/x",
        );
        match isi(&r) {
            Reply::Image { media, .. } => assert_eq!(
                media,
                &Some(oktz_plugin_api::MediaSource::Url {
                    url: "https://cdn.example/0.jpg".to_string(),
                    mimetype: None,
                })
            ),
            lain => panic!("harus image, dapat {lain:?}"),
        }
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
            serde_json::json!({ "success": true, "type": "image", "directImages": [] }),
            serde_json::json!({ "success": true, "type": "image" }),
            serde_json::json!({ "success": true }),
        ] {
            let r = run(Stub::body(body.clone()), "https://www.xiaohongshu.com/x");
            assert!(r.without_mutations().text_body().is_none(), "body {body}");
            assert_eq!(reaction_of(&r), Some("✅"), "body {body}");
        }
    }

    #[test]
    fn the_url_is_sent_as_a_json_body_field() {
        let http = Stub::body(serde_json::json!({ "status": true, "type": "image", "results": [] }));
        let _ = run(http.clone(), "https://www.xiaohongshu.com/x");
        let seen = http.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            seen[0].url,
            "https://rednote.savevideodown.com/api/download"
        );
        assert_eq!(
            seen[0].body.as_ref().unwrap().bytes(),
            br#"{"url":"https://www.xiaohongshu.com/x"}"#
        );
    }

    #[test]
    fn a_transport_failure_reports_the_catch_message() {
        let r = run(Stub::offline(), "https://www.xiaohongshu.com/x");
        assert_eq!(text_of(&r), TRANSPORT_FAILED);
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        let r = run(
            Stub::body(serde_json::json!({ "status": true, "type": "image", "results": ["u"] })),
            "https://www.xiaohongshu.com/x",
        );
        assert!(r.mutations().is_empty());
    }
}