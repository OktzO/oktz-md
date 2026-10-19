//! `plugins/tools/emojitoimage.js` — konversi emoji ke gambar HD via API Neoxr.
//!
//! Catatan: `pub mod emojitoimage;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/tools/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! Kunci `config.APIkey.neoxr` dibaca dari `ctx.config()`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

const STYLES: [&str; 7] = ["apple", "google", "microsoft", "samsung", "whatsapp", "twitter", "facebook"];

fn help(prefix: &str) -> String {
    format!(
        "🖼️ *ᴇᴍᴏᴊɪ ᴛᴏ ɪᴍᴀɢᴇ*\n\n\
         > Konversi emoji ke gambar HD\n\n\
         *Format:*\n\
         > `{prefix}emojitoimage <emoji> [style]`\n\n\
         *Contoh:*\n\
         > `{prefix}emojitoimage 😳 apple`\n\n\
         *Style tersedia:*\n\
         > {}",
        STYLES.join(", ")
    )
}

fn api_url(emoji: &str, style: &str, api_key: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/emoimg?q={}&style={}&apikey={}",
        crate::tools::phisingataubukan::encode_uri_component(emoji),
        style,
        api_key
    )
}

/// `String(v)` untuk nilai JSON yang masuk ke template literal — sama seperti
/// `ytstalk.js`: objek/array jadi `"[object Object]"`.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

#[plugin(
    name = "emojitoimage",
    alias = ["emoji2img", "emojiimg", "e2i"],
    category = "tools",
    description = "Konversi emoji ke gambar HD (style Apple)",
    usage = ".emojitoimage <emoji> [style]",
    example = ".emojitoimage 😳 apple",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn emojitoimage(ctx: Ctx) -> Reply {
    let emoji = ctx.args.first().cloned().unwrap_or_default();
    if emoji.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let style = match ctx.args.get(1) {
        Some(s) if STYLES.contains(&s.to_ascii_lowercase().as_str()) => s.to_ascii_lowercase(),
        _ => "apple".to_string(),
    };

    let api_key = ctx.config().api_keys.neoxr.clone();
    let Ok(res) = ctx.http.get(&api_url(&emoji, &style, &api_key)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);

    let status_ok = body.get("status").is_some_and(crate::common::js_truthy_value);
    let url = body
        .get("data")
        .and_then(|d| d.get("url"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if !status_ok || url.is_none() {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Emoji tidak ditemukan atau API error")
            .with_reaction("❌");
    }
    let url = url.unwrap();

    // `${data.data.code || "-"}` (`emojitoimage.js:65`): nilai falsy JS jadi `-`.
    let code_val = body
        .get("data")
        .and_then(|d| d.get("code"))
        .cloned()
        .unwrap_or(Value::Null);
    let code = if crate::common::js_truthy_value(&code_val) {
        js_str(&code_val)
    } else {
        "-".to_string()
    };
    let caption = format!(
        "🖼️ *ᴇᴍᴏᴊɪ ᴛᴏ ɪᴍᴀɢᴇ*\n\n> Emoji: {emoji}\n> Style: {style}\n> Code: {code}"
    );

    Reply::image(MediaSource::url(url), caption).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
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
            command: "emojitoimage".into(),
            http,
            ..Ctx::default()
        };
        emojitoimage(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "emojitoimage".into(),
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
        emojitoimage(ctx)
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
    fn tanpa_emoji_menghasilkan_kartu_pakai() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_tetap_dipakai_di_url() {
        assert_eq!(
            api_url("😳", "apple", ""),
            "https://api.neoxr.eu/api/emoimg?q=%F0%9F%98%B3&style=apple&apikey="
        );
    }

    #[test]
    fn style_salah_jadi_apple() {
        // `emojitoimage.js:32,47` — style di luar STYLES jatuh ke "apple"
        // sebelum URL dibangun; `api_url` cuma memformat nilai yang diberi.
        let http = Stub::offline();
        let r = run_dengan_kunci(http.clone(), "😳 blabla");
        // Stub offline: permintaan tetap dikirim, balasan jatuh ke error template.
        assert_eq!(reaction_of(&r), Some("☢"));
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.neoxr.eu/api/emoimg?q=%F0%9F%98%B3&style=apple&apikey=KUNCI"]
        );
    }

    #[test]
    fn style_valid_dipakai_apa_adanya() {
        assert_eq!(
            api_url("😳", "samsung", "KUNCI"),
            "https://api.neoxr.eu/api/emoimg?q=%F0%9F%98%B3&style=samsung&apikey=KUNCI"
        );
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_dengan_caption() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "url": "https://cdn/emo.jpg",
                "code": 200
            }
        });
        let r = run_dengan_kunci(Stub::body(body.clone()), "😳 apple");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert!(caption.contains("Emoji: 😳"));
                assert!(caption.contains("Style: apple"));
                assert!(caption.contains("Code: 200"));
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/emo.jpg".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn status_false_atau_data_kosong_menghasilkan_gagal() {
        for body in [
            serde_json::json!({ "status": false, "data": { "url": "x" } }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": null }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "😳");
            assert_eq!(text_of(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Emoji tidak ditemukan atau API error");
            assert_eq!(reaction_of(&r), Some("❌"));
        }
    }

    #[test]
    fn kegagalan_jaringan_membungkus_error_dan_reaksi_sil() {
        let r = run_dengan_kunci(Stub::offline(), "😳");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.emojitoimage` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
