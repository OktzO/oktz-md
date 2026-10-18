//! `plugins/tools/cekxl.js` — cek paket dan kuota XL/Axis via `xl-ku.my.id`.
//!
//! `ponytail:` JS memakai `axios.get(..., { timeout: 30000 })`; `UreqHttp`
//! punya timeout global sendiri, jadi timeout per-permintaan tidak ditiru.
//! Balasan tidak dikutip (JS `m.reply(txt.trim())` polos).

use oktz_plugin_api::{Ctx, HttpCall, HttpError, HttpGet, Reply, plugin};
use serde_json::{json, Value};

use crate::common::{error_template, js_truthy_value};
use crate::tools::phisingataubukan::encode_uri_component;

/// `cleanNumber` (`cekxl.js:20-25`): buang non-digit, awalan `0` → `62`, dan
/// tambah `62` kalau belum ada.
fn clean_number(raw: &str) -> String {
    let num: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    let num = if num.starts_with('0') {
        format!("62{}", &num[1..])
    } else if num.starts_with("62") {
        num
    } else {
        format!("62{num}")
    };
    num
}

fn help(prefix: &str) -> String {
    format!(
        "📱 *CEK XL/AXIS*\n\n\
         Fitur ini digunakan untuk mengecek informasi paket dan kuota yang tersedia pada nomor XL atau Axis kamu secara lengkap dan detail\n\n\
         *Cara pakai:*\n\
         > `{prefix}cekxl <nomor hp>`\n\n\
         *Contoh:*\n\
         > `{prefix}cekxl 083150850721`\n\
         > `{prefix}cekxl 6281234567890`\n\n\
         _Format nomor bisa pakai 08xx, 628xx, atau tanpa awalan_"
    )
}

/// `https://xl-ku.my.id/end.php?check=package&number={n}&version=2`.
fn api_url(number: &str) -> String {
    format!(
        "https://xl-ku.my.id/end.php?check=package&number={}&version=2",
        encode_uri_component(number)
    )
}

/// `❌ Nomor yang kamu masukkan tidak valid...`.
const INVALID: &str =
    "❌ Nomor yang kamu masukkan tidak valid, pastikan nomor tersebut merupakan nomor XL atau Axis yang benar ya";

/// `❌ Tidak bisa mengecek nomor *{cleanNum}*, pastikan ... XL atau Axis yang aktif`.
fn not_found(clean_num: &str) -> String {
    format!(
        "❌ Tidak bisa mengecek nomor *{clean_num}*, pastikan nomor tersebut merupakan nomor XL atau Axis yang aktif ya"
    )
}

#[plugin(
    name = "cekxl",
    alias = ["checkxl", "xlcheck", "xlcek"],
    category = "tools",
    description = "Cek informasi paket dan kuota nomor XL/Axis secara detail",
    usage = ".cekxl <nomor>",
    example = ".cekxl 083150850721",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cekxl(ctx: Ctx) -> Reply {
    let input = ctx.args.first().cloned().unwrap_or_else(|| ctx.text.trim().to_string());
    if input.trim().is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let clean_num = clean_number(&input);
    if clean_num.len() < 10 || clean_num.len() > 15 {
        return Reply::text(INVALID);
    }

    let Ok(res) = ctx.http.call(
        &HttpCall::get(&api_url(&clean_num)).header("Accept", "application/json"),
    ) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    let data = res.json().unwrap_or(Value::Null);

    // `!data || data.error || data.status === false` — ketiganya menghasilkan
    // teks yang sama, gabung jadi satu syarat.
    let data_null = data.is_null();
    let error_truthy = data.get("error").is_some_and(js_truthy_value);
    let status_false = data.get("status").is_some_and(|v| v == &Value::Bool(false));
    if data_null || error_truthy || status_false {
        return Reply::text(not_found(&clean_num)).with_reaction("❌");
    }

    let mut txt = format!("📱 *INFORMASI XL/AXIS*\n\n📞 Nomor: *{clean_num}*\n");

    if let Some(v) = data.get("msisdn") {
        txt.push_str(&format!("🆔 MSISDN: *{}*\n", js_str(v)));
    }
    if let Some(v) = data.get("status") {
        txt.push_str(&format!("📊 Status: *{}*\n", js_str(v)));
    }
    if let Some(v) = data.get("activeDate") {
        txt.push_str(&format!("📅 Aktif Sejak: *{}*\n", js_str(v)));
    }
    if let Some(v) = data.get("expireDate") {
        txt.push_str(&format!("⏰ Masa Aktif: *{}*\n", js_str(v)));
    }
    if let Some(v) = data.get("graceDate") {
        txt.push_str(&format!("⚠️ Masa Tenggang: *{}*\n", js_str(v)));
    }

    if let Some(pkgs) = data.get("packages").and_then(Value::as_array) {
        if !pkgs.is_empty() {
            txt.push_str("\n📦 *DAFTAR PAKET AKTIF*\n\n");
            for pkg in pkgs {
                let name = pkg
                    .get("name")
                    .or_else(|| pkg.get("packageName"))
                    .and_then(Value::as_str)
                    .unwrap_or("Paket");
                txt.push_str(&format!("- *{name}*\n"));
                let quota = pkg
                    .get("remainingQuota")
                    .or_else(|| pkg.get("quota"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
                if let Some(q) = quota {
                    txt.push_str(&format!("  > Sisa Kuota: *{q}*\n"));
                }
                if let Some(v) = pkg.get("totalQuota").and_then(Value::as_str) {
                    txt.push_str(&format!("  > Total Kuota: *{v}*\n"));
                }
                let berlaku = pkg
                    .get("expireDate")
                    .or_else(|| pkg.get("validUntil"))
                    .and_then(Value::as_str);
                if let Some(b) = berlaku {
                    txt.push_str(&format!("  > Berlaku Sampai: *{b}*\n"));
                }
                if let Some(v) = pkg.get("type").and_then(Value::as_str) {
                    txt.push_str(&format!("  > Tipe: *{v}*\n"));
                }
                txt.push('\n');
            }
        }
    }

    if data.get("balance").is_some() || data.get("pulsa").is_some() {
        let pulsa = data
            .get("balance")
            .or_else(|| data.get("pulsa"))
            .and_then(Value::as_str)
            .unwrap_or("");
        txt.push_str(&format!("💰 *SALDO*\n> Pulsa: *{pulsa}*\n\n"));
    }

    if let Some(r) = data.get("result") {
        if r.is_object() {
            if let Some(v) = r.get("name").and_then(Value::as_str) {
                txt.push_str(&format!("👤 Nama Paket: *{v}*\n"));
            }
            if let Some(v) = r.get("quota").and_then(Value::as_str) {
                txt.push_str(&format!("📊 Kuota: *{v}*\n"));
            }
            if let Some(v) = r.get("masa_aktif").and_then(Value::as_str) {
                txt.push_str(&format!("📅 Masa Aktif: *{v}*\n"));
            }
            if let Some(v) = r.get("status").and_then(Value::as_str) {
                txt.push_str(&format!("📊 Status: *{v}*\n"));
            }
        }
    }

    if data.get("packages").is_none() && data.get("result").is_none() {
        let skip = [
            "error", "status", "msisdn", "activeDate", "expireDate", "graceDate",
            "balance", "pulsa",
        ];
        let extra: Vec<&str> = data
            .as_object()
            .into_iter()
            .flat_map(|m| m.keys())
            .filter(|k| !skip.contains(k))
            .copied()
            .collect();
        if !extra.is_empty() {
            txt.push_str("\n📋 *DETAIL LAINNYA*\n\n");
            for key in extra {
                if let Some(v) = data.get(key) {
                    if v.is_string() || v.is_number() {
                        txt.push_str(&format!("- {key}: *{}*\n", js_str(v)));
                    }
                }
            }
        }
    }

    Reply::text(txt.trim().to_string()).with_reaction("✅")
}

/// `String(v)` untuk nilai JSON yang masuk ke template literal (mirip
/// `cekxl.js` yang `${val}`-kan nilai).
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => v.to_string(),
        _ => "[object Object]".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use serde_json::json;
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
            command: "cekxl".into(),
            http,
            ..Ctx::default()
        };
        cekxl(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn tanpa_input_menghasilkan_kartu_pakai() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn nomor_terlalu_pendek_atau_panjang_ditolak() {
        for input in ["0812", "081234567890123456"] {
            let r = run(Stub::offline(), input);
            assert_eq!(text_of(&r), INVALID, "input {input:?}");
            assert_eq!(reaction_of(&r), None);
        }
    }

    #[test]
    fn clean_number_normalisasikan_awalan() {
        assert_eq!(clean_number("083150850721"), "6283150850721");
        assert_eq!(clean_number("6281234567890"), "6281234567890");
        assert_eq!(clean_number("81234567890"), "6281234567890");
        assert_eq!(clean_number("0831-508-507-21"), "6283150850721");
    }

    #[test]
    fn url_membawa_nomor_terencode() {
        assert_eq!(
            api_url("6283150850721"),
            "https://xl-ku.my.id/end.php?check=package&number=6283150850721&version=2"
        );
    }

    #[test]
    fn status_salah_atau_error_hasilkan_not_found() {
        for body in [
            serde_json::json!({ "error": true }),
            serde_json::json!({ "status": false }),
            serde_json::json!({}),
            Value::Null,
        ] {
            let r = run(Stub::body(body.clone()), "083150850721");
            assert_eq!(
                text_of(&r),
                not_found("6283150850721"),
                "body {body:?}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body:?}");
        }
    }

    #[test]
    fn jalur_berhasil_membangun_teks_detail() {
        let body = serde_json::json!({
            "status": true,
            "msisdn": "6283150850721",
            "activeDate": "2024-01-10",
            "expireDate": "2025-01-10",
            "balance": "50000",
            "packages": [
                { "name": "Paket A", "remainingQuota": "2GB", "totalQuota": "5GB", "expireDate": "2025-01-05", "type": "Reguler" },
                { "packageName": "Paket B", "quota": "1GB" },
            ],
            "result": { "name": "Paket Result", "quota": "3GB", "masa_aktif": "30 hari", "status": "Aktif" }
        });
        let r = run(Stub::body(body.clone()), "083150850721");
        assert_eq!(reaction_of(&r), Some("✅"));
        let t = text_of(&r);
        assert!(t.contains("📞 Nomor: *6283150850721*"), "{t}");
        assert!(t.contains("🆔 MSISDN: *6283150850721*"), "{t}");
        assert!(t.contains("- *Paket A*"), "{t}");
        assert!(t.contains("  > Sisa Kuota: *2GB*"), "{t}");
        assert!(t.contains("- *Paket B*"), "{t}");
        assert!(t.contains("👤 Nama Paket: *Paket Result*"), "{t}");
    }

    #[test]
    fn kegagalan_jaringan_menggunakan_error_template() {
        let r = run(Stub::offline(), "083150850721");
        assert_eq!(
            text_of(&r),
            crate::common::error_template(".", "cekxl"),
        );
        assert_eq!(reaction_of(&r), Some("☢"));
    }
}
