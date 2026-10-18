//! `plugins/rpg/ngemis.js` — ngemis di jalanan, ada peluang dapat nasi bungkus.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, roll_incl, rpg_int, rpg_map,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 5;

#[plugin(
    name = "ngemis",
    alias = ["minta", "gembel"],
    category = "rpg",
    description = "Ngemis di jalanan dengan peluang dapat Nasi Bungkus (Tambah stamina)",
    usage = ".ngemis",
    example = ".ngemis",
    cooldown = 120,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn ngemis(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Tenaga habis buat melas! 🥺\n\n\
             Ngemis butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Nggak kuat mangap lagi... 💔"
        ));
    }
    let stamina = stamina - STAMINA_COST;
    rpg.insert("stamina".into(), serde_json::json!(stamina));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let gacha = math_random();

    if gacha < 0.3 {
        // Nasi Padang: stamina pulih, tidak ada koin dan tidak ada exp.
        let heal = roll_incl(10, 29);
        let healed = (stamina + heal).min(100);
        rpg.insert("stamina".into(), serde_json::json!(healed));
        return Reply::text(format!(
            "ALHAMDULILLAH DIKASIH NASI PADANG! 🍱✨\n\n\
             Ada bapak-bapak baik hati yang ngasih kamu bungkus nasi sisa rapet!\n\
             💖 Stamina bertambah: *+{heal}*\n\
             💵 Uang didapat: 0\n\n\
             Wah, perut kenyang hati senang! 🥰"
        ))
        .with_reaction("🍱")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "rpg": rpg }),
        });
    }

    if gacha > 0.9 {
        return Reply::text(format!(
            "DIUSIR PREMAN PASAR! 💢\n\n\
             \"Woi, ini lapak gue! Pergi lo!\"\n\
             Kamu lari ketakutan tanpa dapet sepeser pun...\n\
             ⚡ Stamina: -{STAMINA_COST}\n\n\
             Susah banget nyari lahan ngemis jaman sekarang! 😭"
        ))
        .with_reaction("💢")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "rpg": rpg }),
        });
    }

    let earning = roll_incl(500, 3499);
    let exp_gain = earning / 10;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "HASIL NGEMIS HARI INI! 🤲✨\n\n\
         💵 Pendapatan Receh: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Bersyukur atas nikmat hari ini, walau receh yang penting halal! 🙏",
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

    /// `find("ngemis")` ambigu: `ngemis` juga alias plugin `beg.js`, dan
    /// `find` mengembalikan siapa pun yang terdaftar lebih dulu di
    /// `distributed_slice`. Cari lewat `meta.name` supaya test ini selalu
    /// menguji plugin yang namanya `ngemis`.
    fn plugin() -> &'static oktz_plugin_api::registry::PluginEntry {
        oktz_plugin_api::all_plugins()
            .iter()
            .copied()
            .find(|e| e.meta.name == "ngemis")
            .expect("plugin ngemis harus terdaftar")
    }

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ngemis".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (plugin().handler)(ctx)
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
        let p = plugin();
        assert_eq!(p.meta.name, "ngemis");
        assert_eq!(p.meta.alias, ["minta", "gembel"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(
            p.meta.description,
            "Ngemis di jalanan dengan peluang dapat Nasi Bungkus (Tambah stamina)"
        );
        assert_eq!(p.meta.usage, ".ngemis");
        assert_eq!(p.meta.example, ".ngemis");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["minta", "gembel"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "ngemis");
        }
    }

    #[test]
    fn four_stamina_is_enough_and_four_is_not() {
        // 5 - 5 = 0, lalu cabang nasi bisa memulihkannya lagi ke 10..29.
        let r = run(serde_json::json!({"rpg": {"stamina": 5}}));
        assert!(written(&r)["rpg"]["stamina"].as_i64().unwrap() <= 29);

        let r = run(serde_json::json!({"rpg": {"stamina": 4}}));
        assert!(text_of(&r).contains("sisa stamina kamu *4*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn all_three_gacha_branches_occur_and_agree_with_the_write() {
        let mut nasi = 0;
        let mut usir = 0;
        let mut receh = 0;
        for _ in 0..4000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            let t = text_of(&r);
            if t.starts_with("ALHAMDULILLAH DIKASIH NASI") {
                nasi += 1;
                assert!(d.get("koin").is_none() && d.get("exp").is_none());
                assert!(d["rpg"]["stamina"].as_i64().unwrap() <= 100);
            } else if t.starts_with("DIUSIR PREMAN") {
                usir += 1;
                assert!(d.get("koin").is_none() && d.get("exp").is_none());
                assert!(d["rpg"]["stamina"].as_i64().unwrap() <= 95);
            } else {
                receh += 1;
                let koin = d["koin"].as_i64().unwrap();
                assert!((500..=3499).contains(&koin), "receh {koin}");
                assert_eq!(d["exp"], serde_json::json!(koin / 10), "exp = koin / 10");
            }
        }
        assert!(nasi > 0 && usir > 0 && receh > 0, "{nasi}/{usir}/{receh}");
    }

    #[test]
    fn nasi_padang_heals_towards_a_hundred_and_never_past_it() {
        for _ in 0..4000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 99}, "koin": 0, "exp": 0}));
            let d = written(&r);
            if text_of(&r).starts_with("ALHAMDULILLAH DIKASIH NASI") {
                assert_eq!(d["rpg"]["stamina"], serde_json::json!(100), "99 - 5 + heal, clamp 100");
                return;
            }
        }
        panic!("cabang nasi padang tidak terjadi");
    }

    #[test]
    fn being_chased_still_pays_the_stamina_cost() {
        for _ in 0..4000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            if text_of(&r).starts_with("DIUSIR PREMAN") {
                assert_eq!(d["rpg"]["stamina"], serde_json::json!(95), "100 - 5");
                return;
            }
        }
        panic!("cabang diusir tidak terjadi");
    }
}
