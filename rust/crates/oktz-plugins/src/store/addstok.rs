//! `plugins/store/addstok.js` — tambah stok produk digital atau fisik.
//!
//! Format digital: `<nomor_produk>|<detail>`, fisik: `<nomor_produk> <jumlah>`.
//! Baris pemisah `|` dicari PADA INPUT lebih dulu, jadi `<produk> <jumlah>`
//! tanpa `|` selalu cabang bantuan — bukan "isi kosong".
//!
//! `ponytail:` cabang import dari file `.txt` (reply `documentMessage`
//! `.txt`) TIDAK di-port: `m.quoted.download()` butuh
//! `downloadContentMessage`, yang tidak ada di `QuotedMessage`. Input
//! `.addstok <nomor>` dengan reply file karena itu jatuh ke kartu bantuan,
//! sama seperti file yang bukan `.txt`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::parse_int;
use crate::store::liststok::is_fisik;

const DENIED: &str = "🚫 *Akses Ditolak*\n\n\
     Untuk menjaga privasi data stok 🛡️, penambahan stok hanya dapat dilakukan di *private chat*.\n\n\
     Silakan chat bot secara langsung 📱";

fn no_products(prefix: &str) -> String {
    format!("📭 *Belum ada produk.*\n\nTambahkan produk terlebih dahulu: `{prefix}addproduk` ➕")
}

fn help(prefix: &str) -> String {
    let p = prefix;
    format!(
        "📦 *TAMBAH STOK*\n\n\
         🔑 *Produk Digital* — Tambah data akun/key:\n\
         `{p}addstok <nomor_produk>|<detail>`\n\n\
         📄 *Import dari file .txt:*\n\
         `{p}addstok <nomor_produk>` (reply file .txt)\n\n\
         📦 *Produk Fisik* — Tambah jumlah stok:\n\
         `{p}addstok <nomor_produk> <jumlah>`\n\n\
         📝 *Contoh digital:*\n\
         `{p}addstok 1|Email: user@mail.com;;Password: pass123`\n\n\
         📝 *Contoh fisik:*\n\
         `{p}addstok 2 8` — Tambah 8 pcs untuk produk #2\n\n\
         • Gunakan `;;` untuk baris baru dalam detail 🔑\n\
         • Setiap baris di file .txt = 1 stok item 📄\n\
         • Maksimal 1.000 item per import 📊\n\n\
         _Data stok digital bersifat rahasia 🔒 dan hanya dikirim ke pembeli setelah pembayaran dikonfirmasi_"
    )
}

fn bad_product(prefix: &str) -> String {
    format!("❌ *Nomor produk tidak valid.*\n\nLihat daftar produk: `{prefix}liststok` 📋")
}

fn physical_hint(prefix: &str, product_no: i64, name: &str, stock: &str) -> String {
    format!(
        "📦 *TAMBAH STOK FISIK*\n\n\
         Produk *{name}* bertipe **Fisik** 📦\n\n\
         Format: `{prefix}addstok {product_no} <jumlah>`\n\n\
         📝 *Contoh:*\n\
         `{prefix}addstok {product_no} 8` — Tambah 8 pcs\n\n\
         Stok saat ini: *{stock}*"
    )
}

/// `product.stock === -1 ? '♾️ Unlimited' : stock + ' pcs'` — `stock` sudah
/// berupa string siap tampil.
fn physical_added(prefix: &str, product_no: i64, name: &str, added: i64, total: i64) -> String {
    format!(
        "📦 *STOK FISIK DITAMBAHKAN*\n\n\
         🏷️ Produk: *{name}*\n\
         ➕ Ditambahkan: *{added} pcs*\n\
         📊 Total stok: *{total} pcs*\n\n\
         _Tambah lagi: `{prefix}addstok {product_no} <jumlah>`_"
    )
}

fn digital_added(prefix: &str, product_no: i64, name: &str, total: usize) -> String {
    format!(
        "✅ *STOK DITAMBAHKAN*\n\n\
         🏷️ Produk: *{name}*\n\
         🔑 Total stok saat ini: *{total}* akun\n\n\
         _Tambah lagi: `{prefix}addstok {product_no}|<detail>`_"
    )
}

/// `product.stock = (stock === -1 ? 0 : stock) + add` — `-1` = unlimited,
/// jadi stok tak Terbaca dianggap 0 sebelum penambahan pertama.
fn bumped_stock(p: &serde_json::Value, add: i64) -> i64 {
    let stock = p.get("stock").and_then(|v| v.as_i64()).unwrap_or(0);
    (if stock == -1 { 0 } else { stock }) + add
}

fn stock_text(p: &serde_json::Value) -> String {
    let stock = p.get("stock").and_then(|v| v.as_i64()).unwrap_or(0);
    if stock == -1 {
        "♾️ Unlimited".to_string()
    } else {
        format!("{stock} pcs")
    }
}

fn has_detail(p: &serde_json::Value, detail: &str) -> bool {
    p.get("stockItems")
        .and_then(|v| v.as_array())
        .is_some_and(|a| {
            a.iter()
                .any(|i| i.get("detail").and_then(|d| d.as_str()) == Some(detail))
        })
}

/// Produk fisik bertambah `add` — dipakai oleh cabang dengan dan tanpa `|`.
fn add_physical(ctx: &Ctx, idx: usize, add: i64) -> Reply {
    let product = &ctx.setting_list("storeProducts")[idx];
    let total = bumped_stock(product, add);
    let name = product.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let mut next = ctx.setting_list("storeProducts").to_vec();
    next[idx]["stock"] = serde_json::json!(total);
    let with_pipe = ctx.text.contains('|');
    let text = if with_pipe {
        format!(
            "📦 *STOK FISIK DITAMBAHKAN*\n\n\
             🏷️ Produk: *{name}*\n\
             ➕ Ditambahkan: *{add} pcs*\n\
             📊 Total stok: *{total} pcs*"
        )
    } else {
        physical_added(&ctx.prefix, idx as i64 + 1, name, add, total)
    };
    Reply::text(text)
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "storeProducts".into(),
            value: serde_json::Value::Array(next),
        })
}

#[plugin(
    name = "addstok",
    alias = ["addstock", "importstok", "importstock"],
    category = "store",
    description = "📦 Tambah stok item ke produk (hanya di private chat)",
    usage = ".addstok <nomor_produk>|<detail> atau .addstok <nomor> <jumlah> (fisik)",
    example = ".addstok 1|Email: user@mail.com;;Password: pass123",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = true
)]
pub fn addstok(ctx: Ctx) -> Reply {
    // `m.isGroup` diperiksa PALING DULU, sebelum store dibaca.
    if ctx.is_group {
        return Reply::text(DENIED);
    }

    let products = ctx.setting_list("storeProducts");
    if products.is_empty() {
        return Reply::text(no_products(&ctx.prefix));
    }

    let text = ctx.text.trim();

    if let Some(pipe) = text.find('|') {
        let product_no = parse_int(text[..pipe].trim()).map_or(-1, |n| n - 1);
        let detail = text[pipe + 1..].trim().replace(";;", "\n");

        if product_no < 0 || product_no >= products.len() as i64 {
            return Reply::text(bad_product(&ctx.prefix));
        }
        let idx = product_no as usize;

        if is_fisik(&products[idx]) {
            let Some(add) = parse_int(&detail).filter(|n| *n > 0) else {
                return Reply::text(format!(
                    "📦 *Produk ini bertipe Fisik*\n\n\
                     Gunakan format: `{}addstok {} <jumlah>`\n\n\
                     📝 Contoh: `{}addstok {} 8` — Tambah 8 pcs",
                    ctx.prefix,
                    product_no + 1,
                    ctx.prefix,
                    product_no + 1
                ));
            };
            return add_physical(&ctx, idx, add);
        }

        if detail.chars().count() < 3 {
            return Reply::text(
                "❌ *Detail stok terlalu pendek.*\n\n\
                 Minimal 3 karakter diperlukan agar data stok dapat digunakan 🔑",
            );
        }
        if has_detail(&products[idx], &detail) {
            let name = products[idx].get("name").and_then(|v| v.as_str()).unwrap_or("");
            return Reply::text(format!(
                "⚠️ *Data stok sudah ada.*\n\n\
                 Item dengan detail yang sama sudah terdaftar di produk *{name}* 🔑"
            ));
        }

        let mut next = products.to_vec();
        let mut item = serde_json::json!({});
        item["detail"] = serde_json::json!(detail);
        item["addedAt"] = serde_json::json!(crate::common::now_ms());
        // `if (!product.stockItems) product.stockItems = []` (addstok.js:221).
        if next[idx].get("stockItems").and_then(|v| v.as_array()).is_none() {
            next[idx]["stockItems"] = serde_json::json!([]);
        }
        next[idx]["stockItems"]
            .as_array_mut()
            .expect("sudah berupa array")
            .push(item);
        let total = next[idx]["stockItems"].as_array().map_or(0, Vec::len);
        next[idx]["stock"] = serde_json::json!(total);
        let name = products[idx].get("name").and_then(|v| v.as_str()).unwrap_or("");

        return Reply::text(digital_added(&ctx.prefix, idx as i64 + 1, name, total))
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "storeProducts".into(),
                value: serde_json::Value::Array(next),
            });
    }

    // Tanpa `|`: `<nomor> <jumlah>` untuk produk fisik, sisanya kartu bantuan
    // (jalur import `.txt` tidak di-port — lihat `ponytail:` modul).
    let parts: Vec<&str> = text.split_whitespace().collect();
    let product_no = parts.first().and_then(|p| parse_int(p)).map_or(-1, |n| n - 1);
    if product_no >= 0 && product_no < products.len() as i64 {
        let idx = product_no as usize;
        if is_fisik(&products[idx]) {
            match parts.get(1).and_then(|p| parse_int(p)) {
                Some(add) if add > 0 => return add_physical(&ctx, idx, add),
                _ => {
                    let name = products[idx].get("name").and_then(|v| v.as_str()).unwrap_or("");
                    return Reply::text(physical_hint(
                        &ctx.prefix,
                        product_no + 1,
                        name,
                        &stock_text(&products[idx]),
                    ));
                }
            }
        }
    }

    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(text: &str, settings: Value, is_group: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addstok".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            is_group,
            ..Default::default()
        };
        (oktz_plugin_api::find("addstok").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn products_write(r: &Reply) -> Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "storeProducts");
                value.as_array().expect("harus array").clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn sample() -> Value {
        json!({"storeProducts": [
            {"name": "Netflix", "type": "digital", "price": 25000,
             "stockItems": [
                {"detail": "lama@mail", "addedAt": "2024-01-01T00:00:00.000Z"},
                {"detail": "lain@mail", "addedAt": "2024-01-01T00:00:00.000Z"}
             ],
             "stock": 2},
            {"name": "Kaos", "type": "fisik", "price": 50000, "stock": 10}
        ]})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addstok").unwrap();
        assert_eq!(p.meta.name, "addstok");
        assert_eq!(p.meta.alias, ["addstock", "importstok", "importstock"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(
            p.meta.description,
            "📦 Tambah stok item ke produk (hanya di private chat)"
        );
        assert_eq!(
            p.meta.usage,
            ".addstok <nomor_produk>|<detail> atau .addstok <nomor> <jumlah> (fisik)"
        );
        assert_eq!(
            p.meta.example,
            ".addstok 1|Email: user@mail.com;;Password: pass123"
        );
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && p.meta.is_private);
    }

    #[test]
    fn all_three_aliases_resolve_back_to_this_plugin() {
        for a in ["addstock", "importstok", "importstock"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("addstok")
            );
        }
    }

    #[test]
    fn the_group_branch_is_rejected_before_the_store_is_read() {
        let r = run("1|baru@mail", json!({}), true);
        assert_eq!(text_of(&r), DENIED);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_empty_product_list_is_reported_and_nothing_is_written() {
        for s in [json!({}), json!({"storeProducts": []})] {
            let r = run("1|baru@mail", s.clone(), false);
            assert!(text_of(&r).starts_with("📭 *Belum ada produk.*"), "{s}");
            assert!(r.mutations().is_empty(), "{s}");
        }
    }

    #[test]
    fn a_pipe_input_appends_the_item_and_resyncs_the_stock_count() {
        let r = run("1|Email: baru@mail.com;;Password: p4ss", sample(), false);
        let p = &products_write(&r)[0];
        let items = p["stockItems"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["detail"], json!("lama@mail"), "yang lama utuh");
        assert_eq!(items[2]["detail"], json!("Email: baru@mail.com\nPassword: p4ss"));
        assert_eq!(
            p["stock"],
            json!(3),
            "stock = jumlah stockItems, bukan 2+1"
        );
        assert!(items[2]["addedAt"].is_i64(), "addedAt numeric: {:?}", items[2]["addedAt"]);
    }

    #[test]
    fn a_duplicate_detail_is_refused_and_nothing_is_written() {
        // Perbandingan JS terjadi SETELAH `;;`→`\n` dan `trim()`, jadi hanya
        // detail yang benar-benar sama string-nya yang dianggap duplikat.
        for input in ["1|lama@mail", "1|  lama@mail  ", "1| lain@mail"] {
            let r = run(input, sample(), false);
            let t = text_of(&r);
            assert!(t.starts_with("⚠️ *Data stok sudah ada.*"), "input {input}: {t}");
            assert!(t.contains("produk *Netflix*"), "input {input}: {t}");
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }

    #[test]
    fn a_detail_shorter_than_three_characters_is_refused() {
        for input in ["1|ab", "1|a", "1|  "] {
            let r = run(input, sample(), false);
            assert!(
                text_of(&r).starts_with("❌ *Detail stok terlalu pendek.*"),
                "input {input:?}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn exactly_three_characters_is_accepted() {
        let r = run("1|abc", sample(), false);
        assert_eq!(products_write(&r)[0]["stockItems"][2]["detail"], json!("abc"));
    }

    #[test]
    fn a_product_without_a_stock_items_array_gets_one_created() {
        // `if (!product.stockItems) product.stockItems = []` (addstok.js:221).
        let s = json!({"storeProducts": [{"name": "Netflix", "type": "digital", "price": 1}]});
        let r = run("1|baru@mail", s, false);
        let p = &products_write(&r)[0];
        assert_eq!(p["stockItems"][0]["detail"], json!("baru@mail"));
        assert_eq!(p["stock"], json!(1), "stock = panjang stockItems");
    }

    #[test]
    fn an_out_of_range_product_number_is_refused() {
        for input in ["0|abc", "3|abc", "9|abc", "x|abc"] {
            let r = run(input, sample(), false);
            assert!(
                text_of(&r).starts_with("❌ *Nomor produk tidak valid.*"),
                "input {input}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }

    #[test]
    fn a_physical_product_with_a_non_numeric_pipe_value_explains_itself() {
        let r = run("2|abc", sample(), false);
        let t = text_of(&r);
        assert!(t.starts_with("📦 *Produk ini bertipe Fisik*"), "{t}");
        assert!(t.contains(".addstok 2 <jumlah>"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_physical_product_adds_its_count_through_the_pipe_form() {
        let r = run("2|8", sample(), false);
        let t = text_of(&r);
        assert!(t.starts_with("📦 *STOK FISIK DITAMBAHKAN*"), "{t}");
        assert!(t.contains("🏷️ Produk: *Kaos*"), "{t}");
        assert!(t.contains("➕ Ditambahkan: *8 pcs*"), "{t}");
        assert!(t.contains("📊 Total stok: *18 pcs*"), "{t}");
        assert!(!t.contains("Tambah lagi"), "tanpa `|` tidak ada footer: {t}");
        assert_eq!(products_write(&r)[1]["stock"], json!(18));
    }

    #[test]
    fn a_zero_or_negative_physical_count_is_refused() {
        for input in ["2|0", "2|-5"] {
            let r = run(input, sample(), false);
            assert!(text_of(&r).starts_with("📦 *Produk ini bertipe Fisik*"), "input {input}");
            assert!(r.mutations().is_empty(), "input {input}");
        }
    }

    #[test]
    fn an_unlimited_physical_stock_starts_from_zero_on_the_first_add() {
        // `stock === -1` = unlimited; penambahan pertama meng regards 0 sebagai
        // basis, jadi hasilnya 3 — bukan -1 + 3.
        let s = json!({"storeProducts": [
            {"name": "Kaos", "type": "fisik", "price": 1, "stock": -1}
        ]});
        let r = run("1|3", s, false);
        assert_eq!(products_write(&r)[0]["stock"], json!(3));
        assert!(text_of(&r).contains("Total stok: *3 pcs*"), "{}", text_of(&r));
    }

    #[test]
    fn a_physical_count_without_a_pipe_uses_the_spaced_form() {
        let r = run("2 8", sample(), false);
        let t = text_of(&r);
        assert!(t.starts_with("📦 *STOK FISIK DITAMBAHKAN*"), "{t}");
        assert!(t.contains("📊 Total stok: *18 pcs*"), "{t}");
        assert!(t.contains("_Tambah lagi: `.addstok 2 <jumlah>`_"), "{t}");
        assert_eq!(products_write(&r)[1]["stock"], json!(18));
    }

    #[test]
    fn a_physical_product_without_a_count_shows_the_physical_hint() {
        let r = run("2", sample(), false);
        let t = text_of(&r);
        assert!(t.starts_with("📦 *TAMBAH STOK FISIK*"), "{t}");
        assert!(t.contains("Produk *Kaos* bertipe **Fisik** 📦"), "{t}");
        assert!(t.contains("Stok saat ini: *10 pcs*"), "{t}");
        assert!(r.mutations().is_empty());

        let r = run("2 0", sample(), false);
        assert!(
            text_of(&r).starts_with("📦 *TAMBAH STOK FISIK*"),
            "0 bukan jumlah valid: {}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unlimited_physical_stock_reads_as_the_infinity_sign() {
        let s = json!({"storeProducts": [
            {"name": "Kaos", "type": "fisik", "price": 1, "stock": -1}
        ]});
        assert!(
            text_of(&run("1", s, false)).contains("Stok saat ini: *♾️ Unlimited*"),
            "{}",
            text_of(&run("1", json!({"storeProducts": [
                {"name": "Kaos", "type": "fisik", "price": 1, "stock": -1}
            ]}), false))
        );
    }

    #[test]
    fn input_without_a_pipe_on_a_digital_product_falls_back_to_the_help_card() {
        for input in ["", "1", "1 5", "abc", "9 3"] {
            let r = run(input, sample(), false);
            assert!(
                text_of(&r).starts_with("📦 *TAMBAH STOK*"),
                "input {input:?}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn the_success_replies_react_and_carry_the_write() {
        let r = run("1|baru@mail", sample(), false);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }
}
