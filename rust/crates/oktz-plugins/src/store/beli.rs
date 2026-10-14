//! `plugins/store/beli.js` — buat nomor transaksi untuk satu produk.
//!
//! Transaksi disimpan di `setting("storeTransactions")` dan penomoran di
//! `setting("storeTrxCounter")`, persis seperti di JS.
//!
//! `ponytail:` TIGA langkah JS TIDAK di-port, semuanya di luar `Ctx`:
//! `config.owner.number`, `config.store.payment`, dan `config.store.qris`
//! (isi `config.js`, bukan store); pengiriman gambar/video produk lewat
//! `sock.sendMessage`; dan notifikasi ke admin lewat
//! `sock.sendMessage(ownerJid, ...)`. Reply hanya boleh satu, jadi blok
//! instruksi pembayaran digabung ke balasan pesanan — nomor transaksi,
//! yang justru bagian penting, tetap utuh.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `String(n).padStart(3, '0')` (`beli.js:72`).
///
/// BUKAN `common::pad_start3` — itu alat sel kolom album yang mengisi dengan
/// spasi di kanan, bukan nol di kiri.
fn pad3(n: i64) -> String {
    let s = n.to_string();
    if s.len() >= 3 { s } else { format!("{}{}", "0".repeat(3 - s.len()), s) }
}

/// `storeTrxCounter`.
const COUNTER: &str = "storeTrxCounter";
/// `storeTransactions` — objek `{ trxId → transaksi }`.
const TRX: &str = "storeTransactions";

/// `(Rp " + n.toLocaleString("id-ID"))` — pemisah ribuan titik, sama dengan
/// `formatNumber` yang dipakai `buyfitur.js`.
fn rupiah(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    format!("Rp {}{}", if n < 0 { "-" } else { "" }, out)
}

/// `typeIcon` produk.
fn icon(p: &serde_json::Value) -> &'static str {
    if crate::store::liststok::is_fisik(p) {
        "📦"
    } else {
        "🔑"
    }
}

/// `type === 'fisik' ? 'Fisik' : 'Digital'`.
fn type_label(p: &serde_json::Value) -> &'static str {
    if crate::store::liststok::is_fisik(p) {
        "Fisik"
    } else {
        "Digital"
    }
}

/// Stok `-1` = unlimited. Fisik dilihat dari `stock`, digital dari
/// `stockItems` — `undefined > 0` = false, jadi digital tanpa `stockItems`
/// selalu habis kecuali unlimited.
fn available(p: &serde_json::Value) -> bool {
    if p.get("stock").and_then(|v| v.as_i64()) == Some(-1) {
        return true;
    }
    if crate::store::liststok::is_fisik(p) {
        p.get("stock").and_then(|v| v.as_i64()).is_some_and(|n| n > 0)
    } else {
        p.get("stockItems")
            .and_then(|v| v.as_array())
            .is_some_and(|a| !a.is_empty())
    }
}

fn price_of(p: &serde_json::Value) -> i64 {
    p.get("price").and_then(|v| v.as_i64()).unwrap_or(0)
}

fn name_of(p: &serde_json::Value) -> &str {
    p.get("name").and_then(|v| v.as_str()).unwrap_or("")
}

fn picker(prefix: &str, products: &[serde_json::Value]) -> String {
    let mut txt = format!(
        "🛒 *Pilih Produk*\n\nKetik `{prefix}beli <nomor>` untuk memesan.\n\n"
    );
    for (i, p) in products.iter().enumerate() {
        txt.push_str(&format!(
            "{} *{}.* {} — {} {}\n",
            icon(p),
            i + 1,
            name_of(p),
            rupiah(price_of(p)),
            if available(p) { "✅" } else { "❌" }
        ));
    }
    txt
}

fn out_of_stock(p: &serde_json::Value) -> String {
    format!(
        "❌ *Stok Habis*\n\n\
         {} Produk *{}* saat ini sedang tidak tersedia 😔\n\n\
         Silakan hubungi admin atau cek kembali nanti.\n\n\
         _Kami akan segera mengisi ulang stok_ 🙏",
        icon(p),
        name_of(p)
    )
}

/// Dua blok `m.reply` di JS digabung jadi satu balasan.
fn receipt(trx_id: &str, p: &serde_json::Value) -> String {
    let mut txt = String::from("🛒 *PESANAN DIBUAT*\n\n");
    txt.push_str(&format!("🧾 Nomor Transaksi: `{trx_id}`\n\n"));
    txt.push_str("📦 *Detail Pesanan:*\n");
    txt.push_str(&format!("{} Produk: *{}*\n", icon(p), name_of(p)));
    txt.push_str(&format!("🏷️ Tipe: *{}*\n", type_label(p)));
    txt.push_str(&format!("💰 Harga: *{}*\n", rupiah(price_of(p))));
    if let Some(orig) = p.get("originalPrice").and_then(|v| v.as_i64()) {
        // `if (product.originalPrice)` — 0 falsy, jadi tidak dicetak.
        txt.push_str(&format!("🏷️ ~~{}~~\n", rupiah(orig)));
    }
    if let Some(desc) = p.get("description").and_then(|v| v.as_str()) {
        txt.push_str(&format!("📝 _{desc}_\n"));
    }
    txt.push('\n');
    // `config.store.payment` / `qris` tidak tersedia di `Ctx` — lihat modul.
    txt.push_str(&format!(
        "💳 *INSTRUKSI PEMBAYARAN*\n\n\
         1️⃣ Transfer sebesar *{}* ke nomor admin 💰\n\n\
         2️⃣ Setelah transfer, kirim *bukti pembayaran* ke admin 📸\n\
         3️⃣ Admin akan memverifikasi dan mengirim data produk ke Anda ✅\n\n\
         🧾 Nomor Transaksi Anda: `{trx_id}`\n\
         _Simpan nomor ini untuk referensi_ 📌",
        rupiah(price_of(p))
    ));
    txt
}

#[plugin(
    name = "beli",
    alias = ["order", "pesan", "buy"],
    category = "store",
    description = "🛒 Pesan produk dan dapatkan nomor transaksi",
    usage = ".beli <nomor_produk>",
    example = ".beli 1",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn beli(ctx: Ctx) -> Reply {
    let products = ctx.setting_list("storeProducts");
    if products.is_empty() {
        return Reply::text(format!(
            "📭 *Belum ada produk tersedia.*\n\nKetik `{}listproduk` untuk melihat daftar produk 🛍️",
            ctx.prefix
        ));
    }

    let idx = ctx
        .text
        .split_whitespace()
        .next()
        .and_then(crate::common::parse_int)
        .map_or(-1, |n| n - 1);

    if idx < 0 || idx >= products.len() as i64 {
        return Reply::text(picker(&ctx.prefix, products));
    }

    let product = &products[idx as usize];
    if !available(product) {
        return Reply::text(out_of_stock(product));
    }

    let counter = ctx.setting_i64(COUNTER).unwrap_or(0) + 1;
    let trx_id = format!("TRX-{}", pad3(counter));

    let buyer = ctx
        .user_field("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map_or_else(|| ctx.sender.split('@').next().unwrap_or(&ctx.sender).to_string(), str::to_string);

    let mut transactions = ctx.setting(TRX).cloned().unwrap_or_default();
    if !transactions.is_object() {
        transactions = serde_json::json!({});
    }
    transactions[&trx_id] = serde_json::json!({
        "trxId": trx_id,
        "buyerJid": ctx.sender,
        "buyerName": buyer,
        "purchaseChat": ctx.chat,
        "purchaseIsGroup": ctx.is_group,
        "productIndex": idx,
        "productId": product.get("id").cloned().unwrap_or(serde_json::Value::Null),
        "productName": name_of(product),
        "productType": product.get("type").cloned().unwrap_or(serde_json::Value::Null),
        "price": price_of(product),
        "status": "pending",
        "createdAt": crate::common::now_ms(),
    });

    Reply::text(receipt(&trx_id, product))
        .mutating(vec![
            Mutation::SetSetting {
                key: COUNTER.into(),
                value: serde_json::json!(counter),
            },
            Mutation::SetSetting {
                key: TRX.into(),
                value: transactions,
            },
        ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    /// `beli` juga alias `rpg/shop.js` di JS, jadi `find("beli")`
    /// mengembalikan shop lebih dulu. Lookup by name, bukan by key.
    fn plugin() -> &'static oktz_plugin_api::PluginEntry {
        oktz_plugin_api::all_plugins()
            .iter()
            .copied()
            .find(|p| p.meta.name == "beli")
            .unwrap()
    }

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "beli".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (plugin().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn setting_written(r: &Reply, key: &str) -> Value {
        let m: Vec<&Mutation> = r.mutations().iter().collect();
        let hit = m.iter().find(|m| matches!(m, Mutation::SetSetting { key: k, .. } if k == key));
        match hit.expect("setting tidak ditulis") {
            Mutation::SetSetting { value, .. } => (*value).clone(),
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn sample() -> Value {
        json!({"storeProducts": [
            {"id": "p1", "name": "Netflix", "type": "digital", "price": 25000,
             "originalPrice": 30000, "description": "1 bulan",
             "stockItems": [{"detail": "a@mail"}], "stock": 1},
            {"id": "p2", "name": "Kaos", "type": "fisik", "price": 50000, "stock": 3}
        ]})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = plugin();
        assert_eq!(p.meta.name, "beli");
        assert_eq!(p.meta.alias, ["order", "pesan", "buy"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(
            p.meta.description,
            "🛒 Pesan produk dan dapatkan nomor transaksi"
        );
        assert_eq!(p.meta.usage, ".beli <nomor_produk>");
        assert_eq!(p.meta.example, ".beli 1");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn all_three_aliases_resolve_back_to_this_plugin() {
        for a in ["order", "pesan", "buy"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("beli")
            );
        }
    }

    #[test]
    fn an_empty_catalog_is_reported_and_nothing_is_written() {
        for s in [json!({}), json!({"storeProducts": []})] {
            let r = run("1", s.clone());
            let t = text_of(&r);
            assert!(t.starts_with("📭 *Belum ada produk tersedia.*"), "{s}: {t}");
            assert!(t.contains("`.listproduk`"), "{t}");
            assert!(r.mutations().is_empty(), "{s}");
        }
    }

    #[test]
    fn a_missing_or_out_of_range_number_shows_the_picker() {
        for input in ["", "0", "3", "abc", "-1"] {
            let r = run(input, sample());
            let t = text_of(&r);
            assert!(t.starts_with("🛒 *Pilih Produk*"), "input {input:?}: {t}");
            assert!(t.contains("🔑 *1.* Netflix — Rp 25.000 ✅"), "{t}");
            assert!(t.contains("📦 *2.* Kaos — Rp 50.000 ✅"), "{t}");
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn the_picker_marks_an_empty_product_as_unavailable() {
        let s = json!({"storeProducts": [
            {"id": "p1", "name": "Netflix", "type": "digital", "price": 25000,
             "stockItems": [], "stock": 0}
        ]});
        assert!(text_of(&run("", s)).contains("🔑 *1.* Netflix — Rp 25.000 ❌"));
    }

    #[test]
    fn an_empty_product_is_refused_with_its_name() {
        let s = json!({"storeProducts": [
            {"id": "p1", "name": "Netflix", "type": "digital", "price": 25000,
             "stockItems": [], "stock": 0}
        ]});
        let r = run("1", s);
        let t = text_of(&r);
        assert!(t.starts_with("❌ *Stok Habis*"), "{t}");
        assert!(t.contains("🔑 Produk *Netflix*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_minus_one_stock_means_unlimited_and_stays_buyable() {
        // Sudah `stockItems: []` — tanpa short-circuit `-1` produk ini habis.
        let s = json!({"storeProducts": [
            {"id": "p1", "name": "Kaos", "type": "fisik", "price": 1000, "stock": -1}
        ]});
        let __balasan = run("", s);
        let t = text_of(&__balasan);
        assert!(t.contains("📦 *1.* Kaos — Rp 1.000 ✅"), "{t}");
        assert!(!text_of(&run("1", json!({"storeProducts": [
            {"id": "p1", "name": "Kaos", "type": "fisik", "price": 1000, "stock": -1}
        ]}))).contains("Stok Habis"));
    }

    #[test]
    fn a_successful_order_writes_the_transaction_and_bumps_the_counter() {
        let r = run("1", sample());
        let t = text_of(&r);
        assert!(t.contains("🧾 Nomor Transaksi: `TRX-001`"), "{t}");
        assert!(t.contains("🔑 Produk: *Netflix*"), "{t}");
        assert!(t.contains("🏷️ Tipe: *Digital*"), "{t}");
        assert!(t.contains("💰 Harga: *Rp 25.000*"), "{t}");
        assert!(t.contains("🏷️ ~~Rp 30.000~~"), "harga coret: {t}");
        assert!(t.contains("📝 _1 bulan_"), "{t}");
        assert!(t.contains("💳 *INSTRUKSI PEMBAYARAN*"), "{t}");
        assert_eq!(setting_written(&r, COUNTER), json!(1));

        let trx = setting_written(&r, TRX);
        let entry = &trx["TRX-001"];
        assert_eq!(entry["trxId"], json!("TRX-001"));
        assert_eq!(entry["buyerJid"], json!("628111@s.whatsapp.net"));
        assert_eq!(entry["purchaseChat"], json!("628111@s.whatsapp.net"));
        assert_eq!(entry["purchaseIsGroup"], json!(false));
        assert_eq!(entry["productIndex"], json!(0));
        assert_eq!(entry["productId"], json!("p1"));
        assert_eq!(entry["productName"], json!("Netflix"));
        assert_eq!(entry["productType"], json!("digital"));
        assert_eq!(entry["price"], json!(25000));
        assert_eq!(entry["status"], json!("pending"));
        assert!(entry["createdAt"].is_i64());
    }

    #[test]
    fn the_counter_keeps_climbing_and_pads_to_three_digits() {
        let s = json!({"storeProducts": [
            {"id": "p1", "name": "Kaos", "type": "fisik", "price": 1000, "stock": 5}
        ], "storeTrxCounter": 8});
        let r = run("1", s);
        assert!(text_of(&r).contains("`TRX-009`"), "{}", text_of(&r));
        assert_eq!(setting_written(&r, COUNTER), json!(9));
        assert_eq!(pad3(1234), "1234", "padStart tidak memotong");
        assert_eq!(pad3(0), "000");
    }

    #[test]
    fn a_group_purchase_records_the_group_chat() {
        let s = json!({"storeProducts": [
            {"id": "p1", "name": "Kaos", "type": "fisik", "price": 1000, "stock": 5}
        ]});
        let mut ctx = Ctx {
            prefix: ".".into(),
            command: "beli".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: "1".into(),
            is_group: true,
            settings: s,
            ..Default::default()
        };
        ctx.args = vec!["1".into()];
        let r = (plugin().handler)(ctx);
        let trx = setting_written(&r, TRX);
        assert_eq!(trx["TRX-001"]["purchaseChat"], json!("120363@g.us"));
        assert_eq!(trx["TRX-001"]["purchaseIsGroup"], json!(true));
    }

    #[test]
    fn the_receipt_uses_a_dot_thousand_separator() {
        assert_eq!(rupiah(0), "Rp 0");
        assert_eq!(rupiah(999), "Rp 999");
        assert_eq!(rupiah(1000), "Rp 1.000");
        assert_eq!(rupiah(25000), "Rp 25.000");
        assert_eq!(rupiah(1234567), "Rp 1.234.567");
    }
}
