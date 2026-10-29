//! `plugins/rpg/cook.js` — masak resep PERTAMA yang bahannya cukup, bukan
//! yang diminta.
//!
//! Kursor: bahan TIDAK dihapus dari tas kalau habis (berbeda dari `alchemy`
//! yang `delete`), dan `user.rpg.health || 100` memakai `||` — nol jadi 100.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{inventory_map, or_default, rpg_int, rpg_map};

/// `(key, name, materials, heal)` — urutan object literal = urutan pencarian.
type Recipe = (
    &'static str,
    &'static str,
    &'static [(&'static str, i64)],
    i64,
);

const RECIPES: [Recipe; 4] = [
    ("fish_soup", "🍲 Sup Ikan", &[("fish", 2)], 30),
    ("grilled_meat", "🍖 Daging Panggang", &[("rabbit", 1), ("wood", 1)], 40),
    ("apple_pie", "🥧 Pie Apel", &[("apple", 3)], 25),
    ("steak", "🥩 Steak", &[("boar", 1), ("coal", 1)], 60),
];

fn have(inv: &Map<String, Value>, item: &str) -> i64 {
    or_default(inv.get(item).and_then(Value::as_i64), 0)
}

fn can_cook(inv: &Map<String, Value>, materials: &[(&'static str, i64)]) -> bool {
    materials
        .iter()
        .all(|(item, qty)| have(inv, item) >= *qty)
}

fn book(inv: &Map<String, Value>) -> String {
    let mut txt = String::from(
        "Halo Chef! Mau masak apa hari ini? 🍳👨‍🍳\n\n\
         Ini daftar resep yang bisa kamu bikin:\n\n",
    );
    for (_, name, materials, heal) in RECIPES {
        txt.push_str(&format!("*{name}*\n❤️ Heal: +{heal} HP\n📦 Bahan yang dibutuhin:\n"));
        for (item, qty) in materials {
            let owned = have(inv, item);
            txt.push_str(&format!(
                "• {} {item}: {owned}/{qty}\n",
                if owned >= *qty { "✅" } else { "❌" }
            ));
        }
        txt.push('\n');
    }
    txt.push_str("(Bot bakal otomatis masak resep pertama yang bahannya cukup!)");
    txt
}

#[plugin(
    name = "cook",
    alias = ["masak"],
    category = "rpg",
    description = "Memasak makanan untuk menambah health",
    usage = ".cook",
    example = ".cook",
    cooldown = 60,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn cook(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let mut inv = inventory_map(&ctx);

    // `user.rpg.health || 100` dan `maxHealth || 100` — `||`, bukan `??`.
    let health = or_default(rpg_int(&rpg, "health"), 100);
    let max_health = or_default(rpg_int(&rpg, "maxHealth"), 100);

    if health >= max_health {
        return Reply::text(
            "Perut kamu masih kenyang kak! 🤢\n\
             Nggak usah masak dulu, nanti kekenyangan malah susah jalan! 🏃💨",
        );
    }

    let Some(&(_, name, materials, heal)) = RECIPES.iter().find(|(_, _, m, _)| can_cook(&inv, m))
    else {
        return Reply::text(book(&inv));
    };

    for (item, qty) in materials {
        inv.insert((*item).to_string(), serde_json::json!(have(&inv, item) - qty));
    }
    let healed = (health + heal).min(max_health);
    rpg.insert("health".into(), serde_json::json!(healed));

    Reply::text(format!(
        "NYAM NYAM! Masakan Matang! 🍽️✨\n\n\
         Kamu langsung makan *{name}* dan ngerasa baikan!\n\
         ❤️ HP Pulih: {health} 📈 *{healed}*\n\n\
         Lanjut petualang lagi gass! 🚀🔥"
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cook".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("cook").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write_of(r: &Reply) -> Value {
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
        let p = oktz_plugin_api::find("cook").unwrap();
        assert_eq!(p.meta.name, "cook");
        assert_eq!(p.meta.alias, ["masak"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Memasak makanan untuk menambah health");
        assert_eq!(p.meta.usage, ".cook");
        assert_eq!(p.meta.example, ".cook");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("masak").unwrap().meta.name, "cook");
    }

    #[test]
    fn a_full_health_bar_blocks_the_kitchen() {
        for user in [
            serde_json::json!(null),
            serde_json::json!({}),
            serde_json::json!({ "rpg": { "health": 100, "maxHealth": 100 } }),
        ] {
            let r = run(user.clone());
            assert!(text_of(&r).starts_with("Perut kamu masih kenyang"), "input {user}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn without_a_full_bag_the_whole_recipe_book_is_listed() {
        let r = run(serde_json::json!({ "rpg": { "health": 10 }, "inventory": { "fish": 1 } }));
        let t = text_of(&r);
        assert!(t.starts_with("Halo Chef!"), "{t}");
        assert!(t.ends_with("(Bot bakal otomatis masak resep pertama yang bahannya cukup!)"), "{t}");
        for (_, name, _, heal) in RECIPES {
            assert!(t.contains(&format!("*{name}*\n❤️ Heal: +{heal} HP")), "{name} hilang");
        }
        assert!(t.contains("❌ fish: 1/2"), "1 ikan masih kurang dari 2: {t}");
        assert!(t.contains("❌ rabbit: 0/1"), "{t}");
        assert!(r.mutations().is_empty());
    }

    /// Resep PERTAMA yang bisa dibuat menang — `Object.entries` lalu `break`.
    #[test]
    fn the_first_cookable_recipe_wins_even_when_a_healthier_one_is_possible() {
        let r = run(serde_json::json!({
            "rpg": { "health": 10 },
            "inventory": { "fish": 2, "rabbit": 1, "wood": 1, "boar": 1, "coal": 1 },
        }));
        let t = text_of(&r);
        assert!(t.contains("*🍲 Sup Ikan*"), "fish_soup ada duluan: {t}");
        assert!(!t.contains("🥩 Steak"), "steak tak boleh dipilih duluan: {t}");
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["fish"], serde_json::json!(0), "2 - 2");
        assert_eq!(inv["boar"], serde_json::json!(1), "bahan steak tak boleh tersentuh");
    }

    #[test]
    fn the_recipe_book_marks_each_material_with_a_checkbox() {
        let r = run(serde_json::json!({
            "rpg": { "health": 10 },
            "inventory": { "fish": 1, "apple": 2 },
        }));
        let t = text_of(&r);
        assert!(t.contains("❌ fish: 1/2"), "{t}");
        assert!(t.contains("❌ apple: 2/3"), "{t}");
        assert!(t.contains("❌ wood: 0/1"), "{t}");
    }

    #[test]
    fn healing_stops_at_the_stored_maximum() {
        let r = run(serde_json::json!({
            "rpg": { "health": 90, "maxHealth": 100 },
            "inventory": { "fish": 2 },
        }));
        let t = text_of(&r);
        assert!(t.contains("❤️ HP Pulih: 90 📈 *100*"), "30 HP tapi mentok 100: {t}");
        assert_eq!(write_of(&r)["rpg"]["health"], serde_json::json!(100));
    }

    /// Bahan habis TIDAK dihapus dari tas — `user.inventory[mat] -= qty` saja.
    #[test]
    fn a_depleted_material_stays_in_the_bag_as_zero() {
        let r = run(serde_json::json!({
            "rpg": { "health": 10 },
            "inventory": { "fish": 2 },
        }));
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["fish"], serde_json::json!(0));
        assert!(inv.contains_key("fish"), "kunci tak boleh hilang seperti alchemy");
    }

    #[test]
    fn a_partial_stack_keeps_the_remainder() {
        let r = run(serde_json::json!({
            "rpg": { "health": 10 },
            "inventory": { "fish": 5 },
        }));
        assert_eq!(write_of(&r)["inventory"]["fish"], serde_json::json!(3));
    }

    #[test]
    fn the_recipe_table_keeps_every_js_row() {
        assert_eq!(RECIPES.len(), 4);
        let want: Recipe = ("fish_soup", "🍲 Sup Ikan", &[("fish", 2)], 30);
        assert_eq!(RECIPES[0], want);
        let want: Recipe = ("steak", "🥩 Steak", &[("boar", 1), ("coal", 1)], 60);
        assert_eq!(RECIPES[3], want);
    }
}