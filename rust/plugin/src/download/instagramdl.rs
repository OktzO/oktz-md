//! `plugins/download/instagramdl.js` — unduh video/foto Instagram.
//!
//! Scraper `instagramDownloader` (`src/scraper/ig.js`) sudah ada di crate
//! `oktz-scraper` (`oktz_scraper::ig::instagram_download`), termasuk pemanggilan
//! `igdl` dari paket npm `btch-downloader`. Plugin ini hanya memetakan
//! hasilnya: guard URL, pemisahan caption, dan pemilihan video-vs-image.
//!
//! Tiga penyimpangan yang tercatat apa adanya:
//!
//! - **Item kedua dan seterusnya tidak dikirim.** JS mengirim satu pesan per
//!   item media (`instagramdl.js:59-74`); satu `Reply` hanya satu pesan, jadi
//!   hanya item pertama — yang juga satu-satunya yang boleh membawa caption —
//!   yang ikut. Untuk unggahan multi-foto sisanya hilang.
//! - **Thumbnail tidak dikirim.** Scraper menyediakan `thumbnail`, tapi JS pun
//!   tidak pernah memakainya di plugin ini.
//! - **Reaksi awal `🕕` dibuang** (`instagramdl.js:39`), mengikuti konvensi
//!   repo: `Reply::Reaction` hanya bisa satu, jadi reaksi terakhir yang ikut —
//!   `✅` setelah terkirim, `❌` di cabang gagal.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

/// `IG_REGEX` (`instagramdl.js:18`) tanpa regex crate: `instagram.com/`
/// diikuti salah satu segmen `p|reel|reels|stories|tv` lalu `/`.
pub(crate) fn is_instagram_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find("instagram.com/") {
        let rest = &lower[from + i + "instagram.com/".len()..];
        // Slash penutup wajib: regex JS `\/(p|…)\/` menolak `…/p` tanpa slash.
        let Some(seg_end) = rest.find('/') else {
            return false;
        };
        if matches!(&rest[..seg_end], "p" | "reel" | "reels" | "stories" | "tv") {
            return true;
        }
        from += i + "instagram.com/".len() + seg_end;
    }
    false
}

/// `instagramdl.js:25-29` — kartu penggunaan.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📸 *ɪɴsᴛᴀɢʀᴀᴍ ᴅᴏᴡɴʟᴏᴀᴅᴇʀ*\n\n\
         > `{prefix}igdl <url>`\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > `{prefix}igdl https://www.instagram.com/reel/xxx`\n\
         > `{prefix}igdl https://www.instagram.com/p/xxx`"
    )
}

/// `instagramdl.js:35` — URL bukan Instagram.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link Instagram (reel/post/story).";

/// `instagramdl.js:46` — `result.media` kosong.
pub(crate) const NO_MEDIA: &str = "❌ Gagal mengambil media. Coba link lain.";

/// `instagramdl.js:79` — cabang `catch`.
pub(crate) fn failed(message: &str) -> String {
    format!("❌ *ɢᴀɢᴀʟ ᴍᴇɴɢᴜɴᴅᴜʂ*\n\n> {message}")
}

/// Satu item media setelah pemetaan `src/scraper/ig.js:23-33`.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Item {
    pub is_video: bool,
    pub url: String,
}

/// Bentuk hasil `instagramDownloader` (`src/scraper/ig.js:41-49`) — hanya
/// field yang dibaca plugin.
#[derive(Debug, Default)]
pub(crate) struct Download {
    pub username: String,
    pub caption: String,
    pub media: Vec<Item>,
}

/// `instagramdl.js:50-57` — caption. Item kedua dan seterusnya dikirim tanpa
/// caption (`caption = ""` setelah tiap pengiriman).
pub(crate) fn build_caption(url: &str, download: &Download) -> String {
    let is_story = url.contains("/stories/");
    let head = if is_story {
        "📸 *Instagram Story*\n\n"
    } else {
        "📸 *Instagram Downloader*\n\n"
    };
    let mut caption = String::from(head);
    if !download.username.is_empty() && download.username != "-" {
        caption.push_str(&format!("👤 *Author*: @{}\n", download.username));
    }
    if !download.caption.is_empty() {
        caption.push_str(&format!("📝 *Caption*:\n{}\n", download.caption));
    }
    caption.trim().to_string()
}

/// Baca hasil scraper (`src/scraper/ig.js:41-49`). Field yang tidak ada berarti
/// string kosong di JS juga, karena scraper selalu mengisinya.
fn hasil_dari(data: &Value) -> Download {
    Download {
        username: data
            .get("username")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        caption: data
            .get("caption")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        media: data
            .get("media")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .map(|item| Item {
                        // `instagramdl.js:60` — `video` atau `mp4` jadi video.
                        is_video: matches!(
                            item.get("type").and_then(Value::as_str),
                            Some("video") | Some("mp4")
                        ),
                        url: item
                            .get("url")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

#[plugin(
    name = "instagramdl",
    alias = ["igdl", "ig", "instagram"],
    category = "download",
    description = "Download video/foto Instagram",
    usage = ".instagramdl <url>",
    example = ".instagramdl https://www.instagram.com/reel/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn instagramdl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_instagram_url(url) {
        return Reply::text(BAD_URL);
    }

    let download = match oktz_scraper::ig::instagram_download(ctx.http.as_ref(), url) {
        Ok(value) => hasil_dari(&value),
        // `instagramDownloader` melempar `Gagal mengambil media dari Instagram`
        // dua kali — termasuk saat `media` kosong, jadi cabang `NO_MEDIA` di
        // bawah praktis tidak terjangkau (`ig.js:20` dan `:36`).
        Err(e) => return Reply::text(failed(&e.message)).with_reaction("❌"),
    };

    if download.media.is_empty() {
        return Reply::text(NO_MEDIA).with_reaction("❌");
    }

    let caption = build_caption(url, &download);
    let first = &download.media[0];
    // `instagramdl.js:60-73`: `video`/`mp4` → video, sisanya → image.
    let balasan = if first.is_video {
        Reply::video(MediaSource::url(first.url.clone()), caption)
    } else {
        Reply::image(MediaSource::url(first.url.clone()), caption)
    };
    balasan.with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const SENDER: &str = "628111@s.whatsapp.net";

    /// Stub `HttpGet` yang membalas JSON tetap — scraper dan plugin diuji
    /// tanpa jaringan.
    #[derive(Debug, Default)]
    struct StubHttp {
        dipanggil: Mutex<Vec<String>>,
        balasan: String,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.dipanggil.lock().unwrap().push(req.url.clone());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(balasan: &str) -> Arc<StubHttp> {
        Arc::new(StubHttp {
            dipanggil: Mutex::new(Vec::new()),
            balasan: balasan.to_string(),
        })
    }

    fn run(text: &str) -> Reply {
        (oktz_plugin_api::find("instagramdl").unwrap().handler)(ctx_http(stub("[]"), text))
    }

    fn ctx_http(http: Arc<StubHttp>, text: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "igdl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        }
    }

    fn ctx_dengan(text: &str, balasan: &str) -> Ctx {
        ctx_http(stub(balasan), text)
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

    fn download(items: &[(&str, bool)], username: &str, caption: &str) -> Download {
        Download {
            username: username.to_string(),
            caption: caption.to_string(),
            media: items
                .iter()
                .map(|(url, is_video)| Item {
                    is_video: *is_video,
                    url: url.to_string(),
                })
                .collect(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("instagramdl").unwrap();
        assert_eq!(p.meta.name, "instagramdl");
        assert_eq!(p.meta.alias, ["igdl", "ig", "instagram"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video/foto Instagram");
        assert_eq!(p.meta.usage, ".instagramdl <url>");
        assert_eq!(p.meta.example, ".instagramdl https://www.instagram.com/reel/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["igdl", "ig", "instagram"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "instagramdl"
            );
        }
    }

    #[test]
    fn only_the_five_post_paths_are_accepted() {
        for good in [
            "https://www.instagram.com/p/abc",
            "https://instagram.com/reel/abc",
            "https://www.instagram.com/reels/abc",
            "https://www.instagram.com/stories/user/1",
            "https://www.instagram.com/tv/abc",
            "https://www.INSTAGRAM.COM/P/abc",
        ] {
            assert!(is_instagram_url(good), "harus lolos: {good}");
        }
        for bad in [
            "https://www.instagram.com/user/abc",
            "https://www.instagram.com/",
            "https://www.instagram.com/p",
            "https://www.instagram.com/reel",
            "",
        ] {
            assert!(!is_instagram_url(bad), "harus ditolak: {bad}");
        }
    }

    #[test]
    fn the_unanchored_regex_quirk_of_the_js_is_kept() {
        // `IG_REGEX` tidak dikunci ke host: `p/` di akhir URL sudah cukup, dan
        // host apa pun yang memuat `instagram.com/` ikut lolos. `douyindl`
        // sengaja mengetatkan guard host-nya; di sini Guardian ini harus
        // persis sama dengan JS, jadi kedua kasus ini tetap diterima.
        assert!(is_instagram_url("https://instagram.com/p/"));
        assert!(is_instagram_url("https://notinstagram.com/p/abc"));
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card() {
        for text in ["", "  "] {
            assert_eq!(text_of(&run(text)), help("."));
        }
    }

    #[test]
    fn a_non_instagram_url_is_rejected() {
        assert_eq!(text_of(&run("https://youtube.com/watch?v=x")), BAD_URL);
    }

    #[test]
    fn a_story_url_gets_its_own_heading() {
        let d = download(&[("https://x/a.jpg", false)], "budi", "");
        assert!(
            build_caption("https://www.instagram.com/stories/u/1", &d)
                .starts_with("📸 *Instagram Story*")
        );
        assert!(
            build_caption("https://www.instagram.com/p/a", &d)
                .starts_with("📸 *Instagram Downloader*")
        );
    }

    #[test]
    fn the_author_line_is_skipped_for_the_placeholder_username() {
        let d = download(&[("https://x/a.jpg", false)], "-", "");
        assert!(!build_caption("https://www.instagram.com/p/a", &d).contains("Author"));

        let d = download(&[("https://x/a.jpg", false)], "budi", "");
        assert!(
            build_caption("https://www.instagram.com/p/a", &d)
                .contains("👤 *Author*: @budi")
        );
    }

    #[test]
    fn the_caption_block_only_appears_when_there_is_one() {
        let kosong = build_caption(
            "https://www.instagram.com/p/a",
            &download(&[("https://x/a.jpg", false)], "-", ""),
        );
        assert!(!kosong.contains("Caption"), "{kosong}");
        assert!(!kosong.contains('\n'), "tanpa isi tidak boleh sisa baris: {kosong}");

        let ada = build_caption(
            "https://www.instagram.com/p/a",
            &download(&[("https://x/a.jpg", false)], "-", "liburan"),
        );
        assert!(ada.contains("📝 *Caption*:\nliburan"), "{ada}");
    }

    #[test]
    fn the_scraper_failure_branch_is_the_error_template() {
        // Backend menjawab array kosong → `igdl` mengembalikan `{status:false}`
        // lalu `instagramDownloader` melempar, persis yang terjadi kalau
        // `btch-downloader` gagal di Node.
        let r = run("https://www.instagram.com/p/abc");
        assert_eq!(
            text_of(&r),
            "❌ *ɢᴀɢᴀʟ ᴍᴇɴɢᴜɴᴅᴜʂ*\n\n> Gagal mengambil media dari Instagram"
        );
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "❌"),
            "kegagalan harus bereaksi ❌"
        );
    }

    #[test]
    fn a_photo_post_is_sent_as_an_image_with_the_caption() {
        let balasan = r#"[{"url":"https://cdn.example/a.jpg","thumbnail":"https://cdn.example/t.jpg"}]"#;
        let http = stub(balasan);
        let r = (oktz_plugin_api::find("instagramdl").unwrap().handler)(ctx_http(
            http.clone(),
            "https://www.instagram.com/p/abc",
        ));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "✅"),
            "sukses harus bereaksi ✅"
        );
        match isi(&r) {
            Reply::Image { media, caption, .. } => {
                assert_eq!(caption, "📸 *Instagram Downloader*");
                assert_eq!(
                    media,
                    &Some(oktz_plugin_api::MediaSource::Url {
                        url: "https://cdn.example/a.jpg".to_string(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
        // Scraper benar-benar dipanggil dengan URL yang diketik user.
        let dipanggil = http.dipanggil.lock().unwrap();
        assert_eq!(dipanggil.len(), 1);
        assert!(
            dipanggil[0].starts_with("https://backend1.tioo.eu.org/igdl?url="),
            "endpoint tak terduga: {dipanggil:?}"
        );
        assert!(dipanggil[0].ends_with("https%3A%2F%2Fwww.instagram.com%2Fp%2Fabc"));
    }

    #[test]
    fn a_video_post_is_sent_as_a_video_with_the_caption() {
        let balasan = r#"[{"url":"https://cdn.example/a.mp4","thumbnail":""}]"#;
        let r = (oktz_plugin_api::find("instagramdl").unwrap().handler)(ctx_dengan(
            "https://www.instagram.com/reel/abc",
            balasan,
        ));
        match isi(&r) {
            Reply::Video {
                media,
                caption,
                gif_playback,
                ..
            } => {
                assert_eq!(caption, "📸 *Instagram Downloader*");
                assert!(!gif_playback);
                assert_eq!(
                    media,
                    &Some(oktz_plugin_api::MediaSource::Url {
                        url: "https://cdn.example/a.mp4".to_string(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
    }

    #[test]
    fn the_placeholder_username_keeps_the_author_line_out_of_the_caption() {
        // `igdl` tidak mengembalikan nama penulis, jadi scraper selalu mengisi
        // `"-"` dan baris `👤 *Author*` tidak pernah ikut.
        let balasan = r#"[{"url":"https://cdn.example/a.jpg"}]"#;
        let r = (oktz_plugin_api::find("instagramdl").unwrap().handler)(ctx_dengan(
            "https://www.instagram.com/p/abc",
            balasan,
        ));
        assert_eq!(text_of(&r), "📸 *Instagram Downloader*");
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        let balasan = r#"[{"url":"https://cdn.example/a.jpg"}]"#;
        let r = (oktz_plugin_api::find("instagramdl").unwrap().handler)(ctx_dengan(
            "https://www.instagram.com/p/abc",
            balasan,
        ));
        assert!(r.mutations().is_empty());
    }
}