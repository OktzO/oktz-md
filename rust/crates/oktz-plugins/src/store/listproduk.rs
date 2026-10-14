//! `plugins/store/listproduk.js` — `db.setting('storeProducts')` (katalog).
//!
//! Seller.catalog di JS mengirim dengan `forwardedNewsletterMessageInfo`
//! kalau di grup (`listproduk.js:104-115`); `Reply` tidak punya cara
//! menyetel `contextInfo` forwarding, jadi teks yang sama dikirim sebagai
//! pesan biasa. Isi pesan tidak berubah.

use crate::store::list::js_parse_int;
use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

const EMPTY: &str = "🏪 *Produk Belum Tersedia*\n\nSaat ini belum ada produk yang ditambahkan oleh admin 😔\n\nSilakan cek kembali nanti atau hubungi admin untuk informasi lebih lanjut.\n\n_Terima kasih atas ketertarikan Anda_ 🙏";

/// `(n).toLocaleString("id-ID")` (`listproduk.js:21`) — pemisah ribuan titik,
/// desimal koma. Untuk harga yang selalu bulat, cukup titik ribuan.
fn format_price(n: i64) -> String {
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if neg {
        format!("-Rp {out}")
    } else {
        format!("Rp {out}")
    }
}

/// `p.type || "digital"` (`listproduk.js:44`) — nilai kosong/falsy jadi
/// `digital`, jadi `type` hanya dibaca kalau string non-kosong.
fn product_type(p: &Value) -> &str {
    match p.get("type").and_then(Value::as_str) {
        Some(t) if !t.is_empty() => t,
        _ => "digital",
    }
}

fn is_available(p: &Value) -> bool {
    let stock = p.get("stock").and_then(Value::as_i64);
    if product_type(p) == "digital" {
        p.get("stockItems")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
            || stock == Some(-1)
    } else {
        stock.is_some_and(|s| s > 0) || stock == Some(-1)
    }
}

/// Baris "Stok: …" — `listproduk.js:47-53`.
fn stock_display(p: &Value) -> String {
    let stock = p.get("stock").and_then(Value::as_i64);
    let unlimited = stock == Some(-1);
    let count = p
        .get("stockItems")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if product_type(p) == "digital" {
        if unlimited {
            "♾️ Unlimited".to_string()
        } else {
            format!("{count} akun")
        }
    } else if unlimited {
        "♾️ Unlimited".to_string()
    } else {
        format!("{} pcs", stock.unwrap_or(0))
    }
}

#[plugin(
    name = "listproduk",
    alias = ["produk", "katalog", "catalog"],
    category = "store",
    description = "🛍️ Lihat daftar produk yang tersedia",
    usage = ".listproduk",
    example = ".listproduk",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn listproduk(ctx: Ctx) -> Reply {
    let products = ctx.setting_list("storeProducts");
    if products.is_empty() {
        return Reply::text(EMPTY);
    }

    let mut txt = String::from("🛍️ *DAFTAR PRODUK*\n\n");
    txt.push_str("Berikut adalah produk yang tersedia saat ini 🎉\n");
    txt.push_str(&format!(
        "Untuk pembelian, ketik `{}beli <nomor>`\n\n",
        ctx.prefix
    ));

    for (i, p) in products.iter().enumerate() {
        let t = product_type(p);
        let (type_icon, type_label) = if t == "digital" {
            ("🔑", "Digital")
        } else {
            ("📦", "Fisik")
        };
        // `p.originalPrice ? ... : ""` (`listproduk.js:57`) — `null`/`0`
        // falsy jadi tidak ada harga coret.
        let original = match p.get("originalPrice").and_then(Value::as_i64) {
            Some(o) if o != 0 => format!("~~{}~~ ", format_price(o)),
            _ => String::new(),
        };
        txt.push_str(&format!(
            "*{}.* {type_icon} {}\n",
            i + 1,
            p.get("name").and_then(Value::as_str).unwrap_or("undefined")
        ));
        txt.push_str(&format!(
            "   💰 {original}{}\n",
            format_price(p.get("price").and_then(Value::as_i64).unwrap_or(0))
        ));
        txt.push_str(&format!(
            "   📊 Stok: {} {}\n",
            stock_display(p),
            if is_available(p) { "✅" } else { "❌" }
        ));
        txt.push_str(&format!("   🏷️ Tipe: {type_label}\n"));
        if let Some(d) = p.get("description").and_then(Value::as_str)
            && !d.is_empty()
        {
            let cut = d.chars().take(60).count();
            let body = d.chars().take(60).collect::<String>();
            let ellipsis = if d.chars().count() > 60 { "..." } else { "" };
            let _ = cut;
            txt.push_str(&format!("   📝 _{body}{ellipsis}_\n"));
        }
        txt.push('\n');
    }
    txt.push_str(&format!(
        "💡 _Ketik `{}beli <nomor>` untuk memesan produk_",
        ctx.prefix
    ));
    Reply::text(txt)
}

/// Helper bersama untuk plugin store lain: `parseInt(m.text?.trim()) - 1`
/// dengan validasi rentang, persis pola yang dipakai enam plugin store.
pub(crate) fn index_arg(ctx: &Ctx, len: usize) -> Option<usize> {
    let idx = js_parse_int(ctx.text.trim())?.checked_sub(1)?;
    (idx >= 0 && (idx as usize) < len).then_some(idx as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(settings: serde_json::Value, text: &str, is_group: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "listproduk".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group,
            text: text.into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("listproduk").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listproduk").unwrap();
        assert_eq!(p.meta.name, "listproduk");
        assert_eq!(p.meta.alias, ["produk", "katalog", "catalog"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(p.meta.description, "🛍️ Lihat daftar produk yang tersedia");
        assert_eq!(p.meta.usage, ".listproduk");
        assert_eq!(p.meta.example, ".listproduk");
        assert_eq!(p.meta.cooldown, 5);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["produk", "katalog", "catalog"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "listproduk");
        }
    }

    #[test]
    fn an_absent_setting_shows_the_empty_branch() {
        assert_eq!(text_of(&run(json!({}), "", false)), EMPTY);
    }

    #[test]
    fn a_digital_product_counts_stock_items_and_a_physical_one_counts_stock() {
        let r = run(
            json!({"storeProducts": [
                {"name": "Spotify", "price": 25000, "type": "digital",
                 "stockItems": [{"id": 1}, {"id": 2}], "stock": 2},
                {"name": "Kaos", "price": 65000, "type": "fisik", "stock": 8}
            ]}),
            "",
            false,
        );
        assert!(text_of(&r).contains("📊 Stok: 2 akun ✅"), "{}", text_of(&r));
        assert!(text_of(&r).contains("📊 Stok: 8 pcs ✅"));
    }

    #[test]
    fn a_missing_type_defaults_to_digital() {
        let r = run(
            json!({"storeProducts": [{"name": "X", "price": 1000, "stock": 0}]}),
            "",
            false,
        );
        assert!(
            text_of(&r).contains("🏷️ Tipe: Digital"),
            "type kosong harus fallback ke digital"
        );
        assert!(
            text_of(&r).contains("📊 Stok: 0 akun ❌"),
            "stok 0 tanpa stockItems =habis"
        );
    }

    #[test]
    fn unlimited_stock_marks_both_types_available() {
        let r = run(
            json!({"storeProducts": [
                {"name": "A", "price": 1, "type": "digital", "stock": -1},
                {"name": "B", "price": 1, "type": "fisik", "stock": -1}
            ]}),
            "",
            false,
        );
        assert_eq!(
            text_of(&r).matches("♾️ Unlimited ✅").count(),
            2,
            "stock -1 = unlimited untuk kedua tipe"
        );
    }

    #[test]
    fn a_discount_price_is_shown_struck_through_only_when_truthy() {
        let with = run(
            json!({"storeProducts": [{"name": "A", "price": 30000, "originalPrice": 40000, "stock": 1}]}),
            "",
            false,
        );
        assert!(text_of(&with).contains("💰 ~~Rp 40.000~~ Rp 30.000"));
        let without = run(
            json!({"storeProducts": [{"name": "A", "price": 30000, "originalPrice": 0, "stock": 1}]}),
            "",
            false,
        );
        assert!(text_of(&without).contains("💰 Rp 30.000"), "0 = falsy");
    }

    #[test]
    fn long_descriptions_are_truncated_at_sixty_characters() {
        let long = "a".repeat(70);
        let r = run(
            json!({"storeProducts": [{"name": "A", "price": 1, "stock": 1, "description": long}]}),
            "",
            false,
        );
        assert!(text_of(&r).contains(&format!("_{}..._", "a".repeat(60))));
    }

    // `listproduk.js:104` juga mengirim di grup; teksnya sama persis.
    #[test]
    fn the_group_branch_sends_the_same_text() {
        let s = json!({"storeProducts": [{"name": "A", "price": 1000, "stock": 1}]});
        assert_eq!(text_of(&run(s.clone(), "", true)), text_of(&run(s.clone(), "", false)));
    }

    #[test]
    fn prices_use_indonesian_thousand_separators() {
        assert_eq!(format_price(0), "Rp 0");
        assert_eq!(format_price(999), "Rp 999");
        assert_eq!(format_price(1000), "Rp 1.000");
        assert_eq!(format_price(25000), "Rp 25.000");
        assert_eq!(format_price(1234567), "Rp 1.234.567");
    }

    #[test]
    fn index_arg_matches_the_javascript_range_check() {
        let ctx = |t: &str| Ctx {
            text: t.into(),
            ..Default::default()
        };
        assert_eq!(index_arg(&ctx("1"), 3), Some(0));
        assert_eq!(index_arg(&ctx("3"), 3), Some(2));
        assert_eq!(index_arg(&ctx("4"), 3), None, "di luar jangkauan");
        assert_eq!(index_arg(&ctx("0"), 3), None, "0 jadi -1");
        assert_eq!(index_arg(&ctx("abc"), 3), None);
        assert_eq!(index_arg(&ctx(""), 3), None);
        assert_eq!(index_arg(&ctx("2"), 0), None, "daftar kosong");
    }
}