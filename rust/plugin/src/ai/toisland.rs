//! `plugins/ai/toisland.js` — ubah foto menjadi suasana pulau tropis lewat
//! scraper `seaart::live3d` (`oktz_scraper::seaart::live3d`).
//! PROMPT ditulis apa adanya dari JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

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
    if ctx.quoted().is_some_and(|q| q.is_media()) {
        return ctx.download_quoted_media().ok();
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
        // toisland.js:46-49 — `m.react('❌')` lalu `m.reply` pesan gagal unduh.
        return Reply::text("❌ Gagal mendownload gambar").with_reaction("❌");
    };

    match oktz_scraper::seaart::live3d(ctx.http.as_ref(), &buffer, PROMPT) {
        Ok(gambar) => Reply::image(MediaSource::bytes(gambar), String::new()).with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{
        HttpBody, HttpCall, HttpError, HttpGet, HttpResponse, IncomingKind, IncomingMedia,
        MediaError, MediaFetch, MediaOrigin, QuotedKind, QuotedMessage,
    };
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// Balasan berurutan per `call` (pola sama dengan test `bananagen.rs`);
    /// lima balasan cukup untuk seluruh rangkaian `seaart::live3d`.
    #[derive(Debug, Default)]
    struct Stub {
        balasan: Mutex<Vec<String>>,
        seen: Mutex<Vec<String>>,
        hitung: Mutex<usize>,
    }

    impl Stub {
        fn urutan(daftar: Vec<&str>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(daftar.into_iter().map(str::to_string).collect()),
                seen: Mutex::new(Vec::new()),
                hitung: Mutex::new(0),
            })
        }
        fn offline() -> Arc<Stub> {
            Stub::urutan(Vec::new())
        }
    }

    impl HttpGet for Stub {
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
            let i = {
                let mut hitung = self.hitung.lock().unwrap();
                let i = *hitung;
                *hitung += 1;
                i
            };
            let balasan = self.balasan.lock().unwrap().get(i).cloned();
            match balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
    }

    /// `m.download()` palsu, tanpa jaringan.
    #[derive(Debug)]
    struct MediaStub(Result<Vec<u8>, MediaError>);

    impl MediaFetch for MediaStub {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
            self.0.clone()
        }
    }

    fn ctx_media(
        http: Arc<Stub>,
        media: Option<IncomingKind>,
        quoted_image: bool,
        unduh: Result<Vec<u8>, MediaError>,
    ) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toisland".into(),
            media: media.map(|kind| IncomingMedia {
                kind,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            quoted: if quoted_image {
                Some(QuotedMessage {
                    id: "Q1".into(),
                    sender: ME.into(),
                    chat: ME.into(),
                    text: String::new(),
                    kind: QuotedKind::Image,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
                })
            } else {
                None
            },
            media_fetch: Arc::new(MediaStub(unduh)),
            http,
            ..Ctx::default()
        }
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
        // toisland.js:26-34 — media video bukan `imageMessage` → kartu pakai.
        let r = toisland(ctx_media(
            Stub::offline(),
            Some(IncomingKind::Video),
            false,
            Ok(Vec::new()),
        ));
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn download_gagal_mengembalikan_pesan_dan_reaksi_silang() {
        // toisland.js:46-49 — `m.react('❌')` lalu `m.reply` pesan gagal unduh.
        let r = toisland(ctx_media(
            Stub::offline(),
            Some(IncomingKind::Image),
            false,
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        ));
        assert_eq!(text_of(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn quoted_image_dilayani_sama_dengan_trigger_image() {
        // toisland.js:40-49 — `m.quoted` dicek duluan; unduh gagal → pesan
        // gagal unduh yang sama seperti jalur trigger.
        let r = toisland(ctx_media(
            Stub::offline(),
            None,
            true,
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        ));
        assert_eq!(text_of(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaction_of(&r), Some("❌"));
    }
}
