//! `plugins/store/hapusproduk.js` — hapus satu produk dari `storeProducts`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::store::liststok::{is_fisik, stock_display};
use crate::tools::kalkulatormbg::thousands_id;

/// `db.setting("storeProducts") || []`.
pub(crate) fn products(ctx: &Ctx) -> &[serde_json::Value] {
    ctx.setting_list("storeProducts")
}

pub(crate) fn no_products(prefix: &str) -> String {
    format!("📭 *Belum ada produk.*\n\nTambahkan produk terlebih dahulu dengan `{prefix}addproduk` ➕")
}

/// Daftar untuk memilih: nomor, nama, harga, jumlah stok.
pub(crate) fn render_picker(products: &[serde_json::Value]) -> String {
    let mut txt =
        String::from("🗑️ *Pilih Produk yang Dihapus*\n\nKetik `.hapusproduk <nomor>`\n\n");
    for (i, p) in products.iter().enumerate() {
        let type_icon = if is_fisik(p) { "📦" } else { "🔑" };
        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let price = p.get("price").and_then(|v| v.as_i64()).unwrap_or(0);
        txt.push_str(&format!(
            "{type_icon} *{}.* {name} — Rp {} ({})\n",
            i + 1,
            thousands_id(price as f64),
            stock_display(p)
        ));
    }
    txt
}

pub(crate) fn confirm(p: &serde_json::Value) -> String {
    let type_icon = if is_fisik(p) { "📦" } else { "🔑" };
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let price = p.get("price").and_then(|v| v.as_i64()).unwrap_or(0);
    // `deleted.stock + ' pcs'` — `undefined` jadi "undefined" di JS.
    let stock = if is_fisik(p) {
        match p.get("stock").and_then(|v| v.as_i64()) {
            Some(n) => format!("{n} pcs"),
            None => "undefined pcs".to_string(),
        }
    } else {
        format!("{} akun", p.get("stockItems").and_then(|v| v.as_array()).map_or(0, Vec::len))
    };
    format!(
        "🗑️ *PRODUK DIHAPUS*\n\n\
         {type_icon} Nama: *{name}*\n\
         💰 Harga: *Rp {}*\n\
         📊 Stok terhapus: *{stock}*\n\n\
         ⚠️ _Produk telah dihapus secara permanen dan tidak dapat dikembalikan._",
        thousands_id(price as f64)
    )
}

#[plugin(
    name = "hapusproduk",
    alias = ["delproduk", "delproduct", "deleteproduk"],
    category = "store",
    description = "🗑️ Hapus produk dari toko",
    usage = ".hapusproduk <nomor>",
    example = ".hapusproduk 1",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn hapusproduk(ctx: Ctx) -> Reply {
    let products = products(&ctx);
    if products.is_empty() {
        return Reply::text(no_products(&ctx.prefix));
    }

    // `parseInt(text) - 1`; NaN atau di luar rentang -> daftar pilih.
    let idx = match crate::common::parse_int(ctx.text.trim()) {
        Some(n) => n - 1,
        None => return Reply::text(render_picker(products)),
    };
    let Some(index) = usize::try_from(idx).ok().filter(|i| *i < products.len()) else {
        return Reply::text(render_picker(products));
    };

    let deleted = products[index].clone();
    let mut next = products.to_vec();
    next.remove(index);

    Reply::text(confirm(&deleted))
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "storeProducts".into(),
            value: serde_json::Value::Array(next),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "hapusproduk".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("hapusproduk").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn products_write(r: &Reply) -> &Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "storeProducts");
                value.as_array().expect("harus array")
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn sample() -> Value {
        serde_json::json!({"storeProducts": [
            {"name": "Kaos", "type": "fisik", "price": 50000, "stock": 10},
            {"name": "Netflix", "type": "digital", "price": 25000,
             "stockItems": [{"detail": "a"}, {"detail": "b"}]},
        ]})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("hapusproduk").unwrap();
        assert_eq!(p.meta.name, "hapusproduk");
        assert_eq!(p.meta.alias, ["delproduk", "delproduct", "deleteproduk"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(p.meta.description, "🗑️ Hapus produk dari toko");
        assert_eq!(p.meta.usage, ".hapusproduk <nomor>");
        assert_eq!(p.meta.example, ".hapusproduk 1");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delproduk", "delproduct", "deleteproduk"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "hapusproduk");
        }
    }

    #[test]
    fn an_absent_or_empty_product_list_is_reported_and_nothing_is_written() {
        for s in [serde_json::json!({}), serde_json::json!({"storeProducts": []}), serde_json::json!({"storeProducts": null})] {
            let r = run(s.clone(), "1");
            assert!(text_of(&r).starts_with("📭 *Belum ada produk.*"), "input {s}");
            assert!(r.mutations().is_empty(), "input {s}");
        }
    }

    #[test]
    fn no_argument_shows_the_picker_without_writing() {
        let r = run(sample(), "");
        let t = text_of(&r);
        assert!(t.starts_with("🗑️ *Pilih Produk yang Dihapus*"), "{t}");
        assert!(t.contains("📦 *1.* Kaos — Rp 50.000 (10 pcs)"), "{t}");
        assert!(t.contains("🔑 *2.* Netflix — Rp 25.000 (2 akun)"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unparsable_or_out_of_range_number_shows_the_picker() {
        for bad in ["abc", "0", "3", "99", "-1"] {
            let r = run(sample(), bad);
            assert!(
                text_of(&r).starts_with("🗑️ *Pilih Produk yang Dihapus*"),
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn deleting_by_index_removes_exactly_that_entry() {
        let r = run(sample(), "2");
        let list = products_write(&r);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["name"], serde_json::json!("Kaos"), "produk kedua yang dihapus");
        assert!(text_of(&r).starts_with("🗑️ *PRODUK DIHAPUS*"));
    }

    #[test]
    fn the_confirm_reply_names_the_deleted_product_with_its_stock() {
        let r = run(sample(), "1");
        let t = text_of(&r);
        assert!(t.contains("📦 Nama: *Kaos*"), "{t}");
        assert!(t.contains("💰 Harga: *Rp 50.000*"), "{t}");
        assert!(t.contains("📊 Stok terhapus: *10 pcs*"), "{t}");

        let r = run(sample(), "2");
        let t = text_of(&r);
        assert!(t.contains("🔑 Nama: *Netflix*"), "{t}");
        assert!(t.contains("📊 Stok terhapus: *2 akun*"), "{t}");
    }

    #[test]
    fn the_confirm_reply_is_paired_with_the_write_and_reacts() {
        let r = run(sample(), "1");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn deleting_the_only_product_leaves_an_empty_array() {
        let s = serde_json::json!({"storeProducts": [{"name": "A", "type": "digital", "price": 1, "stockItems": []}]});
        let r = run(s.clone(), "1");
        assert!(products_write(&r).is_empty());
        assert!(text_of(&r).contains("0 akun"));
    }

    #[test]
    fn the_permanent_deletion_warning_is_present() {
        let r = run(sample(), "1");
        assert!(text_of(&r).contains("tidak dapat dikembalikan"));
    }

    #[test]
    fn a_missing_stock_field_shows_the_javascript_stringification() {
        // `deleted.stock + ' pcs'` dengan `stock` undefined -> "undefined".
        let s = serde_json::json!({"storeProducts": [{"name": "X", "type": "fisik", "price": 10}]});
        let r = run(s.clone(), "1");
        assert!(
            text_of(&r).contains("undefined pcs"),
            "{}",
            text_of(&r)
        );
    }
}