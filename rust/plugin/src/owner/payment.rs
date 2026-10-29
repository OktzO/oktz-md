//! `plugins/owner/payment.js` — menampilkan info payment.
//!
//! Catatan: `mod.rs` kategori `owner` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod payment;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/owner/mod.rs`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const BOT_DEFAULT: &str = "Bot";
const OWNER_DEFAULT: &str = "Owner";

fn metode_baris(m: &serde_json::Value) -> String {
    let nama = m
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let nomor = m
        .get("number")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    let pemilik = pemilik_atau_nama(m);
    if pemilik.is_empty() {
        format!("• *{nama}*: {nomor}")
    } else {
        format!("• *{nama}*: {nomor} (a/n: {pemilik})")
    }
}

fn pemilik_atau_nama(m: &serde_json::Value) -> String {
    m.get("holder")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .or_else(|| m.get("name").and_then(serde_json::Value::as_str))
        .unwrap_or("")
        .to_string()
}

/// `buildPaymentText` dari payment.js.
fn build_payment_text(
    payment: &serde_json::Value,
    custom_text: &str,
    bot_name: &str,
    owner_name: &str,
) -> String {
    let methods = payment
        .get("methods")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|m| {
                    m.get("number")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let banks = payment
        .get("banks")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter(|b| {
                    b.get("number")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|s| !s.is_empty())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if !custom_text.is_empty() {
        let methods_txt = methods.iter().copied().map(metode_baris).collect::<Vec<_>>().join("\n");
        let banks_txt = banks.iter().copied().map(metode_baris).collect::<Vec<_>>().join("\n");
        let qris_txt = if payment
            .get("qrisUrl")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|s| !s.is_empty())
        {
            "✅ Tersedia"
        } else {
            "❌ Belum diatur"
        };
        return custom_text
            .replace("{botname}", bot_name)
            .replace("{owner}", owner_name)
            .replace("{methods}", &methods_txt)
            .replace("{banks}", &banks_txt)
            .replace("{qris}", qris_txt);
    }

    let mut text = "💳 *P A Y M E N T*\n━━━━━━━━━━━━━━━━━━\n\n".to_string();

    if !methods.is_empty() {
        text.push_str("📱 *E-Wallet:*\n");
        for m in &methods {
            let nama = m
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            text.push_str(&format!("├─ • *{nama}*\n"));
            let nomor = m
                .get("number")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            text.push_str(&format!("│  `{nomor}`\n"));
            if let Some(p) = m.get("holder").and_then(serde_json::Value::as_str)
                && !p.is_empty()
            {
                text.push_str(&format!("│  a/n: {p}\n"));
            }
        }
        text.push('\n');
    }

    if !banks.is_empty() {
        text.push_str("🏦 *Bank Transfer:*\n");
        for b in &banks {
            let nama = b
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            text.push_str(&format!("├─ • *{nama}*\n"));
            let nomor = b
                .get("number")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            text.push_str(&format!("│  `{nomor}`\n"));
            if let Some(p) = b.get("holder").and_then(serde_json::Value::as_str)
                && !p.is_empty()
            {
                text.push_str(&format!("│  a/n: {p}\n"));
            }
        }
        text.push('\n');
    }

    if let Some(qris) = payment.get("qrisUrl").and_then(serde_json::Value::as_str)
        && !qris.is_empty()
    {
        text.push_str("📸 *QRIS:* Tersedia (lihat gambar)\n\n");
    }

    text.push_str("━━━━━━━━━━━━━━━━━━\n");
    text.push_str(&format!("> {bot_name}"));
    text
}

#[plugin(
    name = "payment",
    alias = ["qris"],
    category = "owner",
    description = "Menampilkan info payment",
    usage = ".payment",
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false,
    cooldown = 5,
    energi = 0
)]
pub fn payment(ctx: Ctx) -> Reply {
    let has_any = !ctx.config().payment.qris_url.is_empty()
        || !ctx.config().payment.methods.is_empty()
        || !ctx.config().payment.banks.is_empty();

    let payment_cfg = if has_any {
        serde_json::json!({
            "qrisUrl": ctx.config().payment.qris_url,
            "customText": ctx.config().payment.custom_text,
            "methods": ctx
                .config()
                .payment
                .methods
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "name": m.name,
                        "number": m.number,
                        "holder": m.holder,
                    })
                })
                .collect::<Vec<_>>(),
            "banks": ctx
                .config()
                .payment
                .banks
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "name": b.name,
                        "number": b.number,
                        "holder": b.holder,
                    })
                })
                .collect::<Vec<_>>(),
        })
    } else {
        serde_json::json!({})
    };

    let custom_text = ctx
        .setting("customPaymentText")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| ctx.config().payment.custom_text.clone());
    let bot_name = if ctx.config().bot.name.is_empty() {
        BOT_DEFAULT.to_string()
    } else {
        ctx.config().bot.name.clone()
    };
    let owner_name = if ctx.config().owner.name.is_empty() {
        OWNER_DEFAULT.to_string()
    } else {
        ctx.config().owner.name.clone()
    };

    let text = build_payment_text(&payment_cfg, &custom_text, &bot_name, &owner_name);

    let qris_url = payment_cfg
        .get("qrisUrl")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");

    if !qris_url.is_empty() {
        return Reply::image(MediaSource::url(qris_url), text);
    }

    Reply::text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "payment".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("payment").unwrap();
        assert_eq!(p.meta.name, "payment");
        assert_eq!(p.meta.alias, ["qris"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn config_kosong_mengembalikan_teks_bot_default() {
        let r = payment(ctx());
        assert!(text_of(&r).contains("💳 *P A Y M E N T*"));
        assert!(text_of(&r).contains("> Bot"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn qris_url_dipakai_sebagai_image() {
        let cfg = serde_json::json!({"qrisUrl": "https://example.com/qris.jpg"});
        let out = build_payment_text(&cfg, "", "Bot", "Owner");
        assert!(out.contains("📸 *QRIS:* Tersedia"));
    }
}
