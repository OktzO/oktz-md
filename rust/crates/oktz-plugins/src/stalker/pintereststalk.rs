//! `plugins/stalker/pintereststalk.js` — informasi akun Pinterest lewat Nexray.
//!
//! Plugin ini satu-satunya di folder stalker yang TIDAK memakai rantai
//! `withFallback`: satu GET langsung ke `api.nexray.eu.cc` dengan header
//! `User-Agent` khusus, tanpa kunci API.
//!
//! `ponytail:` reaksi `🕕` terjadi sebelum permintaan. Karena `Reply::Reaction`
//! hanya bisa membawa satu emoji, yang ikut adalah reaksi terakhir: `❌` saat
//! profil tidak ditemukan, `✅` setelah caption terkirim, `☢` saat gagal.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::{Value, json};

use super::npmstalk::{NEXRAY, js_str, or_dash};
use crate::tools::phisingataubukan::encode_uri_component;

/// `pintereststalk.js:31` — header khusus plugin ini.
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)";

/// `pintereststalk.js:23-25` — pesan tanpa username.
pub(crate) const MISSING_USERNAME: &str = "❌ *Waduh, username Pinterest-nya belum dimasukkan!*\n\nKamu harus mengetikkan username Pinterest yang ingin di-stalk. \n\nContoh: `.pintereststalk dims`";

/// `pintereststalk.js:40` — profil tidak ditemukan.
pub(crate) fn not_found(username: &str) -> String {
    format!("⚠️ *Pencarian Gagal!*\n\nUsername *{username}* tidak ditemukan di Pinterest. Pastikan penulisannya sudah benar ya.")
}

/// `pintereststalk.js:78` — cabang `catch`.
pub(crate) const FAILED: &str = "😔 *Terjadi masalah di sistem kami.* \n\nSistem gagal menarik data dari server Pinterest. Silakan coba beberapa saat lagi ya.";

/// `pintereststalk.js:34` — endpoint + query `username`.
pub(crate) fn api_url(username: &str) -> String {
    format!(
        "{NEXRAY}/stalker/pinterest?username={}",
        encode_uri_component(username)
    )
}

/// `v || 0` untuk statistik — Bedakan dari `?? 0` di `githubstalk`: di sini
/// nilai `0` pun ditulis `0`, dan string kosong pun jadi `0`.
fn or_zero(v: &Value) -> String {
    if crate::common::js_truthy_value(v) {
        js_str(v)
    } else {
        "0".to_string()
    }
}

/// `r.image?.original || … || r.image?.small` — empat kandidat, yang pertama
/// truthy menang.
pub(crate) fn pilih_gambar(image: &Value) -> String {
    for kunci in ["original", "large", "medium", "small"] {
        if crate::common::js_truthy_value(&image[kunci]) {
            return js_str(&image[kunci]);
        }
    }
    String::new()
}

/// `pintereststalk.js:48-63` — caption.
pub(crate) fn caption(r: &Value, username: &str) -> String {
    let s = r.get("stats").cloned().unwrap_or_else(|| json!({}));
    let txt = format!(
        "📌 *PINTEREST STALK - PROFILE INFO* 📌\n\nHalo! Ini dia hasil pencarian profil untuk username *@{u}*:\n\n👤 *INFO PROFIL*\n  - Nama Lengkap: *{n}*\n  - Username: @{u}\n  - Bio: {b}\n  - Tipe Akun: {t}\n  - Akun Dibuat: {c}\n\n📊 *STATISTIK*\n  - Pengikut (Followers): {f}\n  - Diikuti (Following): {g}\n  - Total Pin: {p}\n  - Total Board: {bd}\n\n🔗 *LINK PROFIL*\n  - {url}\n\nSuka mengumpulkan inspirasi dari Pinterest ya? Pamerin ke temanmu yuk! 🚀",
        u = username,
        n = or_dash(&r["full_name"]),
        b = or_dash(&r["bio"]),
        t = or_dash(&r["account_type"]),
        c = or_dash(&r["created_at"]),
        f = or_zero(&s["followers"]),
        g = or_zero(&s["following"]),
        p = or_zero(&s["pins"]),
        bd = or_zero(&s["boards"]),
        url = js_str(&r["profile_url"]),
    );
    txt
}

#[plugin(
    name = "pintereststalk",
    alias = ["pinterestid", "stalkpinterest", "stalkpin"],
    category = "stalker",
    description = "Melihat informasi lengkap akun Pinterest berdasarkan username.",
    usage = ".pintereststalk <username>",
    example = ".pintereststalk dims",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pintereststalk(ctx: Ctx) -> Reply {
    // `m.text?.trim() || m.args[0]` — teks mentah lebih dulu, baru argumen.
    let dari_teks = ctx.text.trim();
    let username = if dari_teks.is_empty() {
        ctx.args.first().cloned().unwrap_or_default()
    } else {
        dari_teks.to_string()
    };
    if username.is_empty() {
        return Reply::text(MISSING_USERNAME);
    }

    let Ok(res) = ctx
        .http
        .call(&oktz_plugin_api::HttpCall::get(api_url(&username)).header("User-Agent", UA))
    else {
        return Reply::text(FAILED).with_reaction("☢");
    };

    let body = res.json().unwrap_or(Value::Null);
    let ok = body.get("status").is_some_and(crate::common::js_truthy_value)
        && body.get("result").is_some_and(|v| !v.is_null());
    if !ok {
        return Reply::text(not_found(&username)).with_reaction("❌");
    }

    let r = &body["result"];
    let teks = caption(r, &username);
    let gambar = pilih_gambar(&r["image"]);
    let balasan = if gambar.is_empty() {
        Reply::text(teks)
    } else {
        Reply::image(MediaSource::url(gambar), teks)
    };
    balasan.with_reaction("✅")
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
        seen: Mutex<Vec<oktz_plugin_api::HttpCall>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn gagal() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini lewat HttpCall::call")
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
        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.clone());
            match &self.balasan {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(HttpError {
                    message: "offline".into(),
                }),
            }
        }
    }

    fn ctx(http: Arc<Stub>, text: &str) -> Ctx {
        let mut args = text.split_whitespace();
        Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "pintereststalk".into(),
            http,
            ..Ctx::default()
        }
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn header(req: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn tanpa_username_menghasilkan_pesan_khusus_tanpa_reaksi() {
        let r = pintereststalk(ctx(Stub::gagal(), ""));
        assert_eq!(r.without_mutations().text_body(), Some(MISSING_USERNAME));
        assert_eq!(reaction_of(&r), None, "reaksi belum jalan");
    }

    #[test]
    fn url_dan_user_agent_sesuai_js() {
        let http = Stub::body(json!({"status": false}));
        pintereststalk(ctx(http.clone(), "dims"));
        let p = http.seen.lock().unwrap();
        assert_eq!(
            p[0].url,
            "https://api.nexray.eu.cc/stalker/pinterest?username=dims"
        );
        assert_eq!(header(&p[0], "User-Agent").as_deref(), Some(UA));
    }

    #[test]
    fn profil_tidak_ditemukan_memberi_reaksi_sil_tan() {
        for body in [
            json!({ "status": false }),
            json!({ "status": true }),
            json!({ "status": true, "result": null }),
        ] {
            let r = pintereststalk(ctx(Stub::body(body.clone()), "dims"));
            assert_eq!(r.without_mutations().text_body(), Some(not_found("dims").as_str()), "body {body}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn caption_menyalin_semua_blok_dan_statistik_kosong_jadi_nol() {
        let r = json!({
            "username": "dims", "full_name": "Dims", "bio": "hi",
            "account_type": "BUSINESS", "created_at": "2020-01-01",
            "stats": { "followers": 12, "following": 0, "pins": 30, "boards": 4 },
            "profile_url": "https://pinterest.com/dims"
        });
        let txt = caption(&r, "dims");
        assert!(txt.starts_with("📌 *PINTEREST STALK - PROFILE INFO* 📌\n\n"));
        assert!(txt.contains("username *@dims*"));
        assert!(txt.contains("  - Nama Lengkap: *Dims*"));
        assert!(txt.contains("  - Bio: hi"));
        assert!(txt.contains("  - Pengikut (Followers): 12"));
        assert!(txt.contains("  - Diikuti (Following): 0"), "nol tetap ditulis 0");
        assert!(txt.ends_with("Pamerin ke temanmu yuk! 🚀"));

        // Field profil kosong jadi `-`, statistik hilang jadi `0`.
        let kosong = json!({ "username": "u" });
        let txt = caption(&kosong, "u");
        assert!(txt.contains("  - Nama Lengkap: *-*"));
        assert!(txt.contains("  - Pengikut (Followers): 0"));
        assert!(txt.contains("  - Total Pin: 0"));
    }

    #[test]
    fn gambar_memilih_kandidat_pertama_yang_truthy() {
        assert_eq!(pilih_gambar(&json!({})), "");
        assert_eq!(
            pilih_gambar(&json!({"original": "o", "large": "l"})),
            "o"
        );
        assert_eq!(
            pilih_gambar(&json!({"original": "", "large": "l"})),
            "l",
            "string kosong dilewati"
        );
        assert_eq!(
            pilih_gambar(&json!({"medium": "m"})),
            "m",
            "medium dipakai saat original dan large kosong"
        );
        assert_eq!(pilih_gambar(&json!({"small": "s"})), "s");
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_lalu_reaksi_centang() {
        let http = Stub::body(json!({
            "status": true,
            "result": {
                "username": "dims", "full_name": "Dims",
                "stats": {"followers": 1}, "profile_url": "https://pinterest.com/dims",
                "image": { "original": "https://cdn/dims.png" }
            }
        }));
        let r = pintereststalk(ctx(http, "dims"));
        assert_eq!(reaction_of(&r), Some("✅"));
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Image { media, caption, .. } => {
                    assert_eq!(
                        media.as_ref().unwrap(),
                        &MediaSource::Url {
                            url: "https://cdn/dims.png".to_string(),
                            mimetype: None,
                        }
                    );
                    assert!(caption.contains("Nama Lengkap: *Dims*"));
                }
                lain => panic!("harus image, dapat {lain:?}"),
            },
            lain => panic!("harus dibungkus reaksi, dapat {lain:?}"),
        }
    }

    #[test]
    fn tanpa_gambar_mengirim_teks_lalu_reaksi_centang() {
        let http = Stub::body(json!({
            "status": true,
            "result": { "username": "dims", "image": {} }
        }));
        let r = pintereststalk(ctx(http, "dims"));
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { reply, .. } if matches!(reply.as_ref(), Reply::Text { .. })
        ));
    }

    #[test]
    fn kegagalan_jaringan_menghasilkan_pesan_sistem_dan_reaksi_sil() {
        let r = pintereststalk(ctx(Stub::gagal(), "dims"));
        assert_eq!(r.without_mutations().text_body(), Some(FAILED));
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
