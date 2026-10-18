//! `plugins/random/quotesimage.js` — random quotes image.
//!
//! Catatan: `mod.rs` kategori `random` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod quotesimage;` perlu ditambahkan
//! manual di `rust/crates/oktz-plugins/src/random/mod.rs`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

#[plugin(
    name = "quotesimage",
    alias = ["quoteimg", "quotes-image", "qimg"],
    category = "random",
    description = "Random quotes image",
    usage = ".quotesimage",
    example = ".quotesimage",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn quotesimage(ctx: Ctx) -> Reply {
    let api_key = ctx.config().api_keys.neoxr.clone();
    let url = format!(
        "https://api.neoxr.eu/api/quotesimage?apikey={api_key}"
    );

    let res = match ctx.http.get(&url) {
        Ok(r) => r,
        Err(_) => {
            return Reply::text("❌ Gagal mengambil quotes image");
        }
    };

    let data = match res.json() {
        Ok(v) => v,
        Err(_) => {
            return Reply::text("❌ Gagal mengambil quotes image");
        }
    };

    let status = data.get("status").and_then(Value::as_bool).unwrap_or(false);
    let url_gambar = data
        .get("data")
        .and_then(|d| d.get("url").and_then(Value::as_str))
        .unwrap_or("");

    if !status || url_gambar.is_empty() {
        return Reply::text("❌ Gagal mengambil quotes image");
    }

    Reply::image(MediaSource::url(url_gambar), "")
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    struct Stub {
        body: Option<Value>,
    }

    impl Stub {
        fn ok(url: &str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(serde_json::json!({ "status": true, "data": { "url": url } })),
            })
        }
        fn fail() -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(serde_json::json!({ "status": false })),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub { body: None })
        }
    }

    impl oktz_plugin_api::HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            match &self.body {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.to_string().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError { message: "offline".into() }),
            }
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "quotesimage".into(),
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    neoxr: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("quotesimage").unwrap();
        assert_eq!(p.meta.name, "quotesimage");
        assert_eq!(
            p.meta.alias,
            ["quoteimg", "quotes-image", "qimg"]
        );
        assert_eq!(p.meta.category, "random");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["quoteimg", "quotes-image", "qimg"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "quotesimage"
            );
        }
    }

    #[test]
    fn sukses_mengembalikan_image() {
        let mut c = ctx();
        c.http = Stub::ok("https://example.com/quote.jpg");
        let r = quotesimage(c);
        match r.without_mutations() {
            Reply::Image { media, .. } => {
                assert_eq!(media.as_ref().unwrap(), &MediaSource::url("https://example.com/quote.jpg"));
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
    }

    #[test]
    fn status_false_mengembalikan_error() {
        let mut c = ctx();
        c.http = Stub::fail();
        let r = quotesimage(c);
        assert!(text_of(&r).contains("❌ Gagal mengambil quotes image"));
    }

    #[test]
    fn jaringan_gagal_mengembalikan_error() {
        let mut c = ctx();
        c.http = Stub::offline();
        let r = quotesimage(c);
        assert!(text_of(&r).contains("❌ Gagal mengambil quotes image"));
    }
}

use serde_json::Value;
