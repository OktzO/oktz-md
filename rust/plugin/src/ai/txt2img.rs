//! `plugins/ai/txt2img.js` — generate gambar dari teks via `api.neoxr.eu` (nama
//! command JS: `text2img3`).
//!
//! `ponytail:` JS memakai `f()` dari `src/lib/http.js` yang tidak pernah
//! melempar (status >= 400 kembali `null`); di sini `ctx.http.get` mengembalikan
//! `Err` untuk status >= 400, jadi cabang gagal ditangani lewat `Err`/null.
//! Satu reaksi akhir (`❌`/`✅`/`☢`) dipasang.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;
use oktz_plugin_api::HttpError;

const API: &str = "https://api.neoxr.eu/api/stablediff";

const STYLES: &[&str] = &[
    "photorealistic",
    "digital-art",
    "impressionist",
    "anime",
    "fantasy",
    "sci-fi",
    "vintage",
];

fn help(prefix: &str) -> String {
    format!(
        "🎨 *ᴛᴇxᴛ ᴛᴏ ɪᴍᴀɢᴇ*\n\n\
         > Generate gambar dari teks dengan AI\n\n\
         `Contoh: {prefix}txt2img beautiful sunset | anime`\n\n\
         🎭 *sᴛʏʟᴇs*\n\
         > `{}`",
        STYLES.join(", ")
    )
}

/// `❌ *API Key belum diset!* ...` (mirip `explainFailure(MissingApiKeyError)`).
fn missing_key(prefix: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{prefix}txt2img` butuh `APIKEY_NEOXR` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

#[plugin(
    name = "text2img3",
    alias = [],
    category = "ai",
    description = "Generate gambar dari teks dengan AI",
    usage = ".txt2img <prompt> | <style>",
    example = ".txt2img beautiful sunset | anime",
    cooldown = 30,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn txt2img(ctx: Ctx) -> Reply {
    let input = ctx.args.join(" ");
    if input.trim().is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let prompt = input.split('|').next().unwrap_or("").trim().to_string();

    let api_key = ctx.config().api_keys.neoxr.clone();
    if api_key.is_empty() {
        return Reply::text(missing_key(&ctx.prefix)).with_reaction("❌");
    }

    let url = format!(
        "{API}?prompt={}&model=default&orientation=potrait&apikey={}",
        encode_uri_component(&prompt),
        api_key
    );

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);

    let Some(url_hasil) = data.get("data").and_then(|d| d.get("url")).and_then(Value::as_str)
    else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    Reply::image(MediaSource::url(url_hasil.to_string()), "").with_reaction("✅")
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
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

    impl oktz_plugin_api::HttpGet for Stub {
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
            command: "txt2img".into(),
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
        txt2img(ctx)
    }

    fn run_tanpa_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "txt2img".into(),
            http,
            ..Ctx::default()
        };
        txt2img(ctx)
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
    fn tanpa_input_menghasilkan_kartu_pakai_dan_style_list() {
        let r = run_tanpa_kunci(Stub::offline(), "");
        assert!(text_of(&r).starts_with("🎨 *ᴛᴇxᴛ ᴛᴏ ɪᴍᴀɢᴇ*"), "{:?}", text_of(&r));
        assert!(text_of(&r).contains("photorealistic"));
        assert!(text_of(&r).contains("vintage"));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_menghasilkan_missing_key_dan_reaksi_silang() {
        let http = Stub::offline();
        let r = run(http.clone(), "sunset");
        assert_eq!(text_of(&r), missing_key("."));
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn prompt_diambil_di_belakang_pipe_dan_terencode() {
        let http = Stub::body(serde_json::json!({ "data": { "url": "https://x/hasil.png" } }));
        let r = run(http.clone(), "indah sekali | anime");
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains("prompt=indah%20sekali"), "{:?}", seen[0]);
        assert!(!seen[0].contains("anime"), "style tidak ikut ke prompt: {:?}", seen[0]);
    }

    #[test]
    fn data_tanpa_url_menghasilkan_error_template() {
        for body in [
            serde_json::json!({}),
            serde_json::json!({ "data": {} }),
            serde_json::json!({ "data": null }),
            Value::Null,
        ] {
            let r = run(Stub::body(body.clone()), "sunset");
            assert_eq!(
                text_of(&r),
                crate::common::error_template(".", "txt2img"),
                "body {body:?}"
            );
            assert_eq!(reaction_of(&r), Some("☢"), "body {body:?}");
        }
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_dari_data_url() {
        let http = Stub::body(serde_json::json!({ "data": { "url": "https://neoxr/gambar.jpg" } }));
        let r = run(http.clone(), "sunset");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, .. } => {
                assert_eq!(
                    media,
                    &Some(oktz_plugin_api::MediaSource::Url {
                        url: "https://neoxr/gambar.jpg".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains("apikey=KUNCI"), "{:?}", seen[0]);
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_error_template() {
        let r = run(Stub::offline(), "sunset");
        assert_eq!(text_of(&r), crate::common::error_template(".", "txt2img"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
