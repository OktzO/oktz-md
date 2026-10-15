//! `plugins/primbon/ramalanjodoh.js` — ramalan jodoh dua orang dari tanggal lahir.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::primbon::artinama::{display, error_reply, truthy};
use crate::tools::phisingataubukan::encode_uri_component;

const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/ramalanjodoh";

fn render(data: &Value) -> String {
    let r = &data["result"];
    let mut out = String::from("💑 *ʀᴀᴍᴀʟᴀɴ ᴊᴏᴅᴏʜ*\n\n");
    for key in ["orang_pertama", "orang_kedua"] {
        let orang = &r[key];
        out.push_str(&format!(
            "👤 *{}*\n> {}\n\n",
            display(orang.get("nama")),
            display(orang.get("tanggal_lahir"))
        ));
    }
    out.push_str("📜 *ʜᴀꜱɪʟ ʀᴀᴍᴀʟᴀɴ:*\n");
    for (i, h) in r["hasil_ramalan"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        out.push_str(&format!("{}. {}\n\n", i + 1, display(Some(h))));
    }
    out.push_str(&format!("> ⚠️ _{}_", display(data.get("peringatan"))));
    out
}

#[plugin(
    name = "ramalanjodoh",
    alias = ["cekjodoh"],
    category = "primbon",
    description = "Ramalan jodoh berdasarkan primbon Jawa",
    usage = ".ramalanjodoh nama1 tgl1 bln1 thn1 nama2 tgl2 bln2 thn2",
    example = ".ramalanjodoh putu 16 11 2007 keyla 1 1 2008",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn ramalanjodoh(ctx: Ctx) -> Reply {
    if ctx.args.len() < 8 {
        return Reply::text(format!(
            "💑 *ʀᴀᴍᴀʟᴀɴ ᴊᴏᴅᴏʜ*\n\n> Format:\nrama1 tgl1 bln1 thn1 nama2 tgl2 bln2 thn2\n\n\
             `Contoh:\n{}ramalanjodoh putu 16 11 2007 keyla 1 1 2008`",
            ctx.prefix
        ));
    }
    let args = &ctx.args[..8];
    let (nama1, nama2) = (&args[0], &args[4]);
    let (tgl1, bln1, thn1) = (&args[1], &args[2], &args[3]);
    let (tgl2, bln2, thn2) = (&args[5], &args[6], &args[7]);

    let url = format!(
        "{ENDPOINT}?nama1={}&tgl1={tgl1}&bln1={bln1}&thn1={thn1}\
         &nama2={}&tgl2={tgl2}&bln2={bln2}&thn2={thn2}",
        encode_uri_component(nama1),
        encode_uri_component(nama2),
    );
    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    // `!data?.status || !data?.data?.result` — cek satu tingkat lebih dalam
    // dibanding plugin primbon lain.
    let Some(body) = resp.json() else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal meramal").with_reaction("❌");
    };
    let Some(data) = body.get("data").filter(|v| truthy(v)) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal meramal").with_reaction("❌");
    };
    if !body.get("status").is_some_and(truthy) || !data.get("result").is_some_and(truthy) {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Gagal meramal").with_reaction("❌");
    }

    Reply::text(render(data)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse, NoHttp};
    use serde_json::json;
    use std::sync::Arc;

    const LENUH: [&str; 8] = ["putu", "16", "11", "2007", "keyla", "1", "1", "2008"];

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
            command: "ramalanjodoh".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.join(" "),
            args: args.iter().map(|a| a.to_string()).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("ramalanjodoh").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn payload() -> Value {
        json!({
            "status": true,
            "data": {
                "result": {
                    "orang_pertama": { "nama": "Putu", "tanggal_lahir": "16-11-2007" },
                    "orang_kedua": { "nama": "Keyla", "tanggal_lahir": "01-01-2008" },
                    "hasil_ramalan": ["Saling Understand", "Sering Marah"]
                },
                "peringatan": "ramalan ini untuk hiburan"
            }
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ramalanjodoh").unwrap();
        assert_eq!(p.meta.name, "ramalanjodoh");
        assert_eq!(p.meta.alias, ["cekjodoh"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Ramalan jodoh berdasarkan primbon Jawa");
        assert_eq!(
            p.meta.usage,
            ".ramalanjodoh nama1 tgl1 bln1 thn1 nama2 tgl2 bln2 thn2"
        );
        assert_eq!(p.meta.example, ".ramalanjodoh putu 16 11 2007 keyla 1 1 2008");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn fewer_than_eight_arguments_prints_the_format_and_skips_http() {
        for n in 0..8 {
            let args: Vec<&str> = LENUH[..n].to_vec();
            let r = run_with(Arc::new(NoHttp), &args);
            let t = reply_text(&r);
            assert!(t.starts_with("💑 *ʀᴀᴍᴀʟᴀɴ ᴊᴏᴅᴏʜ*"), "{n} arg: {t}");
            assert!(t.contains("rama1 tgl1 bln1 thn1 nama2 tgl2 bln2 thn2"), "{t}");
        }
    }

    #[test]
    fn eight_arguments_render_both_people_the_numbered_list_and_the_warning() {
        let r = run_with(Arc::new(Stub(payload())), &LENUH);
        let t = reply_text(&r);
        assert!(t.starts_with("💑 *ʀᴀᴍᴀʟᴀɴ ᴊᴏᴅᴏʜ*"), "{t}");
        assert!(t.contains("👤 *Putu*\n> 16-11-2007"), "{t}");
        assert!(t.contains("👤 *Keyla*\n> 01-01-2008"), "{t}");
        assert!(t.contains("📜 *ʜᴀꜱɪʟ ʀᴀᴍᴀʟᴀɴ:*"), "{t}");
        assert!(t.contains("1. Saling Understand\n\n"), "{t}");
        assert!(t.contains("2. Sering Marah\n\n"), "{t}");
        assert!(t.ends_with("> ⚠️ _ramalan ini untuk hiburan_"), "{t}");
    }

    /// `!data?.status || !data?.data?.result` — `result` kosong = falsy.
    #[test]
    fn a_missing_status_data_or_result_gives_the_failure_message() {
        for body in [
            json!({ "data": { "result": {} } }),
            json!({ "status": true, "data": {} }),
            json!({ "status": true }),
            json!({ "status": true, "data": { "result": null } }),
            json!({}),
        ] {
            let r = run_with(Arc::new(Stub(body.clone())), &LENUH);
            assert_eq!(reply_text(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Gagal meramal", "{body}");
        }
    }

    /// `hasil_ramalan` bukan array — `forEach` JS di-skip, daftar kosong saja.
    #[test]
    fn a_non_array_hasil_ramalan_leaves_the_numbered_list_empty() {
        let mut body = payload();
        body["data"]["result"]["hasil_ramalan"] = json!("bukan array");
        let r = run_with(Arc::new(Stub(body)), &LENUH);
        let t = reply_text(&r);
        assert!(t.contains("📜 *ʜᴀꜱɪʟ ʀᴀᴍᴀʟᴀɴ:*\n"), "{t}");
        assert!(!t.contains("1. bukan array"), "{t}");
    }

    #[test]
    fn a_transport_error_uses_the_error_template() {
        let r = run_with(Arc::new(NoHttp), &LENUH);
        let t = reply_text(&r);
        assert!(t.starts_with("☢ Kayaknya command "), "{t}");
        assert!(t.contains(".ramalanjodoh"), "{t}");
    }
}
