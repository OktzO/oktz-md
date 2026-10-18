//! `plugins/ai/toblack.js` — ubah gambar ke skin tone lebih gelap lewat scraper
//! `seaart::live3d` (`oktz_scraper::seaart::live3d`).
//! PROMPT ditulis apa adanya dari JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component as _;

const PROMPT: &str = "Transform skin tone to a darker complexion, maintain facial features, realistic shadows, high detail, natural skin texture, no distortion";

/// Kartu pakai saat tidak ada gambar.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🖤 *ʙʟᴀᴄᴋ sᴛʏʟᴇ*\n\n> Kirim/reply gambar\n\n`{prefix}toblack`"
    )
}

/// Ambil byte media dari pesan pemicu atau pesan yang di-reply, sama urutan
// pengecekannya dengan JS (`m.quoted` dulu, baru trigger).
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
    name = "toblack",
    alias = ["black", "hitamkan", "hitam", "tohitam"],
    category = "ai",
    description = "Ubah gambar ke skin tone lebih gelap",
    usage = ".toblack (reply gambar)",
    example = ".toblack",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn toblack(ctx: Ctx) -> Reply {
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

    fn ctx_media(buffer: Vec<u8>) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toblack".into(),
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(oktz_plugin_api::NoMediaFetch),
            http: Stub::bytes(buffer),
            ..Ctx::default()
        }
    }

    fn run_media(buffer: Vec<u8>) -> Reply {
        toblack(ctx_media(buffer))
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
        // media_fetch adalah NoMediaFetch → download selalu gagal.
        let r = toblack(Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toblack".into(),
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
    fn jalur_berhasil_mengirim_gambar_hasil_live3d() {
        let r = run_media(b"gambar-asli".to_vec());
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert!(caption.is_empty());
                match media {
                    Some(MediaSource::Bytes { data, .. }) => {
                        assert!(!data.is_empty());
                    }
                    lain => panic!("harus bytes, dapat {lain:?}"),
                }
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }
}
