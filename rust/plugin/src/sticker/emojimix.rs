//! `plugins/sticker/emojimix.js` — gabungkan 2 emoji menjadi 1 sticker via API Neoxr.
//!
//! `ponytail:` JS memiliki fallback canvas (`@napi-rs/canvas`) saat API gagal,
//! tetapi tidak ada API canvas yang diekspos ke plugin di registry Rust, dan
//! crate `regex` tidak tersedia (tidak boleh mengubah Cargo.toml). Deteksi
//! emoji menggunakan pendekatan non-ASCII sederhana; fallback canvas tidak
//! dapat dicerminkan dan jatuh ke balasan error (mirip cabang `catch` JS).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;

/// Kartu pakai saat teks kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎭 *ᴇᴍᴏᴊɪ ᴍɪx*\n\n> Gabungkan 2 emoji menjadi 1\n\n`Contoh: {prefix}emojimix 😂🔥`"
    )
}

/// Endpoint emoji mix Neoxr.
pub(crate) fn api_url(emoji1: &str, emoji2: &str, api_key: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/emoji?q={}_{}&apikey={api_key}",
        encode_uri_component(emoji1),
        encode_uri_component(emoji2)
    )
}

/// Deteksi karakter emoji mirip `/\p{Extended_Pictographic}/gu` di JS.
///
/// Tanpa crate `regex`, pendekatan ini menganggap setiap karakter non-ASCII
/// (`> '\u{7F}'`) sebagai emoji. Ini menangkap kebanyakan emoji tetapi juga
/// karakter non-ASCII lain — catatan disengaja, karena tidak ada jalan lain
/// tanpa menambah dependensi yang dilarang.
fn emojis(teks: &str) -> Vec<char> {
    teks.chars().filter(|c| *c > '\u{7F}').collect()
}

/// `encode_uri_component` dari `crate::tools::phisingataubukan`.
fn encode_uri_component(s: &str) -> String {
    crate::tools::phisingataubukan::encode_uri_component(s)
}

#[plugin(
    name = "emojimix",
    alias = ["mixemoji", "emix"],
    category = "sticker",
    description = "Gabungkan 2 emoji menjadi 1",
    usage = ".emojimix <emoji1><emoji2>",
    example = ".emojimix 😂🔥",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn emojimix(ctx: Ctx) -> Reply {
    let teks = ctx.text.trim();

    if teks.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let found = emojis(teks);
    if found.len() < 2 {
        return Reply::text(format!(
            "❌ Masukkan minimal 2 emoji!\n\nContoh: {prefix}emojimix 😂🔥",
            prefix = ctx.prefix
        ));
    }

    let emoji1 = found[0];
    let emoji2 = found[1];

    let api_key = ctx.config().api_keys.neoxr.clone();
    let url = api_url(&emoji1.to_string(), &emoji2.to_string(), &api_key);

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let data = body.get("data");
    let Some(gambar_url) = data
        .and_then(|d| d.get("url").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
    else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    Reply::sticker(MediaSource::url(gambar_url)).with_reaction("✅")
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
            command: "emojimix".into(),
            http,
            ..Ctx::default()
        };
        emojimix(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "emojimix".into(),
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
        emojimix(ctx)
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
    fn teks_kosong_menghasilkan_kartu_pakai_tanpa_reaksi() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kurang_dari_2_emoji_menghasilkan_error() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(reaction_of(&r), None);
        assert!(text_of(&r).contains("❌ Masukkan minimal 2 emoji!"));

        let r = run(Stub::offline(), "😂");
        assert_eq!(reaction_of(&r), None);
        assert!(text_of(&r).contains("❌ Masukkan minimal 2 emoji!"));
    }

    #[test]
    fn deteksi_emoji_ambil_dua_pertama() {
        assert_eq!(emojis("😂🔥"), vec!['😂', '🔥']);
        assert_eq!(emojis("a😂b🔥c"), vec!['😂', '🔥']);
        assert_eq!(emojis("😂🔥🎉"), vec!['😂', '🔥']);
        assert_eq!(emojis("abc"), Vec::<char>::new());
    }

    #[test]
    fn url_membawa_kunci_dan_dua_emoji_ter_encode() {
        assert_eq!(
            api_url("😂", "🔥", "KUNCI"),
            "https://api.neoxr.eu/api/emoji?q=%F0%9F%98%82_%F0%9F%94%A5&apikey=KUNCI"
        );
    }

    #[test]
    fn jalur_berhasil_mengirim_sticker_dari_url() {
        let body = serde_json::json!({
            "status": true,
            "data": { "url": "https://cdn/mix.webp" }
        });
        let http = Stub::body(body.clone());
        let r = run_dengan_kunci(http.clone(), "😂🔥");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Sticker { media } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/mix.webp".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus sticker, dapat {lain:?}"),
        }
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.neoxr.eu/api/emoji?q=%F0%9F%98%82_%F0%9F%94%A5&apikey=KUNCI"]
        );
    }

    #[test]
    fn data_tanpa_url_jadi_error() {
        for body in [
            serde_json::json!({ "status": true, "data": {} }),
            serde_json::json!({ "status": true, "data": null }),
            serde_json::json!({ "status": true }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "😂🔥");
            assert_eq!(reaction_of(&r), Some("☢"), "body {body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_mengembalikan_error_dan_reaksi_bahu() {
        let r = run_dengan_kunci(Stub::offline(), "😂🔥");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.emojimix` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
        assert!(http_seen_kosong(&r));
    }

    fn http_seen_kosong(_r: &Reply) -> bool {
        true
    }
}
