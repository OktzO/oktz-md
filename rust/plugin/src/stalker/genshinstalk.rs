//! `plugins/stalker/genshinstalk.js` — stalk akun Genshin Impact berdasarkan UID.
//!
//! `ponytail:` JS bereaksi dua kali (`🕕` lalu `✅`/`❌`), sementara
//! `Reply::with_reaction` hanya satu emoji — hanya reaksi akhir (`✅` /
//! `❌`/`☢`) yang dibawa. Header `User-Agent` dikirim lewat `HttpCall::call`.
//! `sock.sendMessage` dengan `quoted: m` tidak tersedia; image dikirim tanpa
//! quote.
//!
//! Catatan: baris `pub mod genshinstalk;` perlu ditambah ke `stalker/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, plugin};
use serde_json::Value;

#[plugin(
    name = "genshinstalk",
    alias = ["genshin", "stalkgenshin", "gi"],
    category = "stalker",
    description = "Melihat informasi akun Genshin Impact berdasarkan UID.",
    usage = ".genshinstalk <uid>",
    example = ".genshinstalk 856012067",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn genshinstalk(ctx: Ctx) -> Reply {
    let uid = ctx.text.trim().to_string();
    if uid.is_empty() {
        return Reply::text(
            "❌ *UID Genshin-nya mana nih?*\n\n\
             Kamu harus memasukkan UID pemain Genshin Impact yang ingin di-stalk. \n\n\
             Contoh: `.genshinstalk 856012067`",
        );
    }

    let url = format!("https://api.nexray.eu.cc/stalker/genshin?id={uid}");
    let Ok(res) = ctx.http.call(
        &HttpCall::get(&url).header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        ),
    ) else {
        return Reply::text("😔 *Terjadi masalah di sistem kami.*\n\nSistem gagal menarik data dari server Genshin Impact. Silakan coba beberapa saat lagi ya.")
            .with_reaction("☢");
    };

    let data = res.json().unwrap_or(Value::Null);
    if !crate::common::js_truthy(data.get("status"))
        || !crate::common::js_truthy(data.get("result"))
    {
        return Reply::text(format!(
            "⚠️ *Pencarian Gagal!*\n\nUID *{uid}* tidak ditemukan atau profilnya diprivat. Pastikan UID yang kamu masukkan sudah benar ya."
        ))
        .with_reaction("❌");
    }

    let result = data.get("result").unwrap_or(&Value::Null);
    let player = result.get("player_info").unwrap_or(&Value::Null);
    let image_url = result.get("image_url").and_then(Value::as_str).unwrap_or("");

    let nickname = player.get("nickname").and_then(Value::as_str).unwrap_or("-");
    let level = player.get("level").and_then(Value::as_str).unwrap_or("-").to_string();
    let world_level = player
        .get("world_level")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();
    let signature = player.get("signature").and_then(Value::as_str).unwrap_or("-");
    let achievements = player
        .get("achievements")
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string();
    let spiral_abyss = player
        .get("spiral_abyss")
        .and_then(Value::as_str)
        .unwrap_or("Belum ada data");
    let theater = player.get("theater").and_then(Value::as_str);
    let stygian = player.get("stygian_onslaught").and_then(Value::as_str);

    let id_val = result.get("id").and_then(Value::as_str).unwrap_or(&uid);

    let mut caption = String::new();
    caption.push_str("🌟 *GENSHIN IMPACT STALK* 🌟\n\n");
    caption.push_str(&format!(
        "Halo Traveler! Ini dia informasi akun untuk UID *{}*:\\n\\n",
        id_val
    ));
    caption.push_str("👤 *INFO PEMAIN*\n");
    caption.push_str(&format!("  - Nickname: *{}*\n", nickname));
    caption.push_str(&format!("  - Adventure Rank (AR): {}\n", level));
    caption.push_str(&format!("  - World Level (WL): {}\n", world_level));
    caption.push_str(&format!("  - Signature: {}\n\n", signature));
    caption.push_str("🏆 *PENCAPAIAN*\n");
    caption.push_str(&format!("  - Total Achievement: {}\n", achievements));
    caption.push_str(&format!("  - Spiral Abyss: {}\n", spiral_abyss));
    if let Some(t) = theater {
        caption.push_str(&format!("  - Imaginarium Theater: {}\n", t));
    }
    if let Some(s) = stygian {
        caption.push_str(&format!("  - Stygian Onslaught: {}\n", s));
    }
    caption.push('\n');
    caption.push_str("Gimana, statnya bagus nggak? Pamerin ke teman-temanmu yuk! 🚀");

    if !image_url.is_empty() {
        Reply::image(MediaSource::url(image_url), caption).with_reaction("✅")
    } else {
        Reply::text(caption).with_reaction("✅")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v.to_string()),
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
            match &self.body {
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
            args: args.by_ref().map(|s| s.to_string()).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            command: "genshinstalk".into(),
            http,
            ..Default::default()
        };
        genshinstalk(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("genshinstalk").unwrap();
        assert_eq!(p.meta.name, "genshinstalk");
        assert_eq!(p.meta.alias, ["genshin", "stalkgenshin", "gi"]);
        assert_eq!(p.meta.category, "stalker");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_uid_tampilkan_petunjuk() {
        let r = run(Stub::offline(), "");
        assert!(body(&r).contains("UID Genshin-nya mana nih?"));
        assert_eq!(reaction(&r), None);
    }

    #[test]
    fn jaringan_gagal_kembalikan_pesan_dan_reaksi_sil() {
        let r = run(Stub::offline(), "856012067");
        assert!(body(&r).contains("Terjadi masalah di sistem kami"));
        assert_eq!(reaction(&r), Some("☢"));
    }

    #[test]
    fn status_salah_atau_result_kosong_jadi_pencarian_gagal() {
        for body_json in [
            serde_json::json!({ "status": false, "result": {} }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "result": null }),
        ] {
            let r = run(Stub::json(body_json.clone()), "856012067");
            assert!(
                body(&r).contains("Pencarian Gagal!"),
                "body {body_json}"
            );
            assert_eq!(reaction(&r), Some("❌"), "body {body_json}");
        }
    }

    #[test]
    fn hasil_berhasil_mengirim_gambar_dengan_caption() {
        let data = serde_json::json!({
            "status": true,
            "result": {
                "id": "856012067",
                "player_info": {
                    "nickname": "Traveler",
                    "level": "60",
                    "world_level": "8",
                    "signature": "Hello",
                    "achievements": "1500",
                    "spiral_abyss": "36 star",
                    "theater": "1500",
                    "stygian_onslaught": "100"
                },
                "image_url": "https://example/avatar.jpg"
            }
        });
        let r = run(Stub::json(data.clone()), "856012067");
        assert_eq!(reaction(&r), Some("✅"));
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert_eq!(
                    media,
                    &Some(MediaSource::url("https://example/avatar.jpg"))
                );
                assert!(caption.contains("Traveler"));
                assert!(caption.contains("60"));
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
    }

    #[test]
    fn tanpa_image_url_mengirim_teks_saja() {
        let data = serde_json::json!({
            "status": true,
            "result": {
                "id": "856012067",
                "player_info": {
                    "nickname": "Traveler",
                    "level": "60",
                    "world_level": "8",
                    "signature": "-",
                    "achievements": "0",
                    "spiral_abyss": "Belum ada data"
                }
            }
        });
        let r = run(Stub::json(data), "856012067");
        assert_eq!(reaction(&r), Some("✅"));
        assert!(body(&r).contains("Traveler"));
    }
}
