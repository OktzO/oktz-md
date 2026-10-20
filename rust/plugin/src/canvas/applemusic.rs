//! `plugins/canvas/applemusic.js` — gambar pemutar Apple Music dari foto.
//!
//! `ponytail:` kunci upload `TERMAI_UPLOAD_KEY` dibaca dari `process.env` di
//! JS, yang tidak ada di `Ctx`; port menggunakan `""` (mirip `|| ""` di JS).
//! Tambahkan kolom kunci ke `Runtime` kalau router mengeksposnya nanti.
//! Catatan: baris `pub mod applemusic;` perlu ditambah ke
//! `src/canvas/mod.rs` secara manual (mod.rs milik plugin lain).

use oktz_plugin_api::{
    Ctx, HttpCall, HttpError, HttpGet, IncomingKind, MediaSource, MultipartPart, Reply, plugin,
};

use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `applemusic.js:4-10` — kartu pakai saat gambar/teks/pemisah `|` belum lengkap.
fn help(prefix: &str) -> String {
    format!(
        "🎵 *APPLE MUSIC CANVAS*\n\n\
         Fitur ini digunakan untuk membuat desain pemutar lagu Apple Music yang keren dari gambarmu!\n\n\
         *Cara Penggunaan:*\n\
         - Kirim gambar dengan caption *{prefix}applemusic-canvas Judul Lagu | Nama Artis*\n\
         - Atau balas (reply) gambar dengan pesan *{prefix}applemusic-canvas Judul Lagu | Nama Artis*\n\n\
         *Contoh:* {prefix}applemusic-canvas Glimpse of Us | Joji"
    )
}

/// `TERMAI_UPLOAD_KEY` hanya ada di `process.env` JS; `Ctx` tidak mengekspos
// env, jadi `""` — sama dengan `process.env.TERMAI_UPLOAD_KEY || ""`.
fn termai_key() -> &'static str {
    ""
}

/// `applemusic.js:21-37` — unggah cover ke termai, ambil `path` atau
// `files[0].url`; kalau keduanya tidak ada → `Error('Upload gagal')`.
fn upload_cover(http: &dyn HttpGet, buffer: &[u8]) -> Result<String, String> {
    let url = format!("https://c.termai.cc/api/upload?key={}", termai_key());
    let res = http
        .call(
            &HttpCall::post(&url).multipart(vec![
                MultipartPart::file("file", "image.jpg", buffer.to_vec()).with_mimetype("image/jpeg"),
            ]),
        )
        .map_err(|e: HttpError| format!("upload gagal: {}", e.message))?;
    let data = res.json().unwrap_or(Value::Null);
    if let Some(Value::String(p)) = data.get("path") {
        return Ok(p.clone());
    }
    if let Some(u) = data
        .get("files")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(|f| f.get("url"))
        .and_then(Value::as_str)
    {
        return Ok(u.to_string());
    }
    Err("Upload gagal".to_string())
}

/// `applemusic.js:71` — API canvas Apple Music.
fn apple_music_url(title: &str, artist: &str, cover_url: &str) -> String {
    format!(
        "https://my.izuka-api.xyz/api/canvas/apple-music?title={}&artist={}&coverUrl={}",
        encode_uri_component(title),
        encode_uri_component(artist),
        encode_uri_component(cover_url)
    )
}

/// Pesan gagal gabung (JS `catch` tangkap semua: upload, API).
const GAGAL_CANVAS: &str =
    "Maaf, gagal membuat canvas Apple Music. Coba lagi beberapa saat.";

#[plugin(
    name = "applemusic-canvas",
    alias = ["applemusic", "applecanvas"],
    category = "canvas",
    description = "Buat gambar pemutar Apple Music dari foto",
    usage = ".applemusic-canvas judul | artis (reply gambar)",
    example = ".applemusic-canvas Glimpse of Us | Joji",
    cooldown = 5,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn applemusic(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(IncomingKind::Image)
        || ctx.quoted().is_some_and(|q| q.kind.is_image());
    let text = ctx.text.trim();
    if !is_image || text.is_empty() || !text.contains('|') {
        return Reply::text(help(&ctx.prefix));
    }

    let mut parts = text.split('|');
    let title = parts.next().unwrap_or("").trim();
    let artist = parts.next().unwrap_or("").trim();

    let buffer = if ctx.quoted().is_some_and(|q| q.kind.is_media()) {
        ctx.download_quoted_media().ok()
    } else if ctx.is_media() {
        ctx.download_media().ok()
    } else {
        None
    };
    let Some(buffer) = buffer else {
        return Reply::text("Maaf, sistem gagal mengunduh gambar cover yang kamu berikan.")
            .with_reaction("❌");
    };

    let cover_url = match upload_cover(&*ctx.http, &buffer) {
        Ok(u) => u,
        Err(_) => return Reply::text(GAGAL_CANVAS).with_reaction("❌"),
    };

    let Ok(res) = ctx.http.get(&apple_music_url(title, artist, &cover_url)) else {
        return Reply::text(GAGAL_CANVAS).with_reaction("❌");
    };

    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse, MediaError, MediaFetch, MediaOrigin};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    /// HTTP + mediaFetch sekaligus: upload (termai) mengembalikan JSON path,
    /// API (izuka) mengembalikan byte gambar, media download mengembalikan avatar.
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
        /// Plugin memakai `HttpCall::multipart` (`applemusic.js:23-27`);
        /// default `HttpGet::call` menolak multipart, jadi stub wajib
        /// menimpanya (pola sama dengan `search/animeapaini.rs`).
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.respond(&req.url)
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
        fn respond(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = if url.contains("termai") {
                b"{\"path\":\"https://cdn/cover.jpg\"}".to_vec()
            } else {
                self.response.clone().unwrap_or_default()
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body),
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
            command: "applemusic-canvas".into(),
            http: http.clone(),
            media_fetch: http,
            media: if is_image {
                Some(media_image())
            } else {
                None
            },
            ..Ctx::default()
        };
        applemusic(ctx)
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
        for (text, is_image) in [("", false), ("Judul Lagu", true), ("Judul|Artis", false)] {
            let r = run(Stub::offline(), text, is_image);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaction_of(&r), None, "input {text:?}");
        }
        let http = Stub::offline();
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn gagal_unduh_cover_mengembalikan_teks_dan_reaksi_silang() {
        let r = run(Stub::offline(), "Glimpse of Us | Joji", true);
        assert_eq!(
            text_of(&r),
            "Maaf, sistem gagal mengunduh gambar cover yang kamu berikan."
        );
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_dan_reaksi_centang() {
        let http = Stub::online();
        let r = run(http.clone(), "Glimpse of Us | Joji", true);
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
        let seen = http.seen.lock().unwrap();
        assert!(seen.iter().any(|u| u.contains("c.termai.cc/api/upload")), "{seen:?}");
        assert!(
            seen.iter().any(|u| u.contains("my.izuka-api.xyz/api/canvas/apple-music")),
            "{seen:?}"
        );
    }

    #[test]
    fn upload_gagal_mengembalikan_pesan_gagal_dan_reaksi_silang() {
        // Unduh sukses (media ada), upload (termai) gagal.
        #[derive(Debug)]
        struct UpFail;
        impl HttpGet for UpFail {
            fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
                unreachable!("API tidak terpanggil kalau upload gagal")
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
        impl MediaFetch for UpFail {
            fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
                Ok(b"avatar".to_vec())
            }
        }
        let ctx = Ctx {
            text: "Glimpse of Us | Joji".into(),
            args: vec!["Glimpse of Us | Joji".into()],
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "applemusic-canvas".into(),
            http: Arc::new(UpFail),
            media_fetch: Arc::new(UpFail),
            media: Some(media_image()),
            ..Ctx::default()
        };
        let r = applemusic(ctx);
        assert_eq!(text_of(&r), GAGAL_CANVAS);
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn api_canvas_gagal_mengembalikan_pesan_gagal_dan_reaksi_silang() {
        // Upload sukses (termai JSON), GET API (izuka) gagal.
        #[derive(Debug)]
        struct UpOk;
        impl HttpGet for UpOk {
            fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
                Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                })
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
                Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b"{\"path\":\"https://cdn/c.jpg\"}".to_vec()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                })
            }
        }
        impl MediaFetch for UpOk {
            fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> {
                Ok(b"avatar".to_vec())
            }
        }
        let ctx = Ctx {
            text: "Glimpse of Us | Joji".into(),
            args: vec!["Glimpse of Us | Joji".into()],
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "applemusic-canvas".into(),
            http: Arc::new(UpOk),
            media_fetch: Arc::new(UpOk),
            media: Some(media_image()),
            ..Ctx::default()
        };
        let r = applemusic(ctx);
        assert_eq!(text_of(&r), GAGAL_CANVAS);
        assert_eq!(reaction_of(&r), Some("❌".to_string()));
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = oktz_plugin_api::find("applemusic-canvas").unwrap();
        assert_eq!(p.meta.name, "applemusic-canvas");
        assert_eq!(p.meta.alias, ["applemusic", "applecanvas"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        // `applemusic` juga nama `plugins/search/applemusic.js`; nama
        // didahulukan (`plugins.js:539`).
        assert_eq!(oktz_plugin_api::find("applemusic").unwrap().meta.name, "applemusic");
        assert_eq!(oktz_plugin_api::find("applecanvas").unwrap().meta.name, "applemusic-canvas");
    }
}
