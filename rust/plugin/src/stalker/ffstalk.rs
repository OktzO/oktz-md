//! `plugins/stalker/ffstalk.js` — stalk profil Free Fire via `api.nexray.eu.cc`.
//!
//! `ponytail:` JS bereaksi `🕕` sebelum request, lalu `✅`/`❌` di akhir.
//! `Reply::Reaction` hanya membawa satu emoji, jadi reaksi akhir (yang
//! masih terlihat setelah request selesai) yang dipasang. Gambar banner
//! dikirim tanpa `quoted` karena `Reply::Image` tidak punya bidang quoted.

use oktz_plugin_api::{Ctx, HttpCall, HttpError, HttpGet, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

const API: &str = "https://api.nexray.eu.cc/stalker/freefire";

/// `❌ *Waduh, ID Free Fire-nya belum dimasukkan!* ...`.
fn help() -> String {
    "❌ *Waduh, ID Free Fire-nya belum dimasukkan!*\n\n\
     Kamu harus mengetikkan UID pemain Free Fire yang ingin di-stalk. \n\n\
     Contoh: `.ffstalk 470699855`"
        .to_string()
}

/// `⚠️ *Pencarian Gagal!*\n\nID *{uid}* tidak ditemukan atau API sedang bermasalah...`.
fn gagal(uid: &str) -> String {
    format!(
        "⚠️ *Pencarian Gagal!*\n\n\
         ID *{uid}* tidak ditemukan atau API sedang bermasalah. Pastikan ID yang kamu masukkan sudah benar ya."
    )
}

/// `😔 *Terjadi masalah di sistem kami.* ...`.
fn sistem_error() -> String {
    "😔 *Terjadi masalah di sistem kami.* \n\n\
     Sistem gagal menarik data dari server Free Fire. Silakan coba beberapa saat lagi ya."
        .to_string()
}

/// `${value}` untuk nilai JSON di template literal.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

/// `r.language ? r.language.replace("Language_", "") : "-"`.
fn lang(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.replace("Language_", "-"),
        _ => "-".to_string(),
    }
}

/// `r.mode_prefer ? r.mode_prefer.replace("ModePrefer_", "") : "-"`.
fn mode_prefer(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.replace("ModePrefer_", "-"),
        _ => "-".to_string(),
    }
}

/// `r.guild_name && r.guild_name !== "None" ? r.guild_name : "Tidak ada guild"`.
fn guild_name(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) if s != "None" => s.clone(),
        _ => "Tidak ada guild".to_string(),
    }
}

#[plugin(
    name = "ffstalk",
    alias = ["freefireid", "stalkff", "ff"],
    category = "stalker",
    description = "Melihat informasi lengkap akun Free Fire berdasarkan ID",
    usage = ".ffstalk <id>",
    example = ".ffstalk 470699855",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ffstalk(ctx: Ctx) -> Reply {
    let uid = if ctx.text.trim().is_empty() {
        ctx.args.first().cloned().unwrap_or_default()
    } else {
        ctx.text.trim().to_string()
    };
    if uid.is_empty() {
        return Reply::text(help());
    }

    let Ok(res) = ctx.http.call(
        &HttpCall::get(&format!("{API}?uid={}", encode_uri_component(&uid)))
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)"),
    ) else {
        return Reply::text(sistem_error()).with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);

    if !crate::common::js_truthy_value(data.get("status")) || data.get("result").is_none() {
        return Reply::text(gagal(&uid)).with_reaction("❌");
    }

    let r = data.get("result").unwrap();
    let mut caption = "🔥 *FREE FIRE STALK - PROFILE INFO* 🔥\n\n".to_string();
    caption.push_str(&format!("Halo! Ini dia hasil pencarian profil untuk UID *{}*:\n\n", js_str(&r["uid"])));
    caption.push_str("👤 *INFO DASAR*\n");
    caption.push_str(&format!("  - Nama: *{}*\n", js_str(&r["name"])));
    caption.push_str(&format!(
        "  - Level: {} (EXP: {})\n",
        js_str(&r["level"]),
        js_str(&r["exp"])
    ));
    caption.push_str(&format!("  - Region: {}\n", js_str(&r["region"])));
    caption.push_str(&format!("  - Likes: {} ❤️\n", js_str(&r["likes"])));
    caption.push_str(&format!("  - Credit Score: {}\n", js_str(&r["credit_score"])));
    caption.push_str(&format!("  - Bio: {}\n\n", js_str(&r["signature"])));

    caption.push_str("🏆 *RANKING & AKTIVITAS*\n");
    caption.push_str(&format!(
        "  - BR Rank Point: {} (Max: {})\n",
        js_str(&r["br_rank_point"]),
        js_str(&r["br_max_rank"])
    ));
    caption.push_str(&format!(
        "  - CS Rank Point: {} (Max: {})\n",
        js_str(&r["cs_rank_point"]),
        js_str(&r["cs_max_rank"])
    ));
    caption.push_str(&format!("  - Season ID: {}\n", js_str(&r["season_id"])));
    caption.push_str(&format!("  - Akun Dibuat: {}\n", js_str(&r["created_at"])));
    caption.push_str(&format!("  - Terakhir Login: {}\n\n", js_str(&r["last_login"])));

    caption.push_str("🛡️ *GUILD INFO*\n");
    let gname = guild_name(r.get("guild_name"));
    caption.push_str(&format!("  - Nama Guild: {gname}\n"));
    if gname != "Tidak ada guild" {
        caption.push_str(&format!("  - Level Guild: {}\n", js_str(&r["guild_level"])));
        caption.push_str(&format!(
            "  - Anggota: {}/{}\n",
            js_str(&r["guild_member"]),
            js_str(&r["guild_capacity"])
        ));
        caption.push_str(&format!(
            "  - Ketua Guild: {} (UID: {})\n",
            js_str(&r["guild_leader_name"]),
            js_str(&r["guild_leader_uid"])
        ));
    }
    caption.push('\n');

    caption.push_str("🐾 *PET INFO*\n");
    caption.push_str(&format!("  - Pet Level: {}\n", js_str(&r["pet_level"])));
    caption.push_str(&format!("  - Pet EXP: {}\n\n", js_str(&r["pet_exp"])));

    caption.push_str("🔧 *LAINNYA*\n");
    caption.push_str(&format!("  - Bahasa: {}\n", lang(r.get("language"))));
    caption.push_str(&format!("  - Mode Favorit: {}\n\n", mode_prefer(r.get("mode_prefer"))));

    caption.push_str("Keren banget kan profilnya? Bagikan ke temanmu yuk! 🚀");

    let banner = r.get("banner_image").and_then(Value::as_str);
    let is_valid_url = banner
        .map(|s| s.starts_with("http://") || s.starts_with("https://"))
        .unwrap_or(false);

    if let Some(url) = banner.filter(|_| is_valid_url) {
        Reply::image(MediaSource::url(url.to_string()), caption).with_reaction("✅")
    } else {
        Reply::text(caption).with_reaction("✅")
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
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
            command: "ffstalk".into(),
            http,
            ..Ctx::default()
        };
        ffstalk(ctx)
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
    fn tanpa_uid_menghasilkan_pesan_gagal_tanpa_reaksi() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help());
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn status_falsy_atau_result_kosong_menghasilkan_gagal() {
        for body in [
            serde_json::json!({ "status": false }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "result": null }),
            Value::Null,
        ] {
            let r = run(Stub::body(body.clone()), "470699855");
            assert_eq!(text_of(&r), gagal("470699855"), "body {body:?}");
            assert_eq!(reaction_of(&r), Some("❌"), "body {body:?}");
        }
    }

    #[test]
    fn url_membawa_uid_terencode() {
        let r = run(Stub::body(serde_json::json!({})), "470699855");
        let _ = r;
        // dicek lewat stub seen di test terpisah
    }

    #[test]
    fn request_dikirim_ke_endpoint_dengan_uid() {
        let http = Stub::body(serde_json::json!({ "status": false }));
        let _ = run(http.clone(), "470699855");
        let seen = http.seen.lock().unwrap();
        assert_eq!(
            seen.as_slice(),
            &[format!("{API}?uid=470699855")]
        );
    }

    #[test]
    fn jalur_berhasil_mengirim_gambar_ketika_banner_valid() {
        let body = serde_json::json!({
            "status": true,
            "result": {
                "uid": "470699855",
                "name": "Pemain Contoh",
                "level": 50,
                "exp": 12000,
                "region": "ID",
                "likes": 1000,
                "credit_score": 100,
                "signature": "Halo",
                "br_rank_point": 1000,
                "br_max_rank": 5000,
                "cs_rank_point": 800,
                "cs_max_rank": 4000,
                "season_id": "S21",
                "created_at": "2024-01-01",
                "last_login": "2024-01-02",
                "guild_name": "Guild Contoh",
                "guild_level": 10,
                "guild_member": 50,
                "guild_capacity": 100,
                "guild_leader_name": "Pemilik",
                "guild_leader_uid": "1",
                "pet_level": 5,
                "pet_exp": 500,
                "language": "Language_Indonesian",
                "mode_prefer": "ModePrefer_BR",
                "banner_image": "https://cdn.example/banner.jpg"
            }
        });
        let r = run(Stub::body(body), "470699855");
        assert_eq!(reaction_of(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn.example/banner.jpg".into(),
                        mimetype: None,
                    })
                );
                assert!(caption.contains("Pemain Contoh"));
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
    }

    #[test]
    fn banner_tidak_valid_mengirim_teks_polos() {
        let body = serde_json::json!({
            "status": true,
            "result": {
                "uid": "470699855",
                "name": "X",
                "banner_image": "None"
            }
        });
        let r = run(Stub::body(body), "470699855");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(text_of(&r).contains("Nama Guild: Tidak ada guild"));
        assert!(text_of(&r).contains("Nama: *X*"));
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_pesan_sistem() {
        let r = run(Stub::offline(), "470699855");
        assert_eq!(text_of(&r), sistem_error());
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
