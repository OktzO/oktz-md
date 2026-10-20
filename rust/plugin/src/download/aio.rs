//! `plugins/download/aio.js` — all-in-one downloader lewat `oktz_scraper::aio`.
//! Ponytail: `sendMedia(..., quoted: m)` tidak ada padanannya pada variant
//! media `Reply`; quoted diabaikan, hanya caption yang dikirim.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📥 *ᴀʟʟ ɪɴ ᴏɴᴇ ᴅᴏᴡɴʟᴏᴀᴅᴇʀ*\n\n> Download dari berbagai platform!\n\n╭┈┈⬡「 🌐 *ᴘʟᴀᴛꜰᴏʀᴍ* 」\n┃ • Instagram\n┃ • TikTok\n┃ • Facebook\n┃ • Twitter/X\n┃ • YouTube\n┃ • Pinterest\n┃ • CapCut\n┃ • Threads / Reddit\n╰┈┈┈┈┈┈┈┈⬡\n\n> *Contoh:* {prefix}aio https://instagram.com/p/xxx",
        prefix = prefix
    )
}

const INVALID_URL: &str = "❌ URL tidak valid! Harus dimulai dengan http/https";
const NO_MEDIA: &str = "❌ Gagal mengambil media. Pastikan URL valid.";

#[plugin(
    name = "aio",
    alias = ["allinone", "download", "dl"],
    category = "download",
    description = "All in one downloader (IG, TikTok, FB, Twitter, YouTube, Pinterest, CapCut, dll)",
    usage = ".aio <url>",
    example = ".aio https://instagram.com/p/xxx",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn aio(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    if !url.starts_with("http") {
        return Reply::text(INVALID_URL);
    }

    let hasil = oktz_scraper::aio::aiodl(&*ctx.http, url);
    let Ok(result) = hasil else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let Some(media) = result.get("media").and_then(Value::as_array) else {
        return Reply::text(NO_MEDIA).with_reaction("❌");
    };
    let Some(item) = media.first() else {
        return Reply::text(NO_MEDIA).with_reaction("❌");
    };
    let tipe = item.get("type").and_then(Value::as_str).unwrap_or("");
    let url_media = item.get("url").and_then(Value::as_str).unwrap_or("");
    let judul = result.get("title").and_then(Value::as_str).unwrap_or("");

    let media_source = MediaSource::url(url_media);
    let reply = match tipe {
        "video" => Reply::video(media_source, judul),
        "audio" => Reply::audio(media_source),
        _ => Reply::image(media_source, judul),
    };
    reply.with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        result: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                result: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                result: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self.result.clone().map_err(|()| oktz_plugin_api::HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes { data: Arc::from(text.into_bytes()), mimetype: None },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> { unimplemented!() }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "aio".into(),
            http,
            ..Ctx::default()
        };
        aio(ctx)
    }

    fn text_of(r: &Reply) -> String { r.without_mutations().text_body().unwrap_or_default().to_string() }
    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() { Reply::Reaction { emoji, .. } => Some(emoji.clone()), _ => None }
    }

    #[test]
    fn url_kosong_tampilkan_bantuan_tanpa_request() {
        let http = Stub::body(serde_json::json!({}));
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_bukan_http_ditolak() {
        let http = Stub::body(serde_json::json!({}));
        let r = run(http.clone(), "belum-ada-protokol");
        assert_eq!(text_of(&r), INVALID_URL);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn platform_tidak_dikenali_diatasi_error_template_tanpa_request() {
        // aiodl melempar sebelum berjaringan, handler tangkap semua error.
        let http = Stub::offline();
        let r = run(http.clone(), "https://contoh.example/x");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.aio` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢".to_string()));
        assert!(http.seen.lock().unwrap().is_empty());
    }
}
