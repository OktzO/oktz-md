//! `plugins/rpg/parkir.js` — tukang parkir minimarket, waspada satpol PP.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, roll_incl, rpg_int, rpg_map,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 12;

#[plugin(
    name = "parkir",
    alias = ["kangparkir", "markir"],
    category = "rpg",
    description = "Jadi tukang parkir minimarket, waspada satpol PP!",
    usage = ".parkir",
    example = ".parkir",
    cooldown = 120,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn parkir(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Kaki pegel kebanyakan berdiri! 😫\n\n\
             Markir butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Istirahat dulu di pos! 🏚️"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let gacha = math_random();

    if gacha < 0.1 {
        let denda = roll_incl(1000, 5999);
        return Reply::text(format!(
            "WADUH ADA RAZIA SATPOL PP! 🚓💨\n\n\
             Kamu lari terbirit-birit dan duit recehanmu jatuh senilai *Rp {}*!\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Apes banget hari ini! 😭",
            thousands_id(denda as f64)
        ))
        .with_reaction("🚨")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": (koin - denda).max(0), "rpg": rpg }),
        });
    }

    if gacha > 0.9 {
        let jackpot = roll_incl(20000, 69999);
        let exp_gain = jackpot / 20;
        let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);
        return Reply::text(format!(
            "JACKPOT! ADA MOBIL SPORT SULTAN! 🏎️✨\n\n\
             Pas mau keluar, kaca mobil turun dan sultan ngasih pecahan 100k!\n\
             💵 Pendapatan: *+Rp {}*\n\
             📈 EXP: *+{exp_gain}*\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Alhamdulillah rejeki anak soleh! 🙏",
            thousands_id(jackpot as f64)
        ))
        .with_reaction("🤑")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin + jackpot, "exp": new_exp, "rpg": rpg }),
        });
    }

    let earning = roll_incl(2000, 9999);
    let exp_gain = earning / 20;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "HASIL MARKIR HARI INI! 🅿️✨\n\n\
         💵 Pendapatan: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Receh demi receh lama-lama jadi bukit! 💪",
        thousands_id(earning as f64)
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": koin + earning, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "parkir".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("parkir").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("parkir").unwrap();
        assert_eq!(p.meta.name, "parkir");
        assert_eq!(p.meta.alias, ["kangparkir", "markir"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Jadi tukang parkir minimarket, waspada satpol PP!"
        );
        assert_eq!(p.meta.usage, ".parkir");
        assert_eq!(p.meta.example, ".parkir");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kangparkir", "markir"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "parkir");
        }
    }

    #[test]
    fn standing_on_twelve_empty_stamina_is_refused() {
        let r = run(serde_json::json!({"rpg": {"stamina": 0}}));
        let t = text_of(&r);
        assert!(t.starts_with("Kaki pegel kebanyakan berdiri!"), "{t}");
        assert!(t.contains("sisa stamina kamu *0*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn all_three_gacha_branches_occur_with_their_own_reward_shapes() {
        let mut razia = 0;
        let mut jackpot = 0;
        let mut biasa = 0;
        for _ in 0..6000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 1000, "exp": 0}));
            let d = written(&r);
            let koin = d["koin"].as_i64().unwrap();
            let t = text_of(&r);
            if t.starts_with("WADUH ADA RAZIA") {
                razia += 1;
                // `Math.max(0, koin - denda)` — denda bisa melampaui saldo.
                assert!((0..=1000).contains(&koin), "sisa {koin}");
                assert!(d.get("exp").is_none());
            } else if t.starts_with("JACKPOT") {
                jackpot += 1;
                assert!((21000..=70999).contains(&koin), "jackpot {koin}");
                assert_eq!(d["exp"].as_i64().unwrap(), (koin - 1000) / 20);
            } else {
                biasa += 1;
                assert!((3000..=10999).contains(&koin), "biasa {koin}");
                assert_eq!(d["exp"].as_i64().unwrap(), (koin - 1000) / 20);
            }
            assert_eq!(d["rpg"]["stamina"], serde_json::json!(88));
        }
        assert!(razia > 0 && jackpot > 0 && biasa > 0, "{razia}/{jackpot}/{biasa}");
    }

    #[test]
    fn a_razia_on_an_empty_purse_leaves_zero_not_a_negative_balance() {
        for _ in 0..6000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            if text_of(&r).starts_with("WADUH ADA RAZIA") {
                assert_eq!(written(&r)["koin"], serde_json::json!(0), "Math.max(0, ...)");
                return;
            }
        }
        panic!("cabang razia tidak terjadi");
    }
}
