//! `plugins/rpg/woodcut.js` — menebang pohon untuk mendapatkan kayu.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//!
//! Catatan kursor: guard stamina memakai `user.rpg.stamina || 100`, bukan `??` —
//! nol dianggap falsy, jadi penebang dengan bar kosong tetap boleh masuk.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::jualan::{
    apply_exp, inventory_map, math_random, or_default, rpg_int, rpg_map, roll_incl,
};

pub(crate) const STAMINA_COST: i64 = 15;

/// `drops` (`woodcut.js:40-45`) — `(item, nama, chance persen, min, max)`.
pub(crate) const DROPS: [(&str, &str, f64, i64, i64); 4] = [
    ("wood", "🪵 Kayu", 70.0, 2, 5),
    ("stick", "🥢 Ranting", 50.0, 1, 3),
    ("apple", "🍎 Apel", 20.0, 1, 2),
    ("rubber", "⚫ Karet", 10.0, 1, 1),
];

#[plugin(
    name = "woodcut",
    alias = ["chop", "nebang", "kayu"],
    category = "rpg",
    description = "Menebang pohon untuk mendapatkan kayu",
    usage = ".woodcut",
    example = ".woodcut",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn woodcut(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = or_default(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Tangan kamu udah lecet kak megang kapak terus! 🥵🪓\n\n\
             Nebang butuh *{STAMINA_COST} Stamina*, kamu cuma punya *{stamina}*. Istirahat kek! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let mut inv = inventory_map(&ctx);
    let mut results: Vec<(&'static str, i64)> = Vec::new();
    for &(item, name, chance, min, max) in DROPS.iter() {
        if math_random() * 100.0 <= chance {
            let qty = roll_incl(min, max);
            let slot = inv.entry(item.to_string()).or_insert(serde_json::json!(0));
            let have = slot.as_i64().unwrap_or(0);
            *slot = serde_json::json!(have + qty);
            results.push((name, qty));
        }
    }
    if results.is_empty() {
        let slot = inv.entry("wood".to_string()).or_insert(serde_json::json!(0));
        let have = slot.as_i64().unwrap_or(0);
        *slot = serde_json::json!(have + 1);
        results.push(("🪵 Kayu", 1));
    }

    let exp_gain = roll_incl(50, 249);
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    let mut txt = String::from("POHONNYA TUMBANG! 🪓✨\n\nKamu mungutin barang-barang ini:\n");
    for (name, qty) in &results {
        txt += &format!("• {name}: *+{qty}*\n");
    }
    txt += &format!(
        "\n📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: *-{STAMINA_COST}*\n\n\
         Awas encok kak, kalau capek istirahat (`.heal`) ya! 🥵🍃"
    );

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "exp": new_exp, "rpg": rpg, "inventory": inv }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "woodcut".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("woodcut").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("woodcut").unwrap();
        assert_eq!(p.meta.name, "woodcut");
        assert_eq!(p.meta.alias, ["chop", "nebang", "kayu"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Menebang pohon untuk mendapatkan kayu");
        assert_eq!(p.meta.usage, ".woodcut");
        assert_eq!(p.meta.example, ".woodcut");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["chop", "nebang", "kayu"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "woodcut");
        }
    }

    #[test]
    fn fourteen_stamina_is_refused_and_zero_is_not() {
        let r = run(serde_json::json!({"rpg": {"stamina": 14}}));
        let t = text_of(&r);
        assert!(t.starts_with("Tangan kamu udah lecet kak"), "{t}");
        assert!(t.contains("kamu cuma punya *14*"), "{t}");
        assert!(r.mutations().is_empty());

        let r = run(serde_json::json!({"rpg": {"stamina": 0}, "exp": 0}));
        assert_eq!(written(&r)["rpg"]["stamina"], serde_json::json!(85), "0 || 100 -> 100 - 15");
    }

    #[test]
    fn the_listed_loot_equals_the_written_bag() {
        for _ in 0..400 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
            let t = text_of(&r);
            let inv = written(&r)["inventory"].as_object().unwrap().clone();
            let lines = t.lines().filter(|l| l.starts_with("• ")).count();
            for (item, name, _, _, _) in DROPS {
                let got = inv.get(item).and_then(Value::as_i64).unwrap_or(0);
                assert_eq!(
                    t.contains(&format!("• {name}:")),
                    got > 0,
                    "{item} = {got} tapi barisnya beda"
                );
            }
            //noletiap item bisa gagal drop — yang pasti ada kayu, entah dari
            //drop 2..=5 atau dari fallback 1.
            assert!(lines > 0, "tidak boleh nol drop sama sekali: {t}");
        }
    }

    #[test]
    fn an_empty_haul_falls_back_to_a_single_log() {
        for _ in 0..6000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
            let t = text_of(&r);
            // Drop `wood` selalu 2..=5, jadi baris `+1` hanya bisa fallback.
            if t.contains("• 🪵 Kayu: *+1*") {
                assert_eq!(
                    t.lines().filter(|l| l.starts_with("• ")).count(),
                    1,
                    "fallback hanya jalan kalau nol drop: {t}"
                );
                assert_eq!(written(&r)["inventory"]["wood"], serde_json::json!(1));
                return;
            }
        }
        panic!("tidak ada sampel dengan nol drop");
    }

    #[test]
    fn every_stump_costs_fifteen_stamina_and_lands_in_the_exp_band() {
        for _ in 0..300 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
            let d = written(&r);
            assert_eq!(d["rpg"]["stamina"], serde_json::json!(85));
            let exp = d["exp"].as_i64().unwrap();
            assert!((50..=249).contains(&exp), "exp {exp}");
        }
    }
}
