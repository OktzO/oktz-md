//! `plugins/rpg/berburu.js` — berburu enam hewan, hasilnya langsung dijual jadi
//! koin.
//!
//! Kursor yang dipertahankan: `user.rpg.stamina = user.rpg.stamina ?? 100`
//! memakai `??` (nol tetap nol), dan `caught.slice(0, 3)` membatasi tiga
//! hewan PERTAMA yang lolos — sisanya tetap menambah exp ke tabel tapi tidak
//! masuk tas maupun ke teks.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value, json};

use crate::rpg::heal::{
    apply_exp, coalesce, inventory_map, or_default, rpg_int, rpg_map, roll_incl, roll_percent,
};
use crate::tools::kalkulatormbg::thousands_id;

const STAMINA_COST: i64 = 25;

/// `(name, item, chance, min, max, exp, money)`.
type Animal = (&'static str, &'static str, i64, i64, i64, i64, i64);

const ANIMALS: [Animal; 6] = [
    ("🐰 Kelinci", "daging_kelinci", 80, 1, 3, 50, 500),
    ("🦌 Rusa", "daging_rusa", 50, 1, 2, 100, 1500),
    ("🐗 Babi Hutan", "daging_babi", 40, 1, 2, 150, 2000),
    ("🦊 Rubah", "bulu_rubah", 30, 1, 1, 200, 3000),
    ("🐻 Beruang", "cakar_beruang", 15, 1, 1, 500, 10000),
    ("🦁 Singa", "taring_singa", 5, 1, 1, 1000, 25000),
];

/// `animals.filter((a) => Math.random() * 100 <= a.chance)` — semua hewan
/// dievaluasi, tidak ada short-circuit.
fn caught() -> Vec<&'static Animal> {
    ANIMALS
        .iter()
        .filter(|(_, _, chance, ..)| roll_percent() <= *chance)
        .collect()
}

fn stamina_or_100(rpg: &Map<String, Value>) -> i64 {
    coalesce(rpg_int(rpg, "stamina"), 100)
}

#[plugin(
    name = "berburu",
    alias = ["huntanimal", "buru"],
    category = "rpg",
    description = "Berburu hewan untuk mendapat item",
    usage = ".berburu",
    example = ".berburu",
    cooldown = 120,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn berburu(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = stamina_or_100(&rpg);

    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Aduh kak, stamina kamu abis nih! 😭⚡\n\n\
             Buat berburu butuh *{STAMINA_COST} Stamina*, tapi punya kamu sisa *{stamina}*.\n\
             Istirahat dulu gih biar seger lagi! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let mut caught = caught();
    if caught.is_empty() {
        return Reply::text(format!(
            "Yahh, apes banget hari ini kak! 😭😭\n\n\
             Hewannya pada lari semua, nggak dapet apa-apa deh.\n\
             Padahal stamina udah kepotong *-{STAMINA_COST}* ⚡. Sabar ya, coba lagi nanti! 🥺🌿"
        ))
        .with_reaction("😢")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "rpg": rpg }),
        });
    }
    caught.truncate(3);

    let mut inv = inventory_map(&ctx);
    let mut lines = String::new();
    let (mut total_exp, mut total_money) = (0, 0);
    for (name, item, _, min, max, exp, money) in caught {
        let qty = roll_incl(*min, *max);
        let have = or_default(inv.get(*item).and_then(Value::as_i64), 0);
        inv.insert((*item).to_string(), json!(have + qty));
        total_exp += exp * qty;
        total_money += money * qty;
        lines.push_str(&format!("• {name}: *+{qty} ekor*\n"));
    }

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, total_exp);

    Reply::text(format!(
        "CROOT! Kena sasaran kak! 🎯🏹\n\n\
         Kamu pulang bawa hasil buruan nih:\n\
         {lines}\n\
         Hasil buruannya otomatis kejual ya! 🎉\n\
         💸 Koin: *+Rp {}*\n\
         📈 EXP: *+{total_exp}*\n\
         ⚡ Stamina terpakai: *-{STAMINA_COST}*\n\n\
         Mantap banget, besok-besok berburu lagi ya kak! 🔥🥩",
        thousands_id(total_money as f64)
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": crate::common::int_field(&ctx, "koin") + total_money,
                      "inventory": inv,
                      "exp": new_exp,
                      "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `alias` plugin `hunt.js` juga memuat "berburu", jadi `find("berburu")`
    /// bisa mengembalikan plugin yang salah — cari lewat `meta.name`.
    fn plugin() -> &'static oktz_plugin_api::registry::PluginEntry {
        oktz_plugin_api::all_plugins()
            .iter()
            .copied()
            .find(|e| e.meta.name == "berburu")
            .expect("plugin berburu harus terdaftar")
    }

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "berburu".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (plugin().handler)(ctx)
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
        let p = plugin();
        assert_eq!(p.meta.name, "berburu");
        assert_eq!(p.meta.alias, ["huntanimal", "buru"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Berburu hewan untuk mendapat item");
        assert_eq!(p.meta.usage, ".berburu");
        assert_eq!(p.meta.example, ".berburu");
        assert_eq!(p.meta.cooldown, 120);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["huntanimal", "buru"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "berburu");
        }
    }

    /// `stamina < 25` — nol TIDAK di-default ke 100 karena `??`, bukan `||`.
    #[test]
    fn no_stamina_at_all_blocks_the_hunt_and_costs_nothing() {
        for stamina in [0, 1, 24] {
            let r = run(json!({ "rpg": { "stamina": stamina }, "koin": 500 }));
            let t = text_of(&r);
            assert!(t.starts_with("Aduh kak, stamina kamu abis"), "stamina {stamina}: {t}");
            assert!(t.contains(&format!("sisa *{stamina}*")), "{t}");
            assert!(r.mutations().is_empty(), "stamina {stamina}");
        }
    }

    #[test]
    fn exactly_twenty_five_stamina_is_enough() {
        let mut went = false;
        for _ in 0..200 {
            let r = run(json!({ "rpg": { "stamina": 25 } }));
            if !text_of(&r).starts_with("Aduh kak, stamina kamu abis") {
                went = true;
                assert_eq!(write_of(&r)["rpg"]["stamina"], json!(0));
            }
        }
        assert!(went, "25 harus lolos");
    }

    /// `?? 100` — key yang hilang berarti 100, key yang 0 berarti kehabisan.
    #[test]
    fn a_missing_stamina_reads_as_one_hundred_but_zero_stays_zero() {
        assert_eq!(stamina_or_100(&Map::new()), 100);
        let mut rpg = Map::new();
        rpg.insert("stamina".into(), json!(0));
        assert_eq!(stamina_or_100(&rpg), 0);
    }

    #[test]
    fn an_empty_hunt_still_charges_the_stamina() {
        let mut empty = 0;
        for _ in 0..600 {
            let r = run(json!({ "rpg": { "stamina": 100 }, "koin": 1_000 }));
            if !text_of(&r).starts_with("CROOT!") {
                empty += 1;
                let data = write_of(&r);
                assert_eq!(data["rpg"]["stamina"], json!(75), "stamina tetap kepotong");
                assert!(data.get("koin").is_none(), "buruan kosong tak menulis koin");
                assert!(data.get("inventory").is_none());
            }
        }
        assert!(empty > 0, "dalam 600 percobaan harus ada buruan kosong");
    }

    #[test]
    fn at_most_three_animals_are_listed_even_when_more_were_caught() {
        for _ in 0..600 {
            let r = run(json!({ "rpg": { "stamina": 100 } }));
            let t = text_of(&r);
            if !t.starts_with("CROOT!") {
                continue;
            }
            let listed = t.matches(" ekor").count();
            assert!((1..=3).contains(&listed), "{listed} baris hewan: {t}");
            let inv = write_of(&r)["inventory"].as_object().unwrap().clone();
            for (_, item, ..) in ANIMALS {
                if let Some(qty) = inv.get(item) {
                    assert!(qty.as_i64().unwrap() > 0);
                }
            }
        }
    }

    #[test]
    fn the_totals_match_the_rows_that_were_kept() {
        for _ in 0..400 {
            let r = run(json!({ "rpg": { "stamina": 100 }, "koin": 0, "exp": 0 }));
            let t = text_of(&r);
            if !t.starts_with("CROOT!") {
                continue;
            }
            let data = write_of(&r);
            let money = data["koin"].as_i64().unwrap();
            let exp = data["exp"].as_i64().unwrap();
            assert!(money > 0 && exp > 0, "koin {money} exp {exp}");
            assert!(t.contains(&format!("+Rp {}", thousands_id(money as f64))), "{t}");
            assert!(t.contains(&format!("*+{exp}*")), "{t}");
        }
    }

    #[test]
    fn the_animal_table_keeps_every_js_row() {
        assert_eq!(ANIMALS.len(), 6);
        assert_eq!(ANIMALS[0], ("🐰 Kelinci", "daging_kelinci", 80, 1, 3, 50, 500));
        assert_eq!(ANIMALS[5], ("🦁 Singa", "taring_singa", 5, 1, 1, 1000, 25000));
    }
}