//! `plugins/rpg/hunt.js` — satu hewan per lemparan.
//!
//! Kursor: `animals.sort((a, b) => a.chance - b.chance)` mengurutkan dari
//! chance TERENDAH, jadi tier paling sulit diburu diperiksa lebih dulu
//! dan paling mungkin tembus. Karena `Math.random() * 100` selalu < 100 dan
//! tier tertinggi hanya punya chance 50, cabang fallback
//! `animals.find(a => a.item === "rabbit")` praktis tak pernah tercapai.
//!
//! Alias `berburu` bentrok dengan plugin `berburu.js` — sama seperti di JS,
//! di mana kedua file mengklaim command itu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::rpg::heal::{
    apply_exp, inventory_map, or_default, rpg_int, rpg_map, roll_percent,
};

const STAMINA_COST: i64 = 25;

/// `(name, item, chance, exp)` — sudah urut naik seperti `sort()` di JS.
const ANIMALS: [(&str, &str, i64, i64); 6] = [
    ("🐉 Anak Naga Kuno", "dragon", 1, 2000),
    ("🦁 Singa Padang Rumput", "lion", 5, 800),
    ("🐻 Beruang Madu", "bear", 10, 500),
    ("🐗 Babi Hutan Liar", "boar", 20, 300),
    ("🦌 Rusa Jantan", "deer", 30, 200),
    ("🐰 Kelinci Hutan", "rabbit", 50, 100),
];

fn caught() -> (&'static str, &'static str, i64) {
    let rand = roll_percent();
    for (name, item, chance, exp) in ANIMALS {
        if rand <= chance {
            return (name, item, exp);
        }
    }
    // `animals.find((a) => a.item === "rabbit")` — hanya kena kalau tak satu
    // pun chance >= rand.
    for (name, item, _, exp) in ANIMALS {
        if item == "rabbit" {
            return (name, item, exp);
        }
    }
    unreachable!("tabel selalu punya kelinci")
}

#[plugin(
    name = "hunt",
    alias = ["berburu", "hunting"],
    category = "rpg",
    description = "Berburu hewan untuk mendapatkan daging dan kulit",
    usage = ".hunt",
    example = ".hunt",
    cooldown = 90,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn hunt(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let mut inv = inventory_map(&ctx);

    // `user.rpg.stamina || 100` — `||`, bukan `??`.
    let stamina = or_default(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "⚡ Aduh kak, stamina kamu habis!\n\n\
             Butuh *{STAMINA_COST} Stamina* buat pergi berburu.\n\
             Stamina kamu sisa: *{stamina}*"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let (name, item, exp_reward) = caught();
    let have = or_default(inv.get(item).and_then(Value::as_i64), 0);
    inv.insert(item.to_string(), serde_json::json!(have + 1));

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

    Reply::text(format!(
        "🏹 *TANGKAPAN BERHASIL!* 🏹\n\n\
         Wah jago banget bidangannya kak! Kamu berhasil menembak:\n\
         🎯 *{name}* (+1)\n\n\
         *Hasil Perburuan:*\n\
         ✨ EXP Bertambah: *+{exp_reward}*\n\
         ⚡ Stamina Terpakai: *-{STAMINA_COST}*"
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "hunt".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("hunt").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("hunt").unwrap();
        assert_eq!(p.meta.name, "hunt");
        assert_eq!(p.meta.alias, ["berburu", "hunting"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Berburu hewan untuk mendapatkan daging dan kulit"
        );
        assert_eq!(p.meta.usage, ".hunt");
        assert_eq!(p.meta.example, ".hunt");
        assert_eq!(p.meta.cooldown, 90);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_hunting_alias_resolves_and_the_collision_is_inherited_from_js() {
        assert_eq!(oktz_plugin_api::find("hunting").unwrap().meta.name, "hunt");
        // `berburu` juga alias plugin `berburu.js` — pertikaian yang sama ada
        // di JS, jadi `find` mengembalikan siapa saja yang terdaftar dulu.
        let owner = oktz_plugin_api::find("berburu").unwrap().meta.name.clone();
        assert!(owner == "hunt" || owner == "berburu", "pemilik {owner}");
    }

    #[test]
    fn not_enough_stamina_blocks_the_hunt() {
        for stamina in [1, 24] {
            let r = run(serde_json::json!({ "rpg": { "stamina": stamina } }));
            let t = text_of(&r);
            assert!(t.starts_with("⚡ Aduh kak, stamina kamu habis!"), "stamina {stamina}: {t}");
            assert!(t.contains(&format!("sisa: *{stamina}*")), "{t}");
            assert!(r.mutations().is_empty(), "stamina {stamina}");
        }
    }

    #[test]
    fn the_animal_table_is_stored_in_the_order_the_sort_produces() {
        assert_eq!(ANIMALS[0], ("🐉 Anak Naga Kuno", "dragon", 1, 2000));
        assert_eq!(ANIMALS[5], ("🐰 Kelinci Hutan", "rabbit", 50, 100));
        assert!(ANIMALS.windows(2).all(|w| w[0].2 < w[1].2));
    }

    #[test]
    fn every_shot_lands_one_animal_with_its_own_exp() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..900 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 }, "exp": 0 }));
            let t = text_of(&r);
            assert!(t.starts_with("🏹 *TANGKAPAN BERHASIL!*"), "{t}");
            let data = write_of(&r);
            let inv = data["inventory"].as_object().unwrap().clone();
            assert_eq!(inv.len(), 1, "tepat satu hewan: {data}");
            let (item, qty) = inv.iter().next().unwrap();
            assert_eq!(*qty, serde_json::json!(1));
            let exp = data["exp"].as_i64().unwrap();
            let row = ANIMALS.iter().find(|(_, i, ..)| i == item).expect("hewan di luar tabel");
            assert_eq!(exp, row.3);
            assert!(t.contains(&format!("✨ EXP Bertambah: *+{exp}*")), "{t}");
            assert_eq!(data["rpg"]["stamina"], serde_json::json!(75));
            seen.insert(item.to_string());
        }
        assert!(seen.len() >= 3, "hanya terlihat {seen:?}");
    }

    #[test]
    fn the_rarer_animal_carries_the_bigger_payout() {
        // Naga: 2.000 exp; kelinci: 100 exp.
        assert!(ANIMALS[0].3 > ANIMALS[5].3 * 10);
    }

    #[test]
    fn a_zero_stamina_reads_as_one_hundred_because_of_or() {
        let mut ran = false;
        for _ in 0..50 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 0 } }));
            if !text_of(&r).starts_with("⚡ Aduh kak, stamina kamu habis!") {
                ran = true;
                assert_eq!(write_of(&r)["rpg"]["stamina"], serde_json::json!(75), "0 -> 100 -> 75");
            }
        }
        assert!(ran, "stamina 0 harus jadi 100");
    }

    #[test]
    fn an_existing_stack_grows_by_one() {
        for _ in 0..300 {
            let r = run(
                serde_json::json!({ "rpg": { "stamina": 100 }, "inventory": { "rabbit": 6 } }),
            );
            let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
            // `user.inventory[item] = (user.inventory[item] || 0) + 1` — tumpukan
            // `rabbit` bawaan 6 jadi 7 kalau yang tertangkap kelinci, kalau tidak
            // ia tetap 6 dan keysatu yang lain bernilai 1.
            match inv.get("rabbit").and_then(Value::as_i64) {
                Some(7) => assert_eq!(inv.len(), 1, "6 + 1, tak ada hewan lain: {inv:?}"),
                Some(6) => {
                    let (item, qty) = inv
                        .iter()
                        .find(|(k, _)| k.as_str() != "rabbit")
                        .expect("kelinci lama + satu hewan baru");
                    assert_eq!(*qty, serde_json::json!(1), "0 + 1 untuk {item}");
                }
                other => panic!("rabbit bukan 6 atau 7: {other:?} — {inv:?}"),
            }
        }
    }

    #[test]
    fn a_missing_user_row_never_panics() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({ "rpg": null })] {
            let r = run(user.clone());
            assert!(!text_of(&r).is_empty(), "input {user}");
        }
    }
}