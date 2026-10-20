//! `plugins/tools/ssweb.js` — screenshot website lewat thum.io.
//!
//! `ponytail:` opsi `--mobile`/`--hp` dan penambahan `https://` di-port apa
//! adanya. Reaksi ganda JS (`🕕` lalu `✅`) disatukan jadi satu reaksi akhir
//! `✅`, karena `Reply::with_reaction` hanya membawa satu emoji.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const THUM: &str = "https://image.thum.io/get";

/// `width` dari mode (`ssweb.js:20-22`).
pub(crate) fn lebar(mode: &str) -> u32 {
    if mode == "mobile" {
        720
    } else {
        1920
    }
}

/// `apiUrl` (`ssweb.js:22`).
pub(crate) fn api_url(url: &str, mode: &str) -> String {
    format!("{THUM}/width/{}/crop/1080/noanimate/{}", lebar(mode), url)
}

/// Kartu pakai (`ssweb.js:34-40`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "📸 *sᴄʀᴇᴇɴsʜᴏᴛ ᴡᴇʙ*\n\n\
         > Screenshot halaman website\n\n\
         > *Contoh:*\n\
         > {prefix}ssweb https://google.com\n\
         > {prefix}ss https://github.com --mobile"
    )
}

/// Pisahkan opsi `--mobile`/`--hp` dari URL (`ssweb.js:44-47`).
pub(crate) fn pisah_mode_opsi(text: &str) -> (&str, String) {
    if text.contains("--mobile") || text.contains("--hp") {
        let bersih = text.replace("--mobile", "").replace("--hp", "").trim().to_string();
        ("mobile", bersih)
    } else {
        ("desktop", text.to_string())
    }
}

#[plugin(
    name = "ssweb",
    alias = ["screenshot", "ss", "webss"],
    category = "tools",
    description = "Screenshot website",
    usage = ".ssweb <url>",
    example = ".ssweb https://google.com",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ssweb(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    let (mode, url) = pisah_mode_opsi(text);
    let url = if url.starts_with("http") {
        url
    } else {
        format!("https://{url}")
    };

    let Ok(res) = ctx.http.get(&api_url(&url, mode)) else {
        return Reply::text(crate::common::error_template(&ctx))
            .with_reaction("☢");
    };
    let bytes = res.body.bytes().to_vec();

    Reply::image(MediaSource::bytes(bytes), String::new()).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        bytes: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(b: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                bytes: Some(b),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub::default())
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.bytes {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone()),
                        mimetype: None,
                    },
                    headers: vec![],
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "ssweb".into(),
            http,
            ..Ctx::default()
        };
        ssweb(ctx)
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
        let p = oktz_plugin_api::find("ssweb").unwrap();
        assert_eq!(p.meta.name, "ssweb");
        assert_eq!(p.meta.alias, ["screenshot", "ss", "webss"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_url_tampilkan_pakai() {
        let http = Stub::offline();
        let r = run(http.clone(), "");
        assert_eq!(teks(&r), pakai("."));
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn lebar_mode() {
        assert_eq!(lebar("mobile"), 720);
        assert_eq!(lebar("desktop"), 1920);
        assert_eq!(lebar(""), 1920);
    }

    #[test]
    fn api_url_bentuk_thum_io() {
        assert_eq!(
            api_url("https://google.com", "desktop"),
            "https://image.thum.io/get/width/1920/crop/1080/noanimate/https://google.com"
        );
        assert_eq!(
            api_url("https://github.com", "mobile"),
            "https://image.thum.io/get/width/720/crop/1080/noanimate/https://github.com"
        );
    }

    #[test]
    fn pisah_mode_opsi_hapus_mobile_dan_hp() {
        assert_eq!(
            pisah_mode_opsi("https://x.com --mobile"),
            ("mobile", "https://x.com".to_string())
        );
        assert_eq!(
            pisah_mode_opsi("https://x.com --hp"),
            ("mobile", "https://x.com".to_string())
        );
        assert_eq!(
            pisah_mode_opsi("https://x.com"),
            ("desktop", "https://x.com".to_string())
        );
    }

    #[test]
    fn url_tanpa_skema_ditambah_https() {
        let _ = pisah_mode_opsi("google.com");
        // menangani manual karena pisah_mode_opsi mengembalikan tuple
        let (_, url) = pisah_mode_opsi("google.com");
        let url = if url.starts_with("http") { url } else { format!("https://{url}") };
        assert_eq!(url, "https://google.com");
    }

    #[test]
    fn sukses_mengembalikan_gambar_dengan_reaksi_centang() {
        let http = Stub::body(b"PNG".to_vec());
        let r = run(http.clone(), "google.com");
        assert_eq!(reaksi(&r), Some("✅"));
        let isi = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref().clone(),
            other => panic!("harus Reaction, dapat {other:?}"),
        };
        match isi {
            Reply::Image { media, .. } => {
                assert_eq!(
                    media,
                    Some(MediaSource::Bytes {
                        data: Arc::from(b"PNG".to_vec()),
                        mimetype: None,
                    })
                );
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].starts_with("https://image.thum.io/get/width/1920/crop/1080/noanimate/https://google.com"));
    }

    #[test]
    fn mobile_menggunakan_lebar_720() {
        let http = Stub::body(b"PNG".to_vec());
        let _ = run(http.clone(), "google.com --mobile");
        let seen = http.seen.lock().unwrap().clone();
        assert!(seen[0].contains("/width/720/"));
    }

    #[test]
    fn transport_gagal_error_template_dan_reaksi_sil() {
        let http = Stub::offline();
        let r = run(http, "google.com");
        assert_eq!(
            teks(&r),
            crate::common::error_template_bawaan(".", "ssweb")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }
}
