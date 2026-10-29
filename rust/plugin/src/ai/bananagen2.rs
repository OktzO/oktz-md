//! `plugins/ai/bananagen2.js` — buat gambar dari prompt lewat YuuLabs Flux.
//!
//! Scraper `fluxImage` (`src/scraper/seaart.js:72-91`) adalah satu POST JSON
//! biasa, jadi ditulis inline di sini; `live3d` (dipakai `to3d`,
//! `toanime`, `toblack`) tidak — lihat laporan batch.
//!
//! `ponytail:` rantai `error.response.data.message || error.response.data.error`
//! di `bananagen2.js:44-45` tidak bisa terjadi: `UreqHttp` mengubah status
//! `>= 400` jadi `Err` tanpa body. Yang tersisa `error.message`, persis
//! seperti `src/lib/axios`/`UreqHttp` sudah dipakai plugin lain di folder ini.

use oktz_plugin_api::{Ctx, HttpGet, MediaSource, Reply, plugin};
use serde_json::{Value};

use crate::common::js_truthy;

/// `fluxImage` (`src/scraper/seaart.js:77`).
const ENDPOINT: &str = "https://api.yuulabs.web.id/api/ai/flux-img";

/// `Err` membawa pesan yang akan tampil: `data.message || data.error ||
/// "Gagal membuat gambar"` (`seaart.js:86`), atau pesan `HttpGet` kalau
/// kegagalannya di transport.
fn flux_image(http: &dyn HttpGet, prompt: &str) -> Result<String, String> {
    let body = serde_json::json!({ "message": prompt, "ratio": "1:1" });
    let res = http.post_json(ENDPOINT, &body).map_err(|e| e.message)?;
    let data: Value = res.json().unwrap_or(Value::Null);

    let url = data
        .get("result")
        .and_then(|r| r.get("url"))
        .filter(|u| js_truthy(Some(u)))
        .and_then(Value::as_str)
        .map(str::to_string);
    match url {
        Some(url) if js_truthy(data.get("status")) => Ok(url),
        _ => {
            let msg = ["message", "error"]
                .into_iter()
                .find_map(|k| data.get(k).filter(|v| js_truthy(Some(v))))
                .map_or("Gagal membuat gambar", |v| {
                    v.as_str().unwrap_or("Gagal membuat gambar")
                })
                .to_string();
            Err(msg)
        }
    }
}

#[plugin(
    name = "bananagen2",
    alias = [],
    category = "ai",
    description = "Buat gambar dengan AI menggunakan prompt",
    usage = ".bananagen2 <prompt>",
    example = ".bananagen2 make it anime style",
    cooldown = 30,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bananagen2(ctx: Ctx) -> Reply {
    // `bananagen2.js:20` memakai `m.text` apa adanya.
    let prompt = ctx.text.clone();
    if prompt.is_empty() {
        return Reply::text(format!(
            "🍌 *BANANA GEN 2*\n\n> Buat gambar dengan AI\n\n`Contoh: {}bananagen2 make a cat`",
            ctx.prefix
        ));
    }

    match flux_image(&*ctx.http, &prompt) {
        // `sock.sendMedia(m.chat, imageUrl, null, m, { type: "image" })`.
        Ok(url) => Reply::image(MediaSource::url(url), "").with_reaction("✅"),
        Err(msg) => Reply::text(format!(
            "🍀 *Waduhh, sepertinya ini ada kendala*\n\n{msg}\n\nSilahkan coba lagi nanti, dimohon jangan spam"
        ))
        .with_reaction("❌"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        resp: Option<String>,
        seen: Mutex<Vec<(String, String)>>,
    }

    impl Stub {
        fn ok(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                resp: None,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn respond(&self) -> Result<HttpResponse, HttpError> {
            let Some(text) = &self.resp else {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.clone().into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen
                .lock()
                .unwrap()
                .push((url.into(), body.to_string()));
            self.respond()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bananagen2".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("bananagen2").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bananagen2").unwrap();
        assert_eq!(p.meta.name, "bananagen2");
        assert!(p.meta.alias.is_empty(), "alias di JS kosong");
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Buat gambar dengan AI menggunakan prompt"
        );
        assert_eq!(p.meta.usage, ".bananagen2 <prompt>");
        assert_eq!(p.meta.example, ".bananagen2 make it anime style");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn an_empty_prompt_shows_the_card_without_calling_the_api() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("🍌 *BANANA GEN 2*"), "{t}");
        assert!(t.contains(".bananagen2 make a cat"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_ratio_is_hardcoded_to_one_by_one() {
        let http = Stub::ok(r#"{"status":true,"result":{"url":"https://cdn/y.png"}}"#);
        let _ = run(http.clone(), "make a cat");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["ratio"], "1:1");
        assert_eq!(sent["message"], "make a cat");
    }

    #[test]
    fn a_result_url_becomes_an_image_with_no_caption() {
        let http = Stub::ok(r#"{"status":true,"result":{"url":"https://cdn/y.png"}}"#);
        let r = run(http.clone(), "kucing");
        match r.without_mutations() {
            Reply::Reaction { emoji, reply } => {
                assert_eq!(emoji, "✅");
                match reply.as_ref() {
                    Reply::Image { caption, .. } => assert!(caption.is_empty(), "{caption}"),
                    other => panic!("harus Image, dapat {other:?}"),
                }
            }
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(matches!(r.media_source(), Some(MediaSource::Url { .. })));
    }

    #[test]
    fn the_api_error_message_is_preferred_over_the_generic_one() {
        let r = run(
            Stub::ok(r#"{"status":false,"message":"prompt ditolak","result":{}}"#),
            "kucing",
        );
        assert_eq!(
            body(&r),
            "🍀 *Waduhh, sepertinya ini ada kendala*\n\nprompt ditolak\n\nSilahkan coba lagi nanti, dimohon jangan spam"
        );
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "❌"
        ));
    }

    #[test]
    fn the_second_error_field_is_used_when_message_is_absent() {
        let r = run(
            Stub::ok(r#"{"status":false,"error":"server keberatan"}"#),
            "kucing",
        );
        assert!(body(&r).contains("server keberatan"), "{}", body(&r));
    }

    #[test]
    fn a_rejected_payload_without_any_message_falls_back_to_the_js_wording() {
        for payload in [r#"{"status":false}"#, r#"{"status":true}"#, "bukan json"] {
            let r = run(Stub::ok(payload), "kucing");
            assert!(
                body(&r).contains("Gagal membuat gambar"),
                "{payload}: {}",
                body(&r)
            );
        }
    }

    #[test]
    fn a_transport_failure_shows_the_transport_message() {
        let r = run(Stub::offline(), "kucing");
        assert!(body(&r).contains("offline"), "{}", body(&r));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "❌"
        ));
    }
}
