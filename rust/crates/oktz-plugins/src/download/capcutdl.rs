//! `plugins/download/capcutdl.js` — unduh video CapCut.
//!
//! `ponytail:` scraper `capcut` datang dari paket npm `btch-downloader`, bukan
//! dari repo ini, jadi tidak bisa di-port. Guard URL, kartu pakai, dan semua
//! teks cabangnya tetap sama dengan JS; yang hilang cuma pemanggilan scraper —
//! `fetch_capcut` selalu gagal, persis seperti scraper itu melempar di Node.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMedia` hanya
//! terkirim sebagai URL teks. Tambahkan `Reply::Video` kalau pipeline sudah
//! bisa mengunggah video.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;


/// `capcutdl.js:31` — `/capcut\.com/i`, tidak dikunci ke host.
pub(crate) fn is_capcut_url(url: &str) -> bool {
    url.to_lowercase().contains("capcut.com")
}

/// `capcutdl.js:23-28` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}ccdl <url>`\n\n\
         > Contoh:\n\
         > `{prefix}ccdl https://www.capcut.com/t/xxx`"
    )
}

/// `capcutdl.js:32` — URL bukan CapCut.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link CapCut.";

/// `capcutdl.js:41` — backend menjawab tanpa `originalVideoUrl`.
pub(crate) const NO_VIDEO: &str = "❌ Gagal mengambil video. Coba link lain.";

/// `capcutdl.js:40` — `data?.status` harus `true`.
pub(crate) fn status_ok(data: &Value) -> bool {
    data.get("status").and_then(Value::as_bool) == Some(true)
}

/// `capcutdl.js:40` — `data?.originalVideoUrl`.
pub(crate) fn original_video_url(data: &Value) -> String {
    data.get("originalVideoUrl")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// `capcut(url)` dari `btch-downloader` — satu-satunya bagian yang tidak bisa
/// di-port. `Err` = lempar, jadi cabang `catch` yang menyala.
fn fetch_capcut(_url: &str) -> Result<Value, ()> {
    Err(())
}

#[plugin(
    name = "capcutdl",
    alias = ["ccdl", "capcut", "cc"],
    category = "download",
    description = "Download video CapCut",
    usage = ".ccdl <url>",
    example = ".ccdl https://www.capcut.com/t/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn capcutdl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_capcut_url(url) {
        return Reply::text(BAD_URL);
    }

    let data = match fetch_capcut(url) {
        Ok(data) => data,
        Err(()) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("🕕");
        }
    };

    let video = original_video_url(&data);
    if !status_ok(&data) || video.is_empty() {
        return Reply::text(NO_VIDEO).with_reaction("🕕");
    }

    // `capcutdl.js:44-50` — `sendMedia` video; tidak ada padanannya di `Reply`.
    Reply::text(video).with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ccdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            ..Default::default()
        };
        (oktz_plugin_api::find("capcutdl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("capcutdl").unwrap();
        assert_eq!(p.meta.name, "capcutdl");
        assert_eq!(p.meta.alias, ["ccdl", "capcut", "cc"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video CapCut");
        assert_eq!(p.meta.usage, ".ccdl <url>");
        assert_eq!(p.meta.example, ".ccdl https://www.capcut.com/t/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        // `cc` diklaim dua kali di JS: `download/capcutdl.js:5` (alias) dan
        // `group/clearchat.js:3` (array `name`). Tabrakan bawaan repo, bukan
        // hasil port — hanya satu yang bisa menang di registry.
        for a in ["ccdl", "capcut"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "capcutdl");
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
        assert_eq!(text_of(&run("capcut co m")), BAD_URL);
    }

    #[test]
    fn the_unanchored_regex_quirk_of_the_js_is_kept() {
        // `/capcut\.com/i` tidak dikunci ke host, jadi `notcapcut.com` dan
        // `capcut.com.evil.example` tetap lolos — Guardian ini harus persis
        // sama dengan JS.
        assert!(is_capcut_url("https://capcut.com.evil.example/x"));
        assert!(is_capcut_url("https://notcapcut.com/x"));
        assert!(is_capcut_url("https://www.CAPCUT.com/t/1"));
    }

    #[test]
    fn a_response_without_the_video_field_is_the_no_video_message() {
        for data in [
            serde_json::json!({ "status": false, "originalVideoUrl": "https://cdn.example/v.mp4" }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "originalVideoUrl": "" }),
            serde_json::json!({ "status": true, "originalVideoUrl": null }),
            serde_json::json!({}),
        ] {
            let video = original_video_url(&data);
            assert!(!status_ok(&data) || video.is_empty(), "{data} lolos");
        }
    }

    #[test]
    fn the_scraper_failure_branch_is_the_error_template() {
        // `fetch_capcut` selalu gagal di port ini, jadi handler tidak pernah
        // mencapai cabang media — persis yang terjadi kalau scraper melempar.
        let r = run("https://www.capcut.com/t/xxx");
        assert_eq!(text_of(&r), crate::common::error_template(".", "ccdl"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "reaksi jam harus tetap ada di cabang catch"
        );
    }
}