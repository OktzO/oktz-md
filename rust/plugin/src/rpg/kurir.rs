//! `plugins/rpg/kurir.js` — nganter paket orang, awas anjing galak.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`; yang
//! dikirim adalah hasil akhir beserta reaksi terakhir.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{apply_exp, coalesce, math_random, or_default, roll_incl, rpg_int, rpg_map};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 15;

/// `items[...]` (`kurir.js:51`) — paket yang diantar.
pub(crate) const ITEMS: [&str; 4] = [
    "Dokumen Rahasia",
    "Baju Online",
    "Skincare Bini Orang",
    "Panci Emak-emak",
];

#[plugin(
    name = "kurir",
    alias = ["antar", "paket"],
    category = "rpg",
    description = "Nganter paket orang, awas anjing galak!",
    usage = ".kurir",
    example = ".kurir",
    cooldown = 120,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn kurir(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Pinggang encok kebanyakan bawa kardus! 😩\n\n\
             Kurir butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Ngurut dulu gih! 💆‍♂️"
        ));
    }
    let stamina = stamina - STAMINA_COST;
    rpg.insert("stamina".into(), serde_json::json!(stamina));

    let koin = int_field(&ctx, "koin");
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let gacha = math_random();

    if gacha < 0.2 {
        let extra = 10;
        rpg.insert("stamina".into(), serde_json::json!((stamina - extra).max(0)));
        let new_exp = apply_exp(&mut rpg, old_exp, 500);
        return Reply::text(format!(
            "GUK GUK GUK! DIKEJAR ANJING GALAK! 🐕💨\n\n\
             Kamu lari keliling komplek demi nyelametin paket orang!\n\
             ⚡ Stamina Tambahan: -{extra}\n\
             📈 EXP Kompensasi Lari: *+500*\n\
             💵 Pendapatan: 0 (Paketnya dilempar ke pagar)\n\n\
             Nafas ngos-ngosan banget asli! 🥵"
        ))
        .with_reaction("🐕")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "exp": new_exp, "rpg": rpg }),
        });
    }

    let item = ITEMS[crate::common::random_index(ITEMS.len())];
    let earning = roll_incl(5000, 14999);
    let tips = if gacha > 0.8 {
        roll_incl(2000, 11999)
    } else {
        0
    };
    let total = earning + tips;
    let exp_gain = total / 20;
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    let mut txt = format!(
        "ALHAMDULILLAH PAKET SAMPAI! 📦✨\n\n\
         Barang: *{item}*\n\
         💵 Ongkir: *+Rp {}*\n",
        thousands_id(earning as f64)
    );
    if tips > 0 {
        txt += &format!("🎁 Tips Tambahan: *+Rp {}*\n", thousands_id(tips as f64));
    }
    txt += &format!(
        "📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Berhasil nganter tepat waktu! 🚚💨"
    );

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": koin + total, "exp": new_exp, "rpg": rpg }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "kurir".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("kurir").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("kurir").unwrap();
        assert_eq!(p.meta.name, "kurir");
        assert_eq!(p.meta.alias, ["antar", "paket"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Nganter paket orang, awas anjing galak!");
        assert_eq!(p.meta.usage, ".kurir");
        assert_eq!(p.meta.example, ".kurir");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["antar", "paket"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "kurir");
        }
    }

    #[test]
    fn zero_stamina_stays_zero_and_is_reported_verbatim() {
        let r = run(serde_json::json!({"rpg": {"stamina": 0}}));
        let t = text_of(&r);
        assert!(t.starts_with("Pinggang encok kebanyakan bawa kardus!"), "{t}");
        assert!(t.contains("sisa stamina kamu *0*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_missing_stamina_row_defaults_to_the_full_bar() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"rpg": {}})] {
            let r = run(user.clone());
            assert!(!text_of(&r).starts_with("Pinggang encok"), "{user}");
            let d = written(&r);
            let stamina = d["rpg"]["stamina"].as_i64().unwrap();
            // 85 kalau pengantaran sukses, 75 kalau dikejar anjing (biaya 15
            // + penalti 10).
            assert!(stamina == 85 || stamina == 75, "{user}: stamina {stamina}");
        }
    }

    #[test]
    fn the_delivery_branch_costs_fifteen_stamina_and_earns_in_range() {
        for _ in 0..300 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            let t = text_of(&r);
            if t.starts_with("GUK GUK GUK") {
                continue;
            }
            assert_eq!(d["rpg"]["stamina"], serde_json::json!(85));
            let koin = d["koin"].as_i64().unwrap();
            assert!((5000..=26998).contains(&koin), "total {koin}");
            let exp = d["exp"].as_i64().unwrap();
            assert_eq!(exp, koin / 20);
            assert!(ITEMS.iter().any(|i| t.contains(i)), "barang tak dikenal: {t}");
        }
    }

    #[test]
    fn the_tips_line_appears_only_when_the_gacha_passes_eighty_percent() {
        let mut with_tips = 0;
        let mut without_tips = 0;
        for _ in 0..3000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            if t.starts_with("GUK GUK GUK") {
                continue;
            }
            if t.contains("Tips Tambahan") {
                with_tips += 1;
                // tips 2000..=11999, ongkir 5000..=14999.
                let koin = written(&r)["koin"].as_i64().unwrap();
                assert!((7000..=26998).contains(&koin), "total dengan tips {koin}");
            } else {
                without_tips += 1;
                assert!(written(&r)["koin"].as_i64().unwrap() <= 14999);
            }
        }
        assert!(with_tips > 0 && without_tips > 0, "kedua varian harus muncul");
    }

    #[test]
    fn being_chased_costs_a_second_chunk_of_stamina_and_pays_flat_exp() {
        for _ in 0..3000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 999, "exp": 0}));
            let d = written(&r);
            if text_of(&r).starts_with("GUK GUK GUK") {
                assert_eq!(d["rpg"]["stamina"], serde_json::json!(75), "100 - 15 - 10");
                assert_eq!(d["exp"], serde_json::json!(500));
                assert!(d.get("koin").is_none(), "tidak dapat uang");
                break;
            }
        }
    }

    #[test]
    fn the_extra_stamina_penalty_never_dips_below_zero() {
        // stamina 15 -> 0 setelah biaya 15, penalti 10 tidak boleh menjatuhkannya.
        for _ in 0..3000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 15}, "koin": 0, "exp": 0}));
            let d = written(&r);
            assert!(d["rpg"]["stamina"].as_i64().unwrap() >= 0);
        }
    }
}
