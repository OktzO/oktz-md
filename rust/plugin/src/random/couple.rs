//! `plugins/random/couple.js` — random gambar pp couple.
//!
//! `ponytail:` JS mengirim `albumMessage` dengan `quoted: m`. `Reply::sequence`
//! tidak membawa `quoted`, dan tiap elemen sequence adalah satu `Reply` tersendiri.
//! Catatan: baris `pub mod couple;` perlu ditambah ke `random/mod.rs` secara
//! manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

#[plugin(
    name = "ppcouple",
    alias = ["cp", "ppcp"],
    category = "random",
    description = "Random gambar pp couple",
    usage = ".ppcouple",
    example = ".ppcouple",
    cooldown = 10,
    energi = 2,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn couple(ctx: Ctx) -> Reply {
    let Ok(res) = ctx.http.get("https://api.deline.web.id/random/ppcouple") else {
        return Reply::text("❌ Gagal mengambil gambar couple").with_reaction("❌");
    };
    let data = res.json().unwrap_or(Value::Null);
    let result = data.get("result");
    let cowo = result
        .and_then(|r| r.get("cowo"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let cewe = result
        .and_then(|r| r.get("cewe"))
        .and_then(Value::as_str)
        .unwrap_or("");

    if cowo.is_empty() || cewe.is_empty() {
        return Reply::text("❌ Gagal mengambil gambar couple").with_reaction("❌");
    }

    Reply::sequence(vec![
        Reply::image(MediaSource::url(cowo), String::new()),
        Reply::image(MediaSource::url(cewe), String::new()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.body {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
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

    fn run(http: Arc<Stub>) -> Reply {
        let ctx = Ctx {
            command: "ppcouple".into(),
            http,
            ..Default::default()
        };
        couple(ctx)
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("ppcouple").unwrap();
        assert_eq!(p.meta.name, "ppcouple");
        assert_eq!(p.meta.alias, ["cp", "ppcp"]);
        assert_eq!(p.meta.category, "random");
        assert!(p.meta.is_group);
    }

    #[test]
    fn jaringan_gagal_kembalikan_teks_gagal() {
        let r = run(Stub::offline());
        assert_eq!(r.text_body().expect("harus ada teks"), "❌ Gagal mengambil gambar couple");
    }

    #[test]
    fn hasil_berhasil_jadi_sequence_dua_gambar() {
        let data = serde_json::json!({
            "result": {
                "cowo": "https://example/cowo.jpg",
                "cewe": "https://example/cewe.jpg"
            }
        });
        let r = run(Stub::json(data.clone()));
        let list = match r {
            Reply::Sequence(list) => list,
            other => panic!("harus Sequence, dapat {other:?}"),
        };
        assert_eq!(list.len(), 2);
        match &list[0] {
            Reply::Image { media, caption, .. } => {
                assert_eq!(media, &Some(MediaSource::url("https://example/cowo.jpg")));
                assert_eq!(caption, "");
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
        match &list[1] {
            Reply::Image { media, .. } => {
                assert_eq!(media, &Some(MediaSource::url("https://example/cewe.jpg")));
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
    }

    #[test]
    fn result_tanpa_cowo_atau_cewe_gagal() {
        let data = serde_json::json!({ "result": { "cowo": "" } });
        let r = run(Stub::json(data));
        assert_eq!(r.text_body().expect("harus ada teks"), "❌ Gagal mengambil gambar couple");
    }
}
