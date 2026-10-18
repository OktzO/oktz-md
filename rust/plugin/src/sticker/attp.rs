//! `plugins/sticker/attp.js` — sticker teks animasi lewat API Neoxr attp3,
//! dengan fallback canvas lokal.
//!
//! `ponytail:` JS memiliki dua fallback yang tidak tersedia di registry Rust:
//! (1) canvas `@napi-rs/canvas` untuk teks animasi lokal, dan (2) `addExifToWebp`
//! dari `src/lib/exif.js`. Keduanya tidak dapat dicerminkan, sehingga saat API
//! gagal plugin jatuh ke balasan error (mirip cabang `catch` JS) dan frame
//! animasi lokal tidak dikirim.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;

/// Kartu pakai saat tidak ada teks.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎨 *ᴀɴɪᴍᴀᴛᴇᴅ ᴛᴇxᴛ sᴛɪᴄᴋᴇʀ*\n\n> Masukkan teks untuk sticker\n\n> Contoh: `{prefix}attp Hello World`"
    )
}

/// Endpoint attp3 Neoxr (warna tetap `FF5733` seperti JS).
pub(crate) fn api_url(text: &str, api_key: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/attp3?text={}&color=FF5733&apikey={api_key}",
        crate::tools::phisingataubukan::encode_uri_component(text)
    )
}

#[plugin(
    name = "attp",
    alias = ["attp3"],
    category = "sticker",
    description = "Membuat sticker animated text",
    usage = ".attp <teks>",
    example = ".attp Hello World",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn attp(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();

    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if text.len() > 100 {
        return Reply::text("❌ Teks terlalu panjang! Maksimal 100 karakter.");
    }

    let api_key = ctx.config().api_keys.neoxr.clone();
    let url = api_url(&text, &api_key);

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let data = body.get("data");
    let Some(sticker_url) = data
        .and_then(|d| d.get("url").and_then(Value::as_str))
        .filter(|s| !s.is_empty())
    else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    Reply::sticker(MediaSource::url(sticker_url)).with_reaction("✅")
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
            command: "attp".into(),
            http,
            ..Ctx::default()
        };
        attp(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "attp".into(),
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
        attp(ctx)
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
    fn teks_lebih_dari_100_karakter_menghasilkan_error_tanpa_reaksi() {
        let terlalu_panjang = "a".repeat(101);
        let r = run(Stub::offline(), &terlalu_panjang);
        assert_eq!(reaction_of(&r), None);
        assert_eq!(
            text_of(&r),
            "❌ Teks terlalu panjang! Maksimal 100 karakter."
        );
    }

    #[test]
    fn teks_persis_100_karakter_lolos_guard() {
        let teks = "a".repeat(100);
        let r = run_dengan_kunci(Stub::offline(), &teks);
        // Kunci kosong karena offline tapi config tidak terisi → Ctx::default()
        // punya api_keys kosong, tapi attp tidak memeriksa kunci sebelum GET,
        // jadi hanya akan gagal jaringan. Yang penting: tidak ada reaksi ☢ dari
        // guard panjang.
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn url_membawa_kunci_dan_text_ter_encode() {
        assert_eq!(
            api_url("Hello World", "KUNCI"),
            "https://api.neoxr.eu/api/attp3?text=Hello%20World&color=FF5733&apikey=KUNCI"
        );
        assert_eq!(
            api_url("a b", ""),
            "https://api.neoxr.eu/api/attp3?text=a%20b&color=FF5733&apikey="
        );
    }

    #[test]
    fn jalur_berhasil_mengirim_sticker_dari_url() {
        let body = serde_json::json!({
            "status": true,
            "data": { "url": "https://cdn/attp.webp" }
        });
        let http = Stub::body(body.clone());
        let r = run_dengan_kunci(http.clone(), "Hello World");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Sticker { media } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/attp.webp".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus sticker, dapat {lain:?}"),
        }
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.neoxr.eu/api/attp3?text=Hello%20World&color=FF5733&apikey=KUNCI"]
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
            let r = run_dengan_kunci(Stub::body(body.clone()), "Hello");
            assert_eq!(reaction_of(&r), Some("☢"), "body {body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_mengembalikan_error_dan_reaksi_bahu() {
        let r = run_dengan_kunci(Stub::offline(), "Hello");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.attp` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
