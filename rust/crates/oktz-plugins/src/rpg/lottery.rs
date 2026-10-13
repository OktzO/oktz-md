//! `plugins/rpg/lottery.js` — gacha berbayar 1..10 tarikan.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//! Kartu level-up (`checkAndNotifyLevelUp`) juga tidak — lihat
//! [`crate::rpg::jualan::apply_exp`].
//!
//! Catatan kursor: `toLocaleString()` tanpa locale memakai koma (locale
//! default Node `en-US`), bukan titik seperti `toLocaleString("id-ID")`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, inventory_map, math_random, or_default, rpg_map, thousands_en,
};

pub(crate) const GACHA_COST: i64 = 500;

/// `GACHA_POOL` (`lottery.js:20-33`) — `(item, nama, chance, qty min, qty max,
/// rarity)`. `trash` tidak punya `qty` di JS, jadi kolomnya diisi 0/0 dan
/// `draw_one` selalu memberi 1.
pub(crate) const POOL: [(&str, &str, f64, i64, i64, &str); 12] = [
    ("trash", "🗑️ Sampah Busuk", 30.0, 0, 0, "common"),
    ("wood", "🪵 Kayu Bakar", 20.0, 3, 8, "common"),
    ("iron", "🔩 Besi Bekas", 15.0, 2, 5, "common"),
    ("gold", "🪙 Emas Batangan", 10.0, 1, 3, "uncommon"),
    ("potion", "🧪 Ramuan Sakti", 8.0, 1, 3, "uncommon"),
    ("diamond", "💎 Berlian Murni", 5.0, 1, 2, "rare"),
    ("goldchest", "🎁 Peti Emas", 3.0, 1, 1, "rare"),
    ("diamondchest", "💎 Peti Berlian", 1.5, 1, 1, "epic"),
    ("mysterybox", "🎲 Kotak Misteri", 0.8, 1, 1, "epic"),
    (
        "goldsword",
        "🗡️ Pedang Excalibur Emas",
        0.3,
        1,
        1,
        "legendary",
    ),
    (
        "diamondarmor",
        "🛡️ Zirah Berlian Abadi",
        0.2,
        1,
        1,
        "legendary",
    ),
    (
        "divinecore",
        "⚡ Inti Dewa (Divine Core)",
        0.1,
        1,
        1,
        "mythic",
    ),
];

pub(crate) fn rarity_icon(rarity: &str) -> &'static str {
    match rarity {
        "uncommon" => "🟢",
        "rare" => "🔵",
        "epic" => "🟣",
        "legendary" => "🟡",
        "mythic" => "🔴",
        _ => "⚪",
    }
}

pub(crate) fn exp_by_rarity(rarity: &str) -> i64 {
    match rarity {
        "uncommon" => 30,
        "rare" => 80,
        "epic" => 150,
        "legendary" => 300,
        "mythic" => 500,
        _ => 10,
    }
}

/// `Math.min(10, Math.max(1, parseInt(args[0]) || 1))`.
pub(crate) fn pulls_of(raw: Option<&String>) -> i64 {
    let n = raw
        .and_then(|a| crate::common::parse_int(a))
        .filter(|n| *n != 0)
        .unwrap_or(1);
    n.clamp(1, 10)
}

/// Satu tarikan: kumulatif `chance`, `roll <= cumulative` (bukan `<`).
pub(crate) fn draw_one() -> (&'static str, &'static str, &'static str, i64) {
    let roll = math_random() * 100.0;
    let mut cumulative = 0.0;
    for &(item, name, chance, min, max, rarity) in POOL.iter() {
        cumulative += chance;
        if roll <= cumulative {
            let qty = if max <= min {
                1
            } else {
                crate::rpg::jualan::roll_incl(min, max)
            };
            return (item, name, rarity, qty);
        }
    }
    ("trash", "🗑️ Sampah Busuk", "common", 1)
}

#[plugin(
    name = "lottery",
    alias = ["undian"],
    category = "rpg",
    description = "Gacha/lottery untuk hadiah random",
    usage = ".lottery <1/10>",
    example = ".lottery 10",
    cooldown = 30,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn lottery(ctx: Ctx) -> Reply {
    let pulls = pulls_of(ctx.args.first());
    let total_cost = GACHA_COST * pulls;
    let koin = int_field(&ctx, "koin");

    if koin < total_cost {
        return Reply::text(format!(
            "💸 *UANG LU KURANG BUAT GACHA!* 💸\n\n\
             Harga Gacha: *Rp {}/Tarikan*\n\
             Total Kebutuhan: *Rp {} ({pulls}x)*\n\n\
             Sisa Duit Lu Cuma: *Rp {}*. Kerja dulu mendingan!",
            thousands_en(GACHA_COST),
            thousands_en(total_cost),
            thousands_en(koin)
        ));
    }

    let mut inv = inventory_map(&ctx);
    let mut rpg = rpg_map(&ctx);

    // `grouped` menjaga urutan kemunculan pertama — sama dengan urutan iterasi
    // `Object.entries` pada objek yang diisi berurutan di JS.
    let mut grouped: Vec<(&'static str, &'static str, &'static str, i64, i64)> = Vec::new();
    let mut total_exp = 0i64;
    for _ in 0..pulls {
        let (item, name, rarity, qty) = draw_one();
        if item != "trash" {
            let slot = inv.entry(item.to_string()).or_insert(json!(0));
            let have = slot.as_i64().unwrap_or(0);
            *slot = json!(have + qty);
            total_exp += exp_by_rarity(rarity);
        }
        match grouped.iter_mut().find(|g| g.0 == item) {
            Some(g) => {
                g.3 += 1;
                g.4 += if item == "trash" { 0 } else { qty };
            }
            None => grouped.push((item, name, rarity, 1, if item == "trash" { 0 } else { qty })),
        }
    }

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, total_exp);

    let mut txt = String::from("🎉 *BAMMM!!! TABUNG TERBUKA!!* 🎉\n\n");
    txt += &format!(
        "Tarik *{pulls}x* | Keluarin Duit: *Rp {}*\n\n",
        thousands_en(total_cost)
    );
    txt += "*🎁 HASIL GACHA LU:* \n";

    let mut has_rare = false;
    let mut has_legendary = false;
    for (item, name, rarity, count, total_qty) in &grouped {
        let icon = rarity_icon(rarity);
        if *item == "trash" {
            txt += &format!("> {icon} {name} *(Ampas x{count})*\n");
        } else {
            txt += &format!("> {icon} {name} *x{total_qty}*\n");
        }
        if matches!(*rarity, "epic" | "legendary" | "mythic") {
            has_rare = true;
        }
        if matches!(*rarity, "legendary" | "mythic") {
            has_legendary = true;
        }
    }
    txt += &format!("\n📈 *Bonus EXP:* +{total_exp} ✨\n");
    txt += if has_legendary {
        "\n🌟🌟 *WOOOYYY!!! LU DAPET ITEM LEGENDARY!! HOKI SEUMUR HIDUP KEPAKE!!* 🌟🌟"
    } else if has_rare {
        "\n✨ *Wihh mayan dapet barang langka bro!*"
    } else {
        "\n🥲 *Ampas... Kebanyakan dapet barang rongsok.*"
    };

    Reply::text(txt)
        .with_reaction(if has_legendary {
            "🌟"
        } else if has_rare {
            "🎉"
        } else {
            "✅"
        })
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin - total_cost, "exp": new_exp, "rpg": rpg, "inventory": inv }),
        })
}

/// `expByRarity` hanya dievaluasi untuk hasil NON-trash — `trash` punya
/// `rarity: "common"` tapi masuk cabang `else`, jadi tidak menambah exp.
#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "lottery".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("lottery").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
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
        let p = oktz_plugin_api::find("lottery").unwrap();
        assert_eq!(p.meta.name, "lottery");
        assert_eq!(p.meta.alias, ["undian"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Gacha/lottery untuk hadiah random");
        assert_eq!(p.meta.usage, ".lottery <1/10>");
        assert_eq!(p.meta.example, ".lottery 10");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("undian").unwrap().meta.name, "lottery");
    }

    #[test]
    fn pulling_requires_exactly_enough_money() {
        let r = run(&["1"], json!({"koin": 499}));
        let t = text_of(&r);
        assert!(t.starts_with("💸 *UANG LU KURANG BUAT GACHA!*"), "{t}");
        assert!(t.contains("Harga Gacha: *Rp 500/Tarikan*"), "{t}");
        assert!(t.contains("Total Kebutuhan: *Rp 500 (1x)*"), "{t}");
        assert!(t.contains("Sisa Duit Lu Cuma: *Rp 499*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_no_arg_format_uses_commas_because_locale_is_not_specified() {
        let r = run(&["10"], json!({"koin": 4999}));
        let t = text_of(&r);
        assert!(t.contains("Total Kebutuhan: *Rp 5,000 (10x)*"), "{t}");
        assert!(t.contains("Sisa Duit Lu Cuma: *Rp 4,999*"), "{t}");
    }

    #[test]
    fn the_pull_count_is_clamped_into_one_to_ten() {
        for (arg, expected) in [
            ("-5", 1),
            ("0", 1),
            ("abc", 1),
            ("3", 3),
            ("10", 10),
            ("99", 10),
            ("10abc", 10),
        ] {
            let r = run(&[arg], json!({"koin": 1_000_000}));
            assert!(
                text_of(&r).contains(&format!("Tarik *{expected}x*")),
                "arg {arg:?} harus jadi {expected}x"
            );
            assert_eq!(written(&r)["koin"], json!(1_000_000 - 500 * expected));
        }
    }

    #[test]
    fn trash_is_grouped_as_waste_and_never_stored() {
        let mut seen = false;
        for _ in 0..500 {
            let r = run(&["1"], json!({"koin": 10_000}));
            let d = written(&r);
            let t = text_of(&r);
            if t.contains("Ampas x") {
                seen = true;
                assert!(
                    d["inventory"].get("trash").is_none(),
                    "trash tidak masuk inventory"
                );
                assert_eq!(d["exp"], json!(0), "trash tidak kasih exp");
                break;
            }
        }
        assert!(seen, "trash (chance 30) harus muncul dalam 500 sampel");
    }

    #[test]
    fn stored_items_accumulate_on_top_of_the_existing_stack() {
        for _ in 0..400 {
            let r = run(
                &["10"],
                json!({"koin": 100_000, "inventory": {"wood": 7, "potion": 2}}),
            );
            let d = written(&r);
            let inv = d["inventory"].as_object().unwrap();
            assert!(inv["wood"].as_i64().unwrap() >= 7);
            assert!(inv["potion"].as_i64().unwrap() >= 2);
            assert_eq!(d["koin"], json!(95_000), "100000 - 10*500");
            assert!(!inv.contains_key("trash"));
        }
    }

    #[test]
    fn one_line_per_distinct_item_never_repeats_a_name() {
        for _ in 0..300 {
            let r = run(&["10"], json!({"koin": 100_000}));
            let t = text_of(&r);
            let listed: Vec<&str> = t
                .lines()
                .filter(|l| l.starts_with("> ") && l.contains(" *"))
                .collect();
            for (i, a) in listed.iter().enumerate() {
                for b in &listed[i + 1..] {
                    let name_a = a.split(" *").next().unwrap();
                    let name_b = b.split(" *").next().unwrap();
                    assert_ne!(name_a, name_b, "baris dobel: {t}");
                }
            }
        }
    }

    #[test]
    fn the_legendary_banner_only_shows_for_legendary_and_mythic() {
        for _ in 0..2000 {
            let r = run(&["10"], json!({"koin": 500_000}));
            let t = text_of(&r);
            let has_l = t.contains("WOOOYYY");
            let has_r = t.contains("Wihh mayan");
            let has_n = t.contains("Ampas...");
            assert_eq!(has_l as u8 + has_r as u8 + has_n as u8, 1, "tepat satu banner: {t}");
        }
    }

    #[test]
    fn the_helper_contracts_hold() {
        for _ in 0..500 {
            let (item, _, _, qty) = draw_one();
            assert!(POOL.iter().any(|p| p.0 == item), "item {item} di luar pool");
            assert!(qty >= 1, "tarikan selalu memberi qty >= 1");
        }
        assert_eq!(exp_by_rarity("mythic"), 500);
        assert_eq!(exp_by_rarity("tidak-ada"), 10);
        assert_eq!(rarity_icon("mythic"), "🔴");
        assert_eq!(rarity_icon("ngawur"), "⚪");
        for item in ["wood", "iron", "potion", "divinecore"] {
            assert!(POOL.iter().any(|p| p.0 == item), "{item} harus ada di pool");
        }
    }
}
