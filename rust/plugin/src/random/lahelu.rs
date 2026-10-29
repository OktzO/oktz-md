//! `plugins/random/lahelu.js` — random gambar lahelu.
//!
//! Catatan: `mod.rs` kategori `random` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod lahelu;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/random/mod.rs`.
//!
//! `encode_uri_component` diimpor dari `crate::tools::phisingataubukan`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

#[plugin(
    name = "lahelu",
    alias = [],
    category = "random",
    description = "Random gambar lahelu",
    usage = ".lahelu",
    example = ".lahelu",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn lahelu(ctx: Ctx) -> Reply {
    let api_key = ctx.config().api_keys.cuki.clone();
    if api_key.is_empty() {
        return Reply::text(
            "❌ *API Key belum diset!*\n\n\
             > Isi `APIKEY_CUKI` di file `.env`, lalu restart bot",
        );
    }

    let url = format!(
        "https://api.cuki.biz.id/api/random/lahelu?apikey={}",
        encode_uri_component(&api_key)
    );

    let res = match ctx.http.get(&url) {
        Ok(r) => r,
        Err(_) => {
            return Reply::text(
                "☢ Terjadi kesalahan saat mengambil data lahelu.",
            )
        }
    };

    let data = match res.json() {
        Some(v) => v,
        None => {
            return Reply::text("❌ Gagal mengambil quotes image");
        }
    };

    let arr = match data.get("data").and_then(Value::as_array) {
        Some(a) if !a.is_empty() => a,
        _ => {
            return Reply::text("❌ Gagal mengambil quotes image");
        }
    };

    let idx = crate::common::random_index(arr.len());
    let random = &arr[idx];
    let media = random
        .get("media")
        .and_then(Value::as_str)
        .unwrap_or("");
    let title = random
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("");

    if media.contains(".mp4") {
        Reply::video(MediaSource::url(media), title)
    } else {
        Reply::image(MediaSource::url(media), title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::Arc;

    #[derive(Debug)]
    struct Stub {
        body: Option<Value>,
    }

    impl Stub {
        fn with(items: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(serde_json::json!({ "data": items })),
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
            command: "lahelu".into(),
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    cuki: "KUNCI".into(),
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
        let p = oktz_plugin_api::find("lahelu").unwrap();
        assert_eq!(p.meta.name, "lahelu");
        assert_eq!(p.meta.alias, [] as [&str; 0]);
        assert_eq!(p.meta.category, "random");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn api_key_kosong_mengembalikan_error() {
        let mut c = ctx();
        c.config = Arc::new(oktz_plugin_api::config::Runtime::default());
        let r = lahelu(c);
        assert!(text_of(&r).contains("API Key belum diset"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn jaringan_gagal_mengembalikan_error() {
        let mut c = ctx();
        c.http = Stub::offline();
        let r = lahelu(c);
        assert!(text_of(&r).contains("kesalahan"));
    }

    #[test]
    fn data_kosong_mengembalikan_error() {
        let mut c = ctx();
        c.http = Stub::with(serde_json::json!([]));
        let r = lahelu(c);
        assert!(text_of(&r).contains("Gagal"));
    }

    #[test]
    fn encode_uri_component_mengikuti_js() {
        assert_eq!(encode_uri_component("a b"), "a%20b");
        assert_eq!(encode_uri_component("a+b"), "a%2Bb");
    }
}
