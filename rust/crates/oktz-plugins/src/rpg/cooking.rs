//! `plugins/rpg/cooking.js` — masak resep yang DIMINTA (berbeda dari `cook`
//! yang otomatis memilih).
//!
//! Batas efek mengikuti level: `maxHealth = 100 + level * 5` dan
//! `maxMana = 50 + level * 3` — bukan `+ (level - 1)` seperti `level.js`.
//! Itu tidak konsisten, tapi itu yang ditulis plugin ini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::rpg::heal::{apply_exp, inventory_map, or_default, rpg_map, user_level};

/// `(stamina, health, mana)` — 0 berarti efek itu tidak ada.
type Effect = (i64, i64, i64);

/// `(key, name, materials, effect, exp)`.
type Recipe = (
    &'static str,
    &'static str,
    &'static [(&'static str, i64)],
    Effect,
    i64,
);

const RECIPES: [Recipe; 10] = [
    ("bread", "🍞 Roti", &[("wheat", 2)], (10, 5, 0), 30),
    ("friedrice", "🍚 Nasi Goreng", &[("rice", 2), ("egg", 1)], (25, 15, 0), 60),
    ("steak", "🥩 Steak", &[("meat", 2), ("herb", 1)], (40, 30, 0), 100),
    ("soup", "🍲 Sup", &[("carrot", 2), ("potato", 2), ("meat", 1)], (35, 40, 0), 90),
    ("sushi", "🍣 Sushi", &[("fish", 3), ("rice", 2)], (30, 25, 0), 80),
    ("cake", "🍰 Kue", &[("wheat", 3), ("egg", 2), ("strawberry", 2)], (50, 20, 0), 120),
    ("ramen", "🍜 Ramen", &[("wheat", 2), ("egg", 1), ("meat", 1), ("herb", 1)], (45, 35, 0), 110),
    ("pizza", "🍕 Pizza", &[("wheat", 3), ("tomato", 2), ("meat", 2)], (60, 30, 0), 140),
    ("smoothie", "🥤 Smoothie", &[("strawberry", 3), ("watermelon", 1)], (30, 0, 20), 70),
    ("elixir_food", "✨ Elixir Food", &[("herb", 5), ("diamond", 1), ("gold", 2)], (100, 100, 50), 300),
];

fn recipe(key: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|(k, ..)| *k == key)
}

fn have(inv: &Map<String, Value>, item: &str) -> i64 {
    inv.get(item).and_then(Value::as_i64).unwrap_or(0)
}

fn materials_line(materials: &[(&'static str, i64)]) -> String {
    materials
        .iter()
        .map(|(m, qty)| format!("{qty}x {m}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `Object.entries(recipe.effect)` — urutan pasangan = urutan field di JS.
fn effect_lines(effect: Effect) -> Vec<(&'static str, i64)> {
    [("STAMINA", effect.0), ("HEALTH", effect.1), ("MANA", effect.2)]
        .into_iter()
        .filter(|(_, v)| *v != 0)
        .collect()
}

fn book() -> String {
    let mut txt = String::from(
        "Halo Master Chef! Mau masak resep yang mana nih? 👨‍🍳🍳\n\n\
         *Daftar Resep Rahasia:*\n",
    );
    for (key, name, materials, effect, _) in RECIPES {
        let effects = effect_lines(effect)
            .iter()
            .map(|(k, v)| format!("+{v} {}", k.to_lowercase()))
            .collect::<Vec<_>>()
            .join(", ");
        txt.push_str(&format!(
            "\n*{name}*\n📦 Bahan: {}\n💫 Efek: {effects}\n👉 Ketik: `.cooking {key}`\n",
            materials_line(materials)
        ));
    }
    txt
}

fn missing(inv: &Map<String, Value>, materials: &[(&'static str, i64)]) -> Vec<String> {
    materials
        .iter()
        .filter(|(item, qty)| have(inv, item) < *qty)
        .map(|(item, qty)| format!("• {item}: {}/{}", have(inv, item), qty))
        .collect()
}

#[plugin(
    name = "cooking",
    alias = ["chef"],
    category = "rpg",
    description = "Masak makanan untuk stamina dan HP",
    usage = ".cooking <recipe>",
    example = ".cooking friedrice",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn cooking(ctx: Ctx) -> Reply {
    let mut inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    let Some(key) = crate::common::first_keyword(&ctx) else {
        return Reply::text(book());
    };

    let Some(&(_, name, materials, effect, exp)) = recipe(&key) else {
        return Reply::text(
            "Hayo, resep apaan tuh? Nggak ada di buku menu kak! 😂\n\
             Cek daftar resepnya pake `.cooking` ya!",
        );
    };

    let short = missing(&inv, materials);
    if !short.is_empty() {
        return Reply::text(format!(
            "Eits, bahan kamu belum lengkap buat masak *{name}*! 😭\n\n\
             Yang kurang:\n{}\n\n\
             Cari bahannya dulu gih! 🛒🏃",
            short.join("\n")
        ));
    }

    for (item, qty) in materials {
        let left = have(&inv, item) - qty;
        if left <= 0 {
            inv.remove(*item);
        } else {
            inv.insert((*item).to_string(), json!(left));
        }
    }

    let level = user_level(&ctx);
    let (max_stamina, max_health, max_mana) = (100, 100 + level * 5, 50 + level * 3);
    let (gain_stamina, gain_health, gain_mana) = effect;

    if gain_stamina != 0 {
        let cur = or_default(rpg.get("stamina").and_then(Value::as_i64), 100);
        rpg.insert("stamina".into(), json!((cur + gain_stamina).min(max_stamina)));
    }
    if gain_health != 0 {
        let cur = or_default(rpg.get("health").and_then(Value::as_i64), 100);
        rpg.insert("health".into(), json!((cur + gain_health).min(max_health)));
    }
    if gain_mana != 0 {
        let cur = or_default(rpg.get("mana").and_then(Value::as_i64), 50);
        rpg.insert("mana".into(), json!((cur + gain_mana).min(max_mana)));
    }

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp);

    let effect_text = effect_lines(effect)
        .iter()
        .map(|(k, v)| format!("• {k}: +{v}"))
        .collect::<Vec<_>>()
        .join("\n");

    Reply::text(format!(
        "TADAA! MASAKAN MATANG! 🍽️✨\n\n\
         Kamu langsung melahap *{name}* yang enak banget!\n\
         Efek yang kamu dapet:\n\
         {effect_text}\n\n\
         📈 Bonus EXP Masak: *+{exp}*\n\n\
         Kenyang banget rasanya, siap tempur lagi! 🔥"
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "inventory": inv, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cooking".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("cooking").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("cooking").unwrap();
        assert_eq!(p.meta.name, "cooking");
        assert_eq!(p.meta.alias, ["chef"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Masak makanan untuk stamina dan HP");
        assert_eq!(p.meta.usage, ".cooking <recipe>");
        assert_eq!(p.meta.example, ".cooking friedrice");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("chef").unwrap().meta.name, "cooking");
    }

    #[test]
    fn without_an_argument_the_whole_recipe_book_is_listed() {
        let t_reply = run(json!({}), "");
        let t = text_of(&t_reply);
        assert!(t.starts_with("Halo Master Chef!"), "{t}");
        for (key, name, ..) in RECIPES {
            assert!(t.contains(&format!("👉 Ketik: `.cooking {key}`")), "{key} hilang");
            assert!(t.contains(&format!("*{name}*")), "{key} hilang");
        }
        assert!(t.contains("💫 Efek: +10 stamina, +5 health"), "{t}");
        assert!(t.contains("💫 Efek: +30 stamina, +20 mana"), "smoothie tak punya health: {t}");
        let pos = |n: &str| t.find(n).expect("resep hilang");
        assert!(pos("🍞 Roti") < pos("🍕 Pizza"));
        assert!(pos("🍕 Pizza") < pos("Elixir Food"));
    }

    #[test]
    fn an_unknown_recipe_is_refused_and_nothing_is_written() {
        for bad in ["nasi", "PIZZA!", "fried_rice"] {
            let r = run(json!({ "inventory": { "wheat": 99 } }), bad);
            assert!(text_of(&r).starts_with("Hayo, resep apaan tuh"), "arg {bad}");
            assert!(r.mutations().is_empty(), "arg {bad}");
        }
    }

    #[test]
    fn a_recipe_name_is_case_insensitive() {
        let r = run(json!({ "inventory": { "wheat": 2 }, "exp": 0 }), "BREAD");
        assert!(text_of(&r).starts_with("TADAA!"), "{}", text_of(&r));
    }

    #[test]
    fn every_short_material_is_listed_with_have_over_need() {
        let r = run(json!({ "inventory": { "wheat": 2, "egg": 0 } }), "cake");
        let t = text_of(&r);
        assert!(t.starts_with("Eits, bahan kamu belum lengkap"), "{t}");
        assert!(t.contains("• egg: 0/2"), "{t}");
        assert!(t.contains("• strawberry: 0/2"), "{t}");
        assert!(t.contains("• wheat: 2/3"), "wheat juga kurang: {t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_depleted_material_leaves_the_bag_and_a_surplus_stays() {
        let r = run(json!({ "inventory": { "rice": 3, "egg": 1 }, "exp": 0 }), "friedrice");
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["rice"], json!(1), "3 - 2");
        assert!(!inv.contains_key("egg"), "1 - 1 = 0 -> dihapus");
    }

    #[test]
    fn each_effect_stat_is_written_with_its_own_ceiling() {
        // smoothie: stamina 30, mana 20, health TIDAK disentuh.
        let r = run(
            json!({ "inventory": { "strawberry": 3, "watermelon": 1 },
                    "rpg": { "stamina": 80, "mana": 40, "health": 33 }, "exp": 0 }),
            "smoothie",
        );
        let rpg = write_of(&r)["rpg"].clone();
        assert_eq!(rpg["stamina"], json!(100), "80 + 30, mentok 100");
        assert_eq!(rpg["mana"], json!(53), "40 + 20, mentok 50 + 1*3 = 53");
        assert_eq!(rpg["health"], json!(33), "smoothie tak punya efek health");
    }

    /// `maxHealth = 100 + userLevel * 5` — BUKAN `100 + (level - 1) * 10`
    /// seperti `level.js`. Level 1 → 105, bukan 100.
    #[test]
    fn the_health_ceiling_scales_with_level() {
        let r = run(
            json!({ "level": 10, "inventory": { "meat": 2, "herb": 1 },
                    "rpg": { "health": 1 }, "exp": 0 }),
            "steak",
        );
        assert_eq!(write_of(&r)["rpg"]["health"], json!(31), "1 + 30, mentok 100 + 10*5 = 150");
    }

    #[test]
    fn the_reply_lists_only_the_effects_that_recipe_actually_has() {
        let r = run(json!({ "inventory": { "wheat": 2 }, "exp": 0 }), "bread");
        let t = text_of(&r);
        assert!(t.contains("• STAMINA: +10"), "{t}");
        assert!(t.contains("• HEALTH: +5"), "{t}");
        assert!(!t.contains("MANA"), "bread tak punya efek mana: {t}");
        assert!(t.contains("📈 Bonus EXP Masak: *+30*"), "{t}");
    }

    #[test]
    fn cooking_pays_the_recipe_exp() {
        let r = run(json!({ "inventory": { "herb": 5, "diamond": 1, "gold": 2 }, "exp": 0 }), "elixir_food");
        assert_eq!(write_of(&r)["exp"], json!(300));
    }

    #[test]
    fn the_recipe_table_keeps_every_js_row() {
        assert_eq!(RECIPES.len(), 10);
        let want: Recipe = ("bread", "🍞 Roti", &[("wheat", 2)], (10, 5, 0), 30);
        assert_eq!(RECIPES[0], want);
        let want: Recipe = ("elixir_food", "✨ Elixir Food",
            &[("herb", 5), ("diamond", 1), ("gold", 2)], (100, 100, 50), 300);
        assert_eq!(RECIPES[9], want);
        assert!(recipe("tidak-ada").is_none());
    }
}