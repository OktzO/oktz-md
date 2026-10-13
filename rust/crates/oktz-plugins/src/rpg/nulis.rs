//! `plugins/rpg/nulis.js` — nulis cerpen buat royalti. Command-nya `cerpen`,
//! bukan `nulis` — nama plugin ikut apa yang tertulis di `pluginConfig`.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, rpg_int, rpg_map, roll_incl,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 10;

#[plugin(
    name = "cerpen",
    alias = ["author", "wattpad"],
    category = "rpg",
    description = "Nulis cerpen atau artikel untuk dapet royalti",
    usage = ".cerpen",
    example = ".cerpen",
    cooldown = 120,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn cerpen(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Ide buntu, writer block menyerang! 😵‍💫\n\n\
             Nulis butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Cari inspirasi dulu! 💡"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let gacha = math_random();

    if gacha < 0.15 {
        // Naskah ditolak: stamina tetap hangsus, royalti dan exp nol.
        return Reply::text(format!(
            "NASKAH DITOLAK PENERBIT! 🚮🥺\n\n\
             Alasannya: \"Ceritanya terlalu klise dan pasaran.\"\n\
             💵 Royalti: 0\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Jangan menyerah, besok nulis lagi! 💪"
        ))
        .with_reaction("🚮")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "rpg": rpg }),
        });
    }

    if gacha > 0.9 {
        let royalti = roll_incl(30000, 89999);
        let exp_gain = royalti / 20;
        let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);
        return Reply::text(format!(
            "CERITAMU VIRAL DAN JADI BEST SELLER! 🌟📘\n\n\
             Banyak yang nangis bombay baca karya kamu, royalti ngalir deres!\n\
             💵 Royalti: *+Rp {}*\n\
             📈 EXP: *+{exp_gain}*\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Otw dilirik sutradara buat difilmin! 🎬",
            thousands_id(royalti as f64)
        ))
        .with_reaction("🌟")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": koin + royalti, "exp": new_exp, "rpg": rpg }),
        });
    }

    let royalti = roll_incl(5000, 19999);
    let exp_gain = royalti / 20;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "ROYALTI HASIL NULIS CAIR! 📝✨\n\n\
         💵 Pendapatan: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Semangat berkarya para pujangga! 🎓",
        thousands_id(royalti as f64)
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin + royalti, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "cerpen".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("cerpen").unwrap().handler)(ctx)
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
    fn the_plugin_is_named_after_the_config_not_after_the_file() {
        let p = oktz_plugin_api::find("cerpen").unwrap();
        assert_eq!(p.meta.name, "cerpen");
        assert_eq!(p.meta.alias, ["author", "wattpad"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Nulis cerpen atau artikel untuk dapet royalti"
        );
        assert_eq!(p.meta.usage, ".cerpen");
        assert_eq!(p.meta.example, ".cerpen");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
        assert!(
            oktz_plugin_api::find("nulis").is_none(),
            "`nulis` bukan nama command di JS"
        );
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["author", "wattpad"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "cerpen");
        }
    }

    #[test]
    fn writer_s_block_sends_the_author_back_to_bed() {
        let r = run(json!({"rpg": {"stamina": 9}}));
        let t = text_of(&r);
        assert!(t.starts_with("Ide buntu, writer block menyerang!"), "{t}");
        assert!(t.contains("sisa stamina kamu *9*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_draft_costs_ten_stamina() {
        for _ in 0..200 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(90));
        }
    }

    #[test]
    fn the_three_outcomes_keep_their_own_royalty_range() {
        let mut ditolak = 0;
        let mut viral = 0;
        let mut biasa = 0;
        for _ in 0..4000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            let t = text_of(&r);
            if t.starts_with("NASKAH DITOLAK") {
                ditolak += 1;
                assert!(d.get("koin").is_none() && d.get("exp").is_none(), "{t}");
            } else if t.starts_with("CERITAMU VIRAL") {
                viral += 1;
                let koin = d["koin"].as_i64().unwrap();
                assert!((30000..=89999).contains(&koin), "{koin}");
                assert_eq!(d["exp"], json!(koin / 20));
            } else {
                biasa += 1;
                let koin = d["koin"].as_i64().unwrap();
                assert!((5000..=19999).contains(&koin), "{koin}");
                assert_eq!(d["exp"], json!(koin / 20));
            }
        }
        assert!(ditolak > 0 && viral > 0 && biasa > 0, "{ditolak}/{viral}/{biasa}");
    }
}
