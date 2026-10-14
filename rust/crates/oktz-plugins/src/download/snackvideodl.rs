//! `plugins/download/snackvideodl.js` — unduh video SnackVideo.
//!
//! `ponytail:` scraper `snackvideo` datang dari paket npm `btch-downloader`,
//! bukan dari repo ini, jadi tidak bisa di-port. Guard URL, kartu pakai, dan
//! semua teks cabangnya tetap sama dengan JS; `fetch_snackvideo` selalu gagal,
//! persis seperti scraper itu melempar di Node.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMedia` hanya
//! terkirim sebagai URL teks.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::fun::tod::error_template;

/// `snackvideodl.js:31` — `/snackvideo\.com/i`, tidak dikunci ke host.
pub(crate) fn is_snackvideo_url(url: &str) -> bool {
    url.to_lowercase().contains("snackvideo.com")
}

/// `snackvideodl.js:23-28` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}svdl <url>`\n\n\
         > Contoh:\n\
         > `{prefix}svdl https://www.snackvideo.com/@xxx/video/xxx`"
    )
}

/// `snackvideodl.js:32` — URL bukan SnackVideo.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link SnackVideo.";

/// `snackvideodl.js:41` — backend menjawab tanpa `status`/`result.videoUrl`.
pub(crate) const NO_VIDEO: &str = "❌ Gagal mengambil video. Coba link lain.";

/// `snackvideodl.js:40` — `data?.status && data?.result?.videoUrl`.
pub(crate) fn accepted(data: &Value) -> bool {
    data.get("status").is_some_and(Value::as_bool == Some(true))
        && data
            .get("result")
            .and_then(|r| r.get("videoUrl"))
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
}

/// `snackvideodl.js:46` — `result.videoUrl`.
pub(crate) fn video_url(data: &Value) -> String {
    data.get("result")
        .and_then(|r| r.get("videoUrl"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// `snackvideo(url)` dari `btch-downloader` — satu-satunya bagian yang tidak
/// bisa di-port. `Err` = lempar, jadi cabang `catch` yang menyala.
fn fetch_snackvideo(_url: &str) -> Result<Value, ()> {
    Err(())
}

#[plugin(
    name = "snackvideodl",
    alias = ["svdl", "snackvideo"],
    category = "download",
    description = "Download video SnackVideo",
    usage = ".svdl <url>",
    example = ".svdl https://www.snackvideo.com/@xxx/video/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn snackvideodl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_snackvideo_url(url) {
        return Reply::text(BAD_URL);
    }

    let data = match fetch_snackvideo(url) {
        Ok(data) => data,
        Err(()) => {
            return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("🕕");
        }
    };

    if !accepted(&data) {
        return Reply::text(NO_VIDEO).with_reaction("🕕");
    }

    // `snackvideodl.js:46-52` — `sendMedia` video; tidak ada padanannya di
    // `Reply`.
    Reply::text(video_url(&data)).with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "svdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            ..Default::default()
        };
        (oktz_plugin_api::find("snackvideodl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("snackvideodl").unwrap();
        assert_eq!(p.meta.name, "snackvideodl");
        assert_eq!(p.meta.alias, ["svdl", "snackvideo"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video SnackVideo");
        assert_eq!(p.meta.usage, ".svdl <url>");
        assert_eq!(
            p.meta.example,
            ".svdl https://www.snackvideo.com/@xxx/video/xxx"
        );
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["svdl", "snackvideo"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "snackvideodl"
            );
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card() {
        for text in ["", "   "] {
            assert_eq!(text_of(&run(text)), help("."), "input {text:?}");
        }
    }

    #[test]
    fn a_url_without_the_domain_is_rejected() {
        assert_eq!(text_of(&run("https://youtube.com/watch?v=x")), BAD_URL);
    }

    #[test]
    fn the_unanchored_regex_quirk_of_the_js_is_kept() {
        assert!(is_snackvideo_url("https://snackvideo.com.evil.example/x"));
        assert!(is_snackvideo_url("https://notsnackvideo.com/x"));
        assert!(is_snackvideo_url("https://www.SnackVideo.com/@a/video/1"));
    }

    #[test]
    fn a_response_without_video_url_is_refused() {
        for data in [
            json!({ "status": false, "result": { "videoUrl": "https://cdn/v.mp4" } }),
            json!({ "status": true }),
            json!({ "status": true, "result": {} }),
            json!({ "status": true, "result": { "videoUrl": "" } }),
            json!({ "status": true, "result": { "videoUrl": null } }),
            json!({}),
        ] {
            assert!(!accepted(&data), "{data} seharusnya ditolak");
            assert_eq!(video_url(&data), "", "{data}");
        }
    }

    #[test]
    fn the_scraper_failure_branch_is_the_error_template() {
        let r = run("https://www.snackvideo.com/@a/video/1");
        assert_eq!(text_of(&r), error_template(".", "svdl"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "reaksi jam harus tetap ada di cabang catch"
        );
    }
}