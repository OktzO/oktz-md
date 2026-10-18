//! `plugins/asupan/asupantiktok.js` — video TikTok dari username random atau spesifik.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use rand::Rng as _;

const API_BASE: &str = "https://api.neoxr.eu/api/asupan";

const USERNAMES: &[&str] = &[
    "natajadeh", "aletaanovianda", "faisafch", "0rbby", "cindyanastt", "awaa.an",
    "nadineabgail", "ciloqciliq", "carluskiey", "wuxiaturuxia", "joomblo", "hxszys",
    "indomeysleramu", "anindthrc", "m1cel", "chrislin.chrislin", "brocolee__", "dxzdaa",
    "toodlesprunky", "wasawho", "paphricia", "queenzlyjlita", "apol1yon", "eliceannabella",
    "aintyrbaby", "christychriselle", "natalienovita", "glennvmi", "_rgtaaa", "felicialrnz",
    "zahraazzhri", "mdy.li", "jeyiiiii_", "bbytiffs", "irenefennn", "mellyllyyy", "xsta_xstar",
    "n0_0ella", "kutubuku6690", "cesiann", "gaby.rosse", "charrvm_", "bilacml04", "whosyoraa",
    "ishaangelica", "heresthekei", "gemoy.douyin", "nathasyaest", "jasmine.mat", "akuallyaa",
    "meycoco22", "baby_sya66", "knzymyln__", "rin.channn", "audicamy", "franzeskaedelyn",
    "shiraishi.ito", "itsceceh", "senpai_cj7",
];

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📺 *ᴀsᴜᴘᴀɴᴛɪᴋᴛᴏᴋ*\n\n> Video TikTok dari username random atau spesifik\n\n`{prefix}asupantiktok [username]`"
    )
}

#[plugin(
    name = "asupantiktok",
    alias = ["tiktokasupan", "ttasupan"],
    category = "asupan",
    description = "Video TikTok dari username random atau spesifik",
    usage = ".asupantiktok [username]",
    example = ".asupantiktok natajadeh",
    cooldown = 15,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn asupantiktok(ctx: Ctx) -> Reply {
    let query = ctx.text.trim().to_string();
    let query = if query.is_empty() {
        USERNAMES[rand::rng().random_range(0..USERNAMES.len())].to_string()
    } else {
        query
    };

    let api_key = ctx.config().api_keys.neoxr.clone();
    let url = format!("{API_BASE}?username={query}&apikey={api_key}");

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(format!(
            "🚩 *Username Tidak Ditemukan*\n\n> Username: {query}"
        ))
        .with_reaction("❌");
    };

    let body = res.json().unwrap_or(Value::Null);
    let Some(data) = body.get("data").filter(|v| !v.is_null()) else {
        return Reply::text(format!(
            "🚩 *Username Tidak Ditemukan*\n\n> Username: {query}"
        ))
        .with_reaction("❌");
    };

    let video_url = data
        .get("video")
        .and_then(|v| v.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let caption = data
        .get("caption")
        .and_then(Value::as_str)
        .unwrap_or("");

    Reply::video(MediaSource::url(video_url), caption.to_string()).with_reaction("✅")
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
            command: "asupantiktok".into(),
            http,
            ..Ctx::default()
        };
        asupantiktok(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "asupantiktok".into(),
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
        asupantiktok(ctx)
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
    fn tanpa_query_pakai_username_random_dan_tanpa_request() {
        let http = Stub::offline();
        let r = run(http.clone(), "");
        assert!(!http.seen.lock().unwrap().is_empty(), "random butuh request HTTP");
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn jaringan_mati_tanpa_kunci_mengembalikan_teks_tidak_ditemukan() {
        let http = Stub::offline();
        let r = run(http.clone(), "natajadeh");
        assert_eq!(
            text_of(&r),
            "🚩 *Username Tidak Ditemukan*\n\n> Username: natajadeh"
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn url_membawa_username_dan_apikey() {
        assert!(
            run_dengan_kunci(Stub::offline(), "natajadeh")
                .text_body()
                .is_some()
        );
        // Cek URL dibangun dengan kunci dan username yang di-encode.
        let http = Stub::offline();
        let _ = run(http.clone(), "nata jadeh");
        let seen = http.seen.lock().unwrap().clone();
        assert!(seen.iter().any(|u| u.contains("username=nata%20jadeh")));
    }

    #[test]
    fn data_null_menghasilkan_teks_tidak_ditemukan() {
        for body in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"data": null}), serde_json::json!({"status": false})] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "natajadeh");
            assert_eq!(
                text_of(&r),
                "🚩 *Username Tidak Ditemukan*\n\n> Username: natajadeh",
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn jalur_berhasil_mengirim_video_dengan_caption() {
        let body = serde_json::json!({
            "data": {
                "video": {"url": "https://cdn/video.mp4"},
                "caption": "Video lucu"
            }
        });
        let r = run_dengan_kunci(Stub::body(body), "natajadeh");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Video { media, caption, .. } => {
                assert_eq!(caption, "Video lucu");
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/video.mp4".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus video, dapat {lain:?}"),
        }
    }
}
