//! `plugins/info/gag2.js` — cek stock Grow a Garden dengan fitur watch.
//!
//! Catatan: `mod.rs` kategori `info` sudah memuat plugin `gag` dan
//! `gagwatch` yang bukan milik tugas ini, jadi baris `pub mod gag2;`
//! perlu ditambahkan manual di `rust/crates/oktz-plugins/src/info/mod.rs`.
//!
//! `setTimeout` / loop 30 detik untuk mode watch tidak punya padanan di
//! `Ctx`, jadi hanya mode stock yang diimplementasikan. Mode watch
//! melaporkan bahwa tidak tersedia di konteks Rust.

use oktz_plugin_api::{Ctx, HttpCall, HttpMethod, Reply, plugin};
use serde_json::Value;

const API: &str = "https://api.rifkyshre.biz.id";
const ROUTE: &str = "/scrape/gag2-stock";

fn emoji_for_item(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("seed") {
        "🌱"
    } else if n.contains("watering") || n.contains("hose") || n.contains("sprinkler") {
        "💧"
    } else if n.contains("crate") || n.contains("box") {
        "📦"
    } else if n.contains("egg") {
        "🥚"
    } else {
        "•"
    }
}

fn format_stock(d: &Value) -> String {
    let mut lines = Vec::new();
    lines.push(format!("*{}*", d.get("message").and_then(Value::as_str).unwrap_or("")));
    lines.push(String::new());
    lines.push(format!(
        "- ⏰ Restock in : *{}*",
        d.get("restockInLabel").and_then(Value::as_str).unwrap_or("")
    ));
    lines.push(format!(
        "- 🔄 Rotation   : *{} (cycle {}s)*",
        d.get("rotationId").and_then(Value::as_str).unwrap_or(""),
        d.get("cycleSeconds").and_then(Value::as_i64).unwrap_or(0)
    ));
    lines.push(format!(
        "- 📊 Status     : *{} ({})*",
        d.get("status").and_then(Value::as_str).unwrap_or(""),
        d.get("staleness").and_then(Value::as_str).unwrap_or("")
    ));
    let cache = d.get("cache").and_then(Value::as_object).cloned().unwrap_or(Value::Null);
    lines.push(format!(
        "- 💾 Cache      : *{} (age {}s/{}s)*",
        if cache.get("servedFromCache").and_then(Value::as_bool).unwrap_or(false) {
            "HIT"
        } else {
            "MISS"
        },
        cache.get("ageSec").and_then(Value::as_i64).unwrap_or(0),
        cache.get("ttlSec").and_then(Value::as_i64).unwrap_or(0)
    ));
    lines.push(String::new());

    if let Some(weather) = d.get("weather").and_then(Value::as_object) {
        if weather.get("active").and_then(Value::as_bool).unwrap_or(false) {
            lines.push(format!(
                "- ⛅ *WEATHER: {}*",
                weather.get("type").and_then(Value::as_str).unwrap_or("").to_uppercase()
            ));
            if let Some(effects) = weather.get("effects").and_then(Value::as_array) {
                for eff in effects {
                    lines.push(format!(
                        "   - ✨ {}",
                        eff.as_str().unwrap_or("")
                    ));
                }
            }
            lines.push(String::new());
        }
    }

    for (key, title, emoji) in [
        ("seeds", "SEEDS", "🌱"),
        ("gear", "GEAR", "⚙️"),
        ("crates", "CRATES", "📦"),
    ] {
        if let Some(items) = d.get(key).and_then(Value::as_array) {
            if !items.is_empty() {
                lines.push(format!("- {} *{} ({})*:", emoji, title, items.len()));
                for item in items {
                    let name = item.get("name").and_then(Value::as_str).unwrap_or("");
                    let qty = item.get("quantity").and_then(Value::as_i64).unwrap_or(0);
                    lines.push(format!("   - {name} × {qty}"));
                }
                lines.push(String::new());
            }
        }
    }

    lines.join("\n")
}

fn gag2_fetch(http: &dyn oktz_plugin_api::HttpGet) -> Result<Value, String> {
    let url = format!("{API}{ROUTE}");
    let res = http
        .call(
            &HttpCall::get(url)
                .header("Accept", "application/json")
                .header("Origin", "https://code.rifkyshre.biz.id")
                .header("Referer", "https://code.rifkyshre.biz.id/")
                .terima_semua_status(),
        )
        .map_err(|e| format!("HTTP {}", e.message))?;
    let data = res.json().unwrap_or(Value::Null);
    if data.get("status").is_none() || !data["status"].as_bool().unwrap_or(false) {
        let err = data
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or(&format!("HTTP {}", res.status))
            .to_string();
        return Err(err);
    }
    Ok(data
        .get("data")
        .cloned()
        .unwrap_or(Value::Null))
}

#[plugin(
    name = "gag2",
    alias = ["growagarden2"],
    category = "info",
    description = "Cek stock Grow a Garden dengan fitur watch",
    usage = ".gag2 [watch] [item]",
    example = ".gag2 watch seed",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn gag2(ctx: Ctx) -> Reply {
    let is_watch = ctx
        .args
        .first()
        .map(|a| a.to_lowercase() == "watch")
        .unwrap_or(false);

    if is_watch {
        let items: Vec<&str> = ctx.args.iter().skip(1).map(|s| s.as_str()).collect();
        if items.is_empty() {
            return Reply::text(
                "❌ Masukkan nama item yang ingin dipantau. Contoh: `.gag2 watch seed apple`",
            );
        }
        return Reply::text(
            "👀 *Watch mode tidak tersedia di konteks Rust.*\nGunakan `.gag2` untuk cek stock saat ini.",
        );
    }

    match gag2_fetch(ctx.http.as_ref()) {
        Ok(data) => Reply::text(format_stock(&data)),
        Err(err) => Reply::text(format!("❌ {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    struct Stub {
        body: Option<Value>,
    }

    impl Stub {
        fn ok() -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(serde_json::json!({
                    "status": true,
                    "data": {
                        "message": "Stock terbaru",
                        "restockInLabel": "10 menit",
                        "rotationId": "r1",
                        "cycleSeconds": 300,
                        "status": "aman",
                        "staleness": "segar",
                        "cache": { "servedFromCache": false, "ageSec": 5, "ttlSec": 60 },
                        "seeds": [ { "name": "Benih Apel", "quantity": 10 } ],
                        "gear": [ { "name": "Minyak", "quantity": 5 } ],
                        "crates": [ { "name": "Kotak Mysterius", "quantity": 2 } ],
                    }
                })),
            })
        }
        fn fail() -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(serde_json::json!({ "status": false, "error": "gagal" })),
            })
        }
    }

    impl oktz_plugin_api::HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.body.clone().unwrap_or(Value::Null).to_string().into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "gag2".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("gag2").unwrap();
        assert_eq!(p.meta.name, "gag2");
        assert_eq!(p.meta.alias, ["growagarden2"]);
        assert_eq!(p.meta.category, "info");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn alias_growagarden2_menunjuk_ke_gag2() {
        assert_eq!(oktz_plugin_api::find("growagarden2").unwrap().meta.name, "gag2");
    }

    #[test]
    fn mode_stock_mengembalikan_format_lengkap() {
        let mut c = ctx();
        c.http = Stub::ok();
        let r = gag2(c);
        let t = text_of(&r);
        assert!(t.contains("*Stock terbaru*"));
        assert!(t.contains("Benih Apel × 10"));
        assert!(t.contains("Minyak × 5"));
        assert!(t.contains("Kotak Mysterius × 2"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn status_false_mengembalikan_error() {
        let mut c = ctx();
        c.http = Stub::fail();
        let r = gag2(c);
        assert!(text_of(&r).contains("❌ gagal"));
    }

    #[test]
    fn watch_tanpa_item_mengembalikan_error() {
        let mut c = ctx();
        c.args = vec!["watch".into()];
        let r = gag2(c);
        assert!(text_of(&r).contains("Masukkan nama item"));
    }

    #[test]
    fn emoji_for_item_berdasar_nama() {
        assert_eq!(emoji_for_item("Benih Apel"), "🌱");
        assert_eq!(emoji_for_item("Minyak Tanam"), "💧");
        assert_eq!(emoji_for_item("Kotak Hias"), "📦");
        assert_eq!(emoji_for_item("Telur Emas"), "🥚");
        assert_eq!(emoji_for_item("Batu"), "•");
    }
}
