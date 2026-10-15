//! `plugins/download/likeedl.js` — unduh video Likee.
//!
//! Scraper `likee` (`src/scraper/likee.js`) ikut dipindah: satu POST form ke
//! `likeedownloader.com` lalu empat regex kecil, tanpa parser HTML.
//!
//! `ponytail:` scraper mengirim `User-Agent`, `Referer`, `origin`, dan
//! `x-requested-with`, tapi trait `HttpGet` tidak punya kontrol header, jadi
//! header itu hilang. Kalau Likee mulai menolak request, tambahkan dukungan
//! header di `HttpGet` — jangan mengganti scraping-nya.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMedia` hanya
//! terkirim sebagai URL teks.

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};

use super::douyindl::query_encode;
use crate::fun::tod::error_template;

/// `likee.js:7` — endpoint scraper.
pub(crate) const ENDPOINT: &str = "https://likeedownloader.com/process";

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

// `likeedl.js:41` (`!data`) tidak punya padanan di sini: `likee()` di
// `src/scraper/likee.js` hanya melempar atau mengembalikan objek, jadi cabang
// itu tidak pernah menyala dan teksnya tidak ikut dipindah.

/// Bagian hasil `likee` (`src/scraper/likee.js:29-34`) yang dibaca plugin.
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

/// `pick(/with_watermark[^>]+href="([^"]+)"/)` (`src/scraper/likee.js:24-27`)
/// tanpa regex crate. `[^>]+` itu serakah: dari `needle` diambil potongan
/// sampai `>` pertama, lalu `href="` yang TERAKHIR di potongan itu yang
/// dipakai — persis seperti backtracking regex.
pub(crate) fn href_after(html: &str, needle: &str) -> String {
    let mut from = 0usize;
    while let Some(i) = html.get(from..).and_then(|rest| rest.find(needle)) {
        let after = from + i + needle.len();
        if let Some(gt) = html.get(after..).and_then(|rest| rest.find('>')) {
            let span = &html[after..after + gt];
            if let Some(href) = span.rfind("href=\"").and_then(|h| span.get(h + 6..))
                && let Some(end) = href.find('"')
            {
                return href[..end].to_string();
            }
        }
        from = after;
    }
    String::new()
}

/// `src/scraper/likee.js:29-34` — dua regex yang dibutuhkan plugin.
pub(crate) fn parse_template(html: &str) -> Likee {
    Likee {
        without_watermark: href_after(html, "without_watermark"),
        with_watermark: href_after(html, "with_watermark"),
    }
}

/// `likee(url)` — `Err` = lempar, jadi cabang `catch` plugin yang menyala.
pub(crate) fn fetch_likee(url: &str, http: &dyn HttpGet) -> Result<Likee, ()> {
    // `new URLSearchParams({ id: url, locale: 'en' })` — spasi jadi `+`.
    let form = format!("id={}&locale=en", query_encode(url));
    let body = http.post_form(ENDPOINT, &form).map_err(|_| ())?.json().ok_or(())?;
    // `res.data && res.data.template ? res.data.template : null` — string
    // kosong itu falsy, jadi ikut jatuh ke lemparan `Fetch failed`.
    let template = body
        .get("template")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or(())?;
    Ok(parse_template(template))
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

    // `src/scraper/likee.js:22` melempar `Fetch failed` kalau `template` kosong.
    let Ok(data) = fetch_likee(url, ctx.http.as_ref()) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("🕕");
    };

    let video = data.video_url();
    if video.is_empty() {
        return Reply::text(NO_VIDEO).with_reaction("🕕");
    }

    // `likeedl.js:50-58` — `sendMedia` video lalu centang hijau.
    Reply::text(video.to_string()).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn template(html: &str) -> Arc<Stub> {
            let body = serde_json::json!({ "template": html }).to_string();
            Arc::new(Stub {
                body: Ok(body),
                seen: Mutex::new(Vec::new()),
            })
        }
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
            Err(HttpError {
                message: format!("cuma POST: {url}"),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST form")
        }
        fn post_form(&self, url: &str, form: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(format!("{url}::{form}"));
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
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST form")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "lkdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
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
        assert_eq!(
            http.seen.lock().unwrap().clone(),
            vec![
                "https://likeedownloader.com/process\
                 ::id=https%3A%2F%2Flikee.video%2F%40x&locale=en"
                    .to_string()
            ]
        );
    }

    #[test]
    fn the_href_picker_takes_the_last_one_before_the_closing_bracket() {
        let html = r#"<a class="without_watermark" href="https://cdn/a.mp4" data-x="1">x</a>"#;
        assert_eq!(href_after(html, "without_watermark"), "https://cdn/a.mp4");

        // Dua `href` sebelum `>`: yang diambil yang terakhir, seperti regex
        // serakah `[^>]+`.
        let greed = r#"without_watermark x href="a" href="b">"#;
        assert_eq!(href_after(greed, "without_watermark"), "b");

        // `[^>]+` minimal satu karakter: `without_watermark"` tanpa attribute
        // tidak cocok.
        assert_eq!(href_after("without_watermark>", "without_watermark"), "");
        assert_eq!(href_after("without_watermark", "without_watermark"), "");
    }

    #[test]
    fn the_watermark_free_url_wins_over_the_watermarked_one() {
        let html = concat!(
            r#"<a id="with_watermark" href="https://cdn/w.mp4">w</a>"#,
            r#"<a id="without_watermark" href="https://cdn/n.mp4">n</a>"#,
        );
        let parsed = parse_template(html);
        assert_eq!(parsed.with_watermark, "https://cdn/w.mp4");
        assert_eq!(parsed.without_watermark, "https://cdn/n.mp4");
        assert_eq!(parsed.video_url(), "https://cdn/n.mp4");

        let only_marked = Likee {
            without_watermark: String::new(),
            with_watermark: "https://cdn/w.mp4".into(),
        };
        assert_eq!(only_marked.video_url(), "https://cdn/w.mp4");

        assert_eq!(Likee::default().video_url(), "");
    }

    #[test]
    fn a_resolved_video_replies_with_the_url_and_a_green_tick() {
        let html = r#"<a id="without_watermark" href="https://cdn/n.mp4">n</a>"#;
        let r = run(Stub::template(html), "https://likee.video/@x");
        assert_eq!(text_of(&r), "https://cdn/n.mp4");
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "✅"),
            "sukses harus bereaksi ✅"
        );
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
            assert_eq!(text_of(&r), error_template(".", "lkdl"), "body {body}");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "https://likee.video/@x");
        assert_eq!(text_of(&r), error_template(".", "lkdl"));
    }
}