//! `plugins/search/playtiktok.js` — cari dan kirim satu video TikTok terbaik.
//!
//! Pakai scraper `oktz_scraper::tiktoksearch::tiktok_search_video` (hasil port
//! `src/scraper/tiktoksearch.js`), bukan menyalin logika scraper.
//!
//! `m.react("🔍")` lalu `m.react("✅")` — `Reply` hanya menampung satu reaksi,
//! jadi yang dipegang adalah yang terakhir (`✅`).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

/// Teks balasan saat query kosong (`playtiktok.js:31-33`).
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎵 *PLAY TIKTOK*\n\n> Contoh:\n`{prefix}playtiktok cewe tiktok`"
    )
}

/// `formatNumber(n)` (`playtiktok.js:20-25`).
///
/// `Number(n) || 0`: non-angka → 0. `>= 1e6` → `x.x M`, `>= 1e3` → `x.x K`,
// sisanya angka bulat apa adanya. Rust `format!("{:.1}")` dan JS `toFixed(1)`
// keduanya pakai pembulatan ke-even pada representasi desimal.
pub(crate) fn format_number(value: f64) -> String {
    if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}K", value / 1_000.0)
    } else {
        value.to_string()
    }
}

/// `stats`-nya boleh berupa angka atau string (mis. `"1.2M"` sudah dibulatkan
/// scraper jadi angka, tapi tetap jaga aman).
pub(crate) fn angka_stats(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).unwrap_or(0.0)
}

#[plugin(
    name = "playtiktok",
    alias = ["ttplay", "tiktokplay"],
    category = "search",
    description = "Cari dan kirim satu video TikTok terbaik",
    usage = ".playtiktok <query>",
    example = ".playtiktok cewe tiktok",
    cooldown = 15,
    energi = 1,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn playtiktok(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim().to_string();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let hasil = match oktz_scraper::tiktoksearch::tiktok_search_video(&*ctx.http, &query) {
        Ok(v) => v,
        Err(_) => {
            return Reply::text(format!("❌ Tidak ditemukan video untuk: {query}"))
                .with_reaction("❌");
        }
    };

    let items = match hasil.get("result").and_then(Value::as_array) {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            return Reply::text(format!("❌ Tidak ditemukan video untuk: {query}"))
                .with_reaction("❌");
        }
    };

    let video = &items[0];
    let title = video
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("-");
    let author = video
        .get("author")
        .and_then(|a| a.get("nickname"))
        .and_then(Value::as_str)
        .unwrap_or("-");
    let stats = video.get("stats").and_then(Value::as_object);
    let plays = angka_stats(stats.and_then(|s| s.get("plays")));
    let likes = angka_stats(stats.and_then(|s| s.get("likes")));
    let comments = angka_stats(stats.and_then(|s| s.get("comments")));
    let shares = angka_stats(stats.and_then(|s| s.get("shares")));
    let music = video.get("music").and_then(Value::as_str).unwrap_or("-");
    let link = video.get("link").and_then(Value::as_str).unwrap_or("");

    let caption = format!(
        "🎵 *PLAY TIKTOK*\n\n\
         📌 *Judul:* {title}\n\
         👤 *Author:* {author}\n\
         👀 *Views:* {}\n\
         ❤️ *Likes:* {}\n\
         💬 *Comments:* {}\n\
         🔁 *Shares:* {}\n\
         🎧 *Music:* {music}\n\
         🔗 *Link:* {link}",
        format_number(plays),
        format_number(likes),
        format_number(comments),
        format_number(shares)
    );

    Reply::video(MediaSource::url(link), caption).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
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
            _: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "playtiktok".into(),
            http,
            ..Ctx::default()
        };
        playtiktok(ctx)
    }

    fn contoh() -> Value {
        serde_json::json!({
            "status": true,
            "result": [
                {
                    "title": "Video Lucu",
                    "author": { "nickname": "Budi", "fullname": "Budi Santoso" },
                    "stats": {
                        "plays": 1_200_000,
                        "likes": 34_000,
                        "comment": 1_200,
                        "share": 450
                    },
                    "music": "Lagu Pop",
                    "link": "https://www.tiktok.com/@budi/video/1"
                }
            ]
        })
    }

    fn video_url(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Video { media, .. } => media.as_ref().and_then(|m| match m {
                MediaSource::Url { url, .. } => Some(url.clone()),
                _ => None,
            }),
            _ => None,
        }
    }

    fn caption_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r.without_mutations();
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji.as_str()),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn query_kosong_menghasilkan_help_tanpa_reaksi() {
        let r = run(Stub::offline(), "");
        assert_eq!(caption_of(&r), help("."));
        assert!(reaction_of(&r).is_none());
    }

    #[test]
    fn hasil_berhasil_mengirim_video_dengan_caption() {
        let r = run(Stub::body(contoh()), "cewe tiktok");
        assert_eq!(video_url(&r), Some("https://www.tiktok.com/@budi/video/1".into()));
        assert!(caption_of(&r).contains("*Judul:* Video Lucu"));
        assert!(caption_of(&r).contains("*Author:* Budi"));
        assert!(caption_of(&r).contains("👀 *Views:* 1.2M"));
        assert!(caption_of(&r).contains("❤️ *Likes:* 34K"));
        assert!(caption_of(&r).contains("💬 *Comments:* 1.2K"));
        assert!(caption_of(&r).contains("🔁 *Shares:* 450"));
        assert!(caption_of(&r).contains("🎧 *Music:* Lagu Pop"));
        assert!(caption_of(&r).contains("🔗 *Link:* https://www.tiktok.com/@budi/video/1"));
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn result_kosong_atau_tidak_ada() {
        for body in [
            serde_json::json!({ "status": true, "result": [] }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": false, "message": "gagal" }),
            Value::Null,
        ] {
            let r = run(Stub::body(body.clone()), "x");
            assert_eq!(
                caption_of(&r),
                "❌ Tidak ditemukan video untuk: x",
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn gagal_jaringan_menampilkan_tidak_ditemukan() {
        let r = run(Stub::offline(), "x");
        assert_eq!(
            caption_of(&r),
            "❌ Tidak ditemukan video untuk: x"
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn format_number_mengikuti_js() {
        assert_eq!(format_number(1_200_000.0), "1.2M");
        assert_eq!(format_number(34_000.0), "34K");
        assert_eq!(format_number(450.0), "450");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(999.0), "999");
        assert_eq!(format_number(1_000.0), "1.0K");
        assert_eq!(format_number(1_000_000.0), "1.0M");
    }
}
