//! `plugins/rpg/mining.js` — menambang untuk mendapatkan ores dan gems.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//!
//! Catatan kursor: guard stamina memakai `user.rpg.stamina || 100`, BUKAN `??`.
//! Artinya stamina 0 diganti 100 dan penambang tetap boleh masuk — bug JS yang
//! sengaja dipertahankan karena `woodcut.js`/`work.js` memakai pola yang sama.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::rpg::jualan::{
    apply_exp, inventory_map, math_random, or_default, rpg_int, rpg_map, roll_incl,
};

pub(crate) const STAMINA_COST: i64 = 20;

/// `drops` (`mining.js:40-47`) — `(item, nama, chance persen, min, max)`.
pub(crate) const DROPS: [(&str, &str, f64, i64, i64); 6] = [
    ("rock", "🪨 Batu", 80.0, 2, 5),
    ("coal", "⚫ Batubara", 50.0, 1, 3),
    ("iron", "⛓️ Besi", 30.0, 1, 2),
    ("gold", "🥇 Emas", 15.0, 1, 1),
    ("diamond", "💠 Berlian", 5.0, 1, 1),
    ("emerald", "💚 Emerald", 2.0, 1, 1),
];

#[plugin(
    name = "mining",
    alias = ["mine", "tambang"],
    category = "rpg",
    description = "Menambang untuk mendapatkan ores dan gems",
    usage = ".mining",
    example = ".mining",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn mining(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    // `|| 100`: nol dianggap falsy, jadi bar yang habis tetap boleh menambang.
    let stamina = or_default(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Aduh kak, badan kamu udah remuk duluan! 🥵\n\n\
             Nambang batu tuh berat, butuh *{STAMINA_COST} Stamina*. Stamina kamu sisa *{stamina}* doang. Istirahat gih! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let mut inv = inventory_map(&ctx);
    let mut results: Vec<(&'static str, i64)> = Vec::new();
    for &(item, name, chance, min, max) in DROPS.iter() {
        // `<=` di JS, bukan `<` — peluang 80 berarti praktis 80%.
        if math_random() * 100.0 <= chance {
            let qty = roll_incl(min, max);
            let slot = inv.entry(item.to_string()).or_insert(json!(0));
            let have = slot.as_i64().unwrap_or(0);
            *slot = json!(have + qty);
            results.push((name, qty));
        }
    }
    if results.is_empty() {
        let slot = inv.entry("rock".to_string()).or_insert(json!(0));
        let have = slot.as_i64().unwrap_or(0);
        *slot = json!(have + 1);
        results.push(("🪨 Batu", 1));
    }

    let exp_gain = roll_incl(100, 599);
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    let mut txt = String::from("CROOT! BATUNYA PECAH KAK! ⛏️✨\n\nKamu berhasil dapetin material ini:\n");
    for (name, qty) in &results {
        txt += &format!("• {name}: *+{qty}*\n");
    }
    txt += &format!(
        "\n📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: *-{STAMINA_COST}*\n\n\
         Simpen baik-baik ya kak, nanti bisa dicraft atau dijual! 💎💰"
    );

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "exp": new_exp, "rpg": rpg, "inventory": inv }),
        })
}

/// Dipakai test: total inventory dari daftar hasil.
#[cfg(test)]
fn total(inv: &serde_json::Map<String, Value>, item: &str) -> i64 {
    inv.get(item).and_then(Value::as_i64).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mining".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("mining").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("mining").unwrap();
        assert_eq!(p.meta.name, "mining");
        assert_eq!(p.meta.alias, ["mine", "tambang"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Menambang untuk mendapatkan ores dan gems");
        assert_eq!(p.meta.usage, ".mining");
        assert_eq!(p.meta.example, ".mining");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mine", "tambang"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mining");
        }
    }

    #[test]
    fn an_empty_stamina_bar_still_lets_the_miner_in_because_the_js_uses_or() {
        let r = run(json!({"rpg": {"stamina": 0}, "exp": 0}));
        let t = text_of(&r);
        assert!(!t.starts_with("Aduh kak"), "0 || 100 -> 100: {t}");
        assert_eq!(written(&r)["rpg"]["stamina"], json!(80), "100 - 20");
    }

    #[test]
    fn nineteen_stamina_is_not_enough() {
        let r = run(json!({"rpg": {"stamina": 19}}));
        let t = text_of(&r);
        assert!(t.starts_with("Aduh kak, badan kamu udah remuk duluan!"), "{t}");
        assert!(t.contains("Stamina kamu sisa *19* doang"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_swing_costs_exactly_twenty_stamina() {
        for _ in 0..200 {
            let r = run(json!({"rpg": {"stamina": 100}, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(80));
        }
    }

    #[test]
    fn the_reply_lines_always_match_the_written_inventory() {
        for _ in 0..400 {
            let r = run(json!({"rpg": {"stamina": 100}, "exp": 0, "inventory": {"rock": 5}}));
            let t = text_of(&r);
            let inv = written(&r)["inventory"].as_object().unwrap().clone();
            for (item, name, _, min, max) in DROPS {
                let baseline = if item == "rock" { 5 } else { 0 };
                let line = t.lines().find(|l| l.contains(&format!("• {name}:")));
                match line {
                    Some(l) => {
                        let qty: i64 = l
                            .rsplit("*+")
                            .next()
                            .unwrap()
                            .trim_end_matches('*')
                            .parse()
                            .unwrap();
                        // `rock` punya dua sumber: drop 2..=5, atau fallback
                        // qty 1 kalau TIDAK ADA drop sama sekali.
                        assert!(
                            (min..=max).contains(&qty)
                                || (item == "rock" && qty == 1),
                            "{item}: {qty} di luar {min}..={max}"
                        );
                        assert_eq!(
                            total(&inv, item),
                            baseline + qty,
                            "{item}: baris dan tas harus sama"
                        );
                    }
                    None => assert_eq!(
                        total(&inv, item),
                        baseline,
                        "{item} disebut tapi tak ditulis"
                    ),
                }
            }
        }
    }

    #[test]
    fn an_empty_haul_still_yields_one_rock_and_a_listed_line() {
        for _ in 0..6000 {
            let r = run(json!({"rpg": {"stamina": 100}, "exp": 0}));
            let t = text_of(&r);
            // Drop `rock` selalu 2..=5, jadi baris `+1` hanya bisa fallback.
            if t.contains("• 🪨 Batu: *+1*") {
                assert_eq!(
                    t.lines().filter(|l| l.starts_with("• ")).count(),
                    1,
                    "fallback hanya jalan kalau nol drop: {t}"
                );
                assert_eq!(written(&r)["inventory"]["rock"], json!(1));
                return;
            }
        }
        panic!("tidak ada sampel dengan nol drop");
    }

    #[test]
    fn experience_is_always_inside_the_documented_band() {
        for _ in 0..400 {
            let r = run(json!({"rpg": {"stamina": 100}, "exp": 0}));
            let t = text_of(&r);
            let exp: i64 = t
                .lines()
                .find_map(|l| l.strip_prefix("📈 EXP: *+"))
                .and_then(|s| s.trim_end_matches('*').parse().ok())
                .unwrap();
            assert!((100..=599).contains(&exp), "exp {exp}");
            assert_eq!(written(&r)["exp"], json!(exp));
        }
    }
}
