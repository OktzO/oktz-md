//! `plugins/primbon/sifatusahabisnis.js` — sifat usaha dari tanggal lahir.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, payload};

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/sifat_usaha_bisnis";

fn render(r: &Value) -> String {
    format!(
        "💼 *sɪꜰᴀᴛ ᴜsᴀʜᴀ/ʙɪsɴɪs*\n\n\
         > Lahir: *{}*\n\n\
         📊 *ᴀɴᴀʟɪsᴀ:*\n{}\n\n\
         > _{}_",
        display(r.get("hari_lahir")),
        display(r.get("usaha")),
        display(r.get("catatan")),
    )
}

#[plugin(
    name = "sifatusahabisnis",
    alias = ["usahabisnis", "sifatbisnis"],
    category = "primbon",
    description = "Cek sifat usaha/bisnis berdasarkan tanggal lahir",
    usage = ".sifatusahabisnis <tgl> <bln> <thn>",
    example = ".sifatusahabisnis 1 1 2000",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn sifatusahabisnis(ctx: Ctx) -> Reply {
    if ctx.args.len() < 3 {
        return Reply::text(format!(
            "💼 *sɪꜰᴀᴛ ᴜsᴀʜᴀ/ʙɪsɴɪs*\n\n> Format: tgl bln thn\n\n\
             `Contoh: {}sifatusahabisnis 1 1 2000`",
            ctx.prefix
        ));
    }
    let tgl = &ctx.args[0];
    let bln = &ctx.args[1];
    let thn = &ctx.args[2];

    let url = format!("{ENDPOINT}?tgl={tgl}&bln={bln}&thn={thn}");
    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let body = resp.json();
    let Some(r) = body.as_ref().and_then(payload) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa").with_reaction("❌");
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
            command: "sifatusahabisnis".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| a.to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("sifatusahabisnis").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("sifatusahabisnis").unwrap();
        assert_eq!(p.meta.name, "sifatusahabisnis");
        assert_eq!(p.meta.alias, ["usahabisnis", "sifatbisnis"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(
            p.meta.description,
            "Cek sifat usaha/bisnis berdasarkan tanggal lahir"
        );
        assert_eq!(p.meta.usage, ".sifatusahabisnis <tgl> <bln> <thn>");
        assert_eq!(p.meta.example, ".sifatusahabisnis 1 1 2000");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn fewer_than_three_arguments_prints_the_format_and_skips_http() {
        for args in [Vec::new(), vec!["1"], vec!["1", "1"]] {
            let r = run_with(Arc::new(NoHttp), &args);
            let t = reply_text(&r);
            assert!(
                t.starts_with("💼 *sɪꜰᴀᴛ ᴜsᴀʜᴀ/ʙɪsɴɪs*"),
                "{args:?}: {t}"
            );
            assert!(t.contains("Format: tgl bln thn"), "{args:?}: {t}");
        }
        // Argumen keempat TIDAK memicu kartu format: `m.args.length < 3` sudah
        // terpenuhi, jadi request tetap dikirim (dan di sini gagal).
        let r = run_with(Arc::new(NoHttp), &["x", "y", "z", "w"]);
        assert!(
            reply_text(&r).starts_with("☢ *ᴇʀʀᴏʀ*"),
            "{}",
            reply_text(&r)
        );
    }

    #[test]
    fn a_good_payload_renders_the_birth_day_analysis_and_note() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": {
                    "hari_lahir": "Sabtu",
                    "usaha": "Cocok bidang jasa.",
                    "catatan": "konsisten"
                }
            }))),
            &["1", "1", "2000"],
        );
        let t = reply_text(&r);
        assert!(t.starts_with("💼 *sɪꜰᴀᴛ ᴜsᴀʜᴀ/ʙɪsɴɪs*"), "{t}");
        assert!(t.contains("> Lahir: *Sabtu*"), "{t}");
        assert!(t.contains("📊 *ᴀɴᴀʟɪsᴀ:*\nCocok bidang jasa."), "{t}");
        assert!(t.ends_with("> _konsisten_"), "{t}");
    }

    /// Field hilang tidak boleh menjatuhkan balasan — `${undefined}` di JS
    /// dicetak apa adanya, jadi `display` mengembalikan teks itu juga.
    /// `data` harus non-kosong: `{}` falsy di JS dan lebih dulu kena cabang
    /// `❌ *ɢᴀɢᴀʟ*`.
    #[test]
    fn a_payload_missing_every_field_still_renders_the_frame() {
        let r = run_with(
            Arc::new(Stub(json!({ "status": true, "data": { "placeholder": 1 } }))),
            &["1", "1", "2000"],
        );
        let t = reply_text(&r);
        assert!(t.contains("> Lahir: *undefined*"), "{t}");
        assert!(t.contains("📊 *ᴀɴᴀʟɪsᴀ:*\nundefined"), "{t}");
        assert!(t.ends_with("> _undefined_"), "{t}");
    }

    #[test]
    fn a_falsy_status_or_data_gives_the_failure_message() {
        for body in [json!({ "data": {} }), json!({ "status": true }), json!({})] {
            let r = run_with(Arc::new(Stub(body)), &["1", "1", "2000"]);
            assert_eq!(reply_text(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa");
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), &["1", "1", "2000"]);
        let t = reply_text(&r);
        assert!(t.starts_with("☢ *ᴇʀʀᴏʀ*"), "{t}");
        assert!(t.contains(".sifatusahabisnis"), "{t}");
    }
}
