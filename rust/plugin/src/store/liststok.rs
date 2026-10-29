//! `plugins/store/liststok.js` — daftar stok `setting('storeProducts')`.
//!
//! Plugin baca-saja: seluruh cabangnya hanya menampilkan isi store, tidak
//! pernah menulis.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::parse_int;

/// `db.setting('storeProducts') || []`.
pub(crate) fn products(ctx: &Ctx) -> &[serde_json::Value] {
    ctx.setting_list("storeProducts")
}

/// `p.type === 'fisik'` — string lain (termasuk `undefined`) berarti digital.
pub(crate) fn is_fisik(p: &serde_json::Value) -> bool {
    p.get("type").and_then(|v| v.as_str()) == Some("fisik")
}

/// Tampilan jumlah stok: fisik → `N pcs` / `♾️`, digital → `N akun`.
pub(crate) fn stock_display(p: &serde_json::Value) -> String {
    let stock = p.get("stock").and_then(|v| v.as_i64()).unwrap_or(0);
    if is_fisik(p) {
        if stock == -1 {
            "♾️".to_string()
        } else {
            format!("{stock} pcs")
        }
    } else {
        format!("{} akun", p.get("stockItems").and_then(|v| v.as_array()).map_or(0, Vec::len))
    }
}

/// `✅` kalau masih ada stok, `⚠️` kalau habis. `-1` = unlimited selalu ✅.
pub(crate) fn icon(p: &serde_json::Value) -> &'static str {
    let stock = p.get("stock").and_then(|v| v.as_i64()).unwrap_or(0);
    let n = if is_fisik(p) {
        stock
    } else {
        p.get("stockItems").and_then(|v| v.as_array()).map_or(0, Vec::len) as i64
    };
    if stock == -1 || n > 0 { "✅" } else { "⚠️" }
}

pub(crate) fn no_products(prefix: &str) -> String {
    format!("📭 *Belum ada produk.*\n\nTambahkan produk terlebih dahulu: `{prefix}addproduk` ➕")
}

/// Daftar indeks produk dengan ikon tipe + jumlah stok.
pub(crate) fn render_index(ctx: &Ctx, products: &[serde_json::Value]) -> String {
    let mut txt = String::from("📋 *DAFTAR STOK PRODUK*\n\nPilih produk untuk melihat stok:\n\n");
    for (i, p) in products.iter().enumerate() {
        let type_icon = if is_fisik(p) { "📦" } else { "🔑" };
        let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
        txt.push_str(&format!(
            "{type_icon} *{}.* {name} — {} {}\n",
            i + 1,
            stock_display(p),
            icon(p)
        ));
    }
    txt.push_str(&format!(
        "\nKetik `{}liststok <nomor>` untuk melihat detail stok 📊",
        ctx.prefix
    ));
    txt
}

pub(crate) fn render_fisik(ctx: &Ctx, p: &serde_json::Value) -> String {
    let n = ctx.prefix.to_string();
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let total = match p.get("stock").and_then(|v| v.as_i64()) {
        Some(-1) => "♾️ Unlimited".to_string(),
        Some(n) => format!("{n} pcs"),
        // `product.stock === -1 ? ... : product.stock + ' pcs'` — `undefined`
        // bukan `-1`, jadi `undefined + ' pcs'` = "undefined pcs" di JS.
        None => "undefined pcs".to_string(),
    };
    format!(
        "📦 *STOK: {name}*\n\n\
         📊 Tipe: *Fisik*\n\
         📦 Total: *{total}*\n\n\
         *Kelola stok:*\n\
         • Tambah: `{n}addstok <nomor> <jumlah>`\n\
         • Edit: `{n}editproduk <nomor> stok <jumlah>`\n\n\
         _Stok fisik diatur berdasarkan jumlah, bukan per-item_ 📦"
    )
}

pub(crate) fn render_empty_items(ctx: &Ctx, p: &serde_json::Value) -> String {
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    format!(
        "🔑 *Stok: {name}*\n\n\
         📭 Belum ada stok item yang ditambahkan.\n\n\
         *Tambah stok:*\n\
         • Manual: `{p}addstok <nomor>|<detail>`\n\
         • Import: `{p}addstok <nomor>` (reply file .txt 📄)\n\n\
         _Stok item bersifat rahasia 🔒 dan hanya dikirim ke pembeli setelah pembayaran dikonfirmasi_",
        p = ctx.prefix
    )
}

/// Detail stok digital: maks 30 item, sisanya diringkas.
pub(crate) fn render_items(ctx: &Ctx, p: &serde_json::Value, items: &[serde_json::Value]) -> String {
    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let mut txt = format!("🔑 *STOK: {name}*\n\n📊 Total: *{}* akun\n\n", items.len());
    for (i, it) in items.iter().take(30).enumerate() {
        let detail = it.get("detail").and_then(|v| v.as_str()).unwrap_or("");
        let preview: String = detail.replace('\n', " ").chars().take(40).collect();
        if detail.chars().count() > 40 {
            txt.push_str(&format!("`{}.` {preview}...\n", i + 1));
        } else {
            txt.push_str(&format!("`{}.` {preview}\n", i + 1));
        }
    }
    if items.len() > 30 {
        txt.push_str(&format!("\n_dan {} item lainnya..._ 📋", items.len() - 30));
    }
    let n = ctx.prefix.to_string();
    txt.push_str(&format!(
        "\n\n🛠️ *Kelola stok:*\n\
         🗑️ Hapus: `{n}hapusstok <nomor> <nomor_item>`\n\
         ✏️ Edit: `{n}editstok <nomor> <nomor_item>|<detail_baru>`\n\
         ➕ Tambah: `{n}addstok <nomor>|<detail>`"
    ));
    txt
}

#[plugin(
    name = "liststok",
    alias = ["liststock", "stok", "stock"],
    category = "store",
    description = "📋 Lihat daftar stok item produk",
    usage = ".liststok <nomor_produk>",
    example = ".liststok 1",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn liststok(ctx: Ctx) -> Reply {
    let products = products(&ctx);
    if products.is_empty() {
        return Reply::text(no_products(&ctx.prefix));
    }

    // `parseInt(text) - 1`, dan `NaN`/di luar rentang → daftar indeks.
    let idx = match ctx.text.trim().parse::<i64>().ok().or_else(|| parse_int(&ctx.text)) {
        Some(n) => n - 1,
        None => return Reply::text(render_index(&ctx, products)),
    };
    let Some(product) = usize::try_from(idx).ok().and_then(|i| products.get(i)) else {
        return Reply::text(render_index(&ctx, products));
    };

    if is_fisik(product) {
        return Reply::text(render_fisik(&ctx, product));
    }
    let items: &[serde_json::Value] = product
        .get("stockItems")
        .and_then(|v| v.as_array())
        .map_or(&[], Vec::as_slice);
    if items.is_empty() {
        return Reply::text(render_empty_items(&ctx, product));
    }
    Reply::text(render_items(&ctx, product, items))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(settings: serde_json::Value, text: &str) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "liststok".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("liststok").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn fisik(stock: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"name": "Kaos", "type": "fisik", "stock": stock})
    }

    fn digital(items: serde_json::Value, stock: i64) -> serde_json::Value {
        serde_json::json!({"name": "Netflix", "type": "digital", "stock": stock, "stockItems": items})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("liststok").unwrap();
        assert_eq!(p.meta.name, "liststok");
        assert_eq!(p.meta.alias, ["liststock", "stok", "stock"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(p.meta.description, "📋 Lihat daftar stok item produk");
        assert_eq!(p.meta.usage, ".liststok <nomor_produk>");
        assert_eq!(p.meta.example, ".liststok 1");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["liststock", "stok", "stock"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "liststok");
        }
    }

    #[test]
    fn an_absent_or_empty_product_list_says_there_is_nothing_yet() {
        for s in [serde_json::json!({}), serde_json::json!({"storeProducts": []}), serde_json::json!({"storeProducts": null})] {
            let out = run(s.clone(), "");
            assert!(out.contains("📭 *Belum ada produk.*"), "input {s}");
            assert!(out.contains(".addproduk"));
        }
    }

    #[test]
    fn a_missing_argument_lists_every_product() {
        let out = run(serde_json::json!({"storeProducts": [fisik(serde_json::json!(5)), digital(serde_json::json!([]), 0)]}), "");
        assert!(out.starts_with("📋 *DAFTAR STOK PRODUK*"));
        assert!(out.contains("📦 *1.* Kaos — 5 pcs ✅"));
        assert!(
            out.contains("🔑 *2.* Netflix — 0 akun ⚠️"),
            "stok habis dapat ikon peringatan: {out}"
        );
        assert!(out.ends_with("Ketik `.liststok <nomor>` untuk melihat detail stok 📊"));
    }

    #[test]
    fn an_unlimited_physical_product_is_always_marked_in_stock() {
        let out = run(serde_json::json!({"storeProducts": [fisik(serde_json::json!(-1))]}), "");
        assert!(out.contains("♾️ ✅"), "unlimited tidak pernah ⚠️: {out}");
    }

    #[test]
    fn a_non_numeric_or_out_of_range_number_falls_back_to_the_index() {
        let set = serde_json::json!({"storeProducts": [fisik(serde_json::json!(1)), digital(serde_json::json!([]), 0)]});
        for bad in ["", "abc", "0", "3", "99", "-1"] {
            let out = run(set.clone(), bad);
            assert!(
                out.starts_with("📋 *DAFTAR STOK PRODUK*"),
                "input {bad:?} harus jatuh ke daftar"
            );
        }
    }

    #[test]
    fn a_physical_product_reports_its_total_in_pcs() {
        let out = run(serde_json::json!({"storeProducts": [fisik(serde_json::json!(7))]}), "1");
        assert!(out.contains("📊 Tipe: *Fisik*"));
        assert!(out.contains("📦 Total: *7 pcs*"));
        assert!(out.contains("`.addstok <nomor> <jumlah>`"));
    }

    #[test]
    fn a_physical_product_with_minus_one_is_unlimited() {
        let out = run(serde_json::json!({"storeProducts": [fisik(serde_json::json!(-1))]}), "1");
        assert!(out.contains("📦 Total: *♾️ Unlimited*"));
    }

    #[test]
    fn a_digital_product_without_items_points_at_the_add_commands() {
        let out = run(serde_json::json!({"storeProducts": [digital(serde_json::json!([]), 0)]}), "1");
        assert!(out.contains("🔑 *Stok: Netflix*"));
        assert!(out.contains("📭 Belum ada stok item yang ditambahkan."));
        assert!(out.contains(".addstok <nomor>|<detail>"));
    }

    #[test]
    fn digital_items_are_numbered_from_one_with_a_truncated_preview() {
        let items = serde_json::json!([{"detail": "akun1@mail.com\npass1"}, {"detail": "short"}]);
        let out = run(serde_json::json!({"storeProducts": [digital(items, 2)]}), "1");
        assert!(out.contains("📊 Total: *2* akun"));
        assert!(
            out.contains("`1.` akun1@mail.com pass1"),
            "baris baru diratakan jadi spasi: {out}"
        );
        assert!(out.contains("`2.` short"), "detail pendek tanpa ellipsis");
    }

    #[test]
    fn a_detail_longer_than_forty_characters_is_truncated_with_an_ellipsis() {
        let long = "x".repeat(60);
        let out = run(
            serde_json::json!({"storeProducts": [digital(serde_json::json!([{"detail": long}]), 1)]}),
            "1",
        );
        let line = out.lines().find(|l| l.starts_with("`1.`")).unwrap();
        assert!(line.ends_with("..."), "baris: {line}");
        assert_eq!(
            line.chars().filter(|c| *c == 'x').count(),
            40,
            "potong di 40 karakter sebelum ellipsis"
        );
    }

    #[test]
    fn more_than_thirty_items_are_summarised_not_listed() {
        let items: Vec<_> = (0..35)
            .map(|i| serde_json::json!({"detail": format!("item{i}")}))
            .collect();
        let out = run(serde_json::json!({"storeProducts": [digital(serde_json::json!(items), 35)]}), "1");
        assert!(out.contains("`30.` item29"));
        assert!(
            !out.contains("`31.`"),
            "item ke-31 dan seterusnya tidak dirender"
        );
        assert!(out.contains("_dan 5 item lainnya..._ 📋"));
        assert!(out.contains("📊 Total: *35* akun"), "total tetap 35");
    }

    #[test]
    fn exactly_thirty_items_get_no_summary_line() {
        let items: Vec<_> = (0..30).map(|i| serde_json::json!({"detail": format!("i{i}")})).collect();
        let out = run(serde_json::json!({"storeProducts": [digital(serde_json::json!(items), 30)]}), "1");
        assert!(out.contains("`30.` i29"));
        assert!(!out.contains("item lainnya"), "tepat 30 tidak perlu ringkasan");
    }

    #[test]
    fn the_reply_is_read_only_and_writes_nothing() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "liststok".into(),
            text: "1".into(),
            args: vec!["1".into()],
            settings: serde_json::json!({"storeProducts": [digital(serde_json::json!([{"detail": "a"}]), 1)]}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("liststok").unwrap().handler)(ctx);
        assert!(
            r.mutations().is_empty(),
            "liststok hanya membaca; mutasi berarti data bocor"
        );
    }

    #[test]
    fn the_helper_contracts_hold_independently() {
        assert!(is_fisik(&serde_json::json!({"type": "fisik"})));
        assert!(!is_fisik(&serde_json::json!({"type": "digital"})));
        assert!(!is_fisik(&serde_json::json!({})), "type undefined = digital");
        assert_eq!(stock_display(&fisik(serde_json::json!(3))), "3 pcs");
        assert_eq!(stock_display(&fisik(serde_json::json!(-1))), "♾️");
        assert_eq!(stock_display(&digital(serde_json::json!([{"detail": "a"}]), 1)), "1 akun");
        assert_eq!(stock_display(&digital(serde_json::json!(null), 0)), "0 akun");
        assert_eq!(icon(&fisik(serde_json::json!(0))), "⚠️");
        assert_eq!(icon(&fisik(serde_json::json!(1))), "✅");
        assert_eq!(icon(&digital(serde_json::json!([]), 0)), "⚠️");
    }
}