//! `plugins/ai/tocartoon.js` — ubah foto menjadi gaya kartun lewat nanobanana.
//!
//! `ponytail:` reaksi ganda JS (`🕕` lalu `✅`) disatukan jadi satu reaksi
//! akhir (`✅`) karena `Reply::with_reaction` hanya satu emoji. `live3d`
//! berasal dari `oktz_scraper::seaart` (bukan disalin ke sini).

use oktz_plugin_api::{Ctx, IncomingKind, MediaSource, Reply, plugin};

use crate::common::error_template;

use oktz_scraper::seaart::live3d;

/// Prompt kartun (`tocartoon.js:19-22`).
const PROMPT: &str = "Transform this image into a vibrant cartoon style like Disney or Pixar animation. \
Apply bold colors, smooth shading, exaggerated features, and that playful cartoon aesthetic. \
Keep the original composition but make it look like a frame from an animated movie with \
clean lines, expressive faces, and bright cheerful colors.";

/// Kartu pakai (`tocartoon.js:28-32`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎬 *ᴛᴏ ᴄᴀʀᴛᴏᴏɴ*\n\n\
         > Kirim/reply gambar untuk diubah ke gaya kartun\n\n\
         `{prefix}tocartoon`"
    )
}

/// `isImage` (`tocartoon.js:25`) — trigger atau quoted adalah gambar.
pub(crate) fn is_image(ctx: &Ctx) -> bool {
    ctx.is_media_kind(IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image())
}

/// Unduh buffer media: quoted dulu, baru trigger (`tocartoon.js:39-43`).
pub(crate) fn unduh_media(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.quoted().is_some() {
        ctx.download_quoted_media().ok()
    } else if ctx.is_media() {
        ctx.download_media().ok()
    } else {
        None
    }
}

#[plugin(
    name = "tocartoon",
    alias = ["cartoon", "cartoonify", "tooncartoon"],
    category = "ai",
    description = "Ubah foto menjadi gaya kartun",
    usage = ".tocartoon (reply/kirim gambar)",
    example = ".tocartoon",
    cooldown = 60,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tocartoon(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(pakai(&ctx.prefix));
    }

    let Some(buffer) = async_download_media(&ctx) else {
        return Reply::text("❌ Gagal mendownload gambar").with_reaction("❌");
    };

    match live3d(&ctx.http, &buffer, PROMPT) {
        Ok(image) => Reply::image(MediaSource::bytes(image), String::new()).with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{IncomingMedia, QuotedKind, QuotedMessage};
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct StubHttp {
        bytes: Option<Vec<u8>>,
        fail: bool,
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _: &str) -> Result<oktz_plugin_api::HttpResponse, oktz_plugin_api::HttpError> {
            if self.fail {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: Arc::from(self.bytes.clone().unwrap_or_default()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
        fn post_json(
            &self,
            _: &str,
            _: &serde_json::Value,
        ) -> Result<oktz_plugin_api::HttpResponse, oktz_plugin_api::HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<oktz_plugin_api::HttpResponse, oktz_plugin_api::HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, oktz_plugin_api::HttpError> {
            unimplemented!()
        }
    }

    #[derive(Debug, Default)]
    struct StubFetch {
        ok: bool,
        bytes: Vec<u8>,
    }

    impl oktz_plugin_api::MediaFetch for StubFetch {
        fn download(&self, _: oktz_plugin_api::MediaOrigin) -> Result<Vec<u8>, oktz_plugin_api::MediaError> {
            if self.ok {
                Ok(self.bytes.clone())
            } else {
                Err(oktz_plugin_api::MediaError {
                    message: "gagal".into(),
                })
            }
        }
    }

    fn run(image: bool, quoted_image: bool, download_ok: bool, live3d_fail: bool) -> Reply {
        let media = if image {
            Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            })
        } else {
            None
        };
        let quoted = if quoted_image {
            Some(QuotedMessage {
                id: "Q1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: String::new(),
                kind: QuotedKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            })
        } else {
            None
        };
        let ctx = Ctx {
            prefix: ".".into(),
            command: "tocartoon".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            media,
            quoted,
            media_fetch: Arc::new(StubFetch {
                ok: download_ok,
                bytes: b"IMG".to_vec(),
            }),
            http: Arc::new(StubHttp {
                bytes: Some(b"CARTOON".to_vec()),
                fail: live3d_fail,
            }),
            ..Ctx::default()
        };
        tocartoon(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("tocartoon").unwrap();
        assert_eq!(p.meta.name, "tocartoon");
        assert_eq!(p.meta.alias, ["cartoon", "cartoonify", "tooncartoon"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 3);
        assert!(p.meta.is_premium);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn bukan_gambar_tampilkan_pakai() {
        let r = run(false, false, true, false);
        assert_eq!(teks(&r), pakai("."));
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn quoted_gambar_diprioritaskan() {
        let r = run(false, true, true, false);
        assert_eq!(reaksi(&r), Some("✅"));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Image { media, .. } => {
                    assert_eq!(
                        media,
                        &Some(MediaSource::Bytes {
                            data: Arc::from(b"CARTOON".to_vec()),
                            mimetype: None,
                        })
                    );
                }
                other => panic!("harus Image, dapat {other:?}"),
            },
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn gagal_unduh_gambar_teks_dan_reaksi_batsu() {
        let r = run(true, false, false, false);
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaksi(&r), Some("❌"));
    }

    #[test]
    fn live3d_gagal_error_template_dan_reaksi_sil() {
        let r = run(true, false, true, true);
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "tocartoon")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn prompt_diperlukan_oleh_live3d() {
        assert!(!PROMPT.is_empty());
        assert!(PROMPT.contains("Disney"));
    }
}
