//! `plugins/ai/bananagen.js` — edit gambar dengan AI menggunakan prompt.
//!
//! `ponytail:` JS bereaksi `🕕` lalu `✅`; `Reply::with_reaction` hanya satu
//! emoji, jadi hanya `✅` dibawa pada sukses. `m.download()` /
//! `m.quoted.download()` dipindah ke `ctx.download_media()` /
//! `download_quoted_media()`. `live3d` dipanggil lewat
//! `oktz_scraper::seaart::live3d`. Hasil dikirim sebagai `Reply::image`
//! byte (MediaSource::bytes) tanpa quote.
//!
//! Catatan: baris `pub mod bananagen;` perlu ditambah ke `ai/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, MediaError, MediaSource, Reply, plugin};
use oktz_scraper::seaart::live3d;
use oktz_plugin_api::HttpError;
use oktz_plugin_api::MediaOrigin;

#[plugin(
    name = "bananagen",
    alias = [],
    category = "ai",
    description = "Edit gambar dengan AI menggunakan prompt",
    usage = ".bananagen <prompt>",
    example = ".bananagen make it anime style",
    cooldown = 30,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bananagen(ctx: Ctx) -> Reply {
    let prompt = ctx.args.join(" ");
    if prompt.trim().is_empty() {
        return Reply::text(
            "🍌 *BANANA GEN*\n\n\
             > Edit gambar dengan AI\n\n\
             `Contoh: .bananagen make it anime style`\n\n\
             > Reply atau kirim gambar dengan caption",
        );
    }

    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image());

    if !is_image {
        return Reply::text(
            "🍌 *ɴᴀɴᴏ ʙᴀɴᴀɴᴀ*\n\n> Reply atau kirim gambar dengan caption",
        );
    }

    let buffer = if ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image) {
        match ctx.download_media() {
            Ok(b) => b,
            Err(_) => {
                return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal mengunduh gambar")
                    .with_reaction("❌");
            }
        }
    } else {
        match ctx.download_quoted_media() {
            Ok(b) => b,
            Err(_) => {
                return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal mengunduh gambar")
                    .with_reaction("❌");
            }
        }
    };

    match live3d(ctx.http.as_ref(), &buffer, &prompt) {
        Ok(result) => Reply::image(MediaSource::bytes(result), String::new())
            .with_reaction("✅"),
        Err(_) => Reply::text(
            "🍀 *Waduhh, sepertinya ini ada kendala*\n\
             Silahkan coba lagi nanti, dimohon jangan spam",
        )
        .with_reaction("❌"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpGet, HttpResponse, MediaFetch};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct HttpStub {
        seen: Mutex<Vec<String>>,
        responses: Mutex<Vec<String>>,
        hit: Mutex<usize>,
    }

    impl HttpStub {
        fn ordered(responses: Vec<&str>) -> Arc<HttpStub> {
            Arc::new(HttpStub {
                seen: Mutex::new(Vec::new()),
                responses: Mutex::new(responses.into_iter().map(|s| s.to_string()).collect()),
                hit: Mutex::new(0),
            })
        }
        fn offline() -> Arc<HttpStub> {
            Arc::new(HttpStub {
                seen: Mutex::new(Vec::new()),
                responses: Mutex::new(Vec::new()),
                hit: Mutex::new(0),
            })
        }
    }

    impl HttpGet for HttpStub {
        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
            let mut hit = self.hit.lock().unwrap();
            let i = *hit;
            *hit += 1;
            let responses = self.responses.lock().unwrap();
            if i >= responses.len() {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            let body = responses.get(i).cloned().unwrap_or_default();
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
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

    #[derive(Debug, Default)]
    struct MediaStub {
        bytes: Vec<u8>,
        fail: bool,
    }

    impl MediaFetch for MediaStub {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
            if self.fail {
                Err(MediaError {
                    message: "unduh gagal".into(),
                })
            } else {
                Ok(self.bytes.clone())
            }
        }
    }

    fn run(
        http: Arc<HttpStub>,
        media: Option<Vec<u8>>,
        media_fail: bool,
        quoted: bool,
        text: &str,
    ) -> Reply {
        let args: Vec<String> = text.split_whitespace().skip(1).map(|s| s.to_string()).collect();
        let ctx = Ctx {
            text: text.to_string(),
            args,
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            command: "bananagen".into(),
            http,
            media: media.map(|b| oktz_plugin_api::IncomingMedia {
                kind: oktz_plugin_api::IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(MediaStub {
                bytes: media.unwrap_or_default(),
                fail: media_fail,
            }),
            quoted: if quoted {
                Some(oktz_plugin_api::QuotedMessage {
                    id: "q1".into(),
                    sender: "628999@s.whatsapp.net".into(),
                    chat: "628111@s.whatsapp.net".into(),
                    text: String::new(),
                    kind: oktz_plugin_api::QuotedKind::Image,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
                })
            } else {
                None
            },
            ..Default::default()
        };
        bananagen(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("bananagen").unwrap();
        assert_eq!(p.meta.name, "bananagen");
        assert!(p.meta.alias.is_empty());
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_prompt_tampilkan_bantuan() {
        let r = run(HttpStub::offline(), None, false, false, ".bananagen");
        assert!(body(&r).contains("BANANA GEN"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn tanpa_gambar_tampilkan_pesan() {
        let r = run(HttpStub::offline(), None, false, false, ".bananagen make it anime");
        assert!(body(&r).contains("ɴᴀɴᴏ ʙᴀɴᴀɴᴀ"));
    }

    #[test]
    fn gambar_tanpa_prompt_bantuan_prioritas() {
        let r = run(
            HttpStub::offline(),
            Some(vec![1, 2, 3]),
            false,
            false,
            ".bananagen",
        );
        assert!(body(&r).contains("BANANA GEN"));
    }

    #[test]
    fn unduh_gagal_kembalikan_pesan_gagal() {
        let r = run(
            HttpStub::offline(),
            Some(vec![1, 2, 3]),
            true,
            false,
            ".bananagen make it anime",
        );
        assert!(body(&r).contains("Gagal mengunduh gambar"));
    }

    #[test]
    fn sukses_kirim_gambar_byte() {
        let http = HttpStub::ordered(vec![
            r#"{"result":{"url":"https://unggah.example/x","object_name":"obj/1"}}"#,
            "",
            r#"{"result":{"url":"https://signed.example/x.jpg"}}"#,
            r#"{"status":true,"data":{"result_url":"https://hasil.example/y.png"}}"#,
            "GAMBAR",
        ]);
        let r = run(http, Some(vec![9, 8, 7]), false, false, ".bananagen make it anime");
        // live3d sukses -> Image dengan reaction ✅
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus Reaction");
        };
        match isi.as_ref() {
            Reply::Image { media, .. } => match media {
                Some(MediaSource::Bytes { data, .. }) => assert_eq!(data.as_ref(), b"GAMBAR"),
                other => panic!("harus Bytes media, dapat {other:?}"),
            },
            other => panic!("harus Image, dapat {other:?}"),
        }
        assert_eq!(r.mutations().is_empty(), true);
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 5);
    }

    #[test]
    fn live3d_gagal_kembalikan_pesan_kendala() {
        // live3d sukses sampai nanobanana tapi status false -> error.
        let http = HttpStub::ordered(vec![
            r#"{"result":{"url":"https://unggah.example/x","object_name":"obj/1"}}"#,
            "",
            r#"{"result":{"url":"https://signed.example/x.jpg"}}"#,
            r#"{"status":false}"#,
        ]);
        let r = run(http, Some(vec![1, 2, 3]), false, false, ".bananagen make it anime");
        assert!(body(&r).contains("kendala"));
        assert_eq!(r.mutations().is_empty(), true);
    }
}
