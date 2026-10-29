//! `plugins/rpg/alchemy.js` — meracik ramuan dari bahan di `inventory`.
//!
//! `ponytail:` `user.inventory[material] -= needed` lalu `delete` kalau
//! `<= 0`, jadi bahan habis HILANG dari tas — bukan disimpan sebagai 0 seperti
//! `sellall`. Itu yang bikin `sellall` dan `alchemy` tidak bisa saling
//! menimpa.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{apply_exp, inventory_map, or_default, rpg_map};

/// `(key, name, materials, effect, exp, result)` — urutan object literal =
/// urutan tampil di buku resep.
type Potion = (
    &'static str,
    &'static str,
    &'static [(&'static str, i64)],
    &'static str,
    i64,
    &'static str,
);

const POTIONS: [Potion; 9] = [
    ("healthpotion", "❤️ Health Potion", &[("herb", 3)], "Pulihkan 50 HP", 80, "healthpotion"),
    ("manapotion", "💙 Mana Potion", &[("herb", 2), ("flower", 1)], "Pulihkan 50 Mana", 90, "manapotion"),
    ("staminapotion", "⚡ Stamina Potion", &[("herb", 2), ("mushroom", 1)], "Pulihkan 30 Stamina", 100, "staminapotion"),
    ("strengthpotion", "💪 Strength Potion", &[("herb", 3), ("dragonscale", 1)], "+20 ATK (5 menit)", 200, "strengthpotion"),
    ("defensepotion", "🛡️ Defense Potion", &[("herb", 3), ("iron", 2)], "+15 DEF (5 menit)", 180, "defensepotion"),
    ("luckpotion", "🍀 Luck Potion", &[("herb", 5), ("diamond", 1)], "+30% Drop Rate (10 menit)", 300, "luckpotion"),
    ("exppotion", "✨ EXP Potion", &[("herb", 4), ("gold", 2)], "+50% EXP (15 menit)", 250, "exppotion"),
    ("antidote", "💊 Antidote", &[("herb", 2)], "Sembuhkan racun", 50, "antidote"),
    ("elixir", "🧪 Elixir", &[("herb", 10), ("diamond", 2), ("gold", 5)], "Pulihkan semua stats", 500, "elixir"),
];

fn potion(key: &str) -> Option<&'static Potion> {
    POTIONS.iter().find(|(k, ..)| *k == key)
}

fn have(inv: &Map<String, Value>, material: &str) -> i64 {
    inv.get(material).and_then(Value::as_i64).unwrap_or(0)
}

/// `Object.entries(pot.materials).map(([m, qty]) => `${qty}x ${m}`)`.
fn materials_line(materials: &[(&'static str, i64)]) -> String {
    materials
        .iter()
        .map(|(m, qty)| format!("{qty}x {m}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn book() -> String {
    let mut txt = String::from(
        "Halo Alchemist! Mau ngeracik ramuan apa hari ini? 🧙‍♂️🧪\n\n\
         *Daftar Buku Resep Ramuan:*\n",
    );
    for (key, name, materials, effect, _, _) in POTIONS {
        txt.push_str(&format!(
            "\n*{name}*\n📦 Bahan: {}\n💫 Efek: {effect}\n👉 Ketik: `.alchemy {key}`\n",
            materials_line(materials)
        ));
    }
    txt.push_str("\n💡 *Tips:* Bahan herba bisa kamu dapetin dari `.garden` atau `.dungeon`! 🌱");
    txt
}

/// `missingMaterials` (`alchemy.js:119-125`) — bahan yang masih kurang,
/// dalam urutan object literal.
fn missing(inv: &Map<String, Value>, materials: &[(&'static str, i64)]) -> Vec<String> {
    materials
        .iter()
        .filter(|(material, needed)| have(inv, material) < *needed)
        .map(|(material, needed)| format!("• {material}: {}/{}", have(inv, material), needed))
        .collect()
}

#[plugin(
    name = "alchemy",
    alias = ["potion", "brew", "ramuan"],
    category = "rpg",
    description = "Buat potion dan ramuan dari herba",
    usage = ".alchemy <potion>",
    example = ".alchemy healthpotion",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn alchemy(ctx: Ctx) -> Reply {
    let mut inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    let Some(key) = crate::common::first_keyword(&ctx) else {
        return Reply::text(book());
    };

    let Some(&(_, name, materials, effect, exp, result)) = potion(&key) else {
        return Reply::text(
            "Waduh, itu racikan berbahaya kak! Resepnya nggak ada di buku! 😂\n\
             Cek list yang bener pake `.alchemy` ya!",
        );
    };

    let short = missing(&inv, materials);
    if !short.is_empty() {
        return Reply::text(format!(
            "Eits, bahannya belum cukup buat ngeracik *{name}* nih! 😭\n\n\
             Kekurangannya:\n{}\n\n\
             Kumpulin herba dulu deh! 🏃💨",
            short.join("\n")
        ));
    }

    for (material, needed) in materials {
        let left = have(&inv, material) - needed;
        if left <= 0 {
            inv.remove(*material);
        } else {
            inv.insert((*material).to_string(), serde_json::json!(left));
        }
    }
    inv.insert(result.to_string(), serde_json::json!(have(&inv, result) + 1));

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp);

    Reply::text(format!(
        "CLLINGGG!! RAMUAN BERHASIL DIBUAT! 🎉🧪\n\n\
         Kamu berhasil meracik:\n\
         📦 Item: *{name}*\n\
         💫 Efek: *{effect}*\n\
         📈 EXP Alchemy: *+{exp}*\n\n\
         Jangan diminum sekaligus kalau nggak mau sakit perut! 😂"
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "alchemy".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("alchemy").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data.clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("alchemy").unwrap();
        assert_eq!(p.meta.name, "alchemy");
        assert_eq!(p.meta.alias, ["potion", "brew", "ramuan"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Buat potion dan ramuan dari herba");
        assert_eq!(p.meta.usage, ".alchemy <potion>");
        assert_eq!(p.meta.example, ".alchemy healthpotion");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["potion", "brew", "ramuan"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "alchemy");
        }
    }

    #[test]
    fn without_an_argument_the_whole_recipe_book_is_listed_in_order() {
        let t_reply = run(serde_json::json!({}), "");
        let t = text_of(&t_reply);
        assert!(t.starts_with("Halo Alchemist!"), "{t}");
        assert!(t.ends_with("dari `.garden` atau `.dungeon`! 🌱"), "{t}");
        for (key, name, ..) in POTIONS {
            assert!(t.contains(&format!("👉 Ketik: `.alchemy {key}`")), "{key} hilang");
            assert!(t.contains(&format!("*{name}*")), "{key} hilang");
        }
        let pos = |n: &str| t.find(n).expect("resep hilang");
        assert!(pos("Health Potion") < pos("Mana Potion"));
        assert!(pos("Mana Potion") < pos("Elixir"));
    }

    #[test]
    fn an_unknown_recipe_name_is_refused_and_nothing_is_written() {
        for bad in ["sword", "POTION", "health", "healthpotionn"] {
            let r = run(serde_json::json!({ "inventory": { "herb": 99 } }), bad);
            assert!(text_of(&r).starts_with("Waduh, itu racikan berbahaya"), "arg {bad}");
            assert!(r.mutations().is_empty(), "arg {bad}");
        }
    }

    #[test]
    fn a_recipe_name_is_case_insensitive() {
        let r = run(serde_json::json!({ "inventory": { "herb": 3 } }), "HeAlThPoTiOn");
        assert!(text_of(&r).contains("CLLINGGG"), "{}", text_of(&r));
    }

    /// `user.inventory[material] -= needed; if (<= 0) delete` — bahan habis
    /// hilang dari tas, bukan jadi 0.
    #[test]
    fn a_spent_material_disappears_from_the_bag_while_a_surplus_stays() {
        let r = run(serde_json::json!({ "inventory": { "herb": 4, "gold": 2 }, "exp": 0 }), "exppotion");
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert!(!inv.contains_key("herb"), "4 - 4 = 0 -> dihapus");
        assert!(!inv.contains_key("gold"), "2 - 2 = 0 -> dihapus");
        assert_eq!(inv["exppotion"], serde_json::json!(1));
    }

    #[test]
    fn a_partly_spent_material_keeps_the_remainder() {
        let r = run(serde_json::json!({ "inventory": { "herb": 10, "gold": 4 } }), "exppotion");
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["herb"], serde_json::json!(6), "10 - 4");
        assert_eq!(inv["gold"], serde_json::json!(2), "4 - 2");
    }

    #[test]
    fn every_missing_material_is_listed_with_have_over_need() {
        let r = run(serde_json::json!({ "inventory": { "herb": 1, "gold": 0 } }), "elixir");
        let t = text_of(&r);
        assert!(t.starts_with("Eits, bahannya belum cukup"), "{t}");
        for line in ["• herb: 1/10", "• gold: 0/5", "• diamond: 0/2"] {
            assert!(t.contains(line), "{line} hilang dari {t}");
        }
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_exact_stock_is_enough_and_pays_the_recipe_exp() {
        let r = run(serde_json::json!({ "inventory": { "herb": 2 }, "exp": 0 }), "antidote");
        let data = write_of(&r);
        assert_eq!(data["exp"], serde_json::json!(50), "exp resep antidote");
        assert_eq!(data["inventory"]["antidote"], serde_json::json!(1));
    }

    #[test]
    fn the_result_stack_accumulates_across_brews() {
        let r = run(serde_json::json!({ "inventory": { "herb": 6 } }), "healthpotion");
        assert_eq!(write_of(&r)["inventory"]["healthpotion"], serde_json::json!(1));
    }

    #[test]
    fn the_table_keeps_every_js_row() {
        assert_eq!(POTIONS.len(), 9);
        assert_eq!(POTIONS[0].1, "❤️ Health Potion");
        assert_eq!(POTIONS[8].1, "🧪 Elixir");
        assert_eq!(POTIONS[8].2, [("herb", 10), ("diamond", 2), ("gold", 5)]);
        assert_eq!(potion("elixir").unwrap().4, 500);
        assert!(potion("tidak-ada").is_none());
    }
}