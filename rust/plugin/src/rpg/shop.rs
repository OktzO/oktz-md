//! `plugins/rpg/shop.js` — beli dan jual item RPG.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::inventory_map;
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const BUYABLE: u8 = 0;
pub(crate) const SELLABLE: u8 = 1;

/// `ITEMS` (`shop.js:19-61`) — `(kunci, harga, tipe, nama)`. Urutan object
/// literal = urutan dua daftar di balasan pembuka.
pub(crate) const ITEMS: [(&str, i64, u8, &str); 36] = [
    ("potion", 500, BUYABLE, "🥤 Health Potion"),
    ("mpotion", 500, BUYABLE, "🧪 Mana Potion"),
    ("stamina", 1000, BUYABLE, "⚡ Stamina Potion"),
    ("common", 2000, BUYABLE, "📦 Common Crate"),
    ("uncommon", 10000, BUYABLE, "🛍️ Uncommon Crate"),
    ("mythic", 50000, BUYABLE, "🎁 Mythic Crate"),
    ("legendary", 200000, BUYABLE, "💎 Legendary Crate"),
    ("wheat", 50, BUYABLE, "🌾 Gandum"),
    ("rice", 50, BUYABLE, "🍚 Beras"),
    ("egg", 100, BUYABLE, "🥚 Telur"),
    ("meat", 300, BUYABLE, "🥩 Daging"),
    ("herb", 150, BUYABLE, "🌿 Herba"),
    ("carrot", 50, BUYABLE, "🥕 Wortel"),
    ("potato", 50, BUYABLE, "🥔 Kentang"),
    ("strawberry", 80, BUYABLE, "🍓 Stroberi"),
    ("watermelon", 100, BUYABLE, "🍉 Semangka"),
    ("apple", 50, BUYABLE, "🍎 Apel"),
    ("rock", 20, SELLABLE, "🪨 Batu"),
    ("coal", 50, SELLABLE, "⚫ Batubara"),
    ("iron", 200, SELLABLE, "⛓️ Besi"),
    ("gold", 1000, SELLABLE, "🥇 Emas"),
    ("diamond", 5000, SELLABLE, "💠 Berlian"),
    ("emerald", 10000, SELLABLE, "💚 Emerald"),
    ("trash", 10, SELLABLE, "🗑️ Sampah"),
    ("fish", 100, SELLABLE, "🐟 Ikan"),
    ("prawn", 200, SELLABLE, "🦐 Udang"),
    ("octopus", 500, SELLABLE, "🐙 Gurita"),
    ("shark", 2000, SELLABLE, "🦈 Hiu"),
    ("whale", 10000, SELLABLE, "🐳 Paus"),
    ("leather", 50, SELLABLE, "👞 Kulit"),
    ("mysterybox", 1500, SELLABLE, "📦 Mystery Box"),
    ("kunai", 100, SELLABLE, "🗡️ Kunai"),
    ("shuriken", 150, SELLABLE, "⚔️ Shuriken"),
    ("chakra", 500, SELLABLE, "🌀 Chakra"),
    ("scroll", 2000, SELLABLE, "📜 Scroll Ninja"),
    ("bowlramen", 800, SELLABLE, "🍜 Ramen"),
];

pub(crate) fn find_item(key: &str) -> Option<&'static (&'static str, i64, u8, &'static str)> {
    ITEMS.iter().find(|i| i.0 == key)
}

/// `Math.max(1, parseInt(args[2]) || 1)`.
pub(crate) fn amount_of(raw: Option<&String>) -> i64 {
    crate::common::parse_int(raw.map(String::as_str).unwrap_or(""))
        .filter(|n| *n != 0)
        .unwrap_or(1)
        .max(1)
}

#[plugin(
    name = "shop",
    alias = ["beli", "jual", "toko", "store", "sell"],
    category = "rpg",
    description = "Beli dan jual item RPG",
    usage = ".shop <buy/sell> <item> <jumlah>",
    example = ".shop buy potion 1",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn shop(ctx: Ctx) -> Reply {
    let koin = int_field(&ctx, "koin");
    let action = ctx.args.first().map(|a| a.to_lowercase());

    if !matches!(action.as_deref(), Some("buy" | "sell")) {
        let mut txt = String::from(
            "🏪 *Toko Kelontong RPG* ✨\n\n\
             Halo kak! Selamat datang di toko kelontong.\n\
             Mau beli potion atau jual barang rongsokan nih? 😂\n\n\
             *Cara Transaksi:* 💸\n\
             Ketik `.shop buy <nama> <jumlah>` buat beli.\n\
             Ketik `.shop sell <nama> <jumlah>` buat jual.\n\n\
             *🛍️ Barang yang Dijual (BUY):*\n",
        );
        for (_, price, _kind, name) in ITEMS.iter().filter(|i| i.2 == BUYABLE) {
            txt += &format!("{name}: *Rp {}*\n", thousands_id(*price as f64));
        }
        txt += "\n*💰 Barang yang Diterima (SELL):*\n";
        for (_, price, _kind, name) in ITEMS.iter().filter(|i| i.2 == SELLABLE) {
            txt += &format!("{name}: *Rp {}*\n", thousands_id(*price as f64));
        }
        return Reply::text(txt);
    }

    let raw_key = ctx.args.get(1).cloned().unwrap_or_default();
    let key = raw_key.to_lowercase();
    let amount = amount_of(ctx.args.get(2));

    let Some(&(_, price, kind, name)) = find_item(&key) else {
        return Reply::text(format!(
            "Aduh kak, barang *{}* nggak ada di daftar! 😭❌\nCoba cek lagi list barangnya ketik `.shop` ya.",
            if raw_key.is_empty() { "itu" } else { &raw_key }
        ));
    };
    let buying = action.as_deref() == Some("buy");

    if buying && kind != BUYABLE {
        return Reply::text(format!(
            "Hayo lho kak, barang *{name}* ini khusus buat dijual, nggak bisa dibeli! 🫣❌"
        ));
    }
    if !buying && kind != SELLABLE {
        return Reply::text(format!(
            "Maaf kak, toko kita nggak nerima barang *{name}* ini! Nggak laku dijual lagi soalnya 😂❌"
        ));
    }

    let mut inv = inventory_map(&ctx);

    if buying {
        let total = price * amount;
        if koin < total {
            return Reply::text(format!(
                "Yahh, koin kamu kurang nih kak buat beli *{amount}x {name}*! 😭😭\n\
                 Koin kamu: *Rp {}*\n\
                 Kurang *Rp {}* lagi. Nyari duit dulu gih! 💸🏃💨",
                thousands_id(koin as f64),
                thousands_id((total - koin) as f64)
            ));
        }
        let slot = inv.entry(key.clone()).or_insert(serde_json::json!(0));
        let have = slot.as_i64().unwrap_or(0);
        *slot = serde_json::json!(have + amount);
        return Reply::text(format!(
            "MAKASIH BANYAK KAK! 🎉✨\n\n\
             Kamu berhasil borong:\n\
             🛒 Item: *{amount}x {name}*\n\
             💸 Total Bayar: *Rp {}*\n\n\
             Ditunggu kedatangannya lagi ya! 💖🛍️",
            thousands_id(total as f64)
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin - total, "inventory": inv }),
        });
    }

    let stock = inv.get(key.as_str()).and_then(Value::as_i64).unwrap_or(0);
    if stock < amount {
        return Reply::text(format!(
            "Loh kak, barangnya kurang nih! 🫣\n\
             Kamu cuma punya *{stock}x {name}*, masa mau jual *{amount}*? Jangan ngibul dong! 😂❌"
        ));
    }
    let total = price * amount;
    // Tidak ada `delete` di JS untuk jalur jual, jadi sisa 0 tetap ditulis.
    inv.insert(key.clone(), serde_json::json!(stock - amount));
    Reply::text(format!(
        "CINGG! UANG MASUK! 💰✨\n\n\
         Kamu berhasil ngejual:\n\
         📦 Item: *{amount}x {name}*\n\
         🤑 Total Dapat: *Rp {}*\n\n\
         Makasih ya udah cuci gudang di sini! 🎉💖",
        thousands_id(total as f64)
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": koin + total, "inventory": inv }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "shop".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("shop").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Versi yang memiliki reply-nya — untuk `text(run(..))` di dalam assert.
    fn text(r: Reply) -> String {
        text_of(&r).to_string()
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, SENDER);
                data
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("shop").unwrap();
        assert_eq!(p.meta.name, "shop");
        assert_eq!(p.meta.alias, ["beli", "jual", "toko", "store", "sell"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Beli dan jual item RPG");
        assert_eq!(p.meta.usage, ".shop <buy/sell> <item> <jumlah>");
        assert_eq!(p.meta.example, ".shop buy potion 1");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        // `beli` juga nama `plugins/store/beli.js`; nama didahulukan
        // (`plugins.js:539`), jadi alias `beli` bukan milik shop.
        assert_eq!(oktz_plugin_api::find("beli").unwrap().meta.name, "beli");
        for a in ["jual", "toko", "store", "sell"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "shop", "alias {a}");
        }
    }

    #[test]
    fn the_front_board_splits_the_two_lists_without_repeating_a_row() {
        for args in [vec![], vec!["jajan"]] {
            let t = text(run(&args, serde_json::json!({})));
            assert!(t.starts_with("🏪 *Toko Kelontong RPG*"), "{args:?}: {t}");
            let (buy, sell) = t
                .split_once("*💰 Barang yang Diterima (SELL):*\n")
                .unwrap_or_else(|| panic!("tidak ada pemisah: {t}"));
            for (_, price, kind, name) in ITEMS {
                if kind == BUYABLE {
                    assert!(
                        buy.contains(&format!("{name}: *Rp {}*", thousands_id(price as f64))),
                        "{args:?} {name} kurang di BUY: {buy}"
                    );
                    assert!(!sell.contains(name), "{args:?} {name} dobel: {t}");
                } else {
                    assert!(
                        sell.contains(&format!("{name}: *Rp {}*", thousands_id(price as f64))),
                        "{args:?} {name} kurang di SELL"
                    );
                    assert!(!buy.contains(name), "{args:?} {name} dobel: {t}");
                }
            }
            assert_eq!(buy.matches("Rp ").count(), 17, "17 barang beliable");
            assert_eq!(sell.matches("Rp ").count(), 19, "19 barang sellable");
        }
    }

    #[test]
    fn buying_stacks_onto_the_existing_stack_and_charges_the_purse() {
        let r = run(
            &["buy", "potion", "4"],
            serde_json::json!({"koin": 5000, "inventory": {"potion": 2}}),
        );
        let d = written(&r);
        assert_eq!(d["koin"], serde_json::json!(3000), "5000 - 4*500");
        assert_eq!(d["inventory"]["potion"], serde_json::json!(6));
    }

    #[test]
    fn a_short_purse_names_the_exact_shortfall() {
        let r = run(&["buy", "mythic", "2"], serde_json::json!({"koin": 60_000}));
        let t = text_of(&r);
        assert!(t.contains("Koin kamu: *Rp 60.000*"), "{t}");
        assert!(t.contains("Kurang *Rp 40.000* lagi"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_sell_only_item_cannot_be_bought_and_buy_only_cannot_be_sold() {
        let buy = text(run(&["buy", "rock"], serde_json::json!({"koin": 999_999})));
        assert!(buy.contains("*🪨 Batu* ini khusus buat dijual"), "{buy}");
        let sell = text(run(&["sell", "potion"], serde_json::json!({})));
        assert!(sell.contains("*🥤 Health Potion* ini"), "{sell}");
        assert!(sell.contains("Nggak laku dijual lagi"), "{sell}");
    }

    #[test]
    fn selling_the_whole_stack_writes_zero_instead_of_deleting_the_key() {
        // Tidak seperti `merchant.js`, `shop.js` tidak pernah `delete`.
        let r = run(&["sell", "trash", "3"], serde_json::json!({"inventory": {"trash": 3}}));
        let d = written(&r);
        assert_eq!(d["inventory"]["trash"], serde_json::json!(0));
        assert_eq!(d["koin"], serde_json::json!(30), "3 * 10");
    }

    #[test]
    fn selling_more_than_the_stock_is_refused_with_both_numbers() {
        let r = run(&["sell", "gold", "2"], serde_json::json!({"inventory": {"gold": 1}}));
        let t = text_of(&r);
        assert!(t.contains("Kamu cuma punya *1x 🥇 Emas*"), "{t}");
        assert!(t.contains("masa mau jual *2*?"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_or_missing_item_name_says_itu() {
        let t = text(run(&["buy", "nasi"], serde_json::json!({})));
        assert!(t.starts_with("Aduh kak, barang *nasi* nggak ada di daftar!"), "{t}");

        let t = text(run(&["buy"], serde_json::json!({})));
        assert!(t.starts_with("Aduh kak, barang *itu* nggak ada di daftar!"), "{t}");
    }

    #[test]
    fn the_item_key_is_case_insensitive() {
        let r = run(&["BUY", "Potion", "1"], serde_json::json!({"koin": 1000}));
        assert_eq!(written(&r)["inventory"]["potion"], serde_json::json!(1));
        assert_eq!(written(&r)["koin"], serde_json::json!(500));
    }

    #[test]
    fn the_amount_defaults_to_one_and_never_reaches_below_it() {
        for args in [
            vec!["buy", "egg"],
            vec!["buy", "egg", "0"],
            vec!["buy", "egg", "-3"],
            vec!["buy", "egg", "abc"],
        ] {
            let r = run(&args, serde_json::json!({"koin": 10_000}));
            assert_eq!(written(&r)["inventory"]["egg"], serde_json::json!(1), "{args:?}");
            assert_eq!(written(&r)["koin"], serde_json::json!(9900), "{args:?}");
        }
    }
}
