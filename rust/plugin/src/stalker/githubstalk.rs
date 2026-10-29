//! `plugins/stalker/githubstalk.js` — stalk akun GitHub lewat Firefly atau
//! Nexray.
//!
//! `fetchGithubProfile` (`stalker-fallback.js:87-137`) mencoba Firefly dulu
//! (butuh `APIKEY_FIREFLY`), lalu Nexray tanpa key. Nexray memakai nama field
//! berbeda (`nickname`, `profile_pic`, `public_repo`), jadi bentuk responsnya
//! dipetakan terpisah lalu disamakan di dalam `Profil`.
//!
//! `ponytail:` `m.react('🔍')` terjadi sebelum permintaan, jadi pada cabang
//! gagal reaksi terakhir yang ikut adalah `☢`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use super::npmstalk::{FIREFLY, NEXRAY, js_str, or_dash};
use crate::tools::phisingataubukan::encode_uri_component;

/// `githubstalk.js:26-29` — kartu pakai saat `m.args[0]` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🐙 *ɢɪᴛʜᴜʙ sᴛᴀʟᴋ*\n\n> Masukkan username GitHub\n\n`Contoh: {prefix}githubstalk torvalds`"
    )
}

/// Bentuk ternormalisasi dari `fetchGithubProfile`.
#[derive(Debug, Clone)]
pub(crate) struct Profil {
    pub username: String,
    pub name: Value,
    pub company: Value,
    pub location: Value,
    pub bio: Value,
    pub url: Value,
    pub avatar: Value,
    pub public_repos: Value,
    pub followers: Value,
    pub following: Value,
}

/// Firefly (`stalker-fallback.js:95-109`).
pub(crate) fn dari_firefly(d: &Value) -> Profil {
    Profil {
        username: js_str(&d["username"]),
        name: d["name"].clone(),
        company: d["company"].clone(),
        location: d["location"].clone(),
        bio: d["bio"].clone(),
        url: d["url"].clone(),
        avatar: d["avatar"].clone(),
        public_repos: d["public_repos"].clone(),
        followers: d["followers"].clone(),
        following: d["following"].clone(),
    }
}

/// Nexray (`stalker-fallback.js:110-132`) — nama fieldnya berbeda.
pub(crate) fn dari_nexray(r: &Value) -> Profil {
    Profil {
        username: js_str(&r["username"]),
        name: r["nickname"].clone(),
        company: r["company"].clone(),
        location: r["location"].clone(),
        bio: r["bio"].clone(),
        url: r["url"].clone(),
        avatar: r["profile_pic"].clone(),
        public_repos: r["public_repo"].clone(),
        followers: r["followers"].clone(),
        following: r["following"].clone(),
    }
}

/// `v ?? 0` — hanya `null` dan `undefined` yang jadi nol; `0` tetap `0`.
fn nullish(v: &Value) -> String {
    if v.is_null() {
        "0".to_string()
    } else {
        js_str(v)
    }
}

/// `githubstalk.js:38-49` — caption. `name`/`company`/`location`/`bio`
/// memakai `|| '-'`, tiga angka Repo/Followers/Following memakai `?? 0`.
pub(crate) fn caption(p: &Profil) -> String {
    format!(
        "🐙 *ɢɪᴛʜᴜʙ sᴛᴀʟᴋ*\n\n👤 *Username:* {}\n📛 *Nama:* {}\n🏢 *Company:* {}\n📍 *Location:* {}\n\n📦 *Public Repos:* {}\n👥 *Followers:* {}\n👤 *Following:* {}\n\n📝 *Bio:*\n{}\n\n🔗 {}",
        p.username,
        or_dash(&p.name),
        or_dash(&p.company),
        or_dash(&p.location),
        nullish(&p.public_repos),
        nullish(&p.followers),
        nullish(&p.following),
        or_dash(&p.bio),
        js_str(&p.url),
    )
}

#[plugin(
    name = "githubstalk",
    alias = ["ghstalk", "stalkgh"],
    category = "stalker",
    description = "Stalk akun GitHub",
    usage = ".githubstalk <username>",
    example = ".githubstalk torvalds",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn githubstalk(ctx: Ctx) -> Reply {
    let username = ctx.args.first().cloned().unwrap_or_default();
    if username.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let firefly_key = ctx.config().api_keys.firefly.clone();
    let mut profil: Option<Profil> = None;

    if !firefly_key.is_empty()
        && let Ok(res) = ctx.http.get(&format!(
            "{FIREFLY}/api/stalk-github?apikey={firefly_key}&username={}",
            encode_uri_component(&username)
        ))
    {
        let body = res.json().unwrap_or(Value::Null);
        if body.get("status").is_some_and(crate::common::js_truthy_value)
            && body.get("data").is_some_and(|v| !v.is_null())
        {
            profil = Some(dari_firefly(&body["data"]));
        }
    }

    if profil.is_none()
        && let Ok(res) = ctx.http.get(&format!(
            "{NEXRAY}/stalker/github?username={}",
            encode_uri_component(&username)
        ))
    {
        let body = res.json().unwrap_or(Value::Null);
        if body.get("status").is_some_and(crate::common::js_truthy_value)
            && body.get("result").is_some_and(|v| !v.is_null())
        {
            profil = Some(dari_nexray(&body["result"]));
        }
    }

    let Some(profil) = profil else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };

    let teks = caption(&profil);
    // `if (d.avatar)` di JS: `undefined` itu falsy. `js_str` akan
    // menulis `"null"` untuk JSON null, jadi yang dicek truthiness-nya, bukan
    // string kosongnya.
    let ada_avatar = crate::common::js_truthy_value(&profil.avatar);
    if !ada_avatar {
        Reply::text(teks).with_reaction("✅")
    } else {
        Reply::image(MediaSource::url(js_str(&profil.avatar)), teks).with_reaction("✅")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Mutex<Vec<String>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn baru(balasan: &[&str]) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(balasan.iter().map(|s| s.to_string()).collect()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn gagal() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Mutex::new(Vec::new()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let mut b = self.balasan.lock().unwrap();
            if b.is_empty() {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            }
            let body = if b.len() == 1 {
                b[0].clone()
            } else {
                b.remove(0)
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
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

    fn dengan_kunci(http: Arc<Stub>, text: &str, kunci: &str) -> Ctx {
        let mut args = text.split_whitespace();
        Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "githubstalk".into(),
            http,
            config: Arc::new(oktz_plugin_api::config::Runtime {
                api_keys: oktz_plugin_api::config::ApiKeys {
                    firefly: kunci.to_string(),
                    ..oktz_plugin_api::config::ApiKeys::default()
                },
                ..oktz_plugin_api::config::Runtime::default()
            }),
            ..Ctx::default()
        }
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_username_menghasilkan_kartu_pakai_tanpa_reaksi() {
        let r = githubstalk(dengan_kunci(Stub::gagal(), "", "K"));
        assert_eq!(
            r.without_mutations().text_body(),
            Some("🐙 *ɢɪᴛʜᴜʙ sᴛᴀʟᴋ*\n\n> Masukkan username GitHub\n\n`Contoh: .githubstalk torvalds`")
        );
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn kunci_kosong_langsung_lompat_ke_nexray() {
        let http = Stub::baru(&[r#"{"status":true,"result":{"username":"t","nickname":"T","url":"https://github.com/t"}}"#]);
        let r = githubstalk(dengan_kunci(http.clone(), "t", ""));
        assert_eq!(reaction_of(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "firefly dilewati: {seen:?}");
        assert_eq!(seen[0], "https://api.nexray.eu.cc/stalker/github?username=t");
    }

    #[test]
    fn firefly_dipakai_dulu_bila_kunci_ada() {
        let http = Stub::baru(&[r#"{"status":true,"data":{"username":"t","name":"T","company":"Linux","location":"US","bio":"bio","url":"https://github.com/t","avatar":"https://cdn/a.png","public_repos":9,"followers":100,"following":5}}"#]);
        let r = githubstalk(dengan_kunci(http.clone(), "t", "K"));
        assert_eq!(reaction_of(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].contains("apikey=K"));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Image { media, caption, .. } => {
                    assert_eq!(
                        media.as_ref().unwrap(),
                        &MediaSource::Url {
                            url: "https://cdn/a.png".to_string(),
                            mimetype: None,
                        }
                    );
                    assert!(caption.contains("👤 *Username:* t"));
                    assert!(caption.contains("📦 *Public Repos:* 9"));
                    assert!(caption.contains("👥 *Followers:* 100"));
                    assert!(caption.contains("👤 *Following:* 5"));
                }
                lain => panic!("harus image, dapat {lain:?}"),
            },
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn nexray_memetakan_nama_field_berbeda() {
        let http = Stub::baru(&[r#"{"status":true,"result":{"username":"t","nickname":"Nick","profile_pic":"https://cdn/p.png","public_repo":12,"url":"https://github.com/t"}}"#]);
        let r = githubstalk(dengan_kunci(http.clone(), "t", ""));
        let txt = r.without_mutations().text_body().unwrap_or("").to_string();
        assert!(txt.contains("📛 *Nama:* Nick"));
        assert!(txt.contains("📦 *Public Repos:* 12"));
        assert!(txt.contains("🏢 *Company:* -"), "company kosong jadi -");
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => {
                assert!(matches!(reply.as_ref(), Reply::Image { .. }));
            }
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn avatar_kosong_menghasilkan_teks_biasa() {
        let http = Stub::baru(&[r#"{"status":true,"result":{"username":"t"}}"#]);
        let r = githubstalk(dengan_kunci(http.clone(), "t", ""));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => {
                assert!(matches!(reply.as_ref(), Reply::Text { .. }));
            }
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn semua_provider_gagal_membungkus_pesan_error_dan_reaksi_sil() {
        let r = githubstalk(dengan_kunci(Stub::gagal(), "t", "K"));
        assert_eq!(
            r.without_mutations().text_body(),
            Some(crate::common::error_template_bawaan(".", "githubstalk").as_str())
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn nullish_hanya_menolong_null_dan_undefined() {
        assert_eq!(nullish(&Value::Null), "0");
        assert_eq!(nullish(&serde_json::json!(0)), "0", "nol bukan nullish");
        assert_eq!(nullish(&serde_json::json!(12)), "12");
        assert_eq!(nullish(&serde_json::json!("x")), "x");
    }
}
