//! `plugins/store/hapusstok.js` — hapus satu item stok (atau kurangi stok fisik).
//!
//! Produk fisik dan digital punyaJALUR berbeda: fisik mengurangi angka
//! `stock`, digital menghapus entri `stockItems`. Nomor argumen berarti
//! berbeda juga — untuk fisik argumen kedua adalah JUMLAH yang dikurangi,
//! untuk digital adalah NOMOR ITEM. Itu perilaku JS, bukan salah port.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{arg_index, parse_int};
use crate::store::liststok::is_fisik;

pub(crate) fn products(ctx: &Ctx) -> &[serde_json::Value] {
    ctx.setting_list("storeProducts")
}

pub(crate) fn no_products(prefix: &str) -> String {
    format!("📭 *Belum ada produk.*\n\nTambahkan produk terlebih dahulu: `{prefix}addproduk` ➕")
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🗑️ *HAPUS STOK*\n\n\
         Format: `{p}hapusstok <nomor_produk> <nomor_item>`\n\n\
         📝 *Contoh:*\n\
         `{p}hapusstok 1 3` — Hapus item ke-3 dari produk ke-1\n\n\
         📋 Lihat nomor item: `{p}liststok <nomor_produk>`",
        p = prefix
    )
}

pub(crate) fn bad_product(n: usize) -> String {
    format!("❌ *Nomor produk tidak valid.*\n\nRentang: 1-{n} 📋")
}

/// Produk fisik: kurangi `stock` sebesar `reduce`, tidak bisa ke bawah nol.
pub(crate) fn reduce_physical(
    prefix: &str,
    product: &serde_json::Value,
    reduce: i64,
) -> Result<String, String> {
    if reduce <= 0 {
        return Err(format!(
            "📦 *Produk Fisik*\n\n\
             Untuk mengurangi stok fisik, gunakan:\n\
             `{p}editproduk <nomor> stok <jumlah_baru>`\n\n\
             Stok saat ini: *{}*",
            stock_text(product),
            p = prefix
        ));
    }
    let current = product.get("stock").and_then(|v| v.as_i64()).unwrap_or(0);
    if current == -1 {
        return Ok(
            "♾️ *Stok unlimited tidak bisa dikurangi.*\n\n\
             Ubah tipe stok terlebih dahulu: `.editproduk <nomor> stok <jumlah>`"
                .to_string(),
        );
    }
    let left = (current - reduce).max(0);
    let name = product.get("name").and_then(|v| v.as_str()).unwrap_or("");
    Ok(format!(
        "📦 *STOK FISIK DIKURANGI*\n\n\
         🏷️ Produk: *{name}*\n\
         ➖ Dikurangi: *{reduce} pcs*\n\
         📊 Sisa stok: *{left} pcs*"
    ))
}

/// `product.stock === -1 ? '♾️ Unlimited' : product.stock + ' pcs'`.
pub(crate) fn stock_text(product: &serde_json::Value) -> String {
    match product.get("stock").and_then(|v| v.as_i64()) {
        Some(-1) => "♾️ Unlimited".to_string(),
        Some(n) => format!("{n} pcs"),
        None => "undefined pcs".to_string(),
    }
}

/// Produk digital: hapus entri `stockItems` pada `item_index`.
pub(crate) fn remove_item(
    product: &serde_json::Value,
    item_index: usize,
) -> Result<(String, usize), String> {
    let items = product
        .get("stockItems")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    if item_index >= items.len() {
        return Err(format!(
            "❌ *Nomor item tidak valid.*\n\nRentang: 1-{}\n\n📋 Lihat daftar: `.liststok <nomor>`",
            items.len()
        ));
    }
    let deleted = items[item_index].clone();
    let detail = deleted
        .get("detail")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .replace('\n', " ");
    let preview: String = detail.chars().take(50).collect();
    Ok((preview, items.len() - 1))
}

#[plugin(
    name = "hapusstok",
    alias = ["delstok", "delstock", "deletestok"],
    category = "store",
    description = "🗑️ Hapus stok item dari produk",
    usage = ".hapusstok <nomor_produk> <nomor_item>",
    example = ".hapusstok 1 3",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn hapusstok(ctx: Ctx) -> Reply {
    let products = products(&ctx);
    if products.is_empty() {
        return Reply::text(no_products(&ctx.prefix));
    }

    // `args.length < 2 || isNaN(productNo) || isNaN(itemNo)` checked BEFORE
    // the range check, jadi argumen yang bukan angka langsung dapat bantuan.
    let product_no = arg_index(&ctx, 0);
    let item_no = arg_index(&ctx, 1);
    if ctx.args.len() < 2
        || ctx.args[0].parse::<i64>().is_err()
        || ctx.args[1].parse::<i64>().is_err()
    {
        return Reply::text(help(&ctx.prefix));
    }
    if product_no < 0 || product_no >= products.len() as i64 {
        return Reply::text(bad_product(products.len()));
    }
    let product = products[product_no as usize].clone();

    if is_fisik(&product) {
        // Argumen kedua = jumlah yang dikurangi, bukan indeks.
        let reduce = parse_int(&ctx.args[1]).unwrap_or(0);
        // `return m.reply('♾️ ...')` di JS keluar SEBELUM `db.setting(...)`,
        // jadi stok unlimited tidak boleh ditulis ulang sama sekali.
        let Some(cur) = product.get("stock").and_then(|v| v.as_i64()) else {
            return Reply::text(reduce_physical(&ctx.prefix, &product, reduce).unwrap_or_default());
        };
        if cur == -1 {
            return Reply::text(reduce_physical(&ctx.prefix, &product, reduce).unwrap_or_default());
        }
        if reduce <= 0 {
            return Reply::text(reduce_physical(&ctx.prefix, &product, reduce).unwrap_err());
        }
        let msg = reduce_physical(&ctx.prefix, &product, reduce).unwrap_or_default();
        let mut next = products.to_vec();
        next[product_no as usize]["stock"] = serde_json::json!((cur - reduce).max(0));
        return Reply::text(msg)
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "storeProducts".into(),
                value: serde_json::Value::Array(next),
            });
    }

    let (preview, left) = match remove_item(&product, item_no.max(0) as usize) {
        Ok(v) => v,
        Err(msg) => return Reply::text(msg),
    };
    let name = product.get("name").and_then(|v| v.as_str()).unwrap_or("");

    let mut next = products.to_vec();
    if let Some(items) = next[product_no as usize]
        .get_mut("stockItems")
        .and_then(|v| v.as_array_mut())
    {
        items.remove(item_no.max(0) as usize);
    }
    // `product.stock = stockItems.length` — jumlah stok disinkronkan.
    next[product_no as usize]["stock"] = serde_json::json!(left as i64);

    Reply::text(format!(
        "🗑️ *STOK DIHAPUS*\n\n\
         🏷️ Produk: *{name}*\n\
         🔑 Item: `{preview}`\n\
         📊 Sisa stok: *{left}* akun"
    ))
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
        let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: "hapusstok".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("hapusstok").unwrap().handler)(ctx)
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

    fn digital() -> Value {
        serde_json::json!({"storeProducts": [
            {"name": "Netflix", "type": "digital", "price": 25000,
             "stockItems": [{"detail": "a@mail"}, {"detail": "b@mail"}, {"detail": "c@mail"}],
             "stock": 3},
        ]})
    }

    fn physical() -> Value {
        serde_json::json!({"storeProducts": [
            {"name": "Kaos", "type": "fisik", "price": 50000, "stock": 10},
        ]})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("hapusstok").unwrap();
        assert_eq!(p.meta.name, "hapusstok");
        assert_eq!(p.meta.alias, ["delstok", "delstock", "deletestok"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(p.meta.description, "🗑️ Hapus stok item dari produk");
        assert_eq!(p.meta.usage, ".hapusstok <nomor_produk> <nomor_item>");
        assert_eq!(p.meta.example, ".hapusstok 1 3");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delstok", "delstock", "deletestok"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "hapusstok");
        }
    }

    #[test]
    fn an_absent_product_list_is_reported_and_nothing_is_written() {
        for s in [serde_json::json!({}), serde_json::json!({"storeProducts": []})] {
            let r = run(s.clone(), "1 1");
            assert!(text_of(&r).starts_with("📭 *Belum ada produk.*"), "input {s}");
            assert!(r.mutations().is_empty(), "input {s}");
        }
    }

    #[test]
    fn fewer_than_two_arguments_shows_help_without_writing() {
        for args in ["", "1"] {
            let r = run(digital(), args);
            assert!(text_of(&r).starts_with("🗑️ *HAPUS STOK*"), "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn non_numeric_arguments_show_help_before_the_range_check() {
        for args in ["a b", "1 b", "a 1"] {
            let r = run(digital(), args);
            assert!(text_of(&r).starts_with("🗑️ *HAPUS STOK*"), "args {args:?}");
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn an_out_of_range_product_number_is_reported() {
        for args in ["0 1", "2 1", "-1 1"] {
            let r = run(digital(), args);
            assert!(
                text_of(&r).starts_with("❌ *Nomor produk tidak valid.*"),
                "args {args:?} — {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn deleting_a_digital_item_removes_it_and_syncs_the_count() {
        let r = run(digital(), "1 2");
        let list = products_write(&r);
        let items = list[0]["stockItems"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["detail"], serde_json::json!("a@mail"), "yang kedua hilang");
        assert_eq!(items[1]["detail"], serde_json::json!("c@mail"));
        assert_eq!(list[0]["stock"], serde_json::json!(2), "`stock` = panjang stockItems");
        assert!(text_of(&r).contains("📊 Sisa stok: *2* akun"));
    }

    #[test]
    fn the_deleted_item_preview_is_flattened_and_capped_at_fifty() {
        let s = serde_json::json!({"storeProducts": [{"name": "X", "type": "digital", "price": 1,
            "stockItems": [{"detail": "a\nb\nc"}], "stock": 1}]});
        let r = run(s.clone(), "1 1");
        assert!(text_of(&r).contains("🔑 Item: `a b c`"), "{}", text_of(&r));
    }

    #[test]
    fn an_out_of_range_item_number_is_reported_for_digital() {
        let r = run(digital(), "1 5");
        assert!(
            text_of(&r).starts_with("❌ *Nomor item tidak valid.*"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_physical_product_reduces_its_stock_instead_of_removing_an_item() {
        let r = run(physical(), "1 3");
        assert_eq!(products_write(&r)[0]["stock"], serde_json::json!(7), "10 - 3");
        assert!(text_of(&r).contains("Dikurangi: *3 pcs*"));
        assert!(text_of(&r).contains("Sisa stok: *7 pcs*"));
    }

    #[test]
    fn a_physical_reduction_never_goes_below_zero() {
        let r = run(physical(), "1 99");
        assert_eq!(products_write(&r)[0]["stock"], serde_json::json!(0), "Math.max(0, ...)");
    }

    #[test]
    fn an_unlimited_physical_product_cannot_be_reduced() {
        let s = serde_json::json!({"storeProducts": [{"name": "Kaos", "type": "fisik", "price": 1, "stock": -1}]});
        let r = run(s.clone(), "1 3");
        assert!(text_of(&r).starts_with("♾️ *Stok unlimited tidak bisa dikurangi.*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_physical_non_positive_reduction_points_at_editproduk() {
        for args in ["1 0", "1 -2"] {
            let r = run(physical(), args);
            assert!(text_of(&r).starts_with("📦 *Produk Fisik*"), "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn the_success_reply_reacts_and_is_paired_with_the_write() {
        let r = run(digital(), "1 1");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(text_of(&r).contains("STOK DIHAPUS"));
        assert_eq!(r.mutations().len(), 1);
    }
}