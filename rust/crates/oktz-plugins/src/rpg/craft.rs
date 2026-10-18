//! `plugins/rpg/craft.js` — rakit alat; sebagian resep memberi bonus stat.
//!
//! Dua kursor: bahan tidak dihapus dari tas (`-= qty` saja, bahkan kalau
//! habis), dan `db.save()` TIDAK dipanggil — perubahan tetap masuk mutasi di
//! sini karena `Reply` ditulis pipeline sebelum pesan dikirim.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{inventory_map, rpg_map};

/// `(key, name, materials, result, bonus, qty)`.
type Recipe = (
    &'static str,
    &'static str,
    &'static [(&'static str, i64)],
    &'static str,
    &'static [(&'static str, i64)],
    i64,
);

const RECIPES: [Recipe; 5] = [
    ("sword", "⚔️ Iron Sword", &[("iron", 5), ("coal", 3)], "sword", &[("attack", 10)], 1),
    ("armor", "🛡️ Iron Armor", &[("iron", 10), ("coal", 5)], "armor", &[("defense", 15)], 1),
    ("pickaxe", "⛏️ Diamond Pickaxe", &[("diamond", 3), ("iron", 2)], "pickaxe", &[("mining", 20)], 1),
    ("rod", "🎣 Golden Rod", &[("gold", 5), ("iron", 2)], "rod", &[("fishing", 20)], 1),
    ("potion", "🥤 Health Potion", &[("fish", 3), ("rabbit", 2)], "potion", &[], 2),
];

fn recipe(key: &str) -> Option<&'static Recipe> {
    RECIPES.iter().find(|(k, ..)| *k == key)
}

fn have(map: &Map<String, Value>, key: &str) -> i64 {
    map.get(key).and_then(Value::as_i64).unwrap_or(0)
}

fn book(inv: &Map<String, Value>) -> String {
    let mut txt = String::from("Halo Petualang! Mau merakit alat apa nih? 🛠️✨\n\n*Daftar Resep Rakitan:*\n");
    for (key, name, materials, _, _, _) in RECIPES {
        txt.push_str(&format!("\n*{name}*\n📦 Bahan yang dibutuhin:\n"));
        for (item, qty) in materials {
            let owned = have(inv, item);
            txt.push_str(&format!(
                "• {} {item}: {owned}/{qty}\n",
                if owned >= *qty { "✅" } else { "❌" }
            ));
        }
        txt.push_str(&format!("👉 Ketik: `.craft {key}`\n"));
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
    name = "craft",
    alias = ["buat", "create"],
    category = "rpg",
    description = "Craft item dari materials",
    usage = ".craft <item>",
    example = ".craft sword",
    cooldown = 30,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn craft(ctx: Ctx) -> Reply {
    let mut inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    let Some(key) = crate::common::first_keyword(&ctx) else {
        return Reply::text(book(&inv));
    };

    let Some(&(_, name, materials, result, bonus, qty)) = recipe(&key) else {
        return Reply::text(
            "Hayo, mau rakit apaan tuh? Barangnya nggak ada di daftar kak! 😂\n\
             Cek list yang bener pake `.craft` ya!",
        );
    };

    let short = missing(&inv, materials);
    if !short.is_empty() {
        return Reply::text(format!(
            "Eits, bahannya belum cukup buat ngerakit *{name}* nih! 😭\n\n\
             Kekurangannya:\n{}\n\n\
             Kumpulin dulu deh, baru balik ke sini! 🏃💨",
            short.join("\n")
        ));
    }

    for (item, qty) in materials {
        inv.insert((*item).to_string(), serde_json::json!(have(&inv, item) - qty));
    }
    inv.insert(result.to_string(), serde_json::json!(have(&inv, result) + qty));

    // `user.rpg[stat] = (user.rpg[stat] || 0) + value` — `||`, jadi stat 0
    // diperlakukan sebagai belum ada dan hasilnya tetap sama.
    for (stat, value) in bonus {
        let cur = rpg.get(*stat).and_then(Value::as_i64).unwrap_or(0);
        rpg.insert((*stat).to_string(), serde_json::json!(cur + value));
    }

    let mut txt = format!(
        "YEAYY! BARANGNYA UDAH JADI! 🎉🛠️\n\n\
         Kamu berhasil merakit:\n\
         📦 Item: *{name} x{qty}*\n"
    );
    if !bonus.is_empty() {
        txt.push_str("\n*Status Bonus Aktif:*\n");
        for (stat, value) in bonus {
            txt.push_str(&format!("📈 {}: *+{value}*\n", stat.to_uppercase()));
        }
    }

    Reply::text(txt).with_reaction("✅").with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "craft".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("craft").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("craft").unwrap();
        assert_eq!(p.meta.name, "craft");
        assert_eq!(p.meta.alias, ["buat", "create"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Craft item dari materials");
        assert_eq!(p.meta.usage, ".craft <item>");
        assert_eq!(p.meta.example, ".craft sword");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["buat", "create"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "craft");
        }
    }

    #[test]
    fn without_an_argument_the_catalogue_lists_every_recipe_with_a_checkbox() {
        let t_reply = run(serde_json::json!({ "inventory": { "iron": 7 } }), "");
        let t = text_of(&t_reply);
        assert!(t.starts_with("Halo Petualang!"), "{t}");
        for (key, name, ..) in RECIPES {
            assert!(t.contains(&format!("👉 Ketik: `.craft {key}`")), "{key} hilang");
            assert!(t.contains(&format!("*{name}*")), "{key} hilang");
        }
        assert!(t.contains("✅ iron: 7/5"), "sword: {t}");
        assert!(t.contains("❌ iron: 7/10"), "armor butuh 10: {t}");
    }

    #[test]
    fn the_catalogue_branch_writes_nothing() {
        let r = run(serde_json::json!({ "inventory": { "iron": 7 } }), "");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_item_is_refused() {
        for bad in ["bow", "sword2", "swords"] {
            let r = run(serde_json::json!({ "inventory": { "iron": 99 } }), bad);
            assert!(text_of(&r).starts_with("Hayo, mau rakit apaan tuh"), "arg {bad}");
            assert!(r.mutations().is_empty(), "arg {bad}");
        }
    }

    #[test]
    fn a_bonus_recipe_raises_the_stat_in_rpg() {
        let r = run(
            serde_json::json!({ "inventory": { "iron": 5, "coal": 3 }, "rpg": { "attack": 4 } }),
            "sword",
        );
        let t = text_of(&r);
        assert!(t.contains("📦 Item: *⚔️ Iron Sword x1*"), "{t}");
        assert!(t.contains("Status Bonus Aktif:"), "{t}");
        assert!(t.contains("📈 ATTACK: *+10*"), "{t}");
        assert_eq!(write_of(&r)["rpg"]["attack"], serde_json::json!(14));
    }

    #[test]
    fn an_absent_stat_starts_from_zero() {
        let r = run(serde_json::json!({ "inventory": { "gold": 5, "iron": 2 } }), "rod");
        assert_eq!(write_of(&r)["rpg"]["fishing"], serde_json::json!(20));
        assert!(write_of(&r)["rpg"].get("attack").is_none(), "tak ada stat yang tak disentuh");
    }

    #[test]
    fn a_recipe_without_a_bonus_writes_no_stat_at_all() {
        let r = run(serde_json::json!({ "inventory": { "fish": 3, "rabbit": 2 } }), "potion");
        let t = text_of(&r);
        assert!(t.contains("📦 Item: *🥤 Health Potion x2*"), "{t}");
        assert!(!t.contains("Status Bonus"), "potion tak punya bonus: {t}");
        let rpg = write_of(&r)["rpg"].clone();
        assert!(rpg.as_object().unwrap().is_empty(), "tak ada stat yang ditulis");
    }

    /// Bahan habis TIDAK dihapus dari tas — `user.inventory[mat] -= qty` saja.
    #[test]
    fn a_depleted_material_stays_in_the_bag_as_zero() {
        let r = run(serde_json::json!({ "inventory": { "iron": 5, "coal": 3 } }), "sword");
        let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
        assert_eq!(inv["iron"], serde_json::json!(0));
        assert_eq!(inv["coal"], serde_json::json!(0));
        assert!(inv.contains_key("iron"), "kunci tak boleh hilang");
    }

    #[test]
    fn every_short_material_is_listed_in_recipe_order() {
        let r = run(serde_json::json!({ "inventory": { "iron": 1, "coal": 9 } }), "sword");
        let t = text_of(&r);
        assert!(t.starts_with("Eits, bahannya belum cukup"), "{t}");
        assert!(t.contains("• iron: 1/5"), "{t}");
        assert!(!t.contains("• coal: 9/3"), "coal cukup: {t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_recipe_table_keeps_every_js_row() {
        assert_eq!(RECIPES.len(), 5);
        let want: Recipe = ("sword", "⚔️ Iron Sword",
            &[("iron", 5), ("coal", 3)], "sword", &[("attack", 10)], 1);
        assert_eq!(RECIPES[0], want);
        let want: Recipe = ("potion", "🥤 Health Potion",
            &[("fish", 3), ("rabbit", 2)], "potion", &[], 2);
        assert_eq!(RECIPES[4], want);
        assert!(recipe("tidak-ada").is_none());
    }
}