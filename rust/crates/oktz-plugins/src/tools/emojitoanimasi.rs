//! `plugins/tools/emojitoanimasi.js` — konversi emoji ke sticker animasi lewat Neoxr.
//!
//! `ponytail:` `contextInfo: saluranCtx()` (`src/lib/context.js`) tidak tersedia
//! di `Reply::Sticker`, dan reaksi ganda JS (`🎭` lalu `✅`) disatukan jadi satu
//! reaksi akhir (`✅`) seperti `phisingataubukan` (`with_reaction` hanya satu
//! emoji). Saat `contextInfo` tersedia di `Reply`, pasang ulang.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

const API: &str = "https://api.neoxr.eu/api/emojito";

/// `apiUrl` (`emojitoanimasi.js:34`).
pub(crate) fn api_url(emoji: &str, apikey: &str) -> String {
    format!("{API}?q={}&apikey={}", encode_uri_component(emoji), apikey)
}

/// `!data?.status || !data?.data?.url` (`emojitoanimasi.js:37`).
pub(crate) fn respons_valid(data: &Value) -> bool {
    crate::common::js_truthy(data.get("status"))
        && data
            .get("data")
            .and_then(|d| d.get("url"))
            .is_some_and(|v| matches!(v, Value::String(s) if !s.is_empty()))
}

/// Teks gagal (`emojitoanimasi.js:39`).
pub(crate) const GAGAL: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Emoji tidak ditemukan atau API error";

/// Kartu pakai (`emojitoanimasi.js:23-28`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎭 *ᴇᴍᴏᴊɪ ᴛᴏ ᴀɴɪᴍᴀsɪ*\n\n\
         > Konversi emoji ke sticker animasi\n\n\
         *Contoh:*\n\
         > `{prefix}emojitoanimasi 😳`"
    )
}

#[plugin(
    name = "emojitoanimasi",
    alias = ["emoji2sticker", "emojisticker", "e2s"],
    category = "tools",
    description = "Konversi emoji ke sticker animasi",
    usage = ".emojitoanimasi <emoji>",
    example = ".emojitoanimasi 😳",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn emojitoanimasi(ctx: Ctx) -> Reply {
    let emoji = ctx.text.trim();
    if emoji.is_empty() {
        return Reply::text(pakai(&ctx.prefix));
    }

    let apikey = ctx.config().api_keys.neoxr.clone();
    let Ok(res) = ctx.http.get(&api_url(emoji, &apikey)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);
    if !respons_valid(&data) {
        return Reply::text(GAGAL).with_reaction("❌");
    }

    let webp_url = data["data"]["url"].as_str().unwrap_or("");
    let Ok(webp_res) = ctx.http.get(webp_url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢");
    };
    let bytes = webp_res.body.bytes().to_vec();

    // `contextInfo: saluranCtx()` tidak tersedia di `Reply::Sticker` —
    // dicatat di `ponytail:` di atas.
    Reply::sticker(MediaSource::bytes(bytes)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    /// Stub dengan antrean respons: panggilan pertama JSON, berikutnya byte.
    #[derive(Debug, Default)]
    struct Stub {
        antrean: Mutex<Vec<Result<String, ()>>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                antrean: Mutex::new(vec![Ok(v.to_string())]),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn webp(bytes: &[u8]) -> Arc<Stub> {
            Arc::new(Stub {
                antrean: Mutex::new(vec![
                    Ok(String::new()), // placeholder, diganti handler uji
                    Ok(String::from_utf8_lossy(bytes).to_string()),
                ]),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                antrean: Mutex::new(vec![Err(())]),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let mut antrean = self.antrean.lock().unwrap();
            let isi = antrean.remove(0);
            match isi {
                Ok(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.into_bytes()),
                        mimetype: None,
                    },
                    headers: vec![],
                }),
                Err(()) => Err(oktz_plugin_api::HttpError {
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

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "emojitoanimasi".into(),
            http,
            ..Ctx::default()
        };
        emojitoanimasi(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("emojitoanimasi").unwrap();
        assert_eq!(p.meta.name, "emojitoanimasi");
        assert_eq!(
            p.meta.alias,
            ["emoji2sticker", "emojisticker", "e2s"]
        );
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn tanpa_emoji_tampilkan_pakai_tanpa_reaksi() {
        let http = Stub::offline();
        let r = run(http, "");
        assert_eq!(teks(&r), pakai("."));
        assert_eq!(reaksi(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn api_url_mengemudikan_emoji_dan_masukkan_kunci() {
        assert_eq!(
            api_url("😳", "KUNCI"),
            "https://api.neoxr.eu/api/emojito?q=%F0%9F%98%B3&apikey=KUNCI"
        );
    }

    #[test]
    fn respons_tanpa_status_atau_url_gagal_dengan_reaksi_batsu() {
        for body in [
            serde_json::json!({}),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": {} }),
            serde_json::json!({ "status": false, "data": { "url": "https://x.webp" } }),
        ] {
            let http = Stub::json(body.clone());
            let r = run(http, "😳");
            assert_eq!(teks(&r), GAGAL, "body {body}");
            assert_eq!(reaksi(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn transport_gagal_error_template_dan_reaksi_sil() {
        let http = Stub::offline();
        let r = run(http, "😳");
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "emojitoanimasi")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn sukses_mengembalikan_sticker_dengan_byte_dan_reaksi_centang() {
        let http = Stub::webp(b"WEBP");
        let r = run(http.clone(), "😳");
        assert_eq!(reaksi(&r), Some("✅"));
        let isi = match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref().clone(),
            other => panic!("harus Reaction, dapat {other:?}"),
        };
        match isi {
            Reply::Sticker { media } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::Bytes {
                        data: Arc::from(b"WEBP".to_vec()),
                        mimetype: None,
                    })
                );
            }
            other => panic!("harus Sticker, dapat {other:?}"),
        }
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2);
        assert!(seen[0].contains("/api/emojito?q="));
        assert!(seen[0].contains("apikey="));
    }

    #[test]
    fn respons_valid_miripkan_js() {
        assert!(!respons_valid(&serde_json::json!({})));
        assert!(!respons_valid(&serde_json::json!({ "status": true })));
        assert!(!respons_valid(&serde_json::json!({ "status": true, "data": {} })));
        assert!(respons_valid(&serde_json::json!({ "status": true, "data": { "url": "https://x.webp" } })));
        assert!(!respons_valid(&serde_json::json!({ "status": true, "data": { "url": "" } })));
        assert!(!respons_valid(&serde_json::json!({ "status": false, "data": { "url": "https://x.webp" } })));
    }
}
