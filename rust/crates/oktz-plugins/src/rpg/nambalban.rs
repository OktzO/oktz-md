//! `plugins/rpg/nambalban.js` — jasa tambal ban, awas ban meledak.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, roll_incl, rpg_int, rpg_map,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 14;

#[plugin(
    name = "nambalban",
    alias = ["tambal", "bengkel"],
    category = "rpg",
    description = "Buka jasa tambal ban, awas ban meledak!",
    usage = ".nambalban",
    example = ".nambalban",
    cooldown = 150,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn nambalban(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Pompa angin macet, tangan kapalan! 🤕\n\n\
             Nambal butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Minum es teh dulu! 🧊"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);

    if math_random() < 0.15 {
        let health_loss = 15;
        let health = coalesce(rpg_int(&rpg, "health"), 100);
        rpg.insert("health".into(), json!((health - health_loss).max(0)));
        return Reply::text(format!(
            "DUAAAAR! BANNYA MELEDAK! 💥😭\n\n\
             Kamu mompa kerasa kenceng dan ban truk itu meledak di depan muka!\n\
             💔 HP berkurang: -{health_loss}\n\
             ⚡ Stamina: -{STAMINA_COST}\n\
             💵 Pendapatan: 0\n\n\
             Muka item kena asep ban, apes bener! 💀"
        ))
        .with_reaction("💥")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "rpg": rpg }),
        });
    }

    let earning = roll_incl(10000, 29999);
    let exp_gain = earning / 25;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "HASIL NAMBAL BAN! 🛠️✨\n\n\
         💵 Pendapatan: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Semoga di jalan nggak bocor lagi tuh orang! 💨",
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

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "nambalban".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("nambalban").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("nambalban").unwrap();
        assert_eq!(p.meta.name, "nambalban");
        assert_eq!(p.meta.alias, ["tambal", "bengkel"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Buka jasa tambal ban, awas ban meledak!");
        assert_eq!(p.meta.usage, ".nambalban");
        assert_eq!(p.meta.example, ".nambalban");
        assert_eq!(p.meta.cooldown, 150);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tambal", "bengkel"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "nambalban");
        }
    }

    #[test]
    fn zero_stamina_is_reported_and_nothing_is_written() {
        let r = run(json!({"rpg": {"stamina": 0}}));
        assert!(text_of(&r).contains("sisa stamina kamu *0*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_exhausted_bar_costs_fourteen_stamina() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 14}, "koin": 0, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(0));
        }
    }

    #[test]
    fn the_pay_branch_matches_its_experience_and_money_ranges() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            if text_of(&r).starts_with("DUAAAAR") {
                continue;
            }
            let koin = d["koin"].as_i64().unwrap();
            assert!((10000..=29999).contains(&koin), "omzet {koin}");
            assert_eq!(d["exp"].as_i64().unwrap(), koin / 25);
            assert!(d.get("rpg").is_some());
        }
    }

    #[test]
    fn the_blowout_costs_fifteen_health_and_pays_nothing() {
        let mut seen = false;
        for _ in 0..4000 {
            let r = run(json!({"rpg": {"stamina": 100, "health": 100}, "koin": 5000}));
            let d = written(&r);
            if text_of(&r).starts_with("DUAAAAR") {
                seen = true;
                assert_eq!(d["rpg"]["stamina"], json!(86));
                assert_eq!(d["rpg"]["health"], json!(85));
                assert!(d.get("koin").is_none(), "pendapatan 0, tidak ditulis");
                assert!(d.get("exp").is_none(), "tanpa exp");
                break;
            }
        }
        assert!(seen, "gacha < 0.15 harusnya muncul dalam 4000 sampel");
    }

    #[test]
    fn health_loss_is_clamped_at_zero() {
        for _ in 0..4000 {
            let r = run(json!({"rpg": {"stamina": 100, "health": 3}, "koin": 0}));
            let d = written(&r);
            if text_of(&r).starts_with("DUAAAAR") {
                assert_eq!(d["rpg"]["health"], json!(0), "Math.max(0, 3 - 15)");
                return;
            }
        }
        panic!("cabang meledak tidak terjadi");
    }
}
