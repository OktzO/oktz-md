//! `plugins/rpg/treasure.js` — buka treasure chest untuk hadiah random.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//! `toLocaleString()` tanpa locale memakai koma (default Node `en-US`).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, inventory_map, math_random, or_default, roll_excl, roll_incl, rpg_map, thousands_en,
};

/// `CHEST_TYPES` (`treasure.js:20-26`) — `(kunci, nama, emas min, emas max,
/// exp min, exp max, rarity)`.
pub(crate) const CHESTS: [(&str, &str, i64, i64, i64, i64, &str); 5] = [
    ("woodenchest", "📦 Wooden Chest", 50, 200, 30, 80, "common"),
    ("ironchest", "🗃️ Iron Chest", 150, 500, 80, 150, "uncommon"),
    ("goldchest", "🎁 Gold Chest", 400, 1200, 150, 300, "rare"),
    ("diamondchest", "💎 Diamond Chest", 1000, 3000, 300, 600, "epic"),
    ("mysterybox", "🎲 Mystery Box", 500, 5000, 200, 800, "legendary"),
];

/// `(item, qty min, qty max, chance persen)`.
pub(crate) type LootEntry = (&'static str, i64, i64, f64);

/// `LOOT_TABLE` (`treasure.js:28-60`) — per rarity.
pub(crate) const LOOT: [(&str, &[LootEntry]); 5] = [
    (
        "common",
        &[
            ("wood", 3, 8, 40.0),
            ("iron", 1, 4, 30.0),
            ("herb", 2, 5, 25.0),
            ("potion", 1, 2, 20.0),
        ],
    ),
    (
        "uncommon",
        &[
            ("iron", 3, 7, 40.0),
            ("gold", 1, 3, 25.0),
            ("leather", 2, 5, 30.0),
            ("potion", 2, 4, 35.0),
        ],
    ),
    (
        "rare",
        &[
            ("gold", 2, 5, 45.0),
            ("diamond", 1, 2, 20.0),
            ("manapotion", 1, 3, 30.0),
            ("strengthpotion", 1, 1, 15.0),
        ],
    ),
    (
        "epic",
        &[
            ("diamond", 2, 4, 40.0),
            ("gold", 5, 10, 50.0),
            ("elixir", 1, 1, 15.0),
            ("dragonscale", 1, 2, 10.0),
        ],
    ),
    (
        "legendary",
        &[
            ("diamond", 3, 8, 50.0),
            ("titancore", 1, 2, 20.0),
            ("divinecore", 1, 1, 10.0),
            ("elixir", 1, 3, 25.0),
            ("goldsword", 1, 1, 5.0),
        ],
    ),
];

pub(crate) fn find_chest(key: &str) -> Option<&'static (&'static str, &'static str, i64, i64, i64, i64, &'static str)> {
    CHESTS.iter().find(|c| c.0 == key)
}

/// `LOOT_TABLE[rarity] || LOOT_TABLE.common`.
pub(crate) fn loot_of(rarity: &str) -> &'static [(&'static str, i64, i64, f64)] {
    LOOT.iter()
        .find(|l| l.0 == rarity)
        .map(|l| l.1)
        .unwrap_or(LOOT[0].1)
}

#[plugin(
    name = "treasure",
    alias = ["chest", "peti", "openbox"],
    category = "rpg",
    description = "Buka treasure chest untuk hadiah random",
    usage = ".treasure",
    example = ".treasure",
    cooldown = 30,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn treasure(ctx: Ctx) -> Reply {
    let mut inv = inventory_map(&ctx);
    let chest_key = ctx.args.first().map(|a| a.to_lowercase());
    let qty = |key: &str| inv.get(key).and_then(Value::as_i64).unwrap_or(0);

    let Some(key) = chest_key else {
        let owned: Vec<&(&str, &str, i64, i64, i64, i64, &str)> =
            CHESTS.iter().filter(|c| qty(c.0) > 0).collect();
        let mut txt = String::from("🎁 *GUDANG HARTA KARUN* 🎁\n\n");
        if owned.is_empty() {
            txt += "Yah kak, kamu belum punya peti harta satupun nih... 😭\n\n\
                    💡 *Tips dapet peti:*\n\
                    > ⚔️ Eksplorasi `.adventure` / Dungeon\n\
                    > 👹 Bunuh Boss\n\
                    > 🗓️ Selesaikan `.daily` / `.weekly`\n\
                    > 🛒 Beli di `.shop`";
        } else {
            txt += "Wah peti kamu ada banyak nih! Mau buka yang mana kak?\n\n";
            for (k, name, ..) in owned {
                txt += &format!(
                    "📦 {name}: *{} pcs*\n   └ Buka: `{}treasure {k}`\n\n",
                    qty(k),
                    ctx.prefix
                );
            }
        }
        return Reply::text(txt);
    };

    let Some(&(_, name, min_gold, max_gold, min_exp, max_exp, rarity)) = find_chest(&key) else {
        return Reply::text(format!("Peti *{key}* nggak ada di database kak!"));
    };

    let have = qty(&key);
    if have < 1 {
        return Reply::text(format!("Peti *{name}* kamu lagi kosong kak! 😅"));
    }

    let left = have - 1;
    if left <= 0 {
        inv.remove(key.as_str());
    } else {
        inv.insert(key.clone(), serde_json::json!(left));
    }

    // Dua undian punya ujung atas EKSKLUSIF di JS (`(max - min)`), bukan inklusif.
    let gold = roll_excl(min_gold, max_gold);
    let exp_reward = roll_excl(min_exp, max_exp);

    let mut dropped: Vec<String> = Vec::new();
    for (item, qmin, qmax, chance) in loot_of(rarity).iter() {
        if math_random() * 100.0 < *chance {
            let q = roll_incl(*qmin, *qmax);
            let slot = inv.entry((*item).to_string()).or_insert(serde_json::json!(0));
            let prev = slot.as_i64().unwrap_or(0);
            *slot = serde_json::json!(prev + q);
            dropped.push(format!("{item} (x{q})"));
        }
    }

    let koin = int_field(&ctx, "koin") + gold;
    let mut rpg = rpg_map(&ctx);
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

    let mut txt = format!(
        "💥 *CRAAASH!! PETI TERBUKA!!* 💥\n\n\
         Wah hebat kak! Kamu dapet harta dari *{name}*:\n\n\
         💰 Emas: *+Rp {}*\n\
         ✨ EXP: *+{exp_reward}*\n",
        thousands_en(gold)
    );
    if dropped.is_empty() {
        txt += "🎒 *Loot Tambahan:* _Aduh sayang banget nggak dapet item tambahan..._\n";
    } else {
        txt += "🎒 *Loot Tambahan:*\n";
        for item in &dropped {
            txt += &format!("  • {item}\n");
        }
    }
    if matches!(rarity, "legendary" | "epic") {
        txt += "\n> _\"HOKI BANGET KAK!\"_ 🌟🔥";
    }

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin, "exp": new_exp, "rpg": rpg, "inventory": inv }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "treasure".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("treasure").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("treasure").unwrap();
        assert_eq!(p.meta.name, "treasure");
        assert_eq!(p.meta.alias, ["chest", "peti", "openbox"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Buka treasure chest untuk hadiah random");
        assert_eq!(p.meta.usage, ".treasure");
        assert_eq!(p.meta.example, ".treasure");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["chest", "peti", "openbox"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "treasure");
        }
    }

    #[test]
    fn an_empty_vault_lists_the_four_ways_to_fill_it() {
        let r = run(&[], serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("🎁 *GUDANG HARTA KARUN*"), "{t}");
        assert!(t.contains("kamu belum punya peti harta satupun"), "{t}");
        assert!(t.contains("`.adventure`") && t.contains("`.shop`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_vault_lists_only_the_chests_actually_owned() {
        let r = run(&[], serde_json::json!({"inventory": {"goldchest": 2, "woodenchest": 5, "mysterybox": 0}}));
        let t = text_of(&r);
        assert!(t.contains("📦 🎁 Gold Chest: *2 pcs*"), "{t}");
        assert!(t.contains("📦 📦 Wooden Chest: *5 pcs*"), "{t}");
        assert!(!t.contains("Mystery Box"), "stok 0 tidak ditampilkan: {t}");
        assert!(t.contains("`.treasure goldchest`"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_chest_and_an_empty_one_have_different_replies() {
        let unknown = text(run(&["emasbesar"], serde_json::json!({})));
        assert_eq!(unknown, "Peti *emasbesar* nggak ada di database kak!");

        let empty = text(run(&["goldchest"], serde_json::json!({"inventory": {"goldchest": 0}})));
        assert_eq!(empty, "Peti *🎁 Gold Chest* kamu lagi kosong kak! 😅");
    }

    #[test]
    fn opening_a_chest_pays_gold_and_exp_inside_the_chest_own_band() {
        for &(key, name, min_gold, max_gold, min_exp, max_exp, _) in CHESTS.iter() {
            for _ in 0..120 {
                let r = run(
                    &[key],
                    serde_json::json!({"koin": 0, "exp": 0, "inventory": {key: 1}}),
                );
                let d = written(&r);
                let t = text_of(&r);
                assert!(t.contains(&format!("dari *{name}*")), "{key}: {t}");
                let gold: i64 = t
                    .split("💰 Emas: *+Rp ")
                    .nth(1)
                    .and_then(|s| s.split('*').next())
                    .and_then(|s| s.replace(',', "").parse().ok())
                    .unwrap();
                let exp: i64 = t
                    .split("✨ EXP: *+")
                    .nth(1)
                    .and_then(|s| s.split('*').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap();
                assert!((min_gold..max_gold).contains(&gold), "{key} emas {gold}");
                assert!((min_exp..max_exp).contains(&exp), "{key} exp {exp}");
                assert_eq!(d["koin"], serde_json::json!(gold));
                assert_eq!(d["exp"], serde_json::json!(exp));
            }
        }
    }

    #[test]
    fn the_chest_itself_is_consumed_and_a_lone_copy_disappears() {
        let r = run(&["ironchest"], serde_json::json!({"inventory": {"ironchest": 1}}));
        let inv = written(&r)["inventory"].as_object().unwrap().clone();
        assert!(!inv.contains_key("ironchest"), "sisa 0 dihapus: {inv:?}");

        let r = run(&["ironchest"], serde_json::json!({"inventory": {"ironchest": 3}}));
        assert_eq!(written(&r)["inventory"]["ironchest"], serde_json::json!(2));
    }

    #[test]
    fn every_listed_loot_line_also_lands_in_the_bag() {
        for _ in 0..400 {
            let r = run(
                &["mysterybox"],
                serde_json::json!({"koin": 0, "exp": 0, "inventory": {"mysterybox": 1}}),
            );
            let t = text_of(&r);
            let inv = written(&r)["inventory"].as_object().unwrap().clone();
            for (item, qmin, qmax, chance) in loot_of("legendary") {
                let listed = t
                    .lines()
                    .find(|l| l.trim_start().starts_with(&format!("• {item} (x")))
                    .map(|l| {
                        l.trim()
                            .rsplit("x")
                            .next()
                            .unwrap()
                            .trim_end_matches(')')
                            .parse::<i64>()
                            .unwrap()
                    });
                match listed {
                    Some(q) => {
                        assert!((*qmin..=*qmax).contains(&q), "{item}: {q} di luar {qmin}..{qmax}");
                        assert!(inv.get(*item).and_then(Value::as_i64).unwrap_or(0) >= q);
                    }
                    None => assert!(
                        !inv.contains_key(*item),
                        "{item} ditulis tanpa baris (chance {chance})"
                    ),
                }
            }
        }
    }

    #[test]
    fn the_high_roll_banner_only_shows_for_epic_and_legendary_chests() {
        for &(key, _, _, _, _, _, rarity) in CHESTS.iter() {
            let loud = matches!(rarity, "epic" | "legendary");
            for _ in 0..60 {
                let r = run(&[key], serde_json::json!({"koin": 0, "exp": 0, "inventory": {key: 1}}));
                assert_eq!(
                    text_of(&r).contains("HOKI BANGET KAK!"),
                    loud,
                    "{key} ({rarity})"
                );
            }
        }
    }

    #[test]
    fn a_run_without_any_loot_says_so_explicitly() {
        let mut seen = false;
        for _ in 0..3000 {
            let r = run(
                &["woodenchest"],
                serde_json::json!({"koin": 0, "exp": 0, "inventory": {"woodenchest": 1}}),
            );
            let t = text_of(&r);
            if t.contains("nggak dapet item tambahan") {
                seen = true;
                let listed = t.lines().filter(|l| l.trim_start().starts_with("• ")).count();
                assert_eq!(listed, 0, "{t}");
                break;
            }
        }
        assert!(seen, "pasti ada sampl dengan loot kosong");
    }
}
