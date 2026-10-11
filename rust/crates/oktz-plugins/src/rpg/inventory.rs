//! `plugins/rpg/inventory.js` — baca `users[sender].inventory`, tanpa tulis.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::int_field;
use crate::tools::kalkulatormbg::thousands_id;

/// `ITEMS` (`inventory.js:11-44`) — cukup `(kunci, emote, nama)`; `emote` dan
/// `name` di plugin dipakai persis begini.
pub(crate) const ITEMS: [(&str, &str, &str); 27] = [
    ("common", "📦", "Common Crate"),
    ("uncommon", "🛍️", "Uncommon Crate"),
    ("mythic", "🎁", "Mythic Crate"),
    ("legendary", "💎", "Legendary Crate"),
    ("rock", "🪨", "Batu"),
    ("coal", "⚫", "Batubara"),
    ("iron", "⛓️", "Besi"),
    ("gold", "🥇", "Emas"),
    ("diamond", "💠", "Berlian"),
    ("emerald", "💚", "Emerald"),
    ("trash", "🗑️", "Sampah"),
    ("fish", "🐟", "Ikan"),
    ("prawn", "🦐", "Udang"),
    ("octopus", "🐙", "Gurita"),
    ("shark", "🦈", "Hiu"),
    ("whale", "🐳", "Paus"),
    ("potion", "🥤", "Health Potion"),
    ("mpotion", "🧪", "Mana Potion"),
    ("stamina", "⚡", "Stamina Potion"),
    ("herb", "🌿", "Herba"),
    ("leather", "👞", "Kulit"),
    ("mysterybox", "📦", "Mystery Box"),
    ("kunai", "🗡️", "Kunai"),
    ("shuriken", "⚔️", "Shuriken"),
    ("chakra", "🌀", "Chakra"),
    ("scroll", "📜", "Scroll Ninja"),
    ("bowlramen", "🍜", "Ramen"),
];

/// `categories` (`inventory.js:57-64`), urutan object literal = urutan tampil.
pub(crate) const CATEGORIES: [(&str, &[&str]); 6] = [
    ("📦 *Koleksi Crates*", &["common", "uncommon", "mythic", "legendary"]),
    (
        "⛏️ *Hasil Tambang*",
        &["rock", "coal", "iron", "gold", "diamond", "emerald"],
    ),
    (
        "🎣 *Hasil Mancing*",
        &["trash", "fish", "prawn", "octopus", "shark", "whale"],
    ),
    ("🌿 *Hasil Dungeon*", &["herb", "leather", "mysterybox"]),
    ("🧪 *Potions & Buffs*", &["potion", "mpotion", "stamina"]),
    (
        "⛩️ *Perlengkapan Shinobi*",
        &["kunai", "shuriken", "chakra", "scroll", "bowlramen"],
    ),
];

/// `ITEMS[key]` — kunci selalu dari `CATEGORIES`, tapi `lookup` defensif
/// supaya tabel dan kategori tidak bisa berbeda diam-diam.
pub(crate) fn item(key: &str) -> Option<(&'static str, &'static str)> {
    ITEMS
        .iter()
        .find(|(k, _, _)| *k == key)
        .map(|(_, emote, name)| (*emote, *name))
}

/// `user.inventory[key] || 0`.
pub(crate) fn qty(ctx: &Ctx, key: &str) -> i64 {
    ctx.user_field("inventory")
        .and_then(|inv| inv.get(key))
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

#[plugin(
    name = "inventory",
    alias = ["inv", "tas", "bag"],
    category = "rpg",
    description = "Melihat isi inventory RPG",
    usage = ".inventory",
    example = ".inventory",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn inventory(ctx: Ctx) -> Reply {
    let mut inv = String::from("🎒 *Isi Tas Kamu Nih Kak!* ✨\n\n");
    inv.push_str(&format!(
        "❤️ HP: *{}*\n💸 Koin: *{}*\n📈 EXP: *{}*\n\n",
        ctx.user_field("rpg.health")
            .and_then(Value::as_i64)
            .unwrap_or(100),
        thousands_id(int_field(&ctx, "koin") as f64),
        thousands_id(int_field(&ctx, "exp") as f64),
    ));

    let mut has_item = false;
    for (cat_name, keys) in CATEGORIES {
        let mut cat_text = String::new();
        for key in keys {
            let count = qty(&ctx, key);
            if count > 0 {
                let (emote, name) = item(key).expect("kunci kategori harus ada di ITEMS");
                cat_text.push_str(&format!("{emote} {name}: *{count}x*\n"));
                has_item = true;
            }
        }
        if !cat_text.is_empty() {
            inv.push_str(cat_name);
            inv.push('\n');
            inv.push_str(&cat_text);
            inv.push('\n');
        }
    }

    if !has_item {
        inv.push_str("Loh, tas kamu masih kosong melompong kak! 🕸️\n");
        inv.push_str("Yuk main command RPG lain buat dapetin item seru! 🚀\n");
    } else {
        inv.push_str("Ketik *.use <nama item>* buat pake barangnya ya! 🎒💖\n");
    }

    Reply::text(inv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: serde_json::Value) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "inventory".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("inventory").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("inventory").unwrap();
        assert_eq!(p.meta.name, "inventory");
        assert_eq!(p.meta.alias, ["inv", "tas", "bag"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Melihat isi inventory RPG");
        assert_eq!(p.meta.usage, ".inventory");
        assert_eq!(p.meta.example, ".inventory");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["inv", "tas", "bag"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "inventory");
        }
    }

    // -- cabang state tersimpan -------------------------------------------

    #[test]
    fn an_absent_user_row_shows_defaults_and_an_empty_bag() {
        for user in [json!(null), json!({}), json!({"rpg": null, "inventory": null})] {
            let out = run(user.clone());
            assert!(
                out.starts_with("🎒 *Isi Tas Kamu Nih Kak!* ✨\n\n❤️ HP: *100*\n💸 Koin: *0*\n📈 EXP: *0*\n\n"),
                "input {user}"
            );
            assert!(out.ends_with("Loh, tas kamu masih kosong melompong kak! 🕸️\nYuk main command RPG lain buat dapetin item seru! 🚀\n"));
            assert!(!out.contains("Koleksi Crates"));
        }
    }

    #[test]
    fn header_uses_stored_health_koin_and_exp() {
        let out = run(json!({"rpg": {"health": 73}, "koin": 25000, "exp": 1250000}));
        assert!(out.starts_with(
            "🎒 *Isi Tas Kamu Nih Kak!* ✨\n\n❤️ HP: *73*\n💸 Koin: *25.000*\n📈 EXP: *1.250.000*\n\n"
        ));
    }

    #[test]
    fn zero_count_items_are_hidden_and_only_their_category_disappears() {
        let out = run(json!({"inventory": {"rock": 0, "coal": 4, "fish": 2}}));
        assert!(out.contains("⛏️ *Hasil Tambang*\n⚫ Batubara: *4x*\n"));
        assert!(
            !out.contains("🪨 Batu:"),
            "count 0 tidak dirender (bukan substring dari `Batubara`)"
        );
        assert!(out.contains("🎣 *Hasil Mancing*\n🐟 Ikan: *2x*\n"));
        assert!(!out.contains("Koleksi Crates"));
        assert!(out.ends_with("Ketik *.use <nama item>* buat pake barangnya ya! 🎒💖\n"));
    }

    #[test]
    fn categories_render_in_the_js_object_order() {
        let out = run(json!({"inventory": {
            "kunai": 1, "potion": 2, "herb": 3, "fish": 4, "iron": 5, "common": 6
        }}));
        let pos = |needle: &str| out.find(needle).expect("kategori hilang");
        assert!(pos("Koleksi Crates") < pos("Hasil Tambang"));
        assert!(pos("Hasil Tambang") < pos("Hasil Mancing"));
        assert!(pos("Hasil Mancing") < pos("Hasil Dungeon"));
        assert!(pos("Hasil Dungeon") < pos("Potions & Buffs"));
        assert!(pos("Potions & Buffs") < pos("Perlengkapan Shinobi"));
    }

    #[test]
    fn every_category_header_and_the_closed_boxes_line_up() {
        let out = run(
            json!({"inventory": {"common": 1, "iron": 1, "fish": 1, "herb": 1, "potion": 1, "kunai": 1}}),
        );
        for (name, _) in CATEGORIES {
            assert!(out.contains(&format!("{name}\n")), "kategori hilang: {name}");
        }
        assert_eq!(
            out.matches("\n\n").count(),
            8,
            "dua di header (judul + baris EXP), satu per kategori"
        );
    }

    #[test]
    fn item_names_and_emotes_come_from_the_js_table() {
        let out = run(json!({"inventory": {"fish": 7, "bowlramen": 1, "diamond": 2}}));
        assert!(out.contains("🐟 Ikan: *7x*"));
        assert!(out.contains("💠 Berlian: *2x*"));
        assert!(out.contains("🍜 Ramen: *1x*"));
        assert_eq!(item("whale"), Some(("🐳", "Paus")));
        assert_eq!(item("tidak-ada"), None);
    }

    #[test]
    fn table_and_categories_stay_in_sync() {
        for (_, keys) in CATEGORIES {
            for k in keys {
                assert!(item(k).is_some(), "kategori menyebut kunci tanpa entri: {k}");
            }
        }
        for (k, _, _) in ITEMS {
            assert!(
                CATEGORIES.iter().any(|(_, keys)| keys.contains(&k)),
                "item tidak ada di kategori mana pun: {k}"
            );
        }
    }

    #[test]
    fn a_non_integer_quantity_is_read_as_absent() {
        let out = run(json!({"inventory": {"fish": "3"}}));
        assert!(!out.contains("Ikan"), "bukan angka -> `|| 0` -> disembunyikan");
        assert!(out.contains("Loh, tas kamu masih kosong"));
    }
}