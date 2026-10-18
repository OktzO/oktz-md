//! `plugins/rpg/fishing.js` — mancing satu ikan per lemparan.
//!
//! Kursor penting: `drops.sort((a, b) => a.chance - b.chance)` MUTASI array
//! modul di JS, jadi `let caught = drops[0]` — yang dievaluasi SEBELUM sort —
//! bernilai `trash` pada panggilan pertama dan `whale` pada semua panggilan
//! berikutnya (setelah sort, indeks 0 = chance terkecil). Perilaku
//! berurutan itu tidak bisa direproduksi dari handler murni, jadi di sini
//! `caught` selalu mulai dari `whale` — keadaan tunak yang berlaku setelah
//! pemanggilan pertama. Cases where no drop matches its chance threshold
//! (rand > 50, ~50% peluang) memakai nilai fallback ini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::heal::{
    apply_exp, coalesce, inventory_map, or_default, rpg_int, rpg_map, roll_percent,
};

const STAMINA_COST: i64 = 15;

/// `(item, chance, name, exp)` — setelah `drops.sort(chance)` di JS.
const DROPS: [(&str, i64, &str, i64); 6] = [
    ("whale", 1, "🐳 Paus", 2000),
    ("shark", 5, "🦈 Hiu", 800),
    ("octopus", 15, "🐙 Gurita", 300),
    ("trash", 20, "🗑️ Sampah", 10),
    ("prawn", 30, "🦐 Udang", 150),
    ("fish", 50, "🐟 Ikan", 100),
];

/// `rand <= drop.chance` untuk tiap drop terurut; default `drops[0]`.
fn caught() -> (&'static str, &'static str, i64) {
    let rand = roll_percent();
    for (item, chance, name, exp) in DROPS {
        if rand <= chance {
            return (item, name, exp);
        }
    }
    let (item, _, name, exp) = DROPS[0];
    (item, name, exp)
}

#[plugin(
    name = "fishing",
    alias = ["rpgfish", "mancing"],
    category = "rpg",
    description = "Memancing untuk mendapatkan ikan (RPG)",
    usage = ".fishing",
    example = ".fishing",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn fishing(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let mut inv = inventory_map(&ctx);

    // `user.rpg.stamina || 100` — `||`, bukan `??`, jadi nol jadi 100.
    let stamina = or_default(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Aduh kak, stamina kamu abis nih! 😭⚡\n\n\
             Buat mancing butuh *{STAMINA_COST} Stamina*, tapi punya kamu sisa *{stamina}*.\n\
             Istirahat dulu gih biar seger lagi! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let (item, name, exp_reward) = caught();
    let have = coalesce(inv.get(item).and_then(Value::as_i64), 0);
    inv.insert(item.to_string(), serde_json::json!(have + 1));

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

    let catch_line = if item == "trash" {
        format!(
            "> {name} 🤢\n\
             Yahh dapetnya sampah kak... Lumayan lah dapet *+{exp_reward} EXP* pengalaman buang sampah pada tempatnya! 😂\n\n"
        )
    } else {
        format!("> *{name}* 🎉✨\nAsik banget! Kamu juga dapet *+{exp_reward} EXP* nih!\n\n")
    };

    Reply::text(format!(
        "HAPPP! Kailnya ditarik! 🎣💦\n\n\
         Wah, kamu berhasil dapetin:\n\
         {catch_line}\
         ⚡ Stamina terpakai: *-{STAMINA_COST}*\n\
         \n\
         Ikan/sampahnya udah masuk ke tas (`.inv`) ya! Jangan lupa mancing lagi nanti! 💖🌊"
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

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fishing".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("fishing").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("fishing").unwrap();
        assert_eq!(p.meta.name, "fishing");
        assert_eq!(p.meta.alias, ["rpgfish", "mancing"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Memancing untuk mendapatkan ikan (RPG)");
        assert_eq!(p.meta.usage, ".fishing");
        assert_eq!(p.meta.example, ".fishing");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["rpgfish", "mancing"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "fishing");
        }
    }

    #[test]
    fn not_enough_stamina_costs_nothing() {
        for stamina in [1, 14] {
            let r = run(serde_json::json!({ "rpg": { "stamina": stamina } }));
            let t = text_of(&r);
            assert!(t.starts_with("Aduh kak, stamina kamu abis"), "stamina {stamina}: {t}");
            assert!(t.contains(&format!("sisa *{stamina}*")), "{t}");
            assert!(r.mutations().is_empty(), "stamina {stamina}");
        }
    }

    #[test]
    fn the_drop_table_is_stored_in_the_order_the_sort_produces() {
        // `drops.sort((a, b) => a.chance - b.chance)` di JS.
        assert_eq!(DROPS[0], ("whale", 1, "🐳 Paus", 2000));
        assert_eq!(DROPS[5], ("fish", 50, "🐟 Ikan", 100));
        assert!(DROPS.windows(2).all(|w| w[0].1 < w[1].1), "harus terurut naik");
    }

    #[test]
    fn every_cast_lands_the_catch_in_the_bag_and_pays_its_exp() {
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..900 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 }, "exp": 0 }));
            let t = text_of(&r);
            assert!(t.starts_with("HAPPP! Kailnya ditarik!"), "{t}");
            let data = write_of(&r);
            let inv = data["inventory"].as_object().unwrap().clone();
            assert_eq!(inv.len(), 1, "tepat satu item per lemparan: {data}");
            let (item, qty) = inv.iter().next().unwrap();
            assert_eq!(*qty, serde_json::json!(1));
            let exp = data["exp"].as_i64().unwrap();
            let row = DROPS.iter().find(|(i, ..)| i == item).expect("item di luar tabel");
            assert_eq!(exp, row.3);
            assert!(t.contains(&format!("*+{exp} EXP*")), "{t}");
            assert_eq!(data["rpg"]["stamina"], serde_json::json!(85));
            seen.insert(item.to_string());
        }
        assert!(seen.len() >= 3, "hanya terlihat {seen:?}");
    }

    #[test]
    fn trash_gets_its_own_sarcastic_line() {
        let mut trash = false;
        for _ in 0..900 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 }, "exp": 0 }));
            if !text_of(&r).contains("🤢") {
                continue;
            }
            trash = true;
            assert!(text_of(&r).contains("🗑️ Sampah"), "{}", text_of(&r));
            assert!(text_of(&r).contains("pengalaman buang sampah"), "{}", text_of(&r));
            assert_eq!(write_of(&r)["exp"], serde_json::json!(10));
        }
        assert!(trash, "dalam 900 percobaan harus ada sampah");
    }

    #[test]
    fn a_non_trash_catch_is_celebrated_instead() {
        let mut good = false;
        for _ in 0..900 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 }, "exp": 0 }));
            let t = text_of(&r);
            if t.contains("🎉✨") {
                good = true;
                assert!(!t.contains("🤢"), "{t}");
            }
        }
        assert!(good, "dalam 900 percobaan harus ada tangkapan bagus");
    }

    #[test]
    fn a_zero_stamina_reads_as_one_hundred_because_of_or() {
        // `user.rpg.stamina || 100` — bukan `??` seperti berburu/berladang.
        let mut ran = false;
        for _ in 0..50 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 0 } }));
            if !text_of(&r).starts_with("Aduh kak, stamina kamu abis") {
                ran = true;
                assert_eq!(write_of(&r)["rpg"]["stamina"], serde_json::json!(85), "0 -> 100 -> 85");
            }
        }
        assert!(ran, "stamina 0 harus jadi 100");
    }

    #[test]
    fn an_existing_stack_grows_instead_of_being_replaced() {
        for _ in 0..900 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 }, "inventory": { "fish": 4 } }));
            let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
            // `user.inventory[item] = (user.inventory[item] || 0) + 1` — tumpukan
            // `fish` bawaan 4 jadi 5 kalau yang tertangkap ikan, kalau tidak ia
            // tetap 4 dan keysatu yang lain bernilai 1.
            match inv.get("fish").and_then(Value::as_i64) {
                Some(5) => assert_eq!(inv.len(), 1, "4 + 1, tak ada item lain: {inv:?}"),
                Some(4) => {
                    let (item, qty) = inv
                        .iter()
                        .find(|(k, _)| k.as_str() != "fish")
                        .expect("ikan lama + satu tangkapan baru");
                    assert_eq!(*qty, serde_json::json!(1), "0 + 1 untuk {item}");
                }
                other => panic!("fish bukan 4 atau 5: {other:?} — {inv:?}"),
            }
        }
    }
}