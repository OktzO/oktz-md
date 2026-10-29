//! `plugins/rpg/mulung.js` — memulung untuk mengumpulkan barang.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, inventory_map, or_default, rpg_int, rpg_map, roll_incl,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 15;

/// `drops` (`mulung.js:40-46`) — `(item, nama, min, max)`. `koran` boleh 0,
/// dan qty 0 berarti dilewati: tidak masuk tas, tidak dihitung pendapatan.
pub(crate) const DROPS: [(&str, &str, i64, i64); 5] = [
    ("botol", "🍶 Botol", 1, 10),
    ("kaleng", "🥫 Kaleng", 1, 8),
    ("kardus", "📦 Kardus", 1, 5),
    ("sampah", "🗑️ Sampah", 1, 15),
    ("koran", "📰 Koran", 0, 3),
];

#[plugin(
    name = "mulung",
    alias = ["scavenge", "kumpulsampah"],
    category = "rpg",
    description = "Memulung untuk mengumpulkan barang",
    usage = ".mulung",
    example = ".mulung",
    cooldown = 300,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn mulung(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Yaela kak, mulung aja butuh tenaga kali! 🥵🗑️\n\n\
             Stamina kamu sisa *{stamina}*, padahal butuh *{STAMINA_COST}*. Istirahat gih! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let mut inv = inventory_map(&ctx);
    let mut results: Vec<(&'static str, i64)> = Vec::new();
    let mut money = 0i64;
    for &(item, name, min, max) in DROPS.iter() {
        let qty = roll_incl(min, max);
        if qty > 0 {
            let slot = inv.entry(item.to_string()).or_insert(serde_json::json!(0));
            let have = slot.as_i64().unwrap_or(0);
            *slot = serde_json::json!(have + qty);
            results.push((name, qty));
            // `Math.floor(Math.random() * 50 + 10)` — diacak PER ITEM, jadi
            // barang yang banyak tidak otomatis lebih mahal.
            money += qty * roll_incl(10, 59);
        }
    }

    let exp_gain = roll_incl(50, 249);
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    let mut txt = String::from("ASIK DAPET RONGSOKAN! 🗑️💸\n\nKamu berhasil ngumpulin barang rongsokan nih kak:\n");
    for (name, qty) in &results {
        txt += &format!("• {name}: *+{qty}*\n");
    }
    txt += &format!(
        "\nRongsokannya langsung dijual ke pengepul ya!\n\
         💵 Hasil Jual: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina terpakai: *-{STAMINA_COST}*\n\n\
         Teruslah memulung sampai kaya raya! 🔥🚀",
        thousands_id(money as f64)
    );

    Reply::text(txt)
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "koin": int_field(&ctx, "koin") + money, "exp": new_exp, "rpg": rpg, "inventory": inv }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mulung".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("mulung").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("mulung").unwrap();
        assert_eq!(p.meta.name, "mulung");
        assert_eq!(p.meta.alias, ["scavenge", "kumpulsampah"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Memulung untuk mengumpulkan barang");
        assert_eq!(p.meta.usage, ".mulung");
        assert_eq!(p.meta.example, ".mulung");
        assert_eq!(p.meta.cooldown, 300);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["scavenge", "kumpulsampah"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mulung");
        }
    }

    #[test]
    fn a_scavenger_with_no_energy_is_sent_home() {
        let r = run(serde_json::json!({"rpg": {"stamina": 0}}));
        let t = text_of(&r);
        assert!(t.starts_with("Yaela kak, mulung aja butuh tenaga kali!"), "{t}");
        assert!(t.contains("Stamina kamu sisa *0*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_pile_costs_fifteen_stamina_and_lands_in_the_exp_band() {
        for _ in 0..400 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let d = written(&r);
            assert_eq!(d["rpg"]["stamina"], serde_json::json!(85));
            let exp = d["exp"].as_i64().unwrap();
            assert!((50..=249).contains(&exp), "exp {exp}");
        }
    }

    #[test]
    fn listed_scrap_matches_the_bag_and_never_lists_a_zero_pile() {
        for _ in 0..400 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
            let t = text_of(&r);
            let inv = written(&r)["inventory"].as_object().unwrap().clone();
            for (item, name, _, _) in DROPS {
                let got = inv.get(item).and_then(Value::as_i64).unwrap_or(0);
                let listed = t.contains(&format!("• {name}: *+"));
                assert_eq!(listed, got > 0, "{item}: tas {got},-butuh baris={listed}");
            }
        }
    }

    #[test]
    fn the_newspaper_sometimes_comes_back_empty() {
        // `koran` min 0, jadi sometime-sometime nol dan barisnya hilang.
        for _ in 0..3000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
            let t = text_of(&r);
            let qty = written(&r)["inventory"]
                .get("koran")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if qty == 0 {
                assert!(!t.contains("• 📰 Koran"), "koran 0 tak boleh disebut: {t}");
                return;
            }
        }
        panic!("tidak ada sampel dengan koran kosong");
    }

    #[test]
    fn the_sale_proceeds_are_added_to_the_purse_not_replacing_it() {
        let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 1234, "exp": 0}));
        let d = written(&r);
        let koin = d["koin"].as_i64().unwrap();
        assert!(koin > 1234, "tambah hasil jual, dapat {koin}");
        let listed: i64 = d["inventory"]
            .as_object()
            .unwrap()
            .values()
            .filter_map(Value::as_i64)
            .sum();
        // 5 jenis x maksimal 15 x maksimal 59 koin per unit.
        assert!(koin - 1234 <= listed * 59, "pendapatan harus <= barang * 59");
    }
}
