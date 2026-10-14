//! `plugins/primbon/nomerhoki.js` — cek keberuntungan nomor HP.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, payload, truthy};

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/nomorhoki";

/// `m.args.join('').replace(/[^0-9]/g, '')` — semua argumen digabung tanpa
/// pemisah dulu, baru semua non-digit dibuang.
fn only_digits(ctx: &Ctx) -> String {
    ctx.args
        .concat()
        .chars()
        .filter(char::is_ascii_digit)
        .collect()
}

fn render(r: &Value) -> String {
    let ep = r.get("energi_positif").and_then(|v| v.get("details"));
    let en = r.get("energi_negatif").and_then(|v| v.get("details"));
    let status = r
        .get("analisis")
        .and_then(|v| v.get("status"))
        .is_some_and(truthy);

    format!(
        "🍀 *ɴᴏᴍᴏʀ ʜᴏᴋɪ*\n\n\
         > Nomor: *{}*\n\n\
         📊 *ᴀɴɢᴋᴀ ʙᴀɢᴜᴀ:* {}%\n\n\
         ✅ *ᴇɴᴇʀɢɪ ᴘᴏꜱɪᴛɪꜰ:* {}%\n\
         ├ Kekayaan: {}\n\
         ├ Kesehatan: {}\n\
         ├ Cinta: {}\n\
         └ Kestabilan: {}\n\n\
         ❌ *ᴇɴᴇʀɢɪ ɴᴇɢᴀᴛɪꜰ:* {}%\n\
         ├ Perselisihan: {}\n\
         ├ Kehilangan: {}\n\
         ├ Malapetaka: {}\n\
         └ Kehancuran: {}\n\n\
         > Status: {}",
        display(r.get("nomor")),
        display(r.get("angka_bagua_shuzi").and_then(|v| v.get("value"))),
        display(r.get("energi_positif").and_then(|v| v.get("total"))),
        display(ep.and_then(|v| v.get("kekayaan"))),
        display(ep.and_then(|v| v.get("kesehatan"))),
        display(ep.and_then(|v| v.get("cinta"))),
        display(ep.and_then(|v| v.get("kestabilan"))),
        display(r.get("energi_negatif").and_then(|v| v.get("total"))),
        display(en.and_then(|v| v.get("perselisihan"))),
        display(en.and_then(|v| v.get("kehilangan"))),
        display(en.and_then(|v| v.get("malapetaka"))),
        display(en.and_then(|v| v.get("kehancuran"))),
        if status { "✅ HOKI" } else { "❌ TIDAK HOKI" },
    )
}

#[plugin(
    name = "nomerhoki",
    alias = ["nomorhoki", "ceknomor"],
    category = "primbon",
    description = "Cek keberuntungan nomor HP",
    usage = ".nomerhoki <nomor>",
    example = ".nomerhoki 6281234567890",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn nomerhoki(ctx: Ctx) -> Reply {
    let nomor = only_digits(&ctx);
    if nomor.is_empty() {
        return Reply::text(format!(
            "🍀 *ɴᴏᴍᴏʀ ʜᴏᴋɪ*\n\n> Masukkan nomor HP\n\n\
             `Contoh: {}nomerhoki 6281234567890`",
            ctx.prefix
        ));
    }

    let url = format!("{ENDPOINT}?phoneNumber={nomor}");
    let gagal = Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa nomor").with_reaction("❌");
    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let Some(body) = resp.json() else {
        return gagal;
    };
    let Some(r) = payload(&body) else {
        return gagal;
    };

    Reply::text(render(r)).with_reaction("✅")
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

    fn run_with(http: Arc<dyn HttpGet>, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "nomerhoki".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| a.to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("nomerhoki").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("nomerhoki").unwrap();
        assert_eq!(p.meta.name, "nomerhoki");
        assert_eq!(p.meta.alias, ["nomorhoki", "ceknomor"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Cek keberuntungan nomor HP");
        assert_eq!(p.meta.usage, ".nomerhoki <nomor>");
        assert_eq!(p.meta.example, ".nomerhoki 6281234567890");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    /// Argumen tanpa digit apa pun — termasuk yang cuma simbol — kembali ke help.
    #[test]
    fn input_without_any_digit_prints_the_help_and_skips_http() {
        for args in [Vec::new(), vec!["abc"], vec!["😀", "!!!"], vec!["+62-81"]] {
            let r = run_with(Arc::new(NoHttp), &args);
            let t = reply_text(&r);
            assert!(t.starts_with("🍀 *ɴᴏᴍᴏʀ ʜᴏᴋɪ*"), "{args:?}: {t}");
            assert!(t.contains("Masukkan nomor HP"), "{args:?}: {t}");
        }
    }

    /// Argumen digabung tanpa spasi, lalu semua non-digit dibuang.
    #[test]
    fn digits_are_joined_without_a_separator_and_non_digits_dropped() {
        let ctx = |args: &[&str]| Ctx {
            args: args.iter().map(|a| a.to_string()).collect(),
            ..Default::default()
        };
        assert_eq!(only_digits(&ctx(&["628", "123"])), "628123");
        assert_eq!(only_digits(&ctx(&["62-81", "23"])), "628123");
        assert_eq!(only_digits(&ctx(&["abc"])), "");
        assert_eq!(only_digits(&ctx(&["😀7😀"])), "7");
    }

    #[test]
    fn a_good_payload_renders_the_tree_and_the_hoki_verdict() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": {
                    "nomor": "6281234567890",
                    "angka_bagua_shuzi": { "value": 8 },
                    "energi_positif": {
                        "total": 75,
                        "details": { "kekayaan": "Besar", "kesehatan": "Baik", "cinta": "Luang", "kestabilan": "Kuat" }
                    },
                    "energi_negatif": {
                        "total": 25,
                        "details": { "perselisihan": "Kecil", "kehilangan": "Sedang", "malapetaka": "Tidak ada", "kehancuran": "Rendah" }
                    },
                    "analisis": { "status": true }
                }
            }))),
            &["6281234567890"],
        );
        let t = reply_text(&r);
        assert!(t.starts_with("🍀 *ɴᴏᴍᴏʀ ʜᴏᴋɪ*"), "{t}");
        assert!(t.contains("> Nomor: *6281234567890*"), "{t}");
        assert!(t.contains("📊 *ᴀɴɢᴋᴀ ʙᴀɢᴜᴀ:* 8%"), "{t}");
        assert!(t.contains("✅ *ᴇɴᴇʀɢɪ ᴘᴏꜱɪᴛɪꜰ:* 75%"), "{t}");
        assert!(t.contains("├ Kekayaan: Besar"), "{t}");
        assert!(t.contains("└ Kestabilan: Kuat"), "{t}");
        assert!(t.contains("❌ *ᴇɴᴇʀɢɪ ɴᴇɢᴀᴛɪꜰ:* 25%"), "{t}");
        assert!(t.contains("└ Kehancuran: Rendah"), "{t}");
        assert!(t.ends_with("> Status: ✅ HOKI"), "{t}");
    }

    /// `r.analisis.status ? '✅ HOKI' : '❌ TIDAK HOKI'` — falsy JS lain
    /// (`0`, `""`, `null`, field hilang) semuanya memilih TIDAK HOKI.
    #[test]
    fn a_falsy_analysis_status_prints_tidak_hoki() {
        for status in [json!(false), json!(0), json!(""), json!(null)] {
            let r = run_with(
                Arc::new(Stub(json!({ "status": true, "data": { "analisis": { "status": status } } }))),
                &["628123"],
            );
            assert!(reply_text(&r).ends_with("> Status: ❌ TIDAK HOKI"), "{status}");
        }
        let r = run_with(
            Arc::new(Stub(json!({ "status": true, "data": {} }))),
            &["628123"],
        );
        assert!(reply_text(&r).ends_with("> Status: ❌ TIDAK HOKI"));
    }

    #[test]
    fn a_falsy_top_level_status_or_data_gives_the_failure_message() {
        for body in [json!({ "data": {} }), json!({ "status": true }), json!({})] {
            let r = run_with(Arc::new(Stub(body)), &["628123"]);
            assert_eq!(
                reply_text(&r),
                "❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa nomor"
            );
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), &["628123"]);
        let t = reply_text(&r);
        assert!(t.starts_with("☢ *ᴇʀʀᴏʀ*"), "{t}");
        assert!(t.contains(".nomerhoki"), "{t}");
    }
}
