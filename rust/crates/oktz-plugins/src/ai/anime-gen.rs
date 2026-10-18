//! `plugins/ai/anime-gen.js` — generate gambar anime AI lewat `api.neoxr.eu`.
//!
//! `ponytail:` dua hal tidak ditulis di sini:
//! 1. Header `User-Agent` per-request (`http.js:30`) — `HttpGet` tidak punya
//!    setter header. `UreqHttp` sudah mengirim UA sendiri.
//! 2. Cabang `catch` (`anime-gen.js:57-64`). `f()` (`http.js`) menelan semua
//!    error dan mengembalikan `null`, jadi `data` selalu `null` saat gagal dan
//!    plugin masuk cabang "ɢᴀɢᴀʟ" di `:47`, bukan `catch`. Menulis `catch`
//!    berarti menambah jalur yang tidak pernah tercapai di JS.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey.neoxr` dibaca dari `ctx.config()`; nilai kosong di
/// produksi sama dengan `process.env.APIKEY_NEOXR || ""` di `config.js:327`.
fn url(prompt: &str, api_key: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/ai-anime?q={}&apikey={api_key}",
        encode_uri_component(prompt)
    )
}

fn help(prefix: &str) -> String {
    format!(
        "🎨 *ᴀɴɪᴍᴇ ᴀʀᴛ ɢᴇɴᴇʀᴀᴛᴏʀ*\n\n\
         > Generate gambar anime AI dari prompt!\n\n\
         *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ:*\n\
         > `{prefix}anime-gen <deskripsi>`\n\n\
         *ᴄᴏɴᴛᴏʛ:*\n\
         > `{prefix}anime-gen girl, vibrant color, smilling, yellow pink gradient hair`\n\
         > `{prefix}anime-gen boy, dark aesthetic, silver hair, red eyes`\n\n\
         *ᴛɪᴘs:*\n\
         > • Gunakan bahasa Inggris\n\
         > • Makin detail prompt, makin bagus hasil\n\
         > • Tambahkan style: vibrant, dark, pastel, etc"
    )
}

#[plugin(
    name = "anime-gen",
    alias = ["animegen", "aianimegen", "genai-anime"],
    category = "ai",
    description = "Generate AI anime art dari prompt",
    usage = ".anime-gen <prompt>",
    example = ".anime-gen girl, vibrant color, smilling",
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
pub fn anime_gen(ctx: Ctx) -> Reply {
    // `anime-gen.js:21` memakai `m.text` apa adanya, bukan `.trim()`.
    let prompt = ctx.text.clone();
    if prompt.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let url = url(&prompt, &ctx.config().api_keys.neoxr);
    // `f()` mengembalikan `null` untuk status >= 400 maupun error jaringan,
    // jadi `ctx.http.get(..).ok().and_then(|r| r.json())` meniru keduanya.
    let fail = || {
        Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal generate gambar. Coba lagi nanti!").with_reaction("❌")
    };
    let Some(data) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        return fail();
    };

    // `anime-gen.js:47` — `!data?.status || !data?.data?.url`.
    if !js_truthy(data.get("status")) || !js_truthy(data.pointer("/data/url")) {
        return fail();
    }

    // `sock.sendMedia(m.chat, result.url, null, m, { type: 'image' })`.
    let image = data
        .pointer("/data/url")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Reply::image(MediaSource::url(image), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        resp: Option<String>,
        seen: Mutex<Vec<String>>,
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

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "anime-gen".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("anime-gen").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("anime-gen").unwrap();
        assert_eq!(p.meta.name, "anime-gen");
        assert_eq!(p.meta.alias, ["animegen", "aianimegen", "genai-anime"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Generate AI anime art dari prompt");
        assert_eq!(p.meta.usage, ".anime-gen <prompt>");
        assert_eq!(p.meta.example, ".anime-gen girl, vibrant color, smilling");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["anime-gen", "animegen", "aianimegen", "genai-anime"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("anime-gen"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_prompt_shows_the_tips_card_without_calling_the_api() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("ᴀɴɪᴍᴇ ᴀʀᴛ ɢᴇɴᴇʀᴀᴛᴏʀ"), "{t}");
        assert!(t.contains(".anime-gen <deskripsi>"), "{t}");
        assert!(
            t.contains("Tambahkan style: vibrant, dark, pastel, etc"),
            "{t}"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_prompt_is_percent_encoded_into_the_query() {
        let http = Stub::ok(r#"{"status":true,"data":{"url":"https://cdn/x.png"}}"#);
        let _ = run(http.clone(), "girl, vibrant color");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://api.neoxr.eu/api/ai-anime?q=girl%2C%20vibrant%20color&apikey="
        );
    }

    #[test]
    fn a_result_url_is_sent_as_an_image_with_an_empty_caption() {
        let http = Stub::ok(r#"{"status":true,"data":{"url":"https://cdn/x.png"}}"#);
        let r = run(http.clone(), "girl");
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
        assert_eq!(
            r.media_source().map(|s| match s {
                MediaSource::Url { url, .. } => url.as_str(),
                other => panic!("harus Url, dapat {other:?}"),
            }),
            Some("https://cdn/x.png")
        );
    }

    #[test]
    fn a_missing_status_or_url_is_the_gagal_branch() {
        for payload in [
            r#"{"status":false,"data":{"url":"https://cdn/x.png"}}"#,
            r#"{"status":true,"data":{}}"#,
            r#"{"status":true}"#,
            "bukan json",
        ] {
            let r = run(Stub::ok(payload), "girl");
            assert_eq!(
                body(&r),
                "❌ *ɢᴀɢᴀʟ*\n\n> Gagal generate gambar. Coba lagi nanti!",
                "{payload}"
            );
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            ));
        }
    }

    #[test]
    fn a_transport_failure_also_lands_on_the_gagal_branch() {
        // `f()` menelan error dan mengembalikan `null` — bukan masuk `catch`.
        let r = run(Stub::offline(), "girl");
        assert_eq!(
            body(&r),
            "❌ *ɢᴀɢᴀʟ*\n\n> Gagal generate gambar. Coba lagi nanti!"
        );
    }

    #[test]
    fn an_empty_url_string_counts_as_missing_like_javascript() {
        let r = run(Stub::ok(r#"{"status":true,"data":{"url":""}}"#), "girl");
        assert!(body(&r).contains("Gagal generate gambar"), "{}", body(&r));
    }
}
