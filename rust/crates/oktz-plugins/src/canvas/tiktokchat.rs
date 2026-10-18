//! `plugins/canvas/tiktokchat.js` — fake chat TikTok dari avatar.
//!
//! Catatan: baris `pub mod tiktokchat;` perlu ditambah ke
//! `src/canvas/mod.rs` secara manual (mod.rs milik plugin lain).

use oktz_plugin_api::{
    Ctx, HttpCall, HttpError, HttpGet, HttpResponse, IncomingKind, MediaError, MediaFetch,
    MediaOrigin, MediaSource, MultipartPart, Reply, plugin,
};

use serde_json::Value;
use oktz_plugin_api::HttpBody;

/// `tiktokchat.js:23-29` — kartu pakai saat gambar/teks/pemisah `|` belum lengkap.
fn help(prefix: &str) -> String {
    format!(
        "💬 *TIKTOK CHAT CANVAS*\n\n\
         Fitur ini digunakan untuk membuat desain chat palsu bergaya TikTok yang estetis!\n\n\
         *Cara Penggunaan:*\n\
         - Kirim foto profil (avatar) dengan caption *{prefix}tiktokchat username | pesan kamu*\n\
         - Atau balas (reply) foto profil dengan pesan *{prefix}tiktokchat username | pesan kamu*\n\n\
         *Contoh:* {prefix}tiktokchat Zann | Halo, hari ini cerah ya!"
    )
}

/// `tiktokchat.js:51-60` — unggah avatar + teks ke canvas TikTok chat.
fn upload_chat(http: &dyn HttpGet, username: &str, message: &str, avatar: &[u8]) -> Result<Vec<u8>, String> {
    let res = http
        .call(
            &HttpCall::post("https://my.izuka-api.xyz/api/canvas/tiktok-chat").multipart(vec![
                MultipartPart::field("username", username),
                MultipartPart::field("text", message),
                MultipartPart::file("avatar", "avatar.jpg", avatar.to_vec()).with_mimetype("image/jpeg"),
            ]),
        )
        .map_err(|e: HttpError| format!("upload gagal: {e}"))?;
    Ok(res.body.bytes().to_vec())
}

const GAGAL_AVATAR: &str = "Maaf, sistem gagal mengunduh gambar avatar yang kamu berikan.";
const GAGAL_CANVAS: &str = "Maaf, gagal membuat canvas TikTok Chat kali ini. Coba lagi beberapa saat.";

#[plugin(
    name = "tiktokchat",
    alias = ["tiktok-chat", "ttchat"],
    category = "canvas",
    description = "Membuat fake chat TikTok dari avatar kamu",
    usage = ".tiktokchat username | pesan (reply gambar)",
    example = ".tiktokchat Zann | Halo, hari ini cerah ya!",
    cooldown = 5,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tiktokchat(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(IncomingKind::Image)
        || ctx.quoted().is_some_and(|q| q.kind.is_image());
    let text = ctx.text.trim();
    if !is_image || text.is_empty() || !text.contains('|') {
        return Reply::text(help(&ctx.prefix));
    }

    let mut parts = text.split('|');
    let username = parts.next().unwrap_or("").trim();
    let message = parts.collect::<Vec<_>>().join("|").trim();

    let buffer = if ctx.quoted().is_some_and(|q| q.kind.is_media()) {
        ctx.download_quoted_media().ok()
    } else if ctx.is_media() {
        ctx.download_media().ok()
    } else {
        None
    };
    let Some(buffer) = buffer else {
        return Reply::text(GAGAL_AVATAR).with_reaction("❌");
    };

    let bytes = match upload_chat(&*ctx.http, username, message, &buffer) {
        Ok(b) => b,
        Err(_) => return Reply::text(GAGAL_CANVAS).with_reaction("❌"),
    };

    Reply::image(MediaSource::bytes(bytes), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        response: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn online() -> Arc<Stub> {
            Arc::new(Stub {
                response: Some(b"image-bytes".to_vec()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                response: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.respond(url)
        }
        fn post_json(&self, url: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            self.respond(url)
        }
        fn post_form(&self, url: &str, _: &str) -> Result<HttpResponse, HttpError> {
            self.respond(url)
        }
        fn post_multipart(
            &self,
            url: &str,
            _: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            self.respond(url)
        }
    }

    impl MediaFetch for Stub {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
            match &self.response {
                Some(_) => Ok(b"avatar-bytes".to_vec()),
                None => Err(MediaError {
                    message: "offline".into(),
                }),
            }
        }
    }

    impl Stub {
        fn respond(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(
                        self.response.clone().unwrap_or_default()
                    ),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn media_image() -> oktz_plugin_api::IncomingMedia {
        oktz_plugin_api::IncomingMedia {
            kind: IncomingKind::Image,
            mimetype: None,
            file_name: None,
            caption: String::new(),
        }
    }

    fn run(http: Arc<Stub>, text: &str, is_image: bool) -> Reply {
        let ctx = Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tiktokchat".into(),
            http: http.clone(),
            media_fetch: http,
            media: if is_image {
                Some(media_image())
            } else {
                None
            },
            ..Ctx::default()
        };
        tiktokchat(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn reaction_of(r: &Reply) -> Option<String> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.clone()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_gambar_atau_pemisah_menghasilkan_kartu_pakai() {
        for (text, is_image) in [("", false), ("Zann | Halo", false), ("Zann", true)] {
            let r = run(Stub::offline(), text, is_image);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaction_of(&r), None, "input {text:?}");
        }
    }

    #[test]
    fn gagal_unduh_avatar_mengembalikan_teks_dan_reaksi_silang() {
        let r = run(Stub::offline(), "Zann | Halo", true);
        assert_eq!(text_of(&r), GAGAL_AVATAR);
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_dan_reaksi_centang() {
        let http = Stub::online();
        let r = run(http.clone(), "Zann | Halo, hari ini cerah ya!", true);
        assert_eq!(reaction_of(&r), Some("✅".to_string()));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(caption, "");
                assert_eq!(
                    media,
                    &Some(MediaSource::Bytes {
                        data: Arc::from(b"image-bytes".to_vec()),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn api_gagal_mengembalikan_pesan_gagal_dan_reaksi_silang() {
        struct UpOk;
        impl HttpGet for UpOk {
            fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_multipart(
                &self,
                _: &str,
                _: &[(&str, oktz_plugin_api::HttpBody)],
            ) -> Result<HttpResponse, HttpError> {
                Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                })
            }
        }
        impl MediaFetch for UpOk {
            fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
                Ok(b"avatar".to_vec())
            }
        }
        let ctx = Ctx {
            text: "Zann | Halo".into(),
            args: vec!["Zann | Halo".into()],
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tiktokchat".into(),
            http: Arc::new(UpOk),
            media_fetch: Arc::new(UpOk),
            media: Some(media_image()),
            ..Ctx::default()
        };
        let r = tiktokchat(ctx);
        assert_eq!(text_of(&r), GAGAL_CANVAS);
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = oktz_plugin_api::find("tiktokchat").unwrap();
        assert_eq!(p.meta.name, "tiktokchat");
        assert_eq!(p.meta.alias, ["tiktok-chat", "ttchat"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        for a in ["tiktok-chat", "ttchat"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "tiktokchat");
        }
    }
}
