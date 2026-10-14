//! `plugins/primbon/potensipenyakit.js` — cek potensi penyakit dari tanggal lahir.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, payload};

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/cek_potensi_penyakit";

fn render(tgl: &str, bln: &str, thn: &str, r: &Value) -> String {
    format!(
        "🏥 *ᴘᴏᴛᴇɴsɪ ᴘᴇɴʏᴀᴋɪᴛ*\n\n\
         > Tanggal: *{tgl}-{bln}-{thn}*\n\n\
         📊 *ᴇʟᴇᴍᴇɴ:*\n{}\n\n\
         ⚠️ *ᴘᴏᴛᴇɴsɪ:*\n{}\n\n\
         > _{}_",
        display(r.get("sektor")),
        display(r.get("elemen")),
        display(r.get("catatan")),
    )
}

#[plugin(
    name = "potensipenyakit",
    alias = ["cekpenyakit", "penyakit"],
    category = "primbon",
    description = "Cek potensi penyakit berdasarkan tanggal lahir",
    usage = ".potensipenyakit <tgl> <bln> <thn>",
    example = ".potensipenyakit 12 05 1998",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn potensipenyakit(ctx: Ctx) -> Reply {
    if ctx.args.len() < 3 {
        return Reply::text(format!(
            "🏥 *ᴘᴏᴛᴇɴsɪ ᴘᴇɴʏᴀᴋɪᴛ*\n\n> Format: tgl bln thn\n\n\
             `Contoh: {}potensipenyakit 12 05 1998`",
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
    let Some(result) = body.as_ref().and_then(payload) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa").with_reaction("❌");
    };

    Reply::text(render(tgl, bln, thn, result)).with_reaction("✅")
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
            command: "potensipenyakit".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| a.to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("potensipenyakit").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("potensipenyakit").unwrap();
        assert_eq!(p.meta.name, "potensipenyakit");
        assert_eq!(p.meta.alias, ["cekpenyakit", "penyakit"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Cek potensi penyakit berdasarkan tanggal lahir");
        assert_eq!(p.meta.usage, ".potensipenyakit <tgl> <bln> <thn>");
        assert_eq!(p.meta.example, ".potensipenyakit 12 05 1998");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    /// Argumen tidak di-parse sebagai angka: `tgl`/`bln`/`thn` ikut mentah ke
    /// query dan ke teks balasan, persis template literal JS.
    #[test]
    fn three_arguments_are_passed_through_verbatim() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": { "sektor": "Timur", "elemen": "Air", "catatan": "istirahat" }
            }))),
            &["12", "05", "1998"],
        );
        let t = reply_text(&r);
        assert!(t.contains("> Tanggal: *12-05-1998*"), "{t}");
        assert!(t.contains("📊 *ᴇʟᴇᴍᴇɴ:*\nTimur"), "{t}");
        assert!(t.contains("⚠️ *ᴘᴏᴛᴇɴsɪ:*\nAir"), "{t}");
        assert!(t.ends_with("> _istirahat_"), "{t}");

        // Angka negatif pun tidak divalidasi — tetap diteruskan mentah.
        let r = run_with(
            Arc::new(Stub(json!({ "status": true, "data": { "catatan": "-" } }))),
            &["-1", "😀", "abc"],
        );
        assert!(reply_text(&r).contains("> Tanggal: *-1-😀-abc*"), "{}", reply_text(&r));
    }

    #[test]
    fn fewer_than_three_arguments_prints_the_format_and_skips_http() {
        for args in [Vec::new(), vec!["12"], vec!["12", "05"]] {
            let r = run_with(Arc::new(NoHttp), &args);
            let t = reply_text(&r);
            assert!(t.starts_with("🏥 *ᴘᴏᴛᴇɴsɪ ᴘᴇɴʏᴀᴋɪᴛ*"), "{args:?}: {t}");
            assert!(t.contains("Format: tgl bln thn"), "{args:?}: {t}");
        }
        // Argumen keempat TIDAK memicu kartu format: `m.args.length < 3` sudah
        // terpenuhi, jadi request tetap dikirim (dan di sini gagal).
        let r = run_with(Arc::new(NoHttp), &["a", "b", "c", "d"]);
        assert!(
            reply_text(&r).starts_with("☢ *ᴇʀʀᴏʀ*"),
            "{}",
            reply_text(&r)
        );
    }

    #[test]
    fn a_falsy_status_or_data_gives_the_failure_message() {
        for body in [json!({ "data": {} }), json!({ "status": true }), json!({})] {
            let r = run_with(Arc::new(Stub(body)), &["12", "05", "1998"]);
            assert_eq!(reply_text(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa");
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), &["12", "05", "1998"]);
        let t = reply_text(&r);
        assert!(t.starts_with("☢ *ᴇʀʀᴏʀ*"), "{t}");
        assert!(t.contains(".potensipenyakit"), "{t}");
    }
}
