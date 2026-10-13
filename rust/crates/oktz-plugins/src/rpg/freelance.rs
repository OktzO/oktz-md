//! `plugins/rpg/freelance.js` — projects klien: 20% di-ghosting, 15% dibayar
//! dolar, sisanya lokal.
//!
//! Kursor: `staminaCost` dipotong SEBELUM lemparan, jadi cabang rugi pun
//! tetap Bahn. Harga dolar tetap 16.000 (kurs beku di source).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;
use serde_json::json;

use crate::rpg::heal::{
    apply_exp, coalesce, math_random, or_default, rpg_int, rpg_map, roll_incl,
};
use crate::tools::kalkulatormbg::thousands_id;

const STAMINA_COST: i64 = 25;
const DOLLAR_RATE: i64 = 16_000;

#[plugin(
    name = "freelance",
    alias = ["desain", "koding"],
    category = "rpg",
    description = "Mengerjakan project online klien bule",
    usage = ".freelance",
    example = ".freelance",
    cooldown = 200,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn freelance(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    // `user.rpg.stamina ?? 100` — `??`, bukan `||`.
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Otak ngebul mikirin error! 🤯\n\n\
             Freelance butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Refreshing dulu woi! 🌿"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = crate::common::int_field(&ctx, "koin");
    let gacha = math_random();

    // Cabang rugi: koin dan exp sama sekali tak tersentuh, hanya stamina.
    if gacha < 0.2 {
        return Reply::text(format!(
            "KLIEN KABUR GAK MAU BAYAR! 📉😡\n\n\
             Udah dikerjain begadang 3 hari 3 malem, eh malah di-ghosting!\n\
             💵 Bayaran: 0\n\
             ⚡ Stamina melayang: -{STAMINA_COST}\n\n\
             Apes banget, lain kali harus pake DP! 😭"
        ))
        .with_reaction("📉")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "rpg": rpg }),
        });
    }

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    if gacha > 0.85 {
        let payment = roll_incl(5, 14);
        let total = payment * DOLLAR_RATE;
        let exp_gain = total / 30;
        let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

        return Reply::text(format!(
            "DIBAYAR PAKE DOLLAR OLEH BULE! 💸✨\n\n\
             Klien luar negeri puas banget dan ngasih ${payment}!\n\
             💵 Bayaran: *+Rp {}*\n\
             📈 EXP: *+{exp_gain}*\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Uang jajan sultan menanti! 🤑",
            thousands_id(total as f64)
        ))
        .with_reaction("💸")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin + total, "exp": new_exp, "rpg": rpg }),
        });
    }

    let earning = roll_incl(15_000, 54_999);
    let exp_gain = earning / 20;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "PROJECT SELESAI DAN ACC! 💻✨\n\n\
         💵 Bayaran Lokal: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Lumayan buat beli kopi kapal api! ☕",
        thousands_id(earning as f64)
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin + earning, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "freelance".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("freelance").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("freelance").unwrap();
        assert_eq!(p.meta.name, "freelance");
        assert_eq!(p.meta.alias, ["desain", "koding"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Mengerjakan project online klien bule");
        assert_eq!(p.meta.usage, ".freelance");
        assert_eq!(p.meta.example, ".freelance");
        assert_eq!(p.meta.cooldown, 200);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["desain", "koding"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "freelance");
        }
    }

    #[test]
    fn a_burned_out_brain_blocks_the_project() {
        for stamina in [0, 24] {
            let r = run(json!({ "rpg": { "stamina": stamina } }));
            let t = text_of(&r);
            assert!(t.starts_with("Otak ngebul mikirin error!"), "stamina {stamina}: {t}");
            assert!(t.contains(&format!("sisa stamina kamu *{stamina}*")), "{t}");
            assert!(r.mutations().is_empty(), "stamina {stamina}");
        }
    }

    #[test]
    fn all_three_outcomes_appear_and_each_costs_the_stamina() {
        let (mut ghosted, mut dollar, mut local) = (0, 0, 0);
        for _ in 0..1200 {
            let r = run(json!({ "rpg": { "stamina": 100 }, "koin": 0, "exp": 0 }));
            let t = text_of(&r);
            let data = write_of(&r);
            assert_eq!(data["rpg"]["stamina"], json!(75), "{t}");
            if t.starts_with("KLIEN KABUR") {
                ghosted += 1;
                assert!(data.get("koin").is_none(), "klien tak bayar: {data}");
                assert!(data.get("exp").is_none(), "klien tak dapat exp");
            } else if t.starts_with("DIBAYAR PAKE DOLLAR") {
                dollar += 1;
                let paid = data["koin"].as_i64().unwrap();
                assert_eq!(paid % DOLLAR_RATE, 0, "bayar {paid} bukan kelipatan 16.000");
                let dollars = paid / DOLLAR_RATE;
                assert!((5..=14).contains(&dollars), "dolar {dollars}");
                assert_eq!(data["exp"], json!(paid / 30));
                assert!(t.contains(&format!("ngasih ${dollars}")), "{t}");
            } else {
                local += 1;
                let paid = data["koin"].as_i64().unwrap();
                assert!((15_000..=54_999).contains(&paid), "bayar {paid}");
                assert_eq!(data["exp"], json!(paid / 20));
                assert!(t.contains(&format!("+Rp {}", thousands_id(paid as f64))), "{t}");
            }
        }
        assert!(ghosted > 0 && dollar > 0 && local > 0, "ghosted={ghosted} dollar={dollar} local={local}");
    }

    /// `user.rpg.stamina ?? 100` — `??` hanya menutup null/undefined, jadi nol
    /// TETAP nol dan langsung memblokir (beda dari fishing/hunt yang pakai `||`).
    #[test]
    fn a_zero_stamina_stays_zero_because_nullish_coalescing_keeps_it() {
        let r = run(json!({ "rpg": { "stamina": 0 } }));
        let t = text_of(&r);
        assert!(t.starts_with("Otak ngebul mikirin error!"), "{t}");
        assert!(t.contains("sisa stamina kamu *0*"), "{t}");
        assert!(r.mutations().is_empty(), "nol tak jadi 100");

        // `null` dan field yang hilang justru jatuh ke 100, lalu lolos gerbang.
        for user in [json!({ "rpg": { "stamina": null } }), json!({ "rpg": {} })] {
            assert!(!text_of(&run(user.clone())).starts_with("Otak ngebul"), "{user}");
        }
    }

    #[test]
    fn a_missing_user_row_never_panics() {
        for user in [json!(null), json!({}), json!({ "rpg": null })] {
            let r = run(user.clone());
            assert!(!text_of(&r).is_empty(), "input {user}");
        }
    }
}