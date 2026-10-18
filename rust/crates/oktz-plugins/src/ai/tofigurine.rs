//! `plugins/ai/tofigurine.js` — ubah foto menjadi figurine/action figure via
//! scraper `seaart` (`live3d`) (nama command JS: `tofigure3`).
//!
//! `ponytail:` `live3d` di Rust mengembalikan `Vec<u8>` (byte gambar mentah),
//! bukan `{ image }`, karena `serde_json::Value` tidak bisa membawa buffer.
//! Satu reaksi akhir (`✅`/`❌`/`☢`) dipasang.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::common::error_template;
use oktz_plugin_api::HttpError;
use serde_json::Value;

const PROMPT: &str = "Using the model, create a 1/7 scale commercialized figurine of the characters in the picture, \
in a realistic style, in a real environment. The figurine is placed on a computer desk. \
The figurine has a round transparent acrylic base, with no text on the base. \
The content on the computer screen is the modeling process of this figurine. \
Next to the computer screen is a BANDAI-style toy packaging box printed with the original artwork. \
The packaging features two-dimensional flat illustrations.";

fn help(prefix: &str) -> String {
    format!(
        "🎭 *ᴛᴏ ꜰɪɢᴜʀ 3*\n\n\
         > Kirim/reply gambar untuk diubah ke figurine/action figure\n\n\
         `{prefix}tofigure3`"
    )
}

#[plugin(
    name = "tofigure3",
    alias = ["figurine3", "tofigure3", "bandai3", "actionfigure3"],
    category = "ai",
    description = "Ubah foto menjadi action figure/figurine koleksi",
    usage = ".tofigure3 (reply/kirim gambar)",
    example = ".tofigure3",
    cooldown = 60,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tofigurine(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image());

    if !is_image {
        return Reply::text(help(&ctx.prefix));
    }

    let origin = if ctx
        .quoted()
        .is_some_and(|q| q.kind.is_image())
    {
        oktz_plugin_api::MediaOrigin::Quoted
    } else {
        oktz_plugin_api::MediaOrigin::Trigger
    };
    let buffer = match ctx.media_fetch.download(origin) {
        Ok(b) => b,
        Err(_) => {
            return Reply::text("❌ Gagal mendownload gambar").with_reaction("❌");
        }
    };
    if buffer.is_empty() {
        return Reply::text("❌ Gagal mendownload gambar").with_reaction("❌");
    }

    let result = match oktz_scraper::seaart::live3d(&*ctx.http, &buffer, PROMPT) {
        Ok(b) => b,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
        }
    };

    Reply::image(MediaSource::bytes(result), "").with_reaction("✅")
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse, IncomingKind, MediaError, MediaFetch};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct StubHttp {
        balasan: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl StubHttp {
        fn bytes(b: Vec<u8>) -> Arc<StubHttp> {
            Arc::new(StubHttp {
                balasan: Some(b),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<StubHttp> {
            Arc::new(StubHttp {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
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
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("lewat call")
        }
    }

    #[derive(Debug, Default)]
    struct StubMedia {
        hasil: Result<Vec<u8>, MediaError>,
    }

    impl MediaFetch for StubMedia {
        fn download(&self, _: oktz_plugin_api::MediaOrigin) -> Result<Vec<u8>, MediaError> {
            self.hasil.clone()
        }
    }

    fn ctx_gambar(buffer: Vec<u8>, quoted: bool) -> Ctx {
        let kind = if quoted {
            oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }
        } else {
            oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }
        };
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tofigure3".into(),
            quoted: if quoted {
                Some(oktz_plugin_api::QuotedMessage {
                    id: "q1".into(),
                    sender: ME.into(),
                    chat: ME.into(),
                    text: String::new(),
                    kind: IncomingKind::Image,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
                })
            } else {
                None
            },
            media: Some(kind),
            media_fetch: Arc::new(StubMedia {
                hasil: Ok(buffer.clone()),
            }),
            http: StubHttp::bytes(vec![1, 2, 3]),
            ..Ctx::default()
        }
    }

    fn run(buffer: Vec<u8>, quoted: bool) -> Reply {
        tofigurine(ctx_gambar(buffer, quoted))
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
    fn bukan_gambar_menghasilkan_kartu_pakai() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tofigure3".into(),
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Video,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(StubMedia::default()),
            http: StubHttp::offline(),
            ..Ctx::default()
        };
        let r = tofigurine(ctx);
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn quoted_bukan_gambar_ditolak() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tofigure3".into(),
            quoted: Some(oktz_plugin_api::QuotedMessage {
                id: "q1".into(),
                sender: ME.into(),
                chat: ME.into(),
                text: String::new(),
                kind: IncomingKind::Video,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            media_fetch: Arc::new(StubMedia::default()),
            http: StubHttp::offline(),
            ..Ctx::default()
        };
        let r = tofigurine(ctx);
        assert_eq!(text_of(&r), help("."));
    }

    #[test]
    fn download_gagal_menghasilkan_galat() {
        let ctx = ctx_gambar(vec![], false);
        let ctx = Ctx {
            media_fetch: Arc::new(StubMedia {
                hasil: Err(MediaError {
                    message: "cdn mati".into(),
                }),
            }),
            ..ctx
        };
        let r = tofigurine(ctx);
        assert_eq!(text_of(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn live3d_gagal_menggunakan_error_template() {
        let ctx = ctx_gambar(vec![9, 9, 9], false);
        let ctx = Ctx {
            http: StubHttp::offline(),
            ..ctx
        };
        let r = tofigurine(ctx);
        assert_eq!(text_of(&r), crate::common::error_template(".", "tofigure3"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_byte_dan_reaksi_check() {
        let http = StubHttp::bytes(vec![7, 8, 9]);
        let ctx = Ctx {
            http,
            ..ctx_gambar(vec![1, 2, 3], false)
        };
        let r = tofigurine(ctx);
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, .. } => match media {
                Some(oktz_plugin_api::MediaSource::Bytes { data, .. }) => {
                    assert_eq!(data.as_ref(), &[7, 8, 9]);
                }
                lain => panic!("harus bytes, dapat {lain:?}"),
            },
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn quoted_digunakan_saat_ada() {
        // quoted image harus dipakai (origin Quoted), bukan media trigger.
        let http = StubHttp::bytes(vec![5, 6]);
        let ctx = Ctx {
            http,
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Video, // trigger adalah video, harus diabaikan
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            quoted: Some(oktz_plugin_api::QuotedMessage {
                id: "q1".into(),
                sender: ME.into(),
                chat: ME.into(),
                text: String::new(),
                kind: IncomingKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            media_fetch: Arc::new(StubMedia {
                hasil: Ok(vec![5, 6]),
            }),
            ..Ctx::default()
        };
        let r = tofigurine(ctx);
        assert_eq!(reaction_of(&r), Some("✅"));
    }
}
