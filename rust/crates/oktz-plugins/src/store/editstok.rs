//! `plugins/store/editstok.js` — ganti detail satu item stok digital.
//!
//! Format: `<nomor_produk> <nomor_item>|<detail_baru>`. Baris pemisah `|`
//! dicari PADA INPUT, jadi `<produk> <item>` tanpa `|` selalu cabang bantuan.
//! `;;` jadi baris baru di detail yang disimpan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::parse_int;
use crate::store::liststok::is_fisik;

pub(crate) fn products(ctx: &Ctx) -> &[serde_json::Value] {
    ctx.setting_list("storeProducts")
}

pub(crate) fn no_products(prefix: &str) -> String {
    format!("📭 *Belum ada produk.*\n\nTambahkan produk terlebih dahulu: `{prefix}addproduk` ➕")
}

pub(crate) fn denied() -> &'static str {
    "🚫 *Akses Ditolak*\n\nUntuk menjaga privasi 🛡️, pengeditan stok hanya dapat dilakukan di *private chat*.\n\nSilakan chat bot secara langsung 📱"
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "✏️ *EDIT STOK*\n\n\
         📋 Format: `{p}editstok <nomor_produk> <nomor_item>|<detail_baru>`\n\n\
         📝 *Contoh:*\n\
         `{p}editstok 1 3|Email: baru@mail.com;;Password: newpass`\n\n\
         • Gunakan `;;` untuk baris baru dalam detail 🔑\n\
         📋 Lihat nomor item: `{p}liststok <nomor_produk>`\n\n\
         ⚠️ _Stok yang sudah terkirim ke pembeli tidak akan berubah_ 🔒",
        p = prefix
    )
}

/// Pratinjau 50 karakter pertama, baris baru diratakan jadi spasi.
pub(crate) fn preview(detail: &str) -> String {
    detail.replace('\n', " ").chars().take(50).collect()
}

pub(crate) fn confirm(name: &str, item_no: usize, old: &str, new: &str) -> String {
    format!(
        "✅ *STOK DIPERBARUI*\n\n\
         🏷️ Produk: *{name}*\n\
         🔑 Item #{}\n\n\
         ❌ Sebelum:\n`{}`\n\n\
         ✅ Sesudah:\n`{}`\n\n\
         ⚠️ _Perubahan hanya berlaku untuk item yang belum dikirim ke pembeli_ 🔒",
        item_no + 1,
        preview(old),
        preview(new)
    )
}

#[plugin(
    name = "editstok",
    alias = ["editstock"],
    category = "store",
    description = "✏️ Edit stok item produk (hanya di private chat)",
    usage = ".editstok <nomor_produk> <nomor_item>|<detail_baru>",
    example = ".editstok 1 3|Email: baru@mail.com;;Password: newpass",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = true,
    is_owner = true
)]
pub fn editstok(ctx: Ctx) -> Reply {
    // `m.isGroup` diperiksa PALING DULU, sebelum store dibaca.
    if ctx.is_group {
        return Reply::text(denied());
    }

    let products = products(&ctx);
    if products.is_empty() {
        return Reply::text(no_products(&ctx.prefix));
    }

    let text = ctx.text.trim();
    // `indexOf('|') === -1` → bantuan; bukan "isi kosong".
    let Some(pipe) = text.find('|') else {
        return Reply::text(help(&ctx.prefix));
    };

    let before = text[..pipe].trim();
    let new_detail = text[pipe + 1..].trim().replace(";;", "\n");

    let parts: Vec<&str> = before.split_whitespace().collect();
    let product_no = parts
        .first()
        .and_then(|p| parse_int(p))
        .map_or(-1, |n| n - 1);
    let item_no = parts
        .get(1)
        .and_then(|p| parse_int(p))
        .map_or(-1, |n| n - 1);

    if product_no < 0 || product_no >= products.len() as i64 {
        return Reply::text(format!(
            "❌ *Nomor produk tidak valid.*\n\nRentang: 1-{} 📋",
            products.len()
        ));
    }
    let product = products[product_no as usize].clone();

    if is_fisik(&product) {
        return Reply::text(format!(
            "📦 *Produk Fisik*\n\n\
             Produk fisik tidak memiliki data per-item 🔑\n\
             Untuk mengubah stok, gunakan:\n\
             `{}editproduk {} stok <jumlah>`",
            ctx.prefix,
            product_no + 1
        ));
    }

    let items_len = product
        .get("stockItems")
        .and_then(|v| v.as_array())
        .map_or(0, Vec::len);
    if item_no < 0 || item_no >= items_len as i64 {
        return Reply::text(format!(
            "❌ *Nomor item tidak valid.*\n\nRentang: 1-{items_len}\n\n📋 Lihat daftar: `{}liststok {}`",
            ctx.prefix,
            product_no + 1
        ));
    }

    if new_detail.chars().count() < 3 {
        return Reply::text("❌ *Detail terlalu pendek.*\n\nMinimal 3 karakter diperlukan 🔑");
    }

    let old_detail = product["stockItems"][item_no as usize]
        .get("detail")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut next = products.to_vec();
    next[product_no as usize]["stockItems"][item_no as usize]["detail"] =
        serde_json::Value::String(new_detail.clone());
    // `updatedAt = new Date().toISOString()` — stempel waktu wall-clock.
    next[product_no as usize]["stockItems"][item_no as usize]["updatedAt"] =
        serde_json::Value::String(iso_now());
    let name = product.get("name").and_then(|v| v.as_str()).unwrap_or("");

    Reply::text(confirm(name, item_no as usize, &old_detail, &new_detail))
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "storeProducts".into(),
            value: serde_json::Value::Array(next),
        })
}

/// `new Date().toISOString()` tanpa dependensi waktu — bentuk yang sama
/// (`YYYY-MM-DDTHH:MM:SS.sssZ`) dari epoch ms, dihitung di zona UTC.
fn iso_now() -> String {
    iso_from_ms(crate::common::now_ms())
}

/// `toISOString()` dari epoch ms, memakai algoritma civil yang sudah ada di
/// `common::days_from_civil`.
fn iso_from_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let millis = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y,
        m,
        d,
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60,
        millis
    )
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(settings: Value, text: &str, is_group: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "editstok".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_group,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("editstok").unwrap().handler)(ctx)
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
        json!({"storeProducts": [
            {"name": "Netflix", "type": "digital", "price": 25000,
             "stockItems": [{"detail": "lama1@mail"}, {"detail": "lama2@mail"}], "stock": 2},
            {"name": "Kaos", "type": "fisik", "price": 50000, "stock": 10},
        ]})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("editstok").unwrap();
        assert_eq!(p.meta.name, "editstok");
        assert_eq!(p.meta.alias, ["editstock"]);
        assert_eq!(p.meta.category, "store");
        assert_eq!(
            p.meta.description,
            "✏️ Edit stok item produk (hanya di private chat)"
        );
        assert_eq!(p.meta.usage, ".editstok <nomor_produk> <nomor_item>|<detail_baru>");
        assert_eq!(
            p.meta.example,
            ".editstok 1 3|Email: baru@mail.com;;Password: newpass"
        );
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("editstock").unwrap().meta.name, "editstok");
    }

    #[test]
    fn the_group_branch_is_rejected_before_the_store_is_read() {
        // Dicek paling awal — termasuk ketika store kosong.
        let r = run(json!({}), "1 1|baru", true);
        assert_eq!(text_of(&r), denied());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_empty_product_list_is_reported_and_nothing_is_written() {
        for s in [json!({}), json!({"storeProducts": []})] {
            let r = run(s.clone(), "1 1|baru", false);
            assert!(text_of(&r).starts_with("📭 *Belum ada produk.*"), "input {s}");
            assert!(r.mutations().is_empty(), "input {s}");
        }
    }

    #[test]
    fn input_without_a_pipe_shows_help_even_when_products_exist() {
        // `indexOf('|') === -1` diperiksa SEBELUM validasi nomor.
        for input in ["", "1 1", "abc def"] {
            let r = run(sample(), input, false);
            assert!(
                text_of(&r).starts_with("✏️ *EDIT STOK*"),
                "input {input:?}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn a_valid_edit_replaces_the_detail_and_stamps_updated_at() {
        let r = run(sample(), "1 2|baru@mail.com", false);
        let items = products_write(&r)[0]["stockItems"].as_array().unwrap().clone();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["detail"], json!("lama1@mail"), "yang lain utuh");
        assert_eq!(items[1]["detail"], json!("baru@mail.com"));
        assert!(
            items[1]["updatedAt"].as_str().unwrap().ends_with('Z'),
            "stempel ISO-8601: {:?}",
            items[1]["updatedAt"]
        );
    }

    #[test]
    fn a_double_semicolon_becomes_a_real_newline_in_the_stored_detail() {
        let r = run(sample(), "1 1|satu;;dua;;tiga", false);
        assert_eq!(
            products_write(&r)[0]["stockItems"][0]["detail"],
            json!("satu\ndua\ntiga")
        );
    }

    #[test]
    fn the_preview_flattens_newlines_and_caps_at_fifty_characters() {
        let long = "y".repeat(80);
        let r = run(sample(), &format!("1 1|{long}"), false);
        let t = text_of(&r);
        let after = t.split("✅ Sesudah:\n`").nth(1).unwrap();
        let line = after.split('`').next().unwrap();
        assert_eq!(line.chars().count(), 50, "baris pratinjau: {line}");
    }

    #[test]
    fn the_confirm_reply_shows_the_item_number_and_both_details() {
        let r = run(sample(), "1 1|baru@mail", false);
        let t = text_of(&r);
        assert!(t.starts_with("✅ *STOK DIPERBARUI*"), "{t}");
        assert!(t.contains("🏷️ Produk: *Netflix*"), "{t}");
        assert!(t.contains("🔑 Item #1"), "nomor item mulai dari 1: {t}");
        assert!(t.contains("❌ Sebelum:\n`lama1@mail`"), "{t}");
        assert!(t.contains("✅ Sesudah:\n`baru@mail`"), "{t}");
    }

    #[test]
    fn an_out_of_range_product_number_is_reported_with_the_range() {
        for input in ["0 1|x", "3 1|x", "9 1|x"] {
            let r = run(sample(), input, false);
            assert!(
                text_of(&r).starts_with("❌ *Nomor produk tidak valid.*"),
                "input {input:?}"
            );
            assert!(text_of(&r).contains("Rentang: 1-2"), "{}", text_of(&r));
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn a_physical_product_is_redirected_to_editproduk() {
        let r = run(sample(), "2 1|apa saja", false);
        assert!(text_of(&r).starts_with("📦 *Produk Fisik*"), "{}", text_of(&r));
        assert!(text_of(&r).contains(".editproduk 2 stok"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_out_of_range_item_number_is_reported_with_the_range() {
        for input in ["1 0|x", "1 3|x", "1 9|x"] {
            let r = run(sample(), input, false);
            assert!(
                text_of(&r).starts_with("❌ *Nomor item tidak valid.*"),
                "input {input:?}"
            );
            assert!(text_of(&r).contains("Rentang: 1-2"), "{}", text_of(&r));
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn a_detail_shorter_than_three_characters_is_rejected() {
        for input in ["1 1|ab", "1 1|a", "1 1|  "] {
            let r = run(sample(), input, false);
            assert!(
                text_of(&r).starts_with("❌ *Detail terlalu pendek.*"),
                "input {input:?}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn exactly_three_characters_is_accepted() {
        let r = run(sample(), "1 1|abc", false);
        assert_eq!(products_write(&r)[0]["stockItems"][0]["detail"], json!("abc"));
    }

    #[test]
    fn the_success_reply_reacts_and_is_paired_with_the_write() {
        let r = run(sample(), "1 1|baru", false);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_iso_timestamp_matches_the_known_epoch() {
        // 2024-01-01T00:00:00.000Z
        assert_eq!(iso_from_ms(1_704_067_200_000), "2024-01-01T00:00:00.000Z");
        // 1970-01-01T00:00:00.000Z
        assert_eq!(iso_from_ms(0), "1970-01-01T00:00:00.000Z");
        // 2024-12-31T23:59:59.999Z
        assert_eq!(iso_from_ms(1_735_689_599_999), "2024-12-31T23:59:59.999Z");
        assert!(iso_now().ends_with('Z'));
    }
}