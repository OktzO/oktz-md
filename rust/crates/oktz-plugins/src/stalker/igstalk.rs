//! `plugins/stalker/igstalk.js` — stalk akun Instagram lewat API Firefly.
//! Nexray tidak punya `/stalker/instagram` (dicek live: 500), jadi tidak ada
//! fallback. Kunci `config.APIkey.firefly` dibaca dari `ctx.config()`.
//!
//! `m.react('🔍')` terjadi sebelum guard kunci, jadi pada cabang "kunci kosong"
//! reaksi terakhir yang ikut adalah `❌`. `Reply::with_reaction` hanya boleh
//! satu emoji, jadi hanya yang terakhir.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `plugins/stalker/igstalk.js` — kartu pakai saat `m.args[0]` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📸 *ɪɴsᴛᴀɢʀᴀᴍ sᴛᴀʟᴋ*\n\n> Masukkan username Instagram\n\n`Contoh: {prefix}igstalk cristiano`"
    )
}

/// `explainFailure(new MissingApiKeyError('firefly', 'APIKEY_FIREFLY'), ..)`
/// (`stalker-fallback.js:30-36`).
pub(crate) const MISSING_KEY: &str =
    "❌ *API Key belum diset!*\n\n\
     > Command `.igstalk` butuh `APIKEY_FIREFLY` di `.env`\n\
     > Saat ini belum ada, jadi tidak bisa dipanggil.";

/// Endpoint stalk Instagram.
pub(crate) fn api_url(username: &str, api_key: &str) -> String {
    format!(
        "https://firefly.maiku.my.id/api/stalk-instagram?apikey={api_key}&username={}",
        encode_uri_component(username)
    )
}

/// `shortNum(num)` (`igstalk.js:22-29`) — `parseInt`, lalu ambang miliar/jt/rb
// dengan `toFixed(1)` kemudian hapus `.0`.
fn short_num(v: Option<&Value>) -> String {
    let n = v
        .and_then(|v| v.as_i64())
        .or_else(|| v.and_then(Value::as_str).and_then(|s| s.parse::<i64>().ok()));
    let Some(n) = n else {
        return "0".to_string();
    };
    if n >= 1_000_000_000 {
        let s = format!("{:.1}", n as f64 / 1_000_000_000.0).replace(".0", "");
        return format!("{s} miliar");
    }
    if n >= 1_000_000 {
        let s = format!("{:.1}", n as f64 / 1_000_000.0).replace(".0", "");
        return format!("{s} jt");
    }
    if n >= 1_000 {
        let s = format!("{:.1}", n as f64 / 1_000.0).replace(".0", "");
        return format!("{s} rb");
    }
    n.to_string()
}

/// `igstalk.js:63-72` — caption hasil stalk.
fn caption(d: &Value, username: &str) -> String {
    let username_d = d
        .get("username")
        .and_then(Value::as_str)
        .unwrap_or(username);
    let full_name = d.get("full_name").and_then(Value::as_str).unwrap_or("-");
    let verified = if d
        .get("is_verified")
        .is_some_and(crate::common::js_truthy_value)
    {
        "Ya"
    } else {
        "Tidak"
    };
    let is_private = if d
        .get("is_private")
        .is_some_and(crate::common::js_truthy_value)
    {
        "Ya"
    } else {
        "Tidak"
    };
    let stats = d.get("stats");
    let followers = short_num(stats.and_then(|s| s.get("followers")));
    let following = short_num(stats.and_then(|s| s.get("following")));
    let posts = short_num(stats.and_then(|s| s.get("posts")));
    let bio = d.get("bio").and_then(Value::as_str).unwrap_or("-");

    format!(
        "📸 *ɪɴsᴛᴀɢʀᴀᴍ sᴛᴀʟᴋ*\n\n\
         👤 *Username:* {username_d}\n\
         📛 *Nama:* {full_name}\n\
         ✅ *Verified:* {verified}\n\
         🔒 *Private:* {is_private}\n\n\
         👥 *Pengikut:* {followers}\n\
         👤 *Mengikuti:* {following}\n\
         📷 *Postingan:* {posts}\n\n\
         📝 *Bio:*\n{bio}\n\n\
         🔗 https://instagram.com/{username}"
    )
}

#[plugin(
    name = "igstalk",
    alias = ["instagramstalk", "stalking"],
    category = "stalker",
    description = "Stalk akun Instagram",
    usage = ".igstalk <username>",
    example = ".igstalk cristiano",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn igstalk(ctx: Ctx) -> Reply {
    let username = ctx
        .args
        .first()
        .map(|s| s.trim_start_matches('@'))
        .unwrap_or_default();

    if username.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let api_key = ctx.config().api_keys.firefly.clone();
    if api_key.is_empty() {
        return Reply::text(MISSING_KEY).with_reaction("❌");
    }

    let Ok(res) = ctx.http.get(&api_url(&username, &api_key)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let Some(d) = body.get("data") else {
        return Reply::text(format!("❌ Akun *@{username}* tidak ditemukan"))
            .with_reaction("❌");
    };
    if !body.get("status").is_some_and(crate::common::js_truthy_value)
        || d.get("username").and_then(Value::as_str).map_or(true, |s| s.is_empty())
    {
        return Reply::text(format!("❌ Akun *@{username}* tidak ditemukan"))
            .with_reaction("❌");
    }

    let cap = caption(d, &username);

    let profile_pic = d.get("profile_pic").and_then(Value::as_str);
    if let Some(pic) = profile_pic.filter(|s| !s.is_empty()) {
        return Reply::image(MediaSource::url(pic), cap).with_reaction("✅");
    }

    Reply::text(cap).with_reaction("✅")
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
            command: "igstalk".into(),
            http,
            ..Ctx::default()
        };
        igstalk(ctx)
    }

    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "igstalk".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    firefly: "KUNCI".into(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        };
        igstalk(ctx)
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
    fn tanpa_username_menghasilkan_kartu_pakai_tanpa_reaksi() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_menghentikan_sebelum_permintaan() {
        let http = Stub::offline();
        let r = run(http.clone(), "cristiano");
        assert_eq!(text_of(&r), MISSING_KEY);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_membawa_kunci_dan_username_ter_encode() {
        assert_eq!(
            api_url("cristiano", "KUNCI"),
            "https://firefly.maiku.my.id/api/stalk-instagram?apikey=KUNCI&username=cristiano"
        );
        assert_eq!(
            api_url("user name", "KUNCI"),
            "https://firefly.maiku.my.id/api/stalk-instagram?apikey=KUNCI&username=user%20name"
        );
    }

    #[test]
    fn short_num_mengikuti_ambang_js() {
        assert_eq!(short_num(None), "0");
        assert_eq!(short_num(Some(&serde_json::json!(null))), "0");
        assert_eq!(short_num(Some(&serde_json::json!(0))), "0");
        assert_eq!(short_num(Some(&serde_json::json!(999))), "999");
        assert_eq!(short_num(Some(&serde_json::json!(1000))), "1 rb");
        assert_eq!(short_num(Some(&serde_json::json!(1500))), "1.5 rb");
        assert_eq!(short_num(Some(&serde_json::json!(999999))), "1 jt");
        assert_eq!(short_num(Some(&serde_json::json!(1500000))), "1.5 jt");
        assert_eq!(short_num(Some(&serde_json::json!(1000000000))), "1 miliar");
        assert_eq!(short_num(Some(&serde_json::json!("2500"))), "2.5 rb");
        assert_eq!(short_num(Some(&serde_json::json!("1500000000"))), "1.5 miliar");
    }

    #[test]
    fn jalur_berhasil_dengan_foto_profil_mengirim_gambar() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "username": "cristiano",
                "full_name": "Cristiano Ronaldo",
                "is_verified": true,
                "is_private": false,
                "stats": { "followers": 600000000, "following": 500, "posts": 3500 },
                "bio": "Footballer",
                "profile_pic": "https://cdn/pic.jpg"
            }
        });
        let http = Stub::body(body.clone());
        let r = run_dengan_kunci(http.clone(), "cristiano");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/pic.jpg".into(),
                        mimetype: None,
                    })
                );
                assert!(caption.contains("👤 *Username:* cristiano"));
                assert!(caption.contains("👥 *Pengikut:* 600 miliar"));
                assert!(caption.contains("📷 *Postingan:* 3.5 rb"));
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn tanpa_foto_profil_mengirim_teks() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "username": "cristiano",
                "full_name": "Cristiano Ronaldo",
                "is_verified": true,
                "is_private": false,
                "stats": {},
                "bio": "-"
            }
        });
        let r = run_dengan_kunci(Stub::body(body), "cristiano");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(text_of(&r).contains("👤 *Username:* cristiano"));
    }

    #[test]
    fn status_false_atau_data_kosong_menghasilkan_tidak_ditemukan() {
        for body in [
            serde_json::json!({ "status": false, "data": { "username": "x" } }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": null }),
            serde_json::json!({ "status": true, "data": { "full_name": "x" } }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "cristiano");
            assert_eq!(
                text_of(&r),
                "❌ Akun *@cristiano* tidak ditemukan",
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_membungkus_pesan_error_dan_reaksi_sil() {
        let r = run_dengan_kunci(Stub::offline(), "cristiano");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.igstalk` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
