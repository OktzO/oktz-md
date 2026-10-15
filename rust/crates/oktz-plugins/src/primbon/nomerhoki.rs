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

/// Pembacaan `a.b` di JS (`nomerhoki.js:37-53`): kalau `a` `undefined`/
/// `null`, Property Access-nya melempar TypeError yang ditangkap `catch` —
/// itu sebabnya `Err(())`. Kalau `a` primitif, hasilnya `undefined` tanpa
/// melempar.
fn js_prop<'a>(a: Option<&'a Value>, key: &str) -> Result<Option<&'a Value>, ()> {
    match a {
        None | Some(Value::Null) => Err(()),
        Some(Value::Object(o)) => Ok(o.get(key)),
        Some(_) => Ok(None),
    }
}

fn render(r: &Value) -> Result<String, ()> {
    // `const ep = r.energi_positif.details` lalu `en = r.energi_negatif.details`.
    let ep = js_prop(r.get("energi_positif"), "details")?;
    let en = js_prop(r.get("energi_negatif"), "details")?;
    let status = js_prop(r.get("analisis"), "status")?.is_some_and(truthy);

    Ok(format!(
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
        display(js_prop(r.get("angka_bagua_shuzi"), "value")?),
        display(js_prop(r.get("energi_positif"), "total")?),
        display(js_prop(ep, "kekayaan")?),
        display(js_prop(ep, "kesehatan")?),
        display(js_prop(ep, "cinta")?),
        display(js_prop(ep, "kestabilan")?),
        display(js_prop(r.get("energi_negatif"), "total")?),
        display(js_prop(en, "perselisihan")?),
        display(js_prop(en, "kehilangan")?),
        display(js_prop(en, "malapetaka")?),
        display(js_prop(en, "kehancuran")?),
        if status { "✅ HOKI" } else { "❌ TIDAK HOKI" },
    ))
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

    // `r.energi_positif.details` dan seterusnya melempar TypeError kalau
    // rantai property-nya melewati `undefined`/`null` → sama seperti `catch` JS.
    match render(r) {
        Ok(text) => Reply::text(text).with_reaction("✅"),
        Err(()) => Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢"),
    }
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

    /// `data` harus TRUTHY supaya lolos `!data?.data` — objek kosong `{}` di JS
    /// falsy, jadi tidak bisa dipakai di sini.
    fn payload_with_status(status: Value) -> Value {
        json!({
            "status": true,
            "data": {
                "nomor": "628123",
                "angka_bagua_shuzi": { "value": 8 },
                "energi_positif": { "total": 75, "details": {} },
                "energi_negatif": { "total": 25, "details": {} },
                "analisis": { "status": status }
            }
        })
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
        for args in [Vec::new(), vec!["abc"], vec!["😀", "!!!"], vec!["+ +"]] {
            let r = run_with(Arc::new(NoHttp), &args);
            let t = reply_text(&r);
            assert!(t.starts_with("🍀 *ɴᴏᴍᴏʀ ʜᴏᴋɪ*"), "{args:?}: {t}");
            assert!(t.contains("Masukkan nomor HP"), "{args:?}: {t}");
        }
    }

    /// `+62-81` masih menyisakan `6281`, jadi lolos ke HTTP — bukan help.
    #[test]
    fn an_input_with_at_least_one_digit_goes_to_http() {
        let r = run_with(Arc::new(NoHttp), &["+62-81"]);
        assert!(reply_text(&r).starts_with("☢ Kayaknya command "), "{}", reply_text(&r));
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
                Arc::new(Stub(payload_with_status(status.clone()))),
                &["628123"],
            );
            assert!(
                reply_text(&r).ends_with("> Status: ❌ TIDAK HOKI"),
                "{status}"
            );
        }
        // `status` bernilai objek kosong: `{}` falsy di JS, jadi bukan HOKI.
        let r = run_with(Arc::new(Stub(payload_with_status(json!({})))), &["628123"]);
        let t = reply_text(&r);
        assert!(
            t.ends_with("> Status: ❌ TIDAK HOKI"),
            "`analisis.status` = `{{}}`: {t}"
        );
    }

    /// Rantai property yang melewati `undefined`/`null` melempar TypeError di
    /// JS (`r.energi_positif.details`), jadi jawabannya template error, bukan
    /// balasan berisi `undefined`.
    #[test]
    fn a_missing_parent_object_falls_into_the_error_template() {
        // `data` non-kosong supaya lolos `!data?.data` dulu.
        for data in [
            json!({ "nomor": "628123" }),
            json!({ "energi_positif": { "total": 1, "details": {} } }),
            json!({ "energi_positif": null }),
        ] {
            let r = run_with(
                Arc::new(Stub(json!({ "status": true, "data": data }))),
                &["628123"],
            );
            assert!(
                reply_text(&r).starts_with("☢ Kayaknya command "),
                "{data}: {}",
                reply_text(&r)
            );
        }
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
        assert!(t.starts_with("☢ Kayaknya command "), "{t}");
        assert!(t.contains(".nomerhoki"), "{t}");
    }
}
