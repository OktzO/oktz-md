//! `plugins/download/instagramdl.js` — unduh video/foto Instagram.
//!
//! `ponytail:` JS memakai `instagramDownloader` dari `src/scraper/ig.js`, yang
//! memanggil `igdl` dari paket npm `btch-downloader` — modul JS, tidak bisa
//! di-port dan tidak ada backend lain di repo ini untuk Instagram. Yang
//! dipindah di sini SEMUA logika plugin: guard URL, pemisahan Caption, dan
//! pemilihan video-vs-image per item. Pemanggilan scraper menjadi satu fungsi
//! `fetch_media` yang selalu gagal di port ini; sisa alurnya tetap utuh dan
//! cabang "gagal" tetap bisa diuji.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

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

/// `fetchMedia` — satu-satunya bagian yang tidak bisa di-port.
///
/// Ganti dengan pemanggilan scraper sungguhan (atau backend HTTP) di sini;
/// semua teks, caption, dan pemilihan tipe media di file ini tetap berlaku.
fn fetch_media(_url: &str) -> Result<Download, String> {
    Err("scraper Instagram (btch-downloader) belum di-port".to_string())
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

    let download = match fetch_media(url) {
        Ok(d) if !d.media.is_empty() => d,
        Ok(_) => return Reply::text(NO_MEDIA).with_reaction("❌"),
        Err(message) => return Reply::text(failed(&message)).with_reaction("❌"),
    };

    let caption = build_caption(url, &download);
    let first = &download.media[0];
    // `instagramdl.js:60-73`: `video`/`mp4` → video, sisanya → image.
    // `Reply` hanya punya `Image` dan `Sticker`, jadi video tidak punya
    // padanan dan ikut hilang; teks caption tetap dikirim.
    if first.is_video {
        return Reply::text(caption);
    }
    Reply::image(MediaSource::url(first.url.clone()), caption)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "igdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            ..Default::default()
        };
        (oktz_plugin_api::find("instagramdl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
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
            assert_eq!(text_of(&run(text.clone())), help("."));
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
        // `fetch_media` selalu gagal di port ini, jadi handler tidak pernah
        // mencapai cabang media — persis yang akan terjadi kalau scraper
        // `btch-downloader` melempar di JS.
        let r = run("https://www.instagram.com/p/abc");
        assert!(text_of(&r).starts_with("❌ *ɢᴀɢᴀʟ ᴍᴇɴɢᴜɴᴅᴜʂ*"), "{}", text_of(&r));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "❌"),
            "kegagalan harus bereaksi ❌"
        );
    }
}