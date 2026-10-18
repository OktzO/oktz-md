//! `plugins/tools/hitungwrmlbb.js` — kalkulator win rate Mobile Legends.
//! Hanya butuh jaringan HTTP (ctx.http), tidak ada capability tambahan.
//! Catatan: `tools/mod.rs` sudah milik plugin lain, jangan sentuh;
//! orchestrator harus menambah baris `pub mod hitungwrmlbb;` ke `tools/mod.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::error_template;

const API_BASE: &str = "https://api.nexray.eu.cc/tools/winrate-mlbb";

/// `hitungwrmlbb.js:16-22` — kartu pakai saat argumen kurang dari 3.
fn kartu_pakai(prefix: &str) -> String {
    format!(
        "Halo *{}* 👋\n\nUntuk menghitung target Win Rate MLBB kamu, gunakan format:\n\
         - `{}hitungwrmlbb <total_match> <wr_sekarang> <wr_target>`\n\nContoh:\n\
         - `{}hitungwrmlbb 4242 22 88`",
        "User", prefix, prefix
    )
}

/// `isNaN(x)` JS — setara `Number(x)` yang NaN.
///
/// Bukan `parse::<f64>()` polos: Rust menerima `"nan"` dan `"inf"` yang di
/// JS justru NaN, dan Rust menolak `"0x10"` yang di JS bernilai 16.
fn is_nan_js(s: &str) -> bool {
    let t = s.trim();
    // Number("") === 0, bukan NaN.
    if t.is_empty() {
        return false;
    }
    // Number("NaN")/Number("inf") === NaN; Rust malah menerimanya.
    if t.eq_ignore_ascii_case("nan") || t.eq_ignore_ascii_case("inf") {
        return true;
    }
    if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return h.is_empty() || !h.chars().all(|c| c.is_ascii_hexdigit());
    }
    match t.parse::<f64>() {
        Ok(v) => v.is_nan(),
        Err(_) => true,
    }
}

/// `hitungwrmlbb.js:43-47` — susun teks hasil dari `data.result`.
fn hasil_teks(total_match: &str, wr_now: &str, wr_target: &str, result: &str) -> String {
    format!(
        "🎮 *KALKULATOR WINRATE MLBB* 🎮\n\n📊 Total Match: *{}*\n📉 WR Sekarang: *{}%*\
         \n📈 WR Target: *{}%*\n\n💡 *Hasil Analisa:*\n{}",
        total_match, wr_now, wr_target, result
    )
}

#[plugin(
    name = "hitungwrmlbb",
    alias = ["wrml", "wrmlbb", "winrate"],
    category = "tools",
    description = "Hitung kebutuhan win tanpa lose untuk mencapai target win rate di Mobile Legends.",
    usage = ".hitungwrmlbb <total_match> <wr_sekarang> <wr_target>",
    example = ".hitungwrmlbb 4242 22 88",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn hitungwrmlbb(ctx: Ctx) -> Reply {
    if ctx.args.len() < 3 {
        return Reply::text(kartu_pakai(&ctx.prefix));
    }

    let total_match = &ctx.args[0];
    let wr_now = &ctx.args[1];
    let wr_target = &ctx.args[2];

    // JS: `isNaN(totalMatch) || isNaN(wrNow) || isNaN(wrTarget)`.
    if is_nan_js(total_match) || is_nan_js(wr_now) || is_nan_js(wr_target) {
        return Reply::text(
            "❌ Angka tidak valid. Pastikan semua yang dimasukkan adalah angka (tanpa %).\n\n\
             Contoh: `.hitungwrmlbb 4242 22 88`",
        );
    }

    // `m.react("🕕")` — dilewatkan, satu reaksi saja yang dapat dibawa Reply.

    let url = format!(
        "{}?total_match={}&wr_now={}&wr_target={}",
        API_BASE, total_match, wr_now, wr_target
    );

    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("❌");
    };

    let data = match res.json() {
        Some(v) => v,
        None => {
            return Reply::text(
                "⚠️ Gagal menghitung Win Rate. Pastikan angkanya masuk akal atau coba lagi nanti.",
            )
            .with_reaction("❌")
        }
    };

    // JS: `if (!data.status || !data.result)`.
    let status_ok = data
        .get("status")
        .is_some_and(|v| v.is_boolean() && v.as_bool().unwrap_or(false));
    let Some(result) = data.get("result").and_then(Value::as_str) else {
        return Reply::text(
            "⚠️ Gagal menghitung Win Rate. Pastikan angkanya masuk akal atau coba lagi nanti.",
        )
        .with_reaction("❌");
    };

    if !status_ok {
        return Reply::text(
            "⚠️ Gagal menghitung Win Rate. Pastikan angkanya masuk akal atau coba lagi nanti.",
        )
        .with_reaction("❌");
    }

    Reply::text(hasil_teks(total_match, wr_now, wr_target, result)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
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
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn ctx(text: &str, http: Arc<Stub>) -> Ctx {
        let mut args = text.split_whitespace();
        Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "hitungwrmlbb".into(),
            http,
            ..Ctx::default()
        }
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
    fn argumen_kurang_dari_tiga_kembalikan_kartu_pakai() {
        for teks in ["", "4242", "4242 22"] {
            let r = hitungwrmlbb(ctx(teks, Stub::offline()));
            assert!(text_of(&r).contains("hitungwrmlbb <total_match>"), "{teks}");
            assert_eq!(reaction_of(&r), None, "{teks}");
        }
    }

    #[test]
    fn argumen_bukan_angka_ditolak() {
        let r = hitungwrmlbb(ctx("abc 22 88", Stub::offline()));
        assert!(text_of(&r).contains("Angka tidak valid"));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn isnan_js_meniru_number_coercion() {
        // isNaN(x) = Number(x) NaN: "50px" invalid, angka turun valid,
        // string kosong jadi 0 (bukan NaN), sama seperti JS.
        assert!(is_nan_js("50px"), "50px harus NaN di JS");
        assert!(is_nan_js("abc"), "abc harus NaN di JS");
        assert!(!is_nan_js(""), "kosong jadi 0, bukan NaN di JS");
        assert!(!is_nan_js("  22 "), "spasi diabaikan di JS");
        assert!(!is_nan_js("4242"), "angka utuh bukan NaN");
        assert!(!is_nan_js("22"), "angka utuh bukan NaN");
        assert!(!is_nan_js("0x10"), "heksadesimal valid di Number() JS");
        assert!(!is_nan_js("Infinity"), "Infinity bukan NaN di JS");
        assert!(!is_nan_js("-Infinity"), "-Infinity bukan NaN di JS");
        assert!(is_nan_js("NaN"), "teks NaN adalah NaN di JS");
        assert!(is_nan_js("inf"), "JS tidak mengenal inf");
    }

    #[test]
    fn jaringan_gagal_kembalikan_pesan_error_dan_reaksi_sil() {
        let r = hitungwrmlbb(ctx("4242 22 88", Stub::offline()));
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.hitungwrmlbb` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn status_false_atau_result_kosong_gagal() {
        for body in [
            serde_json::json!({ "status": false, "result": "x" }),
            serde_json::json!({ "status": true }),
            serde_json::json!({}),
            Value::Null,
        ] {
            let r = hitungwrmlbb(ctx("4242 22 88", Stub::body(body.clone())));
            assert!(
                text_of(&r).contains("Gagal menghitung Win Rate"),
                "body {body}"
            );
            assert_eq!(reaction_of(&r), Some("❌"), "body {body}");
        }
    }

    #[test]
    fn sukses_mengirim_hasil_dengan_reaksi_ceklis() {
        let body = serde_json::json!({
            "status": true,
            "result": "Kamu butuh 100 kemasan..."
        });
        let http = Stub::body(body.clone());
        let r = hitungwrmlbb(ctx("4242 22 88", http.clone()));
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(text_of(&r).contains("Total Match: *4242*"));
        assert!(text_of(&r).contains("WR Sekarang: *22%*"));
        assert!(text_of(&r).contains("WR Target: *88%*"));
        assert!(text_of(&r).contains("Kamu butuh 100 kemasan..."));
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            &["https://api.nexray.eu.cc/tools/winrate-mlbb?total_match=4242&wr_now=22&wr_target=88"]
        );
    }

    #[test]
    fn url_membawa_parameter() {
        let _ = hitungwrmlbb(ctx("4242 22 88", Stub::offline()));
    }
}
