//! `plugins/rpg/nyapu.js` — nyapu jalan, siapa tau nemu barang jatuh.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, roll_incl, rpg_int, rpg_map,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 10;

#[plugin(
    name = "nyapu",
    alias = ["cleaning", "bersih"],
    category = "rpg",
    description = "Nyapu jalan, siapa tau nemu barang jatuh!",
    usage = ".nyapu",
    example = ".nyapu",
    cooldown = 120,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn nyapu(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Tangan pegel megang sapu terus! 😖\n\n\
             Nyapu butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Istirahat di bawah pohon dulu! 🌳"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);

    // Cincin emas: Found branch WITHOUT exp — `addExpWithLevelCheck` tidak
    // dipanggil di sini, jadi exp benar-benar tidak bertambah.
    if math_random() < 0.1 {
        let gold_found = roll_incl(15000, 64999);
        return Reply::text(format!(
            "HOKI PARAH! NEMU CINCIN EMAS JATUH! 💍✨\n\n\
             Pas lagi nyapu pinggir trotoar, kamu nemu cincin emas dan langsung dijual!\n\
             💵 Pendapatan Kaget: *+Rp {}*\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Rejeki nomplok emang nggak kemana! 🥳",
            thousands_id(gold_found as f64)
        ))
        .with_reaction("💍")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin + gold_found, "rpg": rpg }),
        });
    }

    let earning = roll_incl(3000, 10999);
    let exp_gain = earning / 20;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "ALHAMDULILLAH SELESAI BERES-BERES! 🧹✨\n\n\
         💵 Gaji Harian: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Bumi makin bersih dan asri! 🌍",
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
            command: "nyapu".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("nyapu").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("nyapu").unwrap();
        assert_eq!(p.meta.name, "nyapu");
        assert_eq!(p.meta.alias, ["cleaning", "bersih"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Nyapu jalan, siapa tau nemu barang jatuh!");
        assert_eq!(p.meta.usage, ".nyapu");
        assert_eq!(p.meta.example, ".nyapu");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["cleaning", "bersih"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "nyapu");
        }
    }

    #[test]
    fn a_tired_sweeper_is_refused_without_a_write() {
        let r = run(json!({"rpg": {"stamina": 0}}));
        assert!(text_of(&r).contains("sisa stamina kamu *0*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_sweep_costs_ten_stamina() {
        for _ in 0..200 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(90));
        }
    }

    #[test]
    fn the_wage_branch_pays_and_trains_in_step() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            if text_of(&r).starts_with("HOKI PARAH") {
                continue;
            }
            let koin = d["koin"].as_i64().unwrap();
            assert!((3000..=10999).contains(&koin), "gaji {koin}");
            assert_eq!(d["exp"], json!(koin / 20));
        }
    }

    #[test]
    fn the_ring_pays_more_than_a_whole_week_of_wages_and_gives_no_exp() {
        let mut seen = false;
        for _ in 0..4000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 777}));
            let d = written(&r);
            if text_of(&r).starts_with("HOKI PARAH") {
                seen = true;
                let koin = d["koin"].as_i64().unwrap();
                assert!((15000..=64999).contains(&koin), "temuan {koin}");
                assert!(d.get("exp").is_none(), "exp harus utuh");
                assert!(text_of(&r).contains("⚡ Stamina: -10"));
                break;
            }
        }
        assert!(seen, "gacha < 0.1 harusnya muncul dalam 4000 sampel");
    }
}
