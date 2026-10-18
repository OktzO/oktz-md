//! `plugins/random/meme.js` — random meme Indonesia lewat API Neoxr.
//! Satu endpoint GET, tanpa kunci yang wajib (kunci kosong tetap masuk URL).
//!
//! Perilaku JS: `m.react('🕕')` dulu, baru `try`. Karena `Reply::with_reaction`
//! hanya boleh satu emoji, reaksi yang ikut adalah yang terakhir: `✅` saat
//! sukses, `☢` saat gagal (`🕕` hilang — konvensi repo yang sama dengan
//! `instagramdl.js`).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;

/// Kartu pakai (meme tidak butuh argumen, tapi disediakan untuk konsistensi).
pub(crate) fn help(prefix: &str) -> String {
    format!("🍌 *ʀᴀɴᴅᴏᴍ ᴍᴇᴍᴇ ɪɴᴅᴏɴᴇꜱɪᴀ*\n\n`Contoh: {prefix}meme`")
}

#[plugin(
    name = "meme",
    alias = ["randommeme", "memeindonesia"],
    category = "random",
    description = "Random meme Indonesia",
    usage = ".meme",
    example = ".meme",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn meme(ctx: Ctx) -> Reply {
    let api_key = ctx.config().api_keys.neoxr.clone();
    let url = format!("https://api.neoxr.eu/api/meme?apikey={api_key}");

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let data = body.get("data");
    let Some(url_str) = data
        .and_then(|d| d.get("url").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
    else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let title = data
        .and_then(|d| d.get("title").and_then(Value::as_str))
        .unwrap_or("");

    Reply::image(MediaSource::url(url_str), title).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
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
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "meme".into(),
            http,
            ..Ctx::default()
        };
        meme(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "meme".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    neoxr: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        };
        meme(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn kegagalan_jaringan_mengembalikan_pesan_error_dan_reaksi_bahu() {
        let http = Stub::offline();
        let r = run(http.clone(), "meme");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.meme` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_membawa_kunci_neoxr() {
        let http = Stub::offline();
        run_dengan_kunci(http.clone(), "meme");
        let seen = http.seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("apikey=KUNCI"));
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_dengan_judul_caption() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "url": "https://cdn/meme.jpg",
                "title": "Meme lucu"
            }
        });
        let http = Stub::body(body.clone());
        let r = run_dengan_kunci(http.clone(), "meme");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(caption, "Meme lucu");
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/meme.jpg".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.neoxr.eu/api/meme?apikey=KUNCI"]
        );
    }

    #[test]
    fn data_tanpa_url_jadi_error() {
        for body in [
            serde_json::json!({ "status": true, "data": { "title": "x" } }),
            serde_json::json!({ "status": true, "data": null }),
            serde_json::json!({ "status": true }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "meme");
            assert_eq!(reaction_of(&r), Some("☢"), "body {body}");
        }
    }

    #[test]
    fn title_kosong_tetap_dikirim_sebagai_caption_kosong() {
        let body = serde_json::json!({ "status": true, "data": { "url": "https://cdn/x.jpg" } });
        let r = run_dengan_kunci(Stub::body(body), "meme");
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { caption, .. } => assert_eq!(caption, ""),
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }
}
