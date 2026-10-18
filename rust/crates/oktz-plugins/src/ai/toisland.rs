//! `plugins/ai/toisland.js` — ubah foto menjadi suasana pulau tropis lewat
//! scraper `seaart::live3d` (`oktz_scraper::seaart::live3d`).
//! PROMPT ditulis apa adanya dari JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::common::error_template;

const PROMPT: &str = "Transform this image into a tropical island scene. \
Place the subject in a beautiful island environment with clear blue ocean, palm trees, and warm sunlight. \
Add realistic lighting, shadows, and vibrant tropical colors. \
Keep the original identity, high detail, cinematic, photorealistic.";

/// Kartu pakai saat tidak ada gambar.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🏝️ *ᴛᴏ ɪsʟᴀɴᴅ*\n\n> Kirim/reply gambar untuk suasana pulau\n\n`{prefix}toisland`"
    )
}

/// Ambil byte media dari pesan pemicu atau pesan yang di-reply, sama urutan
// pengecekannya dengan JS.
fn ambil_buffer(ctx: &Ctx) -> Option<Vec<u8>> {
    if let Some(q) = ctx.quoted() {
        if q.is_media() {
            return ctx.download_quoted_media().ok();
        }
    }
    if ctx.is_media() {
        return ctx.download_media().ok();
    }
    None
}

#[plugin(
    name = "toisland",
    alias = ["island", "tropical"],
    category = "ai",
    description = "Ubah foto menjadi suasana pulau tropis",
    usage = ".toisland (reply/kirim gambar)",
    example = ".toisland",
    cooldown = 60,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn toisland(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| matches!(q.kind, oktz_plugin_api::QuotedKind::Image));

    if !is_image {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(buffer) = ambil_buffer(&ctx) else {
        return Reply::text("❌ Gagal mendownload gambar");
    };

    match oktz_scraper::seaart::live3d(ctx.http.as_ref(), &buffer, PROMPT) {
        Ok(gambar) => Reply::image(MediaSource::bytes(gambar), String::new()).with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, IncomingKind};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn bytes(b: impl Into<Vec<u8>>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(b.into()),
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
                        data: Arc::from(b.clone()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
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

    fn run_media(buffer: Vec<u8>) -> Reply {
        toisland(Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toisland".into(),
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(oktz_plugin_api::NoMediaFetch),
            http: Stub::bytes(buffer),
            ..Ctx::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn bukan_gambar_menghasilkan_kartu_pakai_tanpa_reaksi() {
        let r = run_media(vec![]);
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn download_gagal_mengembalikan_pesan_tanpa_reaksi() {
        let r = toisland(Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toisland".into(),
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(oktz_plugin_api::NoMediaFetch),
            http: Stub::bytes(vec![1, 2, 3]),
            ..Ctx::default()
        });
        assert_eq!(text_of(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn quoted_image_dilayani_sama_dengan_trigger_image() {
        // quoted berisi media → download_quoted_media dipakai.
        // Karena NoMediaFetch selalu gagal, hasilnya pesan gagal download.
        let r = toisland(Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toisland".into(),
            quoted: Some(oktz_plugin_api::QuotedMessage {
                id: "1".into(),
                sender: ME.into(),
                chat: ME.into(),
                text: String::new(),
                kind: oktz_plugin_api::QuotedKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            media_fetch: Arc::new(oktz_plugin_api::NoMediaFetch),
            http: Stub::bytes(vec![1, 2, 3]),
            ..Ctx::default()
        });
        assert_eq!(text_of(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaction_of(&r), None);
    }
}
