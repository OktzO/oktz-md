//! `plugins/ai/text2img2.js` — generate image from text, balikin byte mentah.
//!
//! `ponytail:` header `User-Agent` per-request (`text2img2.js:30`) tidak bisa
//! dikirim — `HttpGet` tidak punya setter header. reacted `🕕` (`text2img2.js:23`)
//! dilewati karena `Reply::Reaction` hanya menampung satu emoji; reaksi
//! terakhir (`❌`/`✅`) yang ikut.
//!
//! `m.fullArgs` dipakai sebagai sumber prompt. `serialize.js:408` mengisinya
//! dengan `result.text`, jadi sama dengan `ctx.text`. Jalur noprefix
//! (`serialize.js:430`) menghitung ulang dari `body`, tapi hasilnya juga
//! `trim()` dari sisa command — tidak ada selisih yang terlihat di user.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::fun::tod::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

const ENDPOINT: &str = "https://api-abztech.zone.id/ai/genimg";

/// `response.data.length < 100` (`text2img2.js:34`).
const MIN_BYTES: usize = 100;

#[plugin(
    name = "text2img2",
    alias = ["genimg"],
    category = "ai",
    description = "Generate image from text using AI",
    usage = ".text2img2 <prompt>",
    example = ".text2img2 a futuristic city in mars",
    cooldown = 10,
    energi = 5,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn text2img2(ctx: Ctx) -> Reply {
    let prompt = ctx.text.clone();
    if prompt.is_empty() {
        return Reply::text(format!(
            "Silahkan masukkan prompt.\nContoh: {}{} car",
            ctx.prefix, ctx.command
        ));
    }

    let url = format!("{ENDPOINT}?text={}", encode_uri_component(&prompt));
    let data = ctx
        .http
        .get(&url)
        .map(|r| r.body.bytes().to_vec())
        .unwrap_or_default();

    // `text2img2.js:34-36` — byte terlalu pendek dianggap body bukan gambar
    // lalu dilempar, jadi masuk `catch`.
    if data.len() < MIN_BYTES {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("❌");
    }

    Reply::image(MediaSource::bytes(data), prompt).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        resp: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn ok(body: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(body),
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
            let Some(data) = &self.resp else {
                return Err(HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(data.clone()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.into());
            self.respond()
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
    }

    /// PNG palsu yang lewat ambang 100 byte.
    fn fake_png() -> Vec<u8> {
        let mut v = b"\x89PNG\r\n\x1a\n".to_vec();
        v.resize(MIN_BYTES, 0);
        v
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "text2img2".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("text2img2").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("text2img2").unwrap();
        assert_eq!(p.meta.name, "text2img2");
        assert_eq!(p.meta.alias, ["genimg"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Generate image from text using AI");
        assert_eq!(p.meta.usage, ".text2img2 <prompt>");
        assert_eq!(p.meta.example, ".text2img2 a futuristic city in mars");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 5);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["text2img2", "genimg"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("text2img2"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_prompt_asks_for_one_without_calling_the_api() {
        let http = Stub::ok(fake_png());
        let r = run(http.clone(), "");
        assert_eq!(
            body(&r),
            "Silahkan masukkan prompt.\nContoh: .text2img2 car"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_prompt_is_percent_encoded_into_the_query() {
        let http = Stub::ok(fake_png());
        let _ = run(http.clone(), "a futuristic city in mars");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?text=a%20futuristic%20city%20in%20mars")
        );
    }

    #[test]
    fn raw_bytes_are_forwarded_verbatim_with_the_prompt_as_caption() {
        let png = fake_png();
        let r = run(Stub::ok(png.clone()), "kucing");
        match r.media_source() {
            Some(MediaSource::Bytes { data, .. }) => assert_eq!(&**data, &png[..]),
            other => panic!("harus Bytes, dapat {other:?}"),
        }
        assert_eq!(body(&r), "kucing");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_body_shorter_than_one_hundred_bytes_falls_into_the_catch_branch() {
        // `text2img2.js:34` — 99 byte sudah ditolak, 100 byte lolos.
        let r = run(Stub::ok(vec![7u8; 99]), "kucing");
        assert_eq!(body(&r), error_template(".", "text2img2"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "❌"
        ));

        let r = run(Stub::ok(vec![7u8; 100]), "kucing");
        assert!(r.has_media(), "tepat 100 byte harus lolos");
    }

    #[test]
    fn an_empty_or_failed_response_also_lands_in_the_catch_branch() {
        for r in [
            run(Stub::ok(Vec::new()), "kucing"),
            run(Stub::offline(), "kucing"),
        ] {
            assert_eq!(body(&r), error_template(".", "text2img2"));
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            ));
        }
    }
}
