//! `plugins/download/likeedl.js` — unduh video Likee.
//!
//! Scraper `likee` (`src/scraper/likee.js`) sudah ada di crate `oktz-scraper`
//! (`oktz_scraper::likee::likee`): satu POST form ke `likeedownloader.com`
//! lengkap dengan header `User-Agent`, `Referer`, `origin`, dan
//! `x-requested-with`, lalu empat regex kecil. Plugin ini hanya memetakan
//! hasilnya — logika scraper tidak diduplikasi di sini.
//!
//! Dua penyimpangan yang tercatat apa adanya:
//!
//! - **`contextInfo` forwarding tidak ikut** (`likeedl.js:52-55`); `Reply` tidak
//!   punya field forwarding.
//! - **Reaksi terakhir yang dipakai, sesuai tiap cabang JS.** `Reply::Reaction`
//!   hanya bisa satu, jadi `🕕` (`likeedl.js:35`) ikut sebagai reaksi terakhir di
//!   cabang gagal dan `✅` (`:58`) di cabang sukses.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;


/// `likeedl.js:31` — `/likee\.(video|com)/i`, tidak dikunci ke host.
pub(crate) fn is_likee_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.contains("likee.video") || lower.contains("likee.com")
}

/// `likeedl.js:23-28` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}lkdl <url>`\n\n\
         > Contoh:\n\
         > `{prefix}lkdl https://likee.video/@xxx`"
    )
}

/// `likeedl.js:32` — URL bukan Likee.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link Likee.";

/// `likeedl.js:47` — kedua field watermark kosong.
pub(crate) const NO_VIDEO: &str = "❌ Video tidak ditemukan.";

// `likeedl.js:41` (`!data`) tidak punya padanan di sini: `likee()` hanya
// melempar atau mengembalikan objek, jadi cabang itu tidak pernah menyala dan
// teksnya tidak ikut dipindah.

/// Bagian hasil `likee` (`src/scraper/likee.js:29-34`) yang dibaca plugin.
/// Regex yang tidak cocok mengisi `null` di scraper, yang oleh `||` di
/// `likeedl.js:44` diperlakukan sama dengan string kosong.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Likee {
    pub without_watermark: String,
    pub with_watermark: String,
}

impl Likee {
    /// `likeedl.js:44` — `without_watermark || with_watermark`.
    pub(crate) fn video_url(&self) -> &str {
        if self.without_watermark.is_empty() {
            &self.with_watermark
        } else {
            &self.without_watermark
        }
    }
}

/// Baca hasil scraper.
fn hasil_dari(data: &Value) -> Likee {
    let teks = |k: &str| {
        data.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Likee {
        without_watermark: teks("without_watermark"),
        with_watermark: teks("with_watermark"),
    }
}

#[plugin(
    name = "likeedl",
    alias = ["lkdl", "likee", "lk"],
    category = "download",
    description = "Download video Likee",
    usage = ".lkdl <url>",
    example = ".lkdl https://likee.video/@xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn likeedl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_likee_url(url) {
        return Reply::text(BAD_URL);
    }

    // `src/scraper/likee.js:22` melempar `Fetch failed` kalau `template`
    // kosong, dan `catch` di plugin ikut membawa `te(...)` dengan reaksi jam
    // yang sudah menyala sejak baris 35.
    let Ok(value) = oktz_scraper::likee::likee(ctx.http.as_ref(), url) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("🕕");
    };

    let data = hasil_dari(&value);
    let video = data.video_url();
    if video.is_empty() {
        return Reply::text(NO_VIDEO).with_reaction("🕕");
    }

    // `likeedl.js:50-58` — `sendMedia` video tanpa caption, lalu centang hijau.
    Reply::video(MediaSource::url(video), "").with_reaction("✅")
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
        /// Balasan backend dengan template HTML Likee.
        fn template(html: &str) -> Arc<Stub> {
            let body = serde_json::json!({ "template": html }).to_string();
            Arc::new(Stub {
                balasan: Ok(body),
                seen: Mutex::new(Vec::new()),
            })
        }
        /// Balasan JSON bebas.
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
            unreachable!("scraper ini mengirim form")
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
            command: "lkdl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("likeedl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
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
        let p = oktz_plugin_api::find("likeedl").unwrap();
        assert_eq!(p.meta.name, "likeedl");
        assert_eq!(p.meta.alias, ["lkdl", "likee", "lk"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video Likee");
        assert_eq!(p.meta.usage, ".lkdl <url>");
        assert_eq!(p.meta.example, ".lkdl https://likee.video/@xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["lkdl", "likee", "lk"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "likeedl");
        }
    }

    #[test]
    fn the_guard_accepts_both_likee_domains() {
        for good in [
            "https://likee.video/@xxx",
            "https://www.likee.com/@xxx",
            "https://LIKEE.VIDEO/@x",
        ] {
            assert!(is_likee_url(good), "harus lolos: {good}");
        }
        for bad in ["https://youtube.com/watch?v=x", "", "likee video"] {
            assert!(!is_likee_url(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card_without_any_request() {
        let http = Stub::template("");
        for text in ["", "   "] {
            assert_eq!(text_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_foreign_host_is_rejected_without_any_reaction() {
        let http = Stub::template("");
        let r = run(http.clone(), "https://youtube.com/watch?v=x");
        assert_eq!(text_of(&r), BAD_URL);
        assert!(
            !matches!(r.without_mutations(), Reply::Reaction { .. }),
            "guard host tidak boleh bereaksi"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_form_body_is_encoded_the_way_url_search_params_does_it() {
        let http = Stub::template("");
        let _ = run(http.clone(), "https://likee.video/@x");
        let seen = http.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].url, "https://likeedownloader.com/process");
        assert_eq!(
            seen[0].body,
            Some(HttpBody::Form(
                "id=https%3A%2F%2Flikee.video%2F%40x&locale=en".to_string()
            )),
            "spasi jadi `+` dan `@` jadi `%40`, gaya URLSearchParams"
        );
    }

    #[test]
    fn the_watermark_free_url_wins_over_the_watermarked_one() {
        let parsed = hasil_dari(&serde_json::json!({
            "with_watermark": "https://cdn/w.mp4",
            "without_watermark": "https://cdn/n.mp4"
        }));
        assert_eq!(parsed.with_watermark, "https://cdn/w.mp4");
        assert_eq!(parsed.without_watermark, "https://cdn/n.mp4");
        assert_eq!(parsed.video_url(), "https://cdn/n.mp4");

        // Regex `without_watermark` yang tidak cocok mengisi `null`, jadi
        // `||` jatuh ke `with_watermark`.
        let only_marked = hasil_dari(&serde_json::json!({
            "with_watermark": "https://cdn/w.mp4",
            "without_watermark": null
        }));
        assert_eq!(only_marked.video_url(), "https://cdn/w.mp4");

        assert_eq!(Likee::default().video_url(), "");
    }

    #[test]
    fn a_resolved_video_is_sent_as_a_video_with_a_green_tick() {
        let html = r#"<a id="without_watermark" href="https://cdn/n.mp4">n</a>"#;
        let r = run(Stub::template(html), "https://likee.video/@x");
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "✅"),
            "sukses harus bereaksi ✅"
        );
        match isi(&r) {
            Reply::Video {
                media,
                caption,
                gif_playback,
                ..
            } => {
                assert_eq!(caption, "", "sendMedia dikirim tanpa caption");
                assert!(!gif_playback);
                assert_eq!(
                    media,
                    &Some(oktz_plugin_api::MediaSource::Url {
                        url: "https://cdn/n.mp4".to_string(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
    }

    #[test]
    fn a_template_without_any_link_gets_the_no_video_message() {
        let r = run(Stub::template("<p>Tidak ada</p>"), "https://likee.video/@x");
        assert_eq!(text_of(&r), NO_VIDEO);
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "gagal tanpa video harus bereaksi 🕕"
        );
    }

    #[test]
    fn a_response_without_a_template_falls_into_the_error_template() {
        for body in [
            serde_json::json!({}),
            serde_json::json!({ "template": null }),
            serde_json::json!({ "template": "" }),
        ] {
            let r = run(Stub::body(body.clone()), "https://likee.video/@x");
            assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "lkdl"), "body {body}");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "https://likee.video/@x");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "lkdl"));
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        let html = r#"<a id="without_watermark" href="https://cdn/n.mp4">n</a>"#;
        let r = run(Stub::template(html), "https://likee.video/@x");
        assert!(r.mutations().is_empty());
    }
}