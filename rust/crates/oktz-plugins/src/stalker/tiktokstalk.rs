//! `plugins/stalker/tiktokstalk.js` — stalk akun TikTok lewat Firefly atau
//! Nexray.
//!
//! `fetchTiktokProfile` (`stalker-fallback.js:335-397`) mencoba Firefly dulu
//! (butuh `APIKEY_FIREFLY`), lalu Nexray tanpa key.
//!
//! Bedanya penting: Firefly mengirim boolean untuk `verified` dan `private`,
//! sedangkan Nexray mengirim string `"Verified"` / `"No"` dan `"Yes"`. Keduanya
//! dinormalisasi di dalam masing-masing pemetaan, bukan setelahnya.
//!
//! `ponytail:` `m.react('🔍')` terjadi sebelum permintaan, jadi pada cabang
//! gagal reaksi terakhir yang ikut adalah `☢`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::{Value, json};

use super::npmstalk::{FIREFLY, NEXRAY, js_str, or_dash, short_num};
use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `tiktokstalk.js:38-41` — kartu pakai; username sudah dibuang `@` pertama.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎵 *ᴛɪᴋᴛᴏᴋ sᴛᴀʟᴋ*\n\n> Masukkan username TikTok\n\n`Contoh: {prefix}tiktokstalk mrbeast`"
    )
}

/// `m.args[0]?.replace('@', '')` — hanya kemunculan pertama, bukan global.
pub(crate) fn bersihkan_username(raw: &str) -> String {
    raw.replacen('@', "", 1)
}

/// Bentuk ternormalisasi dari `fetchTiktokProfile`.
#[derive(Debug, Clone)]
pub(crate) struct Profil {
    pub username: String,
    pub name: Value,
    pub avatar: Value,
    pub bio: Value,
    pub verified: bool,
    pub private: bool,
    pub followers: Value,
    pub following: Value,
    pub likes: Value,
    pub videos: Value,
    pub link: Value,
}

/// `raw_followers ?? followers` dan tiga pasangan lainnya — hanya `null` dan
/// `undefined` yang jatuh ke nilai kedua.
fn nullish<'a>(v: &'a Value, kedua: &'a Value) -> &'a Value {
    if v.is_null() { kedua } else { v }
}

/// Firefly (`stalker-fallback.js:337-361`) — `!!d.verified` dan `!!d.private`,
/// sedangkan Nexray dibaca dari stringnya.
pub(crate) fn dari_firefly(d: &Value) -> Profil {
    let s = d.get("stats").cloned().unwrap_or_else(json_kosong);
    let username = js_str(&d["username"]);
    Profil {
        link: Value::String(format!("https://tiktok.com/@{username}")),
        username,
        name: d["nickname"].clone(),
        avatar: d["avatar"].clone(),
        bio: d["signature"].clone(),
        verified: crate::common::js_truthy_value(&d["verified"]),
        private: crate::common::js_truthy_value(&d["private"]),
        followers: s["followers"].clone(),
        following: s["following"].clone(),
        likes: s["hearts"].clone(),
        videos: s["videos"].clone(),
    }
}

/// Nexray (`stalker-fallback.js:362-395`) — boolean bisa berupa string
/// `"Verified"` / `"Yes"`, dan angka punya pasangan `raw_*`.
pub(crate) fn dari_nexray(r: &Value) -> Profil {
    let s = r.get("stats").cloned().unwrap_or_else(json_kosong);
    Profil {
        username: js_str(&r["username"]),
        name: r["name"].clone(),
        avatar: r["avatar"].clone(),
        bio: r["bio"].clone(),
        verified: r["verified"] == Value::Bool(true)
            || r["verified"] == json!("Verified"),
        private: r["private"] == Value::Bool(true)
            || r["private"] == json!("Yes"),
        followers: nullish(&s["raw_followers"], &s["followers"]).clone(),
        following: nullish(&s["raw_following"], &s["following"]).clone(),
        likes: nullish(&s["raw_likes"], &s["likes"]).clone(),
        videos: nullish(&s["raw_videos"], &s["videos"]).clone(),
        link: r["link"].clone(),
    }
}

/// `d.stats || {}` — objek baru kalau `stats` tidak ada.
fn json_kosong() -> Value {
    Value::Object(serde_json::Map::new())
}

/// `tiktokstalk.js:41-50` — caption. Empat angka melewati `shortNum`; dua
/// flag terner jadi `Ya` atau `Tidak`.
pub(crate) fn caption(p: &Profil) -> String {
    format!(
        "🎵 *ᴛɪᴋᴛᴏᴋ sᴛᴀʟᴋ*\n\n👤 *Username:* @{u}\n📛 *Nama:* {n}\n✅ *Verified:* {v}\n🔒 *Private:* {p}\n\n👥 *Followers:* {f}\n👤 *Following:* {g}\n❤️ *Likes:* {l}\n🎬 *Videos:* {v2}\n\n📝 *Bio:*\n{b}\n\n🔗 {link}",
        u = p.username,
        n = or_dash(&p.name),
        v = if p.verified { "Ya" } else { "Tidak" },
        p = if p.private { "Ya" } else { "Tidak" },
        f = short_num(&p.followers),
        g = short_num(&p.following),
        l = short_num(&p.likes),
        v2 = short_num(&p.videos),
        b = or_dash(&p.bio),
        link = js_str(&p.link),
    )
}

#[plugin(
    name = "tiktokstalk",
    alias = ["ttstalk", "stalktt"],
    category = "stalker",
    description = "Stalk akun TikTok",
    usage = ".tiktokstalk <username>",
    example = ".tiktokstalk mrbeast",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tiktokstalk(ctx: Ctx) -> Reply {
    let username = ctx
        .args
        .first()
        .map(|a| bersihkan_username(a))
        .unwrap_or_default();
    if username.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let firefly_key = ctx.config().api_keys.firefly.clone();
    let mut profil: Option<Profil> = None;

    if !firefly_key.is_empty()
        && let Ok(res) = ctx.http.get(&format!(
            "{FIREFLY}/api/stalk-tiktok?apikey={firefly_key}&username={}",
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
            "{NEXRAY}/stalker/tiktok?username={}",
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
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    let teks = caption(&profil);
    if !crate::common::js_truthy_value(&profil.avatar) {
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
                return Err(HttpError {
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
            command: "tiktokstalk".into(),
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
        let r = tiktokstalk(dengan_kunci(Stub::gagal(), "", "K"));
        assert_eq!(
            r.without_mutations().text_body(),
            Some("🎵 *ᴛɪᴋᴛᴏᴋ sᴛᴀʟᴋ*\n\n> Masukkan username TikTok\n\n`Contoh: .tiktokstalk mrbeast`")
        );
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn hanya_at_pertama_dibuang_bukan_seluruhnya() {
        // `String.replace('@', '')` tanpa `/g` hanya kena yang pertama.
        assert_eq!(bersihkan_username("@abc"), "abc");
        assert_eq!(bersihkan_username("a@b"), "ab");
        assert_eq!(bersihkan_username("@@x"), "@x");
        assert_eq!(bersihkan_username("abc"), "abc");
    }

    #[test]
    fn firefly_membangun_link_dari_username_dan_membaca_boolean() {
        let d = json!({
            "username": "u", "nickname": "Nick", "avatar": "https://cdn/a.png",
            "signature": "bio", "verified": 1, "private": 0,
            "stats": {"followers": 1500, "following": 200, "hearts": 1_500_000, "videos": 42}
        });
        let p = dari_firefly(&d);
        assert_eq!(p.link, json!("https://tiktok.com/@u"));
        assert!(p.verified, "1 truthy di JS");
        assert!(!p.private, "0 falsy di JS");
        let txt = caption(&p);
        assert!(txt.contains("👤 *Username:* @u"));
        assert!(txt.contains("✅ *Verified:* Ya"));
        assert!(txt.contains("🔒 *Private:* Tidak"));
        assert!(txt.contains("👥 *Followers:* 1.5K"));
        assert!(txt.contains("❤️ *Likes:* 1.5M"));
        assert!(txt.contains("🎬 *Videos:* 42"));
    }

    #[test]
    fn nexray_membaca_string_verified_dan_private() {
        let r = json!({
            "username": "u", "name": "N", "avatar": "", "bio": "b",
            "verified": "Verified", "private": "No",
            "stats": {"raw_followers": 3000000000u64, "followers": 3, "likes": 10, "videos": 1},
            "link": "https://tiktok.com/@u"
        });
        let p = dari_nexray(&r);
        assert!(p.verified, "\"Verified\" dihitung benar");
        assert!(!p.private, "\"No\" bukan benar");
        assert_eq!(p.followers, json!(3000000000u64), "raw_ menang");
        assert_eq!(p.likes, json!(10), "raw_ null jadi yang biasa");
        let txt = caption(&p);
        assert!(txt.contains("👥 *Followers:* 3B"));
        assert!(txt.contains("🔒 *Private:* Tidak"));
    }

    #[test]
    fn stats_hilang_tidak_menggagalkan_hasil() {
        let p = dari_firefly(&json!({ "username": "u" }));
        assert_eq!(p.followers, Value::Null);
        assert!(caption(&p).contains("👥 *Followers:* 0"));
        let p = dari_nexray(&json!({ "username": "u" }));
        assert_eq!(p.likes, Value::Null);
        assert!(caption(&p).contains("❤️ *Likes:* 0"));
    }

    #[test]
    fn bio_kosong_dan_avatar_kosong_menghasilkan_teks_biasa() {
        let http = Stub::baru(&[r#"{"status":true,"data":{"username":"u","nickname":"N","signature":"","avatar":""}}"#]);
        let r = tiktokstalk(dengan_kunci(http, "u", "K"));
        assert_eq!(reaction_of(&r), Some("✅"));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => {
                assert!(matches!(reply.as_ref(), Reply::Text { .. }));
                assert!(reply.text_body().unwrap().contains("📝 *Bio:*\n-"));
            }
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn semua_provider_gagal_membungkus_pesan_error_dan_reaksi_sil() {
        let r = tiktokstalk(dengan_kunci(Stub::gagal(), "u", "K"));
        assert_eq!(
            r.without_mutations().text_body(),
            Some(error_template(".", "tiktokstalk").as_str())
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
