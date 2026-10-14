//! `plugins/primbon/zodiak.js` — ramalan zodiak per-APY.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, payload};

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/zodiak";

const VALID_ZODIACS: [&str; 12] = [
    "aries",
    "taurus",
    "gemini",
    "cancer",
    "leo",
    "virgo",
    "libra",
    "scorpio",
    "sagitarius",
    "capricorn",
    "aquarius",
    "pisces",
];

fn help(prefix: &str) -> String {
    format!(
        "⭐ *ᴢᴏᴅɪᴀᴋ*\n\n> Masukkan nama zodiak:\n\n{}\n\n`Contoh: {}zodiak aries`",
        VALID_ZODIACS
            .iter()
            .map(|z| format!("• {z}"))
            .collect::<Vec<_>>()
            .join("\n"),
        prefix
    )
}

fn render(zodiac: &str, r: &Value) -> String {
    format!(
        "⭐ *ᴢᴏᴅɪᴀᴋ {}*\n\n{}\n\n\
         🔢 *ɴᴏᴍᴏʀ:* {}\n\
         🌸 *ʙᴜɴɢᴀ:* {}\n\
         🎨 *ᴡᴀʀɴᴀ:* {}\n\
         💎 *ʙᴀᴛᴜ:* {}\n\
         🔥 *ᴇʟᴇᴍᴇɴ:* {}\n\
         🪐 *ᴘʟᴀɴᴇᴛ:* {}\n\
         💕 *ᴘᴀsᴀɴɢᴀɴ:* {}",
        zodiac.to_uppercase(),
        display(r.get("zodiak")),
        display(r.get("nomor_keberuntungan")),
        display(r.get("bunga_keberuntungan")),
        display(r.get("warna_keberuntungan")),
        display(r.get("batu_keberuntungan")),
        display(r.get("elemen_keberuntungan")),
        display(r.get("planet_yang_mengitari")),
        display(r.get("pasangan_zodiak")),
    )
}

#[plugin(
    name = "zodiak",
    alias = ["horoscope", "ramalan"],
    category = "primbon",
    description = "Ramalan zodiak",
    usage = ".zodiak <nama zodiak>",
    example = ".zodiak aries",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn zodiak(ctx: Ctx) -> Reply {
    let raw = ctx.args.first().map(|a| a.to_lowercase());
    let Some(zodiac) = raw.filter(|z| VALID_ZODIACS.contains(&z.as_str())) else {
        return Reply::text(help(&ctx.prefix));
    };

    let url = format!("{ENDPOINT}?zodiak={zodiac}");
    let gagal = Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal mendapatkan ramalan").with_reaction("❌");
    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let Some(body) = resp.json() else {
        return gagal;
    };
    let Some(r) = payload(&body) else {
        return gagal;
    };

    Reply::text(render(&zodiac, r)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, NoHttp};
    use serde_json::json;
    use std::sync::Arc;

    #[derive(Debug)]
    struct Stub(Value);

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(serde_json::to_vec(&self.0).unwrap()),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak dipakai")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak dipakai")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak dipakai")
        }
    }

    fn run_with(http: Arc<dyn HttpGet>, arg: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "zodiak".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.unwrap_or("").into(),
            args: arg.map(|a| vec![a.to_string()]).unwrap_or_default(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("zodiak").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("zodiak").unwrap();
        assert_eq!(p.meta.name, "zodiak");
        assert_eq!(p.meta.alias, ["horoscope", "ramalan"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Ramalan zodiak");
        assert_eq!(p.meta.usage, ".zodiak <nama zodiak>");
        assert_eq!(p.meta.example, ".zodiak aries");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn an_unknown_or_missing_sign_lists_the_twelve_valid_ones() {
        for arg in [None, Some(""), Some("aries "), Some("cancer-ish"), Some("😀")] {
            let r = run_with(Arc::new(NoHttp), arg);
            let t = reply_text(&r);
            assert!(t.starts_with("⭐ *ᴢᴏᴅɪᴀᴋ*"), "{arg:?}: {t}");
            assert!(t.contains("• aries"), "{arg:?}: {t}");
            assert!(t.contains("• pisces"), "{arg:?}: {t}");
            // Tidak ada reaksi: JS `return` sebelum `m.react`.
            assert!(matches!(r.without_mutations(), Reply::Text { .. }));
        }
    }

    /// `.zodiak Aries` — `toLowerCase()` sebelum cek ke `validZodiacs`.
    #[test]
    fn the_sign_is_lowercased_before_the_membership_check() {
        let r = run_with(Arc::new(NoHttp), Some("ARIES"));
        assert!(reply_text(&r).starts_with("☢ *ᴇʀʀᴏʀ*"), "tidak masuk daftar");
    }

    #[test]
    fn a_good_payload_renders_every_labelled_field() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": {
                    "zodiak": "Ramalan hari ini gemilang",
                    "nomor_keberuntungan": 7,
                    "bunga_keberuntungan": "Mawar",
                    "warna_keberuntungan": "Merah",
                    "batu_keberuntungan": "Safir",
                    "elemen_keberuntungan": "Api",
                    "planet_yang_mengitari": "Mars",
                    "pasangan_zodiak": "Taurus"
                }
            }))),
            Some("aries"),
        );
        let t = reply_text(&r);
        assert!(t.starts_with("⭐ *ᴢᴏᴅɪᴀᴋ ARIES*"), "{t}");
        assert!(t.contains("Ramalan hari ini gemilang"), "{t}");
        assert!(t.contains("🔢 *ɴᴏᴍᴏʀ:* 7"), "{t}");
        assert!(t.contains("🌸 *ʙᴜɴɢᴀ:* Mawar"), "{t}");
        assert!(t.contains("💎 *ʙᴀᴛᴜ:* Safir"), "{t}");
        assert!(t.contains("🪐 *ᴘʟᴀɴᴇᴛ:* Mars"), "{t}");
        assert!(t.contains("💕 *ᴘᴀsᴀɴɢᴀɴ:* Taurus"), "{t}");
    }

    #[test]
    fn a_falsy_status_or_data_gives_the_failure_message() {
        for body in [json!({ "data": {} }), json!({ "status": true }), json!({ "status": 0, "data": 1 })] {
            let r = run_with(Arc::new(Stub(body)), Some("aries"));
            assert_eq!(
                reply_text(&r),
                "❌ *ɢᴀɢᴀʟ*\n\n> Gagal mendapatkan ramalan"
            );
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), Some("aries"));
        let t = reply_text(&r);
        assert!(t.starts_with("☢ *ᴇʀʀᴏʀ*"), "{t}");
        assert!(t.contains("`.zodiak`"), "{t}");
    }
}
