//! `plugins/rpg/merchant.js` — jual beli item ke NPC merchant.
//!
//! `toLocaleString()` tanpa locale memakai koma (locale default Node `en-US`).
//! `handler` di JS bukan `async` dan TIDAK selalu return: kalau `action` bukan
//! buy/sell/list, cabang fall-through mengembalikan `undefined` — tidak ada
//! pesan. Di sini semua jalur tetap punya balasan karena setiap jalur yang bisa
//! dicapai sudah Dixplicit `return m.reply(...)` di JS.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{inventory_map, thousands_en};

/// `SHOP_ITEMS` (`merchant.js:18-33`) — `(kunci, nama, beli, jual, deskripsi)`;
/// urutan object literal = urutan tampil di `list`.
pub(crate) const SHOP_ITEMS: [(&str, &str, i64, i64, &str); 14] = [
    ("potion", "🧪 Potion", 100, 50, "Pulihkan 50 HP"),
    ("manapotion", "💙 Mana Potion", 150, 75, "Pulihkan 50 Mana"),
    ("antidote", "💊 Antidote", 80, 40, "Sembuhkan racun"),
    ("bread", "🍞 Roti", 30, 15, "Pulihkan 10 stamina"),
    ("energydrink", "⚡ Energy Drink", 200, 100, "Pulihkan 50 stamina"),
    ("pickaxe", "⛏️ Beliung", 500, 250, "Untuk mining"),
    ("fishingrod", "🎣 Joran", 400, 200, "Untuk memancing"),
    ("wood", "🪵 Kayu", 50, 25, "Material dasar"),
    ("iron", "🔩 Besi", 80, 40, "Material logam"),
    ("leather", "🧶 Kulit", 60, 30, "Material armor"),
    ("string", "🧵 Benang", 40, 20, "Material busur"),
    ("herb", "🌿 Herba", 70, 35, "Bahan alchemy"),
    ("gold", "🪙 Emas", 500, 250, "Material langka"),
    ("diamond", "💎 Berlian", 2000, 1000, "Material mewah"),
];

pub(crate) fn find_item(key: &str) -> Option<(&'static str, &'static str, i64, i64, &'static str)> {
    SHOP_ITEMS
        .iter()
        .find(|i| i.0 == key)
        .copied()
}

/// `Math.max(1, parseInt(args[2]) || 1)`.
pub(crate) fn qty_of(raw: Option<&String>) -> i64 {
    crate::common::parse_int(raw.map(String::as_str).unwrap_or(""))
        .filter(|n| *n != 0)
        .unwrap_or(1)
        .max(1)
}

#[plugin(
    name = "merchant",
    alias = ["npc", "tokoku"],
    category = "rpg",
    description = "Jual beli item ke NPC merchant",
    usage = ".merchant <buy/sell> <item> <qty>",
    example = ".merchant buy potion 5",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn merchant(ctx: Ctx) -> Reply {
    let koin = int_field(&ctx, "koin");
    let arg = |i: usize| ctx.args.get(i).map(|s| s.to_lowercase());
    let action = arg(0);
    let item_key = arg(1);
    let qty = qty_of(ctx.args.get(2));

    if !matches!(action.as_deref(), Some("buy" | "sell" | "list")) {
        return Reply::text(format!(
            "🏪 *ᴍᴇʀᴄʜᴀɴᴛ sʜᴏᴘ*\n\n\
             > Selamat datang di toko!\n\n\
             *📋 *ᴄᴏᴍᴍᴀɴᴅ:*
\n\
             > {}merchant list\n\
             > {}merchant buy <item> <qty>\n\
             > {}merchant sell <item> <qty>\n\n\n\
             💰 *Balance:* {}",
            ctx.prefix,
            ctx.prefix,
            ctx.prefix,
            thousands_en(koin)
        ));
    }

    if action.as_deref() == Some("list") {
        let mut txt = String::from("🏪 *ᴅᴀꜰᴛᴀʀ ɪᴛᴇᴍ*\n\n*📦 *sʜᴏᴘ:*\n\n");
        for (key, name, buy, sell, desc) in SHOP_ITEMS {
            txt += &format!(
                "> {name}\n> 💵 Beli: {}\n> 💰 Jual: {}\n> 📝 {desc}\n> → `{key}`\n> \n",
                thousands_en(buy),
                thousands_en(sell),
            );
        }
        return Reply::text(txt);
    }

    let Some(key) = item_key else {
        return Reply::text(format!(
            "❌ Tentukan item!\n\n> Contoh: `{}merchant {} potion 5`",
            ctx.prefix,
            action.as_deref().unwrap_or("buy")
        ));
    };
    let Some((name, _, buy, sell, _)) = find_item(&key) else {
        return Reply::text(if action.as_deref() == Some("sell") {
            "❌ Item tidak bisa dijual ke merchant!".to_string()
        } else {
            format!(
                "❌ Item tidak ditemukan!\n\n> Ketik `{}merchant list` untuk melihat daftar.",
                ctx.prefix
            )
        });
    };

    let mut inv = inventory_map(&ctx);

    if action.as_deref() == Some("buy") {
        let total = buy * qty;
        if koin < total {
            return Reply::text(format!(
                "❌ *ʙᴀʟᴀɴᴄᴇ ᴋᴜʀᴀɴɢ*\n\n> Harga: {}\n> Balance: {}",
                thousands_en(total),
                thousands_en(koin)
            ));
        }
        let slot = inv.entry(key.to_string()).or_insert(serde_json::json!(0));
        let have = slot.as_i64().unwrap_or(0);
        *slot = serde_json::json!(have + qty);
        return Reply::text(format!(
            "✅ *ᴘᴇᴍʙᴇʟɪᴀɴ ʙᴇʀʜᴀsɪʟ*\n\n\
             *🛒 *ᴅᴇᴛᴀɪʟ:*
\n\
             > 📦 Item: *{name}*\n\
             > 📊 Qty: *{qty}*\n\
             > 💵 Total: *-{}*\n\
             > 💰 Sisa: *{}*\n",
            thousands_en(total),
            thousands_en(koin - total)
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin - total, "inventory": inv }),
        });
    }

    // sell
    let have = inv.get(key.as_str()).and_then(Value::as_i64).unwrap_or(0);
    if have < qty {
        return Reply::text(format!(
            "❌ *ɪᴛᴇᴍ ᴋᴜʀᴀɴɢ*\n\n> Punya: {have}\n> Mau jual: {qty}"
        ));
    }
    let total = sell * qty;
    let left = have - qty;
    // `if (user.inventory[itemKey] <= 0) delete ...`
    if left <= 0 {
        inv.remove(key.as_str());
    } else {
        inv.insert(key.to_string(), serde_json::json!(left));
    }
    Reply::text(format!(
        "✅ *ᴘᴇɴᴊᴜᴀɴ ʙᴇʀʜᴀsɪʟ*\n\n\
         *💰 *ᴅᴇᴛᴀɪʟ:*
\n\
         > 📦 Item: *{name}*\n\
         > 📊 Qty: *{qty}*\n\
         > 💵 Total: *+{}*\n\
         > 💰 Balance: *{}*\n",
        thousands_en(total),
        thousands_en(koin + total)
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
            command: "merchant".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("merchant").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("merchant").unwrap();
        assert_eq!(p.meta.name, "merchant");
        assert_eq!(p.meta.alias, ["npc", "tokoku"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Jual beli item ke NPC merchant");
        assert_eq!(p.meta.usage, ".merchant <buy/sell> <item> <qty>");
        assert_eq!(p.meta.example, ".merchant buy potion 5");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["npc", "tokoku"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "merchant");
        }
    }

    #[test]
    fn an_unknown_action_shows_the_welcome_board() {
        for args in [vec![], vec!["jajan"], vec!["BUY2"]] {
            let r = run(&args, serde_json::json!({"koin": 1500}));
            let t = text(run(&args, serde_json::json!({"koin": 1500})));
            assert!(t.starts_with("🏪 *ᴍᴇʀᴄʜᴀɴᴛ sʜᴏᴘ*"), "{args:?} -> {t}");
            assert!(t.contains("💰 *Balance:* 1,500"), "{args:?} -> {t}");
            assert!(r.mutations().is_empty(), "{args:?}");
        }
    }

    #[test]
    fn the_action_is_lowercased_before_it_is_matched() {
        let r = run(&["BUY"], serde_json::json!({"koin": 1000}));
        assert_eq!(
            text_of(&r),
            "❌ Tentukan item!\n\n> Contoh: `.merchant buy potion 5`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_list_follows_the_js_table_order_and_prices() {
        let t = text(run(&["list"], serde_json::json!({})));
        let pos = |s: &str| t.find(s).unwrap_or_else(|| panic!("{s} hilang"));
        assert!(pos("🧪 Potion") < pos("💎 Berlian"), "{t}");
        assert!(t.contains("> 💵 Beli: 2,000\n> 💰 Jual: 1,000\n> 📝 Material mewah\n> → `diamond`\n"));
        assert!(!t.contains("> → ``"), "kunci tidak boleh kosong");
    }

    #[test]
    fn buying_moves_money_and_stacks_the_item() {
        let r = run(&["buy", "potion", "3"], serde_json::json!({"koin": 500, "inventory": {"potion": 2}}));
        let d = written(&r);
        assert_eq!(d["koin"], serde_json::json!(200), "500 - 3*100");
        assert_eq!(d["inventory"]["potion"], serde_json::json!(5), "2 + 3");
        assert!(text_of(&r).contains("> 💵 Total: *-300*"), "{}", text_of(&r));
    }

    #[test]
    fn a_short_purse_buys_nothing() {
        let r = run(&["buy", "potion", "3"], serde_json::json!({"koin": 299}));
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ʙᴀʟᴀɴᴄᴇ ᴋᴜʀᴀɴɢ*"), "{t}");
        assert!(t.contains("> Harga: 300"), "{t}");
        assert!(t.contains("> Balance: 299"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn selling_the_last_copy_removes_the_key_entirely() {
        let r = run(&["sell", "iron", "2"], serde_json::json!({"koin": 0, "inventory": {"iron": 2, "wood": 9}}));
        let d = written(&r);
        let inv = d["inventory"].as_object().unwrap();
        assert!(!inv.contains_key("iron"), "sisa 0 dihapus: {inv:?}");
        assert_eq!(inv["wood"], serde_json::json!(9), "item lain tidak boleh tersentuh");
        assert_eq!(d["koin"], serde_json::json!(80), "2 * sellPrice 40");
    }

    #[test]
    fn selling_a_partial_stack_keeps_the_remainder() {
        let r = run(&["sell", "iron", "1"], serde_json::json!({"inventory": {"iron": 5}}));
        assert_eq!(written(&r)["inventory"]["iron"], serde_json::json!(4));
        assert_eq!(written(&r)["koin"], serde_json::json!(40));
    }

    #[test]
    fn selling_more_than_you_own_is_refused() {
        let r = run(&["sell", "iron", "3"], serde_json::json!({"inventory": {"iron": 2}}));
        let t = text_of(&r);
        assert!(t.starts_with("❌ *ɪᴛᴇᴍ ᴋᴜʀᴀɴɢ*"), "{t}");
        assert!(t.contains("> Punya: 2\n> Mau jual: 3"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_two_unknown_item_replies_differ_between_buy_and_sell() {
        let buy = text(run(&["buy", "bintang"], serde_json::json!({"koin": 9999})));
        assert!(buy.contains("Item tidak ditemukan!"), "{buy}");
        let sell = text(run(&["sell", "bintang"], serde_json::json!({"koin": 9999})));
        assert_eq!(sell, "❌ Item tidak bisa dijual ke merchant!");
    }

    #[test]
    fn the_quantity_defaults_to_one_and_never_goes_below_it() {
        for args in [
            vec!["buy", "potion"],
            vec!["buy", "potion", "abc"],
            vec!["buy", "potion", "0"],
            vec!["buy", "potion", "-7"],
        ] {
            let r = run(&args, serde_json::json!({"koin": 10_000}));
            assert_eq!(written(&r)["inventory"]["potion"], serde_json::json!(1), "{args:?}");
            assert_eq!(written(&r)["koin"], serde_json::json!(9900), "{args:?}");
        }
    }
}
