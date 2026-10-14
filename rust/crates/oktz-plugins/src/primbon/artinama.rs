//! `plugins/primbon/artinama.js` — cek arti nama lewat API primbon.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

pub(crate) const ENDPOINT: &str = "https://api.siputzx.my.id/api/primbon/artinama";

/// `te(prefix, command, pushName)` dengan template default (`src/lib/error.js`).
///
/// `ponytail:` `Ctx` tidak membawa `pushName` dan plugin tidak bisa membaca
/// `config.errorTemplate`, jadi keduanya jatuh ke fallback JS.
pub(crate) fn error_reply(prefix: &str, command: &str) -> String {
    format!(
        "☢ *ᴇʀʀᴏʀ*\n\n> Terjadi kesalahan pada command `{prefix}{command}`\n\
         > Silahkan coba lagi nanti, User\n\n_Jika masalah berlanjut, hubungi owner_"
    )
}

/// `${v}` — interpolasi string literal JS. String lewat apa adanya, angka jadi
/// digit JS-nya, field yang tidak ada jadi `"undefined"` — persis yang dicetak
/// template literal, bukan string kosong.
pub(crate) fn display(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(other) => other.to_string(),
    }
}

/// Kebenaran JS: `null`, `false`, `0`, `""`, `NaN`, dan array/object kosong
/// semuanya falsy.
pub(crate) fn truthy(v: &Value) -> bool {
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `!data?.status || !data?.data` → `None` kalau salah satunya falsy.
pub(crate) fn payload(body: &Value) -> Option<&Value> {
    let data = body.get("data").filter(|v| truthy(v))?;
    body.get("status").filter(|v| truthy(v)).map(|_| data)
}

#[plugin(
    name = "artinama",
    alias = ["namameaning", "artinamaku"],
    category = "primbon",
    description = "Cek arti nama menurut primbon",
    usage = ".artinama <nama>",
    example = ".artinama putu",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false
)]
pub fn artinama(ctx: Ctx) -> Reply {
    let nama = ctx.args.join(" ");
    if nama.is_empty() {
        return Reply::text(format!(
            "📛 *ᴀʀᴛɪ ɴᴀᴍᴀ*\n\n> Masukkan nama\n\n`Contoh: {}artinama putu`",
            ctx.prefix
        ));
    }

    let url = format!("{ENDPOINT}?nama={}", encode_uri_component(&nama));
    let error = error_reply(&ctx.prefix, &ctx.command);

    let Ok(resp) = ctx.http.get(&url) else {
        return Reply::text(error).with_reaction("☢");
    };
    let Some(body) = resp.json() else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat menganalisa nama")
            .with_reaction("❌");
    };

    let Some(result) = payload(&body) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat menganalisa nama")
            .with_reaction("❌");
    };

    Reply::text(format!(
        "📛 *ᴀʀᴛɪ ɴᴀᴍᴀ*\n\n> Nama: *{}*\n\n{}\n\n> _{}_",
        display(result.get("nama")),
        display(result.get("arti")),
        display(result.get("catatan")),
    ))
    .with_reaction("✅")
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
            command: "artinama".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.unwrap_or("").into(),
            args: arg.map(|a| vec![a.to_string()]).unwrap_or_default(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("artinama").unwrap().handler)(ctx)
    }

    fn reply_text(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn ok(payload: Value) -> Reply {
        run_with(
            Arc::new(Stub(json!({ "status": true, "data": payload }))),
            Some("putu"),
        )
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("artinama").unwrap();
        assert_eq!(p.meta.name, "artinama");
        assert_eq!(p.meta.alias, ["namameaning", "artinamaku"]);
        assert_eq!(p.meta.category, "primbon");
        assert_eq!(p.meta.description, "Cek arti nama menurut primbon");
        assert_eq!(p.meta.usage, ".artinama <nama>");
        assert_eq!(p.meta.example, ".artinama putu");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn without_a_name_it_asks_for_one_and_does_not_call_http() {
        let r = run_with(Arc::new(NoHttp), None);
        let t = reply_text(&r);
        assert!(t.starts_with("📛 *ᴀʀᴛɪ ɴᴀᴍᴀ*"), "{t}");
        assert!(t.contains("Masukkan nama"), "{t}");
        assert!(t.contains(".artinama putu"), "{t}");
    }

    #[test]
    fn a_good_payload_is_rendered_field_by_field() {
        let r = ok(json!({
            "nama": "Putu",
            "arti": " meanings ",
            "catatan": "catatan dari primbon",
        }));
        let t = reply_text(&r);
        assert!(t.starts_with("📛 *ᴀʀᴛɪ ɴᴀᴍᴀ*"), "{t}");
        assert!(t.contains("> Nama: *Putu*"), "{t}");
        assert!(t.contains("meanings"), "{t}");
        assert!(t.ends_with("> _catatan dari primbon_"), "{t}");
    }

    /// `!data?.status || !data?.data` — kedua cabang sama-sama "GAGAL" + ❌.
    #[test]
    fn a_missing_status_or_data_falls_into_the_failure_branch() {
        for body in [json!({ "data": {} }), json!({ "status": true }), json!({})] {
            let r = run_with(Arc::new(Stub(body)), Some("putu"));
            assert_eq!(
                reply_text(&r),
                "❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat menganalisa nama"
            );
        }
    }

    #[test]
    fn a_transport_error_uses_the_error_template_and_the_skull_reaction() {
        let r = run_with(Arc::new(NoHttp), Some("putu"));
        let t = reply_text(&r);
        assert!(t.starts_with("☢ *ᴇʀʀᴏʀ*"), "{t}");
        assert!(t.contains(".artinama"), "{t}");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn the_name_is_url_encoded_before_it_reaches_the_api() {
        assert_eq!(encode_uri_component("putu"), "putu");
        assert_eq!(encode_uri_component("a b&c=d"), "a%20b%26c%3Dd");
    }
}
