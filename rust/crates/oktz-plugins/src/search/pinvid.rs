//! `plugins/search/pinvid.js` — cari video Pinterest.
//!
//! `ponytail:` `config.APIkey?.firefly` tidak ada di `Ctx`, jadi kunci ikut
//! nilai bawaan `"default"` persis seperti `|| "default"` di JS saat `.env`
//! kosong (`pinvid.js:53`).
//!
//! `ponytail:` `fs`, `child_process` (`ffmpeg`), `crypto`, album WA
//! (`sock.relayMessage`), dan `config.saluran` tidak ada di API plugin:
//! konversi `.m3u8`, berkas sementara, thumbnail newsletter, dan pengiriman
//! album tidak ikut. Yang dipindah adalah guard, pemotongan daftar, ambang
//! ukuran, dan seluruh susunan pesan.
//!
//! Catatan dari audit: `firefly.maiku.my.id` — tunnel Argo-nya mati
//! (Cloudflare 530 / 1033), jadi endpoint ini praktis selalu gagal.

#![allow(dead_code)] // jalur unduh/ffmpeg tidak ada padanannya

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.firefly || "default"` (`pinvid.js:53`).
pub(crate) const FIREFLY_APIKEY: &str = "default";

/// `pinvid.js:42-47` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📌 *ᴘɪɴᴛᴇʀᴇsᴛ ᴠɪᴅᴇᴏ sᴇᴀʀᴄʜ*\n\n\
         > Masukkan query pencarian\n\n\
         `{prefix}pinvid anime`"
    )
}

/// `pinvid.js:61` — endpoint mati atau `data` kosong.
pub(crate) fn tidak_ditemukan(query: &str) -> String {
    format!("❌ Tidak ditemukan video untuk: {query}")
}

/// `pinvid.js:128` — tidak ada satu pun video yang berhasil diproses.
pub(crate) const GAGAL_UNDUH: &str = "❌ Gagal mengunduh video";

/// `pinvid.js:113` — ambang ukuran buffer video.
pub(crate) const MIN_BYTES: usize = 1000;

/// `config.saluran?.id || "120363400911374213@newsletter"` (`pinvid.js:179`).
#[allow(dead_code)]
pub(crate) const SALURAN_ID: &str = "120363400911374213@newsletter";

/// `config.saluran?.name || config.bot?.name || "Bot"` (`pinvid.js:180`).
#[allow(dead_code)]
pub(crate) const SALURAN_NAME: &str = "Bot";

/// `pinvid.js:52-57` — endpoint aggregator.
pub(crate) fn api_url(query: &str) -> String {
    format!(
        "https://firefly.maiku.my.id/api/pinterestvideo?apikey={FIREFLY_APIKEY}&q={}",
        encode_uri_component(query)
    )
}

/// `!res.data?.status || !res.data?.data?.length` (`pinvid.js:59`).
pub(crate) fn kosong(body: &Value) -> bool {
    !crate::common::js_truthy(body.get("status"))
        || body
            .get("data")
            .and_then(Value::as_array)
            .is_none_or(|a| a.is_empty())
}

/// `res.data.data.slice(0, 5)` (`pinvid.js:64`).
pub(crate) fn take_videos(body: &Value) -> Vec<Value> {
    body.get("data")
        .and_then(Value::as_array)
        .map(|a| a.iter().take(5).cloned().collect())
        .unwrap_or_default()
}

/// `videoUrl.includes(".m3u8")` menentukan jalur ffmpeg; sisanya bisa
/// diunduh langsung (`pinvid.js:85`).
pub(crate) fn perlu_ffmpeg(url: &str) -> bool {
    url.contains(".m3u8")
}

#[plugin(
    name = "pinvid",
    alias = ["pinvideo", "pinterestv", "pinv"],
    category = "search",
    description = "Search video Pinterest (album)",
    usage = ".pinvid <query>",
    example = ".pinvid anime",
    cooldown = 30,
    energi = 2,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pinvid(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `m.react("🕕")` di `:49` sudah jalan sebelum request.
    let Ok(res) = ctx.http.get(&api_url(query)) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let body = res.json().unwrap_or(Value::Null);
    if kosong(&body) {
        return Reply::text(tidak_ditemukan(query)).with_reaction("❌");
    }

    // `pinvid.js:73-124` — unduh tiap video: `.m3u8` lewat `ffmpeg` ke berkas
    // sementara, sisanya `axios.get` biasa. Keduanya butuh `child_process`
    // atau buffer yang tidak bisa dikirim lewat `Reply` satu-per-video
    // (album butuh `sock.relayMessage`), jadi `mediaList` selalu kosong.
    let _ = take_videos(&body);
    Reply::text(GAGAL_UNDUH).with_reaction("❌")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        body: Option<Value>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn with(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v),
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
            let v = self
                .body
                .clone()
                .ok_or_else(|| HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(v.to_string().into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
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
        let ctx = Ctx {
            prefix: ".".into(),
            command: "pinvid".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("pinvid").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("pinvid").unwrap();
        assert_eq!(p.meta.name, "pinvid");
        assert_eq!(p.meta.alias, ["pinvideo", "pinterestv", "pinv"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Search video Pinterest (album)");
        assert_eq!(p.meta.usage, ".pinvid <query>");
        assert_eq!(p.meta.example, ".pinvid anime");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 2);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["pinvideo", "pinterestv", "pinv"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "pinvid");
        }
    }

    #[test]
    fn an_empty_query_shows_the_help_card() {
        for text in ["", "  ", "\t\n"] {
            let t = text_of(&run(Stub::offline(), text));
            assert!(t.starts_with("📌 *ᴘɪɴᴛᴇʀᴇsᴛ ᴠɪᴅᴇᴏ sᴇᴀʀᴄʜ*"), "input {text:?}");
            assert!(t.contains("`.pinvid anime`"), "{t}");
        }
        assert!(help("!").contains("`!pinvid anime`"));
    }

    #[test]
    fn the_request_uses_the_default_key_and_encodes_the_query() {
        let http = Stub::with(json!({ "status": false }));
        let _ = run(http.clone(), "anime girls");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://firefly.maiku.my.id/api/pinterestvideo?apikey=default&q=anime%20girls"
        );
    }

    #[test]
    fn an_empty_or_failed_response_reports_not_found() {
        for body in [
            json!({ "status": false, "data": [ { "video_url": "u" } ] }),
            json!({ "status": true, "data": [] }),
            json!({ "status": true }),
            json!({}),
            json!(null),
        ] {
            let r = run(Stub::with(body.clone()), "anime");
            assert_eq!(text_of(&r), "❌ Tidak ditemukan video untuk: anime", "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"));
        }
    }

    #[test]
    fn videos_that_cannot_be_sent_end_at_the_download_failure_branch() {
        let body = json!({
            "status": true,
            "data": [ { "video_url": "https://v/1.m3u8" }, { "video_url": "https://v/2.mp4" } ],
        });
        let r = run(Stub::with(body), "anime");
        assert_eq!(text_of(&r), "❌ Gagal mengunduh video");
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "anime");
        assert_eq!(text_of(&r), error_template(".", "pinvid"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in ["<b>", "a|b", "🎬", "'\"", "a\u{202e}b", "%00"] {
            let r = run(Stub::with(json!({ "status": false })), input);
            assert!(text_of(&r).contains("Tidak ditemukan"), "input {input:?}");
        }
        let panjang = "a".repeat(5000);
        assert!(text_of(&run(Stub::offline(), &panjang)).contains("kendala"));
    }

    #[test]
    fn at_most_five_videos_are_considered() {
        let semua: Vec<Value> = (0..9)
            .map(|i| json!({ "video_url": format!("https://v/{i}.mp4") }))
            .collect();
        let videos = take_videos(&json!({ "data": semua }));
        assert_eq!(videos.len(), 5);
        assert!(take_videos(&json!({})).is_empty());
    }

    #[test]
    fn only_m3u8_urls_take_the_ffmpeg_branch() {
        assert!(perlu_ffmpeg("https://v/1.m3u8"));
        assert!(perlu_ffmpeg("https://v/master.m3u8?token=x"));
        assert!(!perlu_ffmpeg("https://v/1.mp4"));
        assert!(!perlu_ffmpeg(""));
    }

    #[test]
    fn the_newsletter_defaults_are_kept() {
        assert_eq!(SALURAN_ID, "120363400911374213@newsletter");
        assert_eq!(SALURAN_NAME, "Bot");
        assert_eq!(MIN_BYTES, 1000);
    }
}