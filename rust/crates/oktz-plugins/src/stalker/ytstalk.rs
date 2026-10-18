//! `plugins/stalker/ytstalk.js` — stalk channel YouTube lewat API Firefly.
//!
//! Kunci `config.APIkey.firefly` dibaca dari `ctx.config()`. Nexray tidak
//! punya endpoint `/stalker/youtube` (dicek live: 500), jadi tidak ada
//! fallback di JS dan tidak ada di sini.
//!
//! `ponytail:` `m.react('🔍')` terjadi sebelum guard kunci, jadi pada cabang
//! "kunci kosong" reaksi terakhir yang ikut adalah `❌`. `Reply::Reaction`
//! hanya bisa membawa satu emoji, jadi hanya yang terakhir.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `ytstalk.js:25-27` — kartu pakai saat `m.args[0]` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📺 *ʏᴏᴜᴛᴜʙᴇ sᴛᴀʟᴋ*\n\n> Masukkan username YouTube\n\n`Contoh: {prefix}ytstalk mrbeast`"
    )
}

/// `explainFailure(new MissingApiKeyError('firefly', 'APIKEY_FIREFLY'), ..)`
/// (`stalker-fallback.js:30-36`).
pub(crate) const MISSING_KEY: &str = "❌ *API Key belum diset!*\n\n> Command `.ytstalk` butuh `APIKEY_FIREFLY` di `.env`\n> Saat ini belum ada, jadi tidak bisa dipanggil.";

/// `ytstalk.js:33` — endpoint + query `username`.
pub(crate) fn api_url(username: &str, api_key: &str) -> String {
    format!(
        "https://firefly.maiku.my.id/api/stalk-youtube?apikey={api_key}&username={}",
        encode_uri_component(username)
    )
}

/// `String(v)` untuk nilai JSON yang masuk ke template literal. Objek dan
/// array tidak punya `toString` yang berguna di sini, jadi `String({})` JS
/// juga menghasilkan `[object Object]`; kedua bentuk disalin apa adanya.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

/// `ytstalk.js:46-56` — caption hasil. `about` kosong jatuh ke `-`, dan
/// `verified` truthy jadi "Ya".
pub(crate) fn caption(c: &Value, username: &str) -> String {
    let about = match c.get("about") {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => "-".to_string(),
    };
    let verified = if c.get("verified").is_some_and(crate::common::js_truthy_value) {
        "Ya"
    } else {
        "Tidak"
    };
    format!(
        "📺 *ʏᴏᴜᴛᴜʙᴇ sᴛᴀʟᴋ*\n\n👤 *Nama:* {nama}\n🔗 *Username:* @{u}\n✅ *Verified:* {v}\n\n👥 *Subscribers:* {s}\n🎬 *Total Video:* {c}\n\n📝 *Deskripsi:*\n{a}\n\n🔗 {url}",
        nama = js_str(&c["name"]),
        u = username,
        v = verified,
        s = js_str(&c["subscribers"]),
        c = js_str(&c["video_count"]),
        a = about,
        url = js_str(&c["url"]),
    )
}

#[plugin(
    name = "ytstalk",
    alias = ["youtubestalk", "stalkyt"],
    category = "stalker",
    description = "Stalk channel YouTube",
    usage = ".ytstalk <username>",
    example = ".ytstalk mrbeast",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ytstalk(ctx: Ctx) -> Reply {
    let username = ctx.args.first().cloned().unwrap_or_default();
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
    // `!res.data?.status || !res.data?.data` — dua-duanya menghasilkan teks
    // yang sama, jadi satu syarat cukup: `status` truthy DAN `data` ada.
    let status_ok = body.get("status").is_some_and(crate::common::js_truthy_value);
    let Some(c) = body
        .get("data")
        .filter(|v| status_ok && !v.is_null())
    else {
        return Reply::text(format!("❌ Channel *{username}* tidak ditemukan"))
            .with_reaction("❌");
    };

    Reply::image(
        MediaSource::url(js_str(&c["thumbnail"])),
        caption(c, &username),
    )
    .with_reaction("✅")
}

/// Bentuk `status: true` yang dipakai test.
#[cfg(test)]
fn caption_of(body: &Value) -> String {
    caption(&body["data"], "mrbeast")
}

#[cfg(test)]
fn contoh() -> Value {
    serde_json::json!({
        "status": true,
        "data": {
            "name": "MrBeast",
            "verified": true,
            "subscribers": "300000000",
            "video_count": 1900,
            "about": "Channel resmi",
            "url": "https://youtube.com/@mrbeast",
            "thumbnail": "https://cdn/mrbeast.jpg"
        }
    })
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
            command: "ytstalk".into(),
            http,
            ..Ctx::default()
        };
        ytstalk(ctx)
    }

    /// Jalur berhasil butuh config berisi kunci. Field `Runtime` semuanya
    /// publik, jadi test bisa mengisi langsung tanpa bawaan produksi.
    fn run_dengan_kunci(http: Arc<Stub>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ytstalk".into(),
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
        ytstalk(ctx)
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
        // `Ctx::default()` punya config kosong, jadi `APIKEY_FIREFLY` kosong.
        let http = Stub::offline();
        let r = run(http.clone(), "mrbeast");
        assert_eq!(text_of(&r), MISSING_KEY);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_membawa_kunci_dan_username_ter_encode() {
        assert_eq!(
            api_url("mr beast", "KUNCI"),
            "https://firefly.maiku.my.id/api/stalk-youtube?apikey=KUNCI&username=mr%20beast"
        );
        // Kunci kosong tetap masuk URL kalau guard dilewati.
        assert!(api_url("a", "").contains("apikey=&username=a"));
    }

    #[test]
    fn caption_menyalin_seluruh_baris_dan_about_kosong_jadi_tanda_hubung() {
        let c = &contoh()["data"];
        let txt = caption(c, "mrbeast");
        assert!(txt.starts_with("📺 *ʏᴏᴜᴛᴜʙᴇ sᴛᴀʟᴋ*\n\n"));
        assert!(txt.contains("👤 *Nama:* MrBeast"));
        assert!(txt.contains("🔗 *Username:* @mrbeast"));
        assert!(txt.contains("✅ *Verified:* Ya"));
        assert!(txt.contains("👥 *Subscribers:* 300000000"));
        assert!(txt.contains("🎬 *Total Video:* 1900"));
        assert!(txt.contains("📝 *Deskripsi:*\nChannel resmi"));
        assert!(txt.ends_with("\n\n🔗 https://youtube.com/@mrbeast"));

        let kosong = serde_json::json!({ "name": "x", "verified": false });
        let txt = caption(&kosong, "u");
        assert!(txt.contains("✅ *Verified:* Tidak"));
        assert!(txt.contains("📝 *Deskripsi:*\n-"));
    }

    #[test]
    fn string_nilai_json_meniru_to_string_js() {
        assert_eq!(js_str(&serde_json::json!("teks")), "teks");
        assert_eq!(js_str(&serde_json::json!(42)), "42");
        assert_eq!(js_str(&serde_json::json!(true)), "true");
        assert_eq!(js_str(&Value::Null), "null");
        assert_eq!(js_str(&serde_json::json!({"a": 1})), "[object Object]");
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_dengan_caption() {
        let http = Stub::body(contoh());
        let r = run_dengan_kunci(http.clone(), "mrbeast");
        assert_eq!(reaction_of(&r), Some("✅"));
        // `with_reaction` membungkus isi, jadi gambar ada di dalam `Reaction`.
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(caption, &caption_of(&contoh()));
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/mrbeast.jpg".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://firefly.maiku.my.id/api/stalk-youtube?apikey=KUNCI&username=mrbeast"]
        );
    }

    #[test]
    fn status_false_atau_data_kosong_menghasilkan_teks_channel_tidak_ditemukan() {
        for body in [
            serde_json::json!({ "status": false, "data": { "name": "x" } }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": null }),
            Value::Null,
        ] {
            let r = run_dengan_kunci(Stub::body(body.clone()), "mrbeast");
            assert_eq!(text_of(&r), "❌ Channel *mrbeast* tidak ditemukan", "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_membungkus_pesan_error_dan_reaksi_sil() {
        let r = run_dengan_kunci(Stub::offline(), "mrbeast");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.ytstalk` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn caption_dipakai_oleh_handler_utuh() {
        let c = &contoh()["data"];
        assert_eq!(c["name"], "MrBeast");
        assert!(caption(c, "mrbeast").contains("MrBeast"));
    }
}
