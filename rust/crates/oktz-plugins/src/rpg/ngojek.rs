//! `plugins/rpg/ngojek.js` — narik ojek untuk mendapat uang.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, math_random, or_default, rpg_int, rpg_map, roll_incl,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 15;

/// `orders` (`ngojek.js:37-43`) — `(tipe, jarak, tarif min, tarif max)`.
pub(crate) const ORDERS: [(&str, &str, i64, i64); 5] = [
    ("🍔 GoFood", "2km", 5000, 15000),
    ("👤 GoRide", "5km", 10000, 25000),
    ("📦 GoSend", "3km", 8000, 20000),
    ("🛒 GoMart", "4km", 12000, 30000),
    ("👥 GoRide Plus", "10km", 20000, 50000),
];

#[plugin(
    name = "ngojek",
    alias = ["ojek", "gojek", "ojol"],
    category = "rpg",
    description = "Ngojek untuk mendapat uang",
    usage = ".ngojek",
    example = ".ngojek",
    cooldown = 120,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn ngojek(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Mesin motornya udah panas banget kak, mending istirahat dulu! 🥵🏍️💨\n\n\
             Ngojek butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}* doang. Ngopi dulu gih! ☕"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    // `distance` hanya muncul di pesan antara yang tidak bisa dikirim di sini.
    let (kind, _distance, min, max) = ORDERS[crate::common::random_index(ORDERS.len())];
    let earning = roll_incl(min, max);
    // `Math.random() > 0.7` — 30% dapat tip.
    let tips = if math_random() > 0.7 {
        roll_incl(1000, 5999)
    } else {
        0
    };
    let total = earning + tips;

    let exp_gain = total / 20;
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    let mut txt = format!(
        "ALHAMDULILLAH ORDERAN KELAR! 🏍️✨\n\n\
         Rincian narik ojol hari ini:\n\
         📱 Tipe: *{kind}*\n\
         💵 Tarif: *+Rp {}*\n",
        thousands_id(earning as f64)
    );
    if tips > 0 {
        txt += &format!(
            "🎁 Tips Customer: *+Rp {}*\n",
            thousands_id(tips as f64)
        );
    }
    txt += &format!(
        "📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: *-{STAMINA_COST}\n\n\
         Lumayan buat nambah-nambahin jajan kak! Semangat narik lagi nanti! 🔥💪"
    );

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": int_field(&ctx, "koin") + total, "exp": new_exp, "rpg": rpg }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ngojek".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("ngojek").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("ngojek").unwrap();
        assert_eq!(p.meta.name, "ngojek");
        assert_eq!(p.meta.alias, ["ojek", "gojek", "ojol"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Ngojek untuk mendapat uang");
        assert_eq!(p.meta.usage, ".ngojek");
        assert_eq!(p.meta.example, ".ngojek");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["ojek", "gojek", "ojol"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "ngojek");
        }
    }

    #[test]
    fn a_hot_engine_ends_the_shift() {
        let r = run(json!({"rpg": {"stamina": 14}}));
        let t = text_of(&r);
        assert!(t.starts_with("Mesin motornya udah panas banget kak"), "{t}");
        assert!(t.contains("sisa stamina kamu *14* doang"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_ride_costs_fifteen_stamina() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], json!(85));
        }
    }

    #[test]
    fn the_fare_stays_inside_the_band_of_the_printed_order_type() {
        let mut seen = [false; 5];
        for _ in 0..2000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            let (_, _, min, max) = ORDERS
                .iter()
                .find(|(kind, _, _, _)| t.contains(&format!("📱 Tipe: *{kind}*")))
                .copied()
                .unwrap_or_else(|| panic!("tipe order tak dikenal: {t}"));
            let fare = fare_of(t);
            assert!((min..=max).contains(&fare), "{t}");
            for (i, (kind, _, _, _)) in ORDERS.iter().enumerate() {
                seen[i] |= t.contains(&format!("📱 Tipe: *{kind}*"));
            }
        }
        assert!(seen.iter().all(|s| *s), "kelima order harus muncul");
    }

    #[test]
    fn the_tip_line_is_the_only_thing_that_separates_the_payout_from_the_fare() {
        let mut with = 0;
        let mut without = 0;
        for _ in 0..3000 {
            let r = run(json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            let koin = written(&r)["koin"].as_i64().unwrap();
            assert_eq!(written(&r)["exp"], json!(koin / 20), "{t}");
            if t.contains("Tips Customer") {
                with += 1;
                assert!((fare_of(t) + 1000..=fare_of(t) + 5999).contains(&koin), "{t}");
            } else {
                without += 1;
                assert_eq!(koin, fare_of(t), "tanpa tip total = tarif: {t}");
            }
        }
        assert!(with > 0 && without > 0, "{with}/{without}");
    }

    fn fare_of(t: &str) -> i64 {
        t.split("💵 Tarif: *+Rp ")
            .nth(1)
            .and_then(|s| s.split('*').next())
            .and_then(|s| s.replace('.', "").parse().ok())
            .unwrap_or_else(|| panic!("tidak ada tarif di: {t}"))
    }
}
