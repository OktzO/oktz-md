//! `plugins/stalker/discordstalk.js` — stalk akun Discord berdasarkan User ID.
//!
//! `ponytail:` `m.react("🔍")` terjadi sebelum pemanggilan API, sementara
//! `Reply::with_reaction` hanya mampu membawa satu reaksi — yang ikut ke
//! balasan hanyalah reaksi penutup (`✅`/`❌`/`☢`), `🔍` hilang. Begitu pula
//! `sock.sendMessage(..., { quoted: m })` untuk avatar tidak tersedia di
//! `Ctx`; kirim avatar jadi `Reply::image` tanpa quoted.
//!
//! `d.created_at` di JS di-parse `new Date(...)` (terima string ISO). Di sini
//! hanya angka epoch ms (atau string numerik) yang diformat; nilai lain jadi
//! `-`. Tambahkan pemroses ISO ke `crate::common` kalau butuh ketepatan penuh.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

/// `discordstalk.js:28-31` — kartu pakai saat `userId` kosong.
fn help(prefix: &str) -> String {
    format!(
        "🎮 *ᴅɪsᴄᴏʀᴅ sᴛᴀʟᴋ*\n\n\
         > Masukkan Discord User ID\n\n\
         `Contoh: {prefix}discordstalk 297574907510784000`"
    )
}

/// `discordstalk.js:42` — endpoint + query `id` dan `apikey` (key boleh kosong).
/// `userId` sudah divalidasi hanya angka, jadi tidak perlu di-encode.
fn api_url(user_id: &str, api_key: &str) -> String {
    format!(
        "https://api.neoxr.eu/api/dcstalk?id={user_id}&apikey={api_key}"
    )
}

/// `String(v)` untuk nilai JSON yang masuk ke template literal: objek/array
/// jadi `[object Object]`, angka/boolean/null pakai representasinya.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

/// `discordstalk.js:55-57` — `fromTimestamp(created_at, "D MMMM YYYY")`.
/// Hanya epoch ms (detik sebagai string numerik juga diterima) yang
/// diformat ke zona Asia/Jakarta; yang tak tertolak jadi `-`.
fn created_at_text(value: Option<&Value>) -> String {
    let ms = match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::String(s)) => s.trim().parse::<i64>().ok(),
        _ => None,
    };
    match ms {
        Some(ms) => {
            let (y, m, d, ..) = crate::common::jakarta_parts(ms);
            format!(
                "{} {} {}",
                d,
                crate::common::MONTHS_ID[(m - 1) as usize],
                y
            )
        }
        None => "-".to_string(),
    }
}

/// `${d.key || "-"}` — nilai falsy JS (`null`, `0`, `""`, `false`, hilang)
/// jadi `-`.
fn field_str(d: &Value, key: &str) -> String {
    match d.get(key) {
        Some(v) if crate::common::js_truthy_value(v) => js_str(v),
        _ => "-".to_string(),
    }
}

/// `discordstalk.js:59-66` — caption hasil stalk.
fn caption(d: &Value) -> String {
    let discriminator = match d.get("discriminator") {
        Some(v) if crate::common::js_truthy_value(v) => js_str(v),
        _ => "0".to_string(),
    };
    let created_date = created_at_text(d.get("created_at"));
    format!(
        "🎮 *ᴅɪsᴄᴏʀᴅ sᴛᴀʟᴋ*\n\n\
         👤 *Username:* {}\n\
         📛 *Display Name:* {}\n\
         🔢 *Discriminator:* #{}\n\
         🆔 *User ID:* {}\n\n\
         📅 *Dibuat:* {}\n\n\
         > _Discord User Lookup_",
        field_str(d, "username"),
        field_str(d, "global_name"),
        discriminator,
        js_str(d.get("id").unwrap_or(&Value::Null)),
        created_date,
    )
}

#[plugin(
    name = "discordstalk",
    alias = ["dcstalk", "dsstalk", "stalkdc"],
    category = "stalker",
    description = "Stalk akun Discord berdasarkan User ID",
    usage = ".discordstalk <userid>",
    example = ".discordstalk 297574907510784000",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn discordstalk(ctx: Ctx) -> Reply {
    let user_id = ctx.args.first().map(|s| s.trim()).unwrap_or("");
    if user_id.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !user_id.bytes().all(|b| b.is_ascii_digit()) {
        return Reply::text(
            "❌ User ID harus berupa angka. Contoh: 297574907510784000",
        );
    }

    let api_key = ctx.config().api_keys.neoxr.clone();
    let Ok(res) = ctx.http.get(&api_url(user_id, &api_key)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = res.json().unwrap_or(Value::Null);
    let status_ok = body.get("status").is_some_and(crate::common::js_truthy_value);
    // `!res.data?.status || !res.data?.data` — data falsy ikut gagal.
    let Some(d) = body
        .get("data")
        .filter(|v| status_ok && crate::common::js_truthy_value(v))
    else {
        return Reply::text(format!("❌ User ID *{user_id}* tidak ditemukan"))
            .with_reaction("❌");
    };

    let caption_text = caption(d);
    // `if (d.avatar_url)` — nilai falsy JS tidak dikirim sebagai gambar.
    let avatar = d
        .get("avatar_url")
        .filter(|v| crate::common::js_truthy_value(v))
        .map(js_str);

    if let Some(url) = avatar {
        Reply::image(MediaSource::url(url), caption_text).with_reaction("✅")
    } else {
        Reply::text(caption_text).with_reaction("✅")
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
            command: "discordstalk".into(),
            http,
            ..Ctx::default()
        };
        discordstalk(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("discordstalk").unwrap();
        assert_eq!(p.meta.name, "discordstalk");
        assert_eq!(p.meta.alias, ["dcstalk", "dsstalk", "stalkdc"]);
        assert_eq!(p.meta.category, "stalker");
        assert_eq!(p.meta.description, "Stalk akun Discord berdasarkan User ID");
        assert_eq!(p.meta.usage, ".discordstalk <userid>");
        assert_eq!(p.meta.example, ".discordstalk 297574907510784000");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve_to_this_plugin() {
        for a in ["discordstalk", "dcstalk", "dsstalk", "stalkdc"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "discordstalk",
                "alias {a}"
            );
        }
    }

    #[test]
    fn tanpa_user_id_menampilkan_kartu_pakai_tanpa_permintaan() {
        let http = Stub::offline();
        let r = run(http.clone(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn user_id_bukan_angka_ditolak_tanpa_permintaan_atau_reaksi() {
        let http = Stub::offline();
        let r = run(http.clone(), "abc123");
        assert_eq!(
            text_of(&r),
            "❌ User ID harus berupa angka. Contoh: 297574907510784000"
        );
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn url_membawa_id_dan_apikey() {
        assert_eq!(
            api_url("297574907510784000", "KUNCI"),
            "https://api.neoxr.eu/api/dcstalk?id=297574907510784000&apikey=KUNCI"
        );
        assert_eq!(
            api_url("123", ""),
            "https://api.neoxr.eu/api/dcstalk?id=123&apikey="
        );
    }

    #[test]
    fn data_tidak_ditemukan_menghasilkan_teks_dan_reaksi_sil() {
        for body in [
            serde_json::json!({ "status": false, "data": { "username": "x" } }),
            serde_json::json!({ "status": true }),
            serde_json::json!({ "status": true, "data": null }),
            Value::Null,
        ] {
            let http = Stub::body(body.clone());
            let r = run(http.clone(), "297574907510784000");
            assert_eq!(
                text_of(&r),
                "❌ User ID *297574907510784000* tidak ditemukan",
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn berhasil_dengan_avatar_mengirim_gambar_dengan_caption() {
        // 1768564200000 = 2026-01-16T11:50:00Z, WIB 18:50 → 16 Januari 2026
        // (`discordstalk.js:55-57`).
        let created_at = 1_768_564_200_000i64;
        let body = serde_json::json!({
            "status": true,
            "data": {
                "username": "oktzz",
                "global_name": "Oktz",
                "discriminator": "1234",
                "id": 297574907510784000i64,
                "created_at": created_at,
                "avatar_url": "https://cdn/avatar.png"
            }
        });
        let http = Stub::body(body.clone());
        let r = run(http.clone(), "297574907510784000");
        assert_eq!(reaction_of(&r), Some("✅"));
        // Balasan dibungkus reaksi ✅ (`discordstalk.js:68-78`), jadi isi pesan
        // diambil dari dalam `Reply::Reaction`.
        let Reply::Reaction { reply: isi, .. } = r.without_mutations() else {
            panic!("harus dibungkus reaksi");
        };
        match isi.as_ref() {
            Reply::Image { media, caption, .. } => {
                assert!(caption.contains("👤 *Username:* oktzz"));
                assert!(caption.contains("📛 *Display Name:* Oktz"));
                assert!(caption.contains("🔢 *Discriminator:* #1234"));
                assert!(caption.contains("🆔 *User ID:* 297574907510784000"));
                assert!(caption.contains("📅 *Dibuat:* 16 Januari 2026"));
                assert_eq!(
                    media,
                    &Some(MediaSource::Url {
                        url: "https://cdn/avatar.png".into(),
                        mimetype: None,
                    })
                );
            }
            lain => panic!("harus image, dapat {lain:?}"),
        }
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.neoxr.eu/api/dcstalk?id=297574907510784000&apikey="]
        );
    }

    #[test]
    fn tanpa_avatar_mengirim_teks_polos() {
        let body = serde_json::json!({
            "status": true,
            "data": {
                "username": "x",
                "global_name": null,
                "discriminator": null,
                "id": 123,
                "created_at": null
            }
        });
        let http = Stub::body(body);
        let r = run(http, "123");
        assert_eq!(reaction_of(&r), Some("✅"));
        let t = text_of(&r);
        assert!(t.contains("📛 *Display Name:* -"));
        assert!(t.contains("🔢 *Discriminator:* #0"));
        assert!(t.contains("🆔 *User ID:* 123"));
        assert!(t.contains("📅 *Dibuat:* -"));
    }

    #[test]
    fn created_at_hanya_terima_epoch_angka() {
        // 1767225600000 = 2026-01-01T00:00:00Z -> 07:00 WIB, 1 Januari 2026
        assert_eq!(created_at_text(Some(&serde_json::json!(1767225600000i64))), "1 Januari 2026");
        assert_eq!(created_at_text(Some(&serde_json::json!("1767225600000"))), "1 Januari 2026");
        // String ISO tidak bisa di-parse tanpa crate tanggal -> "-"
        assert_eq!(
            created_at_text(Some(&serde_json::json!("2026-01-15T08:30:00.000Z"))),
            "-"
        );
        assert_eq!(created_at_text(Some(&Value::Null)), "-");
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_pesan_error_dan_reaksi_sil() {
        let r = run(Stub::offline(), "297574907510784000");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.discordstalk` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
