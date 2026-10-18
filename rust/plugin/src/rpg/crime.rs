//! `plugins/rpg/crime.js` — bobol ATM, 50% berhasil.
//!
//! Kursor: denda dibatasi saldo yang ada (`Math.min(fine, user.koin || 0)`)
//! dan `Math.max(0, ...)` lagi diftykai setelahnya, jadi koin tak pernah
//! negatif; HP juga dijepit di 0 (berbeda dari `boss` yang minimal 1).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::rpg::heal::{apply_exp, math_random, or_default, rpg_int, rpg_map, roll_incl};
use crate::tools::kalkulatormbg::thousands_id;

const SUCCESS_RATE: f64 = 0.5;

#[plugin(
    name = "crime",
    alias = ["jahat"],
    category = "rpg",
    description = "Melakukan kejahatan membobol ATM (risiko tinggi)",
    usage = ".crime",
    example = ".crime",
    cooldown = 300,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn crime(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let koin = crate::common::int_field(&ctx, "koin");

    if math_random() < SUCCESS_RATE {
        let stolen = roll_incl(5_000, 19_999);
        let exp_gain = stolen / 20;
        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

        return Reply::text(format!(
            "HACKING SUKSES!! 💻💵\n\n\
             Mesin ATM ngeluarin duit kayak air hujan! Lu langsung kabur bawa koper penuh duit.\n\n\
             💰 Hasil Bobol: *+Rp {}*\n\
             📈 EXP Kriminal: *+{exp_gain}*",
            thousands_id(stolen as f64)
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin + stolen, "exp": new_exp, "rpg": rpg }),
        });
    }

    let fine = roll_incl(5_000, 14_999).min(koin);
    let left_hp = (or_default(rpg_int(&rpg, "health"), 100) - 15).max(0);
    rpg.insert("health".into(), serde_json::json!(left_hp));

    Reply::text(format!(
        "NGIIING NGIING!! ALARM BUNYI!! 🚨🚓\n\n\
         Sialan, mesinnya error dan polisi langsung ngepung dari segala arah!\n\
         Lu dipentung pake tongkat polisi terus dipaksa bayar denda.\n\n\
         💸 Denda Pidana: *-Rp {}*\n\
         🤕 Memar Kena Pentung: *-15 HP*",
        thousands_id(fine as f64)
    ))
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": (koin - fine).max(0), "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "crime".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("crime").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("crime").unwrap();
        assert_eq!(p.meta.name, "crime");
        assert_eq!(p.meta.alias, ["jahat"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Melakukan kejahatan membobol ATM (risiko tinggi)");
        assert_eq!(p.meta.usage, ".crime");
        assert_eq!(p.meta.example, ".crime");
        assert_eq!(p.meta.cooldown, 300);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("jahat").unwrap().meta.name, "crime");
    }

    #[test]
    fn both_branches_appear_and_each_writes_exactly_one_row() {
        let (mut win, mut caught) = (0, 0);
        for _ in 0..600 {
            let r = run(serde_json::json!({ "koin": 100_000, "exp": 0 }));
            let t = text_of(&r);
            let data = write_of(&r);
            if t.starts_with("HACKING SUKSES!!") {
                win += 1;
                let stolen = data["koin"].as_i64().unwrap() - 100_000;
                assert!((5_000..=19_999).contains(&stolen), "curian {stolen}");
                assert_eq!(data["exp"], serde_json::json!(stolen / 20), "exp = curian / 20");
                assert!(t.contains(&format!("*+Rp {}*", thousands_id(stolen as f64))), "{t}");
                assert!(data["rpg"].get("health").is_none(), "k menang tak kena HP");
            } else {
                caught += 1;
                let fine = 100_000 - data["koin"].as_i64().unwrap();
                assert!((5_000..=14_999).contains(&fine), "denda {fine}");
                let health = data["rpg"]["health"].as_i64().unwrap();
                assert!((85..=100).contains(&health), "HP {health}");
                assert!(t.contains(&format!("*-Rp {}*", thousands_id(fine as f64))), "{t}");
                assert!(data.get("exp").is_none(), "kalah tak dapat exp");
            }
        }
        assert!(win > 0 && caught > 0, "win={win} caught={caught}");
    }

    /// `Math.min(fine, user.koin || 0)` lalu `Math.max(0, ...)` — dompet
    /// tipis hanya kena denda sebesar yang ada.
    #[test]
    fn the_fine_never_exceeds_the_purse_and_never_goes_negative() {
        for purse in [0, 1_000] {
            for _ in 0..300 {
                let r = run(serde_json::json!({ "koin": purse, "rpg": { "health": 100 } }));
                if text_of(&r).starts_with("NGIIING") {
                    let after = write_of(&r)["koin"].as_i64().unwrap();
                    assert_eq!(after, 0, "dompet {purse} -> {after}");
                }
            }
        }
    }

    #[test]
    fn the_health_floor_is_zero_here_not_one_as_in_boss() {
        let mut floored = false;
        for _ in 0..600 {
            let r = run(serde_json::json!({ "koin": 50_000, "rpg": { "health": 3 } }));
            if text_of(&r).starts_with("NGIIING") {
                assert_eq!(write_of(&r)["rpg"]["health"], serde_json::json!(0), "Math.max(0, 3 - 15)");
                floored = true;
            }
        }
        assert!(floored, "dalam 600 percobaan harus ada yang menyentuh lantai");
    }

    #[test]
    fn a_missing_user_row_never_panics() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({ "koin": null })] {
            let r = run(user.clone());
            assert!(!text_of(&r).is_empty(), "input {user}");
        }
    }
}