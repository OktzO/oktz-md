//! `plugins/download/cocofundl.js` — unduh video CocoFun.
//!
//! `ponytail:` scraper `cocofun` datang dari paket npm `btch-downloader`, bukan
//! dari repo ini, jadi tidak bisa di-port. Guard URL, kartu pakai, dan semua
//! teks cabangnya tetap sama dengan JS; `fetch_cocofun` selalu gagal, persis
//! seperti scraper itu melempar di Node.
//!
//! `ponytail:` `Reply` tidak punya variant video, jadi hasil `sendMedia` hanya
//! terkirim sebagai URL teks.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::fun::tod::error_template;

/// `cocofundl.js:31` — `/cocofun\.com/i`, tidak dikunci ke host.
pub(crate) fn is_cocofun_url(url: &str) -> bool {
    url.to_lowercase().contains("cocofun.com")
}

/// `cocofundl.js:23-28` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}cfdl <url>`\n\n\
         > Contoh:\n\
         > `{prefix}cfdl https://www.cocofun.com/share/post/xxx`"
    )
}

/// `cocofundl.js:32` — URL bukan CocoFun.
pub(crate) const BAD_URL: &str = "❌ URL tidak valid. Gunakan link CocoFun.";

/// `cocofundl.js:41` — backend menjawab tanpa `status`/`result`.
pub(crate) const NO_RESULT: &str = "❌ Gagal mengambil video. Coba link lain.";

/// `cocofundl.js:48` — `result` ada tapi kedua field URL kosong.
pub(crate) const NO_VIDEO: &str = "❌ Video tidak ditemukan.";

/// `cocofundl.js:40` — `data?.status` harus `true` dan `data?.result` harus ada.
pub(crate) fn accepted(data: &Value) -> bool {
    data.get("status").and_then(Value::as_bool) == Some(true)
        && data.get("result").is_some_and(|r| !r.is_null())
}

/// `cocofundl.js:45` — `result.no_watermark || result.watermark`.
pub(crate) fn video_url(data: &Value) -> String {
    let result = data.get("result").unwrap_or(&Value::Null);
    result
        .get("no_watermark")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| result.get("watermark").and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

/// `cocofun(url)` dari `btch-downloader` — satu-satunya bagian yang tidak bisa
/// di-port. `Err` = lempar, jadi cabang `catch` yang menyala.
fn fetch_cocofun(_url: &str) -> Result<Value, ()> {
    Err(())
}

#[plugin(
    name = "cocofundl",
    alias = ["cfdl", "cocofun"],
    category = "download",
    description = "Download video CocoFun",
    usage = ".cfdl <url>",
    example = ".cfdl https://www.cocofun.com/share/post/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cocofundl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_cocofun_url(url) {
        return Reply::text(BAD_URL);
    }

    let data = match fetch_cocofun(url) {
        Ok(data) => data,
        Err(()) => {
            return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("🕕");
        }
    };

    if !accepted(&data) {
        return Reply::text(NO_RESULT).with_reaction("🕕");
    }

    let video = video_url(&data);
    if video.is_empty() {
        return Reply::text(NO_VIDEO).with_reaction("🕕");
    }

    // `cocofundl.js:51-57` — `sendMedia` video; tidak ada padanannya di `Reply`.
    Reply::text(video).with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cfdl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: vec![text.into()],
            ..Default::default()
        };
        (oktz_plugin_api::find("cocofundl").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cocofundl").unwrap();
        assert_eq!(p.meta.name, "cocofundl");
        assert_eq!(p.meta.alias, ["cfdl", "cocofun"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video CocoFun");
        assert_eq!(p.meta.usage, ".cfdl <url>");
        assert_eq!(p.meta.example, ".cfdl https://www.cocofun.com/share/post/xxx");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["cfdl", "cocofun"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "cocofundl");
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
        assert!(is_cocofun_url("https://cocofun.com.evil.example/x"));
        assert!(is_cocofun_url("https://notcocofun.com/x"));
        assert!(is_cocofun_url("https://www.CocoFun.com/share/post/1"));
    }

    #[test]
    fn the_two_failure_messages_are_told_apart() {
        // Tanpa `result` → "Gagal mengambil video"; `result` ada tapi URL-nya
        // kosong → "Video tidak ditemukan".
        for data in [
            json!({ "status": false, "result": { "no_watermark": "https://cdn/v.mp4" } }),
            json!({ "status": true }),
            json!({ "status": true, "result": null }),
            json!({}),
        ] {
            assert!(!accepted(&data), "{data} seharusnya ditolak");
        }
        assert_eq!(video_url(&json!({})), "");

        for result in [
            json!({ "no_watermark": "", "watermark": "" }),
            json!({}),
            json!({ "no_watermark": null, "watermark": null }),
        ] {
            let data = json!({ "status": true, "result": result });
            assert!(accepted(&data));
            assert_eq!(video_url(&data), "", "{data}");
        }
    }

    #[test]
    fn the_watermarked_url_is_only_the_second_choice() {
        assert_eq!(
            video_url(&json!({
                "status": true,
                "result": { "no_watermark": "https://cdn/a.mp4", "watermark": "https://cdn/b.mp4" }
            })),
            "https://cdn/a.mp4"
        );
        assert_eq!(
            video_url(&json!({
                "status": true,
                "result": { "no_watermark": "", "watermark": "https://cdn/b.mp4" }
            })),
            "https://cdn/b.mp4"
        );
    }

    #[test]
    fn the_scraper_failure_branch_is_the_error_template() {
        let r = run("https://www.cocofun.com/share/post/xxx");
        assert_eq!(text_of(&r), error_template(".", "cfdl"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "reaksi jam harus tetap ada di cabang catch"
        );
    }
}