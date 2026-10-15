//! `plugins/primbon/kecocokannamapasangan.js` — cek kecocokan nama pasangan.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, payload};
use crate::tools::phisingataubukan::encode_uri_component;

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/kecocokan_nama_pasangan";

fn render(r: &Value) -> String {
    format!(
        "💕 *ᴋᴇᴄᴏᴄᴏᴋᴀɴ ɴᴀᴍᴀ ᴘᴀsᴀɴɢᴀɴ*\n\n\
         > 👤 {}\n\
         > 💑 {}\n\n\
         ✅ *ꜱɪꜱɪ ᴘᴏꜱɪᴛɪꜰ:*\n{}\n\n\
         ❌ *ꜱɪꜱɪ ɴᴇɢᴀᴛɪꜰ:*\n{}\n\n\
         > _{}_",
        display(r.get("nama_anda")),
        display(r.get("nama_pasangan")),
        display(r.get("sisi_positif")),
        display(r.get("sisi_negatif")),
        display(r.get("catatan")),
    )
}

#[plugin(
    name = "kecocokannamapasangan",
    alias = ["cocoknama", "matchname"],
    category = "primbon",
    description = "Cek kecocokan nama pasangan",
    usage = ".kecocokannamapasangan <nama1> <nama2>",
    example = ".kecocokannamapasangan putu keyla",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn kecocokannamapasangan(ctx: Ctx) -> Reply {
    if ctx.args.len() < 2 {
        return Reply::text(format!(
            "💕 *ᴋᴇᴄᴏᴄᴏᴋᴀɴ ɴᴀᴍᴀ*\n\n> Format: nama1 nama2\n\n\
             `Contoh: {}kecocokannamapasangan putu keyla`",
            ctx.prefix
        ));
    }
    let nama1 = &ctx.args[0];
    let nama2 = &ctx.args[1];

    let url = format!(
        "{ENDPOINT}?nama1={}&nama2={}",
        encode_uri_component(nama1),
        encode_uri_component(nama2)
    );
    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let gagal = Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa").with_reaction("❌");
    let Some(body) = resp.json() else {
        return gagal;
    };
    let Some(result) = payload(&body) else {
        return gagal;
    };

    Reply::text(render(result)).with_reaction("✅")
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
            command: "kecocokannamapasangan".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| a.to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("kecocokannamapasangan").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("kecocokannamapasangan").unwrap();
        assert_eq!(p.meta.name, "kecocokannamapasangan");
        assert_eq!(p.meta.alias, ["cocoknama", "matchname"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Cek kecocokan nama pasangan");
        assert_eq!(p.meta.usage, ".kecocokannamapasangan <nama1> <nama2>");
        assert_eq!(p.meta.example, ".kecocokannamapasangan putu keyla");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    /// `m.args.length < 2` — satu argumen—even yang panjang—tetap ditolak.
    #[test]
    fn fewer_than_two_names_prints_the_format_and_skips_http() {
        let panjang = "a".repeat(5000);
        for args in [
            Vec::new(),
            vec!["putu"],
            vec![panjang.as_str()],
            // Emoji tidak menambah argumen: `m.args` sudah token whitespace.
            vec!["😀😀"],
        ] {
            let r = run_with(Arc::new(NoHttp), &args);
            let t = reply_text(&r);
            assert!(t.starts_with("💕 *ᴋᴇᴄᴏᴄᴏᴋᴀɴ ɴᴀᴍᴀ*"), "{t}");
            assert!(t.contains("Format: nama1 nama2"), "{t}");
            assert!(matches!(r.without_mutations(), Reply::Text { .. }));
        }
    }

    #[test]
    fn a_good_payload_renders_both_sides_and_the_note() {
        let r = run_with(
            Arc::new(Stub(json!({
                "status": true,
                "data": {
                    "nama_anda": "Putu",
                    "nama_pasangan": "Keyla",
                    "sisi_positif": "Saling VERTICAL.",
                    "sisi_negatif": "Sering bertengkar.",
                    "catatan": "catatan primbon"
                }
            }))),
            &["putu", "keyla"],
        );
        let t = reply_text(&r);
        assert!(t.starts_with("💕 *ᴋᴇᴄᴏᴄᴏᴋᴀɴ ɴᴀᴍᴀ ᴘᴀsᴀɴɢᴀɴ*"), "{t}");
        assert!(t.contains("> 👤 Putu"), "{t}");
        assert!(t.contains("> 💑 Keyla"), "{t}");
        assert!(t.contains("✅ *ꜱɪꜱɪ ᴘᴏꜱɪᴛɪꜰ:*\nSaling VERTICAL."), "{t}");
        assert!(t.contains("❌ *ꜱɪꜱɪ ɴᴇɢᴀᴛɪꜰ:*\nSering bertengkar."), "{t}");
        assert!(t.ends_with("> _catatan primbon_"), "{t}");
    }

    #[test]
    fn a_falsy_status_or_data_gives_the_failure_message() {
        for body in [json!({ "data": {} }), json!({ "status": true }), json!({})] {
            let r = run_with(Arc::new(Stub(body)), &["putu", "keyla"]);
            assert_eq!(reply_text(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Gagal menganalisa");
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), &["putu", "keyla"]);
        let t = reply_text(&r);
        assert!(t.starts_with("☢ Kayaknya command "), "{t}");
        assert!(t.contains(".kecocokannamapasangan"), "{t}");
    }
}
