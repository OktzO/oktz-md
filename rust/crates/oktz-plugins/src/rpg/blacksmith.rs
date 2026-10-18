//! `plugins/rpg/blacksmith.js` — tempa senjata/armor dari material.
//!
//! Bedanya dari `alchemy`: bahan HABIS dihapus dari tas (kalau `<= 0`), dan
//! satu resep (`arrow`) menghasilkan 10 buah sekaligus lewat `recipe.qty`.
//! `recipe.price` tidak pernah dipakai plugin ini — harga jual tetap milik
//! `sellall`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{apply_exp, inventory_map, or_default, rpg_map};

/// `(key, name, materials, result, exp, qty)` — `qty` = `recipe.qty || 1`.
type Recipe = (
    &'static str,
    &'static str,
    &'static [(&'static str, i64)],
    &'static str,
    i64,
    i64,
);

const RECIPES: [Recipe; 10] = [
    ("sword", "⚔️ Pedang Besi", &[("iron", 3), ("wood", 2)], "sword", 200, 1),
    ("shield", "🛡️ Perisai Besi", &[("iron", 4), ("leather", 2)], "shield", 250, 1),
    ("helmet", "⛑️ Helm Besi", &[("iron", 2), ("leather", 1)], "helmet", 150, 1),
    ("armor", "🦺 Armor Besi", &[("iron", 5), ("leather", 3)], "armor", 350, 1),
    ("axe", "🪓 Kapak Besi", &[("iron", 2), ("wood", 3)], "axe", 180, 1),
    ("pickaxe", "⛏️ Beliung", &[("iron", 3), ("wood", 2)], "pickaxe", 180, 1),
    ("bow", "🏹 Busur", &[("wood", 4), ("string", 2)], "bow", 200, 1),
    ("arrow", "🏹 Anak Panah x10", &[("wood", 1), ("iron", 1)], "arrow", 50, 10),
    ("goldsword", "🗡️ Pedang Emas", &[("gold", 5), ("diamond", 2), ("iron", 3)], "goldsword", 500, 1),
    ("diamondarmor", "💎 Armor Berlian", &[("diamond", 8), ("iron", 5), ("leather", 3)], "diamondarmor", 800, 1),
];

fn recipe(key: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|(k, ..)| *k == key)
}

fn have(inv: &Map<String, Value>, material: &str) -> i64 {
    inv.get(material).and_then(Value::as_i64).unwrap_or(0)
}

fn materials_line(materials: &[(&'static str, i64)]) -> String {
    materials
        .iter()
        .map(|(m, qty)| format!("{qty}x {m}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn book() -> String {
    let mut txt = String::from(
        "Halo petualang! Selamat datang di tempat Pandai Besi! 🔨⚒️\n\
         Ada yang bisa kubantu tempa hari ini?\n\n\
         *Daftar Senjata & Armor:*\n",
    );
    for (key, name, materials, _, exp, _) in RECIPES {
        txt.push_str(&format!(
            "\n*{name}*\n📦 Bahan: {}\n📈 EXP: +{exp}\n👉 Ketik: `.blacksmith {key}`\n",
            materials_line(materials)
        ));
    }
    txt.push_str("\n💡 *Tips:* Materialnya bisa dicari lewat `.mining` atau `.hunt` lho!");
    txt
}

fn missing(inv: &Map<String, Value>, materials: &[(&'static str, i64)]) -> Vec<String> {
    materials
        .iter()
        .filter(|(material, needed)| have(inv, material) < *needed)
        .map(|(material, needed)| format!("• {material}: {}/{}", have(inv, material), needed))
        .collect()
}

#[plugin(
    name = "blacksmith",
    alias = ["tempa", "forge", "pandai"],
    category = "rpg",
    description = "Tempa senjata dan armor dari material",
    usage = ".blacksmith <item>",
    example = ".blacksmith sword",
    cooldown = 120,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn blacksmith(ctx: Ctx) -> Reply {
    let mut inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    let Some(key) = crate::common::first_keyword(&ctx) else {
        return Reply::text(book());
    };

    let Some(&(_, name, materials, result, exp, qty)) = recipe(&key) else {
        return Reply::text(
            "Hadeh, panduan bikin apaan tuh? Nggak ada di catatanku kak! 😂\n\
             Cek list yang bener pake `.blacksmith` ya!",
        );
    };

    let short = missing(&inv, materials);
    if !short.is_empty() {
        return Reply::text(format!(
            "Eits, bahannya belum cukup buat nempa *{name}* nih! 😭\n\n\
             Kekurangannya:\n{}\n\n\
             Kumpulin dulu deh, baru balik ke sini! 🏃💨",
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
    inv.insert(result.to_string(), serde_json::json!(have(&inv, result) + qty));

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp);

    Reply::text(format!(
        "TEMPAAN BERHASIL KAK! ⚔️🛡️\n\n\
         Gila, hasilnya rapi banget! Ini barang buatan tangan kita:\n\
         🔨 Item: *{name}*\n\
         📊 Jumlah: *+{qty}*\n\
         📈 EXP Crafting: *+{exp}*\n\n\
         Siap buat dipake berantem nih! 😎🔥"
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
            command: "blacksmith".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("blacksmith").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("blacksmith").unwrap();
        assert_eq!(p.meta.name, "blacksmith");
        assert_eq!(p.meta.alias, ["tempa", "forge", "pandai"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Tempa senjata dan armor dari material");
        assert_eq!(p.meta.usage, ".blacksmith <item>");
        assert_eq!(p.meta.example, ".blacksmith sword");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tempa", "forge", "pandai"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "blacksmith");
        }
    }

    #[test]
    fn without_an_argument_the_whole_catalogue_is_listed_in_order() {
        let t_reply = run(serde_json::json!({}), "");
        let t = text_of(&t_reply);
        assert!(t.starts_with("Halo petualang!"), "{t}");
        assert!(t.ends_with("`.mining` atau `.hunt` lho!"), "{t}");
        for (key, name, materials, _, exp, _) in RECIPES {
            assert!(t.contains(&format!("👉 Ketik: `.blacksmith {key}`")), "{key} hilang");
            assert!(t.contains(&format!("*{name}*\n📦 Bahan: {}", materials_line(materials))), "{key} salah");
            assert!(t.contains(&format!("📈 EXP: +{exp}")), "{key} salah");
        }
        let pos = |n: &str| t.find(n).expect("resep hilang");
        assert!(pos("Pedang Besi") < pos("Pedang Emas"));
        assert!(pos("Pedang Emas") < pos("Armor Berlian"));
    }

    #[test]
    fn an_unknown_item_name_is_refused_and_nothing_is_written() {
        for bad in ["potion", "Sword!", "arrow2"] {
            let r = run(serde_json::json!({ "inventory": { "iron": 99 } }), bad);
            assert!(text_of(&r).starts_with("Hadeh, panduan bikin apaan tuh"), "arg {bad}");
            assert!(r.mutations().is_empty(), "arg {bad}");
        }
    }

    /// `qty` hanya ada di `arrow`; sisanya `recipe.qty || 1`.
    #[test]
    fn arrow_is_the_only_recipe_that_yields_more_than_one() {
        let arrow = run(serde_json::json!({ "inventory": { "wood": 1, "iron": 1 }, "exp": 0 }), "arrow");
        let data = write_of(&arrow);
        assert_eq!(data["inventory"]["arrow"], serde_json::json!(10));
        assert!(text_of(&arrow).contains("📊 Jumlah: *+10*"));

        let sword = run(serde_json::json!({ "inventory": { "iron": 3, "wood": 2 }, "exp": 0 }), "sword");
        assert_eq!(write_of(&sword)["inventory"]["sword"], serde_json::json!(1));
        assert!(text_of(&sword).contains("📊 Jumlah: *+1*"));
    }

    #[test]
    fn a_spent_material_leaves_the_bag_and_a_surplus_stays() {
        let r = run(
            serde_json::json!({ "inventory": { "iron": 4, "wood": 3 }, "exp": 0 }),
            "sword",
        );
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["iron"], serde_json::json!(1), "4 - 3");
        assert_eq!(inv["wood"], serde_json::json!(1), "3 - 2");
    }

    #[test]
    fn an_exactly_empty_material_is_removed_from_the_bag() {
        let r = run(
            serde_json::json!({ "inventory": { "iron": 3, "wood": 2 }, "exp": 0 }),
            "sword",
        );
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert!(!inv.contains_key("iron"), "3 - 3 = 0 -> dihapus");
        assert!(!inv.contains_key("wood"), "2 - 2 = 0 -> dihapus");
        assert_eq!(inv["sword"], serde_json::json!(1));
    }

    #[test]
    fn every_short_material_is_listed_with_have_over_need() {
        let r = run(serde_json::json!({ "inventory": { "iron": 1 } }), "sword");
        let t = text_of(&r);
        assert!(t.starts_with("Eits, bahannya belum cukup"), "{t}");
        assert!(t.contains("• iron: 1/3"), "{t}");
        assert!(t.contains("• wood: 0/2"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn forging_pays_the_recipe_exp_and_leaves_other_items_alone() {
        let r = run(
            serde_json::json!({ "inventory": { "iron": 3, "wood": 2, "potion": 7 }, "exp": 0 }),
            "sword",
        );
        let data = write_of(&r);
        assert_eq!(data["exp"], serde_json::json!(200));
        assert_eq!(data["inventory"]["potion"], serde_json::json!(7), "barang lain tak boleh hilang");
    }

    #[test]
    fn the_recipe_table_keeps_every_js_row() {
        assert_eq!(RECIPES.len(), 10);
        assert_eq!(RECIPES[0].1, "⚔️ Pedang Besi");
        let want: Recipe = ("diamondarmor", "💎 Armor Berlian",
            &[("diamond", 8), ("iron", 5), ("leather", 3)], "diamondarmor", 800, 1);
        assert_eq!(RECIPES[9], want);
        assert!(recipe("tidak-ada").is_none());
    }
}