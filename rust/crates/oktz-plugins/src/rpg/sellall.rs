//! `plugins/rpg/sellall.js` — jual SEMUA item yang punya harga jual.
//!
//! `ponytail:` `SELL_PRICES` di JS hanya 22 kunci dan TIDAK memuat
//! `potion`/`mpotion`/`stamina`/`kunai`/`shuriken`/`chakra`/`scroll`
//! (meski ada di `inventory.js`). Item yang tidak punya harga jual di sini
//! tidak terjual dan TIDAK dikosongkan — persis seperti di JS.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::tools::kalkulatormbg::thousands_id;

/// `SELL_PRICES` (`sellall.js:19-42`) — urutan object literal = urutan tampil
/// di balasan.
pub(crate) const SELL_PRICES: [(&str, i64); 22] = [
    ("rock", 20),
    ("coal", 50),
    ("iron", 200),
    ("gold", 1000),
    ("diamond", 5000),
    ("emerald", 10000),
    ("trash", 10),
    ("fish", 100),
    ("prawn", 200),
    ("octopus", 500),
    ("shark", 2000),
    ("whale", 10000),
    ("wood", 30),
    ("stick", 15),
    ("apple", 50),
    ("rubber", 100),
    ("rabbit", 150),
    ("deer", 300),
    ("boar", 500),
    ("bear", 1000),
    ("lion", 2000),
    ("dragon", 10000),
];

/// `(item, qty, earned)` untuk setiap item yang punya stok > 0.
pub(crate) fn sellable(ctx: &Ctx) -> Vec<(&'static str, i64, i64)> {
    let inv = ctx.user_field("inventory");
    SELL_PRICES
        .iter()
        .filter_map(|(item, price)| {
            let qty = inv
                .and_then(|i| i.get(item))
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0);
            (qty > 0).then_some((*item, qty, qty * price))
        })
        .collect()
}

/// Reply; `items` boleh kosong — pemanggil sudah memeriksa sebelumnya.
pub(crate) fn render(items: &[(&'static str, i64, i64)], total: i64) -> String {
    let mut txt = String::from("💰 *sᴇʟʟ ᴀʟʟ sᴜᴋsᴇs*\n\n*📦 *ɪᴛᴇᴍ ᴛᴇʀᴊᴜᴀʟ:*\n\n");
    for (item, qty, earned) in items.iter().take(10) {
        txt.push_str(&format!(
            "> {item}: {qty}x = Rp {}\n",
            thousands_id(*earned as f64)
        ));
    }
    if items.len() > 10 {
        txt.push_str(&format!("> ... dan {} item lainnya\n", items.len() - 10));
    }
    txt.push_str(&format!(
        "\n\n> 💵 Total: *Rp {}*",
        thousands_id(total as f64)
    ));
    txt
}

#[plugin(
    name = "sellall",
    alias = ["jualsemua", "quicksell"],
    category = "rpg",
    description = "Jual semua item yang bisa dijual sekaligus",
    usage = ".sellall",
    example = ".sellall",
    cooldown = 30,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn sellall(ctx: Ctx) -> Reply {
    let items = sellable(&ctx);
    if items.is_empty() {
        return Reply::text("❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ɪᴛᴇᴍ*\n\n> Tidak ada item yang bisa dijual!");
    }

    let total: i64 = items.iter().map(|(_, _, e)| e).sum();
    let koin = crate::common::int_field(&ctx, "koin") + total;

    // Hanya item yang TERJUAL yang dikosongkan; `inventory.potion` dan
    // walau ada di inventory.js — jadi seluruh `inventory` ditulis ulang
    // dari salinan dengan entri terjual di-nolkan.
    let mut inv = ctx
        .user_field("inventory")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
    if let Some(map) = inv.as_object_mut() {
        for (item, _, _) in &items {
            map.insert((*item).to_string(), serde_json::Value::from(0));
        }
    }

    Reply::text(render(&items, total)).with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "koin": koin }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "sellall".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("sellall").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("sellall").unwrap();
        assert_eq!(p.meta.name, "sellall");
        assert_eq!(p.meta.alias, ["jualsemua", "quicksell"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Jual semua item yang bisa dijual sekaligus");
        assert_eq!(p.meta.usage, ".sellall");
        assert_eq!(p.meta.example, ".sellall");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["jualsemua", "quicksell"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "sellall");
        }
    }

    #[test]
    fn an_empty_bag_is_reported_and_nothing_is_written() {
        for user in [
            json!(null),
            json!({}),
            json!({"inventory": {}}),
            json!({"inventory": null}),
        ] {
            let r = run(user.clone());
            assert!(
                text_of(&r).starts_with("❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ɪᴛᴇᴍ*"),
                "input {user}"
            );
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn a_bag_holding_only_unsellable_items_is_reported_as_empty() {
        // Potion ada di inventory tapi TIDAK ada di SELL_PRICES.
        let r = run(json!({"inventory": {"potion": 5, "kunai": 3}}));
        assert!(text_of(&r).starts_with("❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ɪᴛᴇᴍ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_sellable_item_is_priced_and_zeroed() {
        let r = run(json!({
            "inventory": {"rock": 2, "fish": 3, "dragon": 1},
            "koin": 100,
        }));
        let data = write_of(&r);
        // 2*20 + 3*100 + 1*10000 = 10340
        assert_eq!(data["koin"], json!(100 + 10340));
        let inv = data["inventory"].as_object().unwrap();
        for item in ["rock", "fish", "dragon"] {
            assert_eq!(inv[item], json!(0), "{item} harus terjual");
        }
    }

    #[test]
    fn unsellable_items_are_untouched_in_the_written_inventory() {
        let r = run(json!({"inventory": {"rock": 1, "potion": 9}}));
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["rock"], json!(0));
        assert_eq!(inv["potion"], json!(9), "tidak punya harga jual, tetap");
    }

    #[test]
    fn the_reply_lists_items_and_the_total_matches_the_write() {
        let r = run(json!({"inventory": {"rock": 2, "gold": 1}}));
        let t = text_of(&r);
        assert!(t.starts_with("💰 *sᴇʟʟ ᴀʟʟ sᴜᴋsᴇs*"), "{t}");
        assert!(t.contains("> rock: 2x = Rp 40"), "{t}");
        assert!(t.contains("> gold: 1x = Rp 1.000"), "{t}");
        assert!(t.contains("> 💵 Total: *Rp 1.040*"), "{t}");
        assert_eq!(write_of(&r)["koin"], json!(1040));
    }

    #[test]
    fn items_are_rendered_in_the_js_table_order_not_alphabetical() {
        // Dragon (index terakhir) harus tampil sebelum rock (index pertama)
        // kalau urutannya benar-benar mengikuti object literal.
        let r = run(json!({"inventory": {"rock": 1, "trash": 1, "dragon": 1}}));
        let t = text_of(&r);
        let pos = |n: &str| t.find(n).expect("item hilang");
        assert!(pos("rock") < pos("trash"), "{t}");
        assert!(pos("trash") < pos("dragon"), "{t}");
    }

    #[test]
    fn more_than_ten_item_lines_are_summarised() {
        // 12 item berbeda -> 10 baris + ringkasan.
        let mut inv = serde_json::Map::new();
        for (item, _) in SELL_PRICES.iter().take(12) {
            inv.insert((*item).to_string(), json!(1));
        }
        let r = run(json!({ "inventory": inv }));
        let t = text_of(&r);
        assert!(t.contains("> ... dan 2 item lainnya"), "{t}");
        assert!(!t.contains("> bear:"), "item ke-11/12 tidak dirender");
    }

    #[test]
    fn exactly_ten_item_lines_get_no_summary() {
        let mut inv = serde_json::Map::new();
        for (item, _) in SELL_PRICES.iter().take(10) {
            inv.insert((*item).to_string(), json!(1));
        }
        let r = run(json!({ "inventory": inv }));
        let t = text_of(&r);
        assert!(!t.contains("item lainnya"), "tepat 10 tidak perlu ringkasan");
    }

    #[test]
    fn a_zero_quantity_item_is_not_sold_and_stays_zero() {
        let r = run(json!({"inventory": {"rock": 0, "fish": 1}}));
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["rock"], json!(0));
        assert!(text_of(&r).contains("> fish: 1x = Rp 100"));
        assert!(!text_of(&r).contains("rock"), "qty 0 tidak masuk daftar");
    }

    #[test]
    fn a_missing_koin_reads_as_zero_and_still_gains_the_sale() {
        let r = run(json!({"inventory": {"rock": 1}}));
        assert_eq!(write_of(&r)["koin"], json!(20), "`user.koin || 0`");
    }

    #[test]
    fn the_sale_reply_is_paired_with_the_write() {
        let r = run(json!({"inventory": {"rock": 1}}));
        assert!(text_of(&r).contains("sᴇʟʟ ᴀʟʟ sᴜᴋsᴇs"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_price_table_matches_the_js_values() {
        assert_eq!(SELL_PRICES.len(), 22);
        assert_eq!(SELL_PRICES[0], ("rock", 20));
        assert_eq!(SELL_PRICES[6], ("trash", 10));
        assert_eq!(SELL_PRICES[21], ("dragon", 10000));
    }
}