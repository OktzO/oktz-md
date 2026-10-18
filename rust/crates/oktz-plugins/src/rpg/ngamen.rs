//! `plugins/rpg/ngamen.js` — ngamen di jalanan untuk mencari koin.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, or_default, roll_incl, rpg_int, rpg_map,
};
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) const STAMINA_COST: i64 = 10;

/// `locations` (`ngamen.js:37-43`) — `(nama, tarif minimum, tarif maksimum)`;
/// urutan object literal = urutan tabel.
pub(crate) const LOCATIONS: [(&str, i64, i64); 5] = [
    ("Perempatan Lampu Merah", 3000, 10000),
    ("Warung Kopi", 5000, 15000),
    ("Depan Minimarket", 4000, 12000),
    ("Kafe Gaul", 8000, 25000),
    ("Angkringan", 2000, 8000),
];

#[plugin(
    name = "ngamen",
    alias = ["nyanyi", "konser"],
    category = "rpg",
    description = "Ngamen di jalanan untuk mencari koin",
    usage = ".ngamen",
    example = ".ngamen",
    cooldown = 120,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn ngamen(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Suara serak, tenggorokan kering! 🥵\n\n\
             Ngamen butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*. Minum es teh dulu gih! ☕"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let (loc_name, min, max) = LOCATIONS[crate::common::random_index(LOCATIONS.len())];
    let earning = roll_incl(min, max);
    let koin = int_field(&ctx, "koin");
    let exp_gain = earning / 20;
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "ALHAMDULILLAH HASIL NGAMEN! 🎸✨\n\n\
         Lokasi: *{loc_name}*\n\
         💵 Pendapatan: *+Rp {}*\n\
         📈 EXP: *+{exp_gain}*\n\
         ⚡ Stamina: -{STAMINA_COST}\n\n\
         Lumayan buat beli nasi bungkus hari ini! 🤤",
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
            command: "ngamen".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("ngamen").unwrap().handler)(ctx)
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

    fn earning_of(t: &str) -> i64 {
        let line = t.lines().find(|l| l.contains("Pendapatan")).unwrap_or(t);
        line.split("*+Rp ")
            .nth(1)
            .and_then(|s| s.split('*').next())
            .and_then(|s| s.replace('.', "").parse().ok())
            .unwrap_or_else(|| panic!("tidak ada nominal di: {t}"))
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ngamen").unwrap();
        assert_eq!(p.meta.name, "ngamen");
        assert_eq!(p.meta.alias, ["nyanyi", "konser"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Ngamen di jalanan untuk mencari koin");
        assert_eq!(p.meta.usage, ".ngamen");
        assert_eq!(p.meta.example, ".ngamen");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["nyanyi", "konser"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "ngamen");
        }
    }

    #[test]
    fn an_empty_stamina_bar_is_refused_without_a_write() {
        let r = run(serde_json::json!({"rpg": {"stamina": 9}}));
        let t = text_of(&r);
        assert!(t.starts_with("Suara serak, tenggorokan kering!"), "{t}");
        assert!(t.contains("sisa stamina kamu *9*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_chosen_location_bounds_the_tariff_it_pays() {
        let mut seen = [false; 5];
        for _ in 0..2000 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            let (_, min, max) = LOCATIONS
                .iter()
                .find(|(name, _, _)| t.contains(&format!("Lokasi: *{name}*")))
                .copied()
                .unwrap_or_else(|| panic!("lokasi tak dikenal: {t}"));
            let got = earning_of(t);
            assert!((min..=max).contains(&got), "{t}");
            assert_eq!(written(&r)["exp"], serde_json::json!(got / 20));
            for (i, (name, _, _)) in LOCATIONS.iter().enumerate() {
                seen[i] |= t.contains(&format!("Lokasi: *{name}*"));
            }
        }
        assert!(seen.iter().all(|s| *s), "kelima lokasi harus muncul");
    }

    #[test]
    fn the_existing_balance_is_added_to_and_never_replaced() {
        let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 5000, "exp": 0}));
        let d = written(&r);
        let koin = d["koin"].as_i64().unwrap();
        assert_eq!(koin - earning_of(text_of(&r)), 5000);
    }

    #[test]
    fn every_ten_stamina_the_busker_costs() {
        for _ in 0..200 {
            let r = run(serde_json::json!({"rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            assert_eq!(written(&r)["rpg"]["stamina"], serde_json::json!(90));
        }
    }
}
