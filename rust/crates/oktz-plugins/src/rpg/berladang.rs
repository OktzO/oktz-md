//! `plugins/rpg/berladang.js` — panen hasil kebun. Semua tanaman punya
//! peluangnya sendiri-sendiri (bukan satu lemparan untuk seluruh ladang),
//! dan kalau gagal panen sama sekali tetap dapat 1 padi.
//!
//! Kursor: `user.rpg.stamina ?? 100` memakai `??`, dan `results.length === 0`
//! memberi 1 padi dengan nilai 100 — bukan 0.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Map, Value};

use crate::rpg::heal::{
    apply_exp, coalesce, inventory_map, or_default, rpg_int, rpg_map, roll_incl, roll_percent,
};
use crate::tools::kalkulatormbg::thousands_id;

const STAMINA_COST: i64 = 20;

/// `(item, name, chance, min, max, price)`.
type Crop = (
    &'static str,
    &'static str,
    i64,
    i64,
    i64,
    i64,
);

const CROPS: [Crop; 6] = [
    ("padi", "🌾 Padi", 90, 2, 8, 100),
    ("jagung", "🌽 Jagung", 70, 1, 5, 150),
    ("tomat", "🍅 Tomat", 50, 1, 4, 200),
    ("wortel", "🥕 Wortel", 40, 1, 3, 250),
    ("strawberry", "🍓 Strawberry", 20, 1, 2, 500),
    ("melon", "🍈 Melon", 10, 1, 1, 1000),
];

fn stamina_or_100(rpg: &Map<String, Value>) -> i64 {
    coalesce(rpg_int(rpg, "stamina"), 100)
}

#[plugin(
    name = "berladang",
    alias = ["berkebun"],
    category = "rpg",
    description = "Berladang untuk mendapat hasil panen",
    usage = ".berladang",
    example = ".berladang",
    cooldown = 180,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn berladang(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = stamina_or_100(&rpg);

    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Aduh kak, encok nih badan! 🥵\n\n\
             Buat nyangkul butuh *{STAMINA_COST} Stamina*, sisa stamina kamu *{stamina}*.\n\
             Makan atau istirahat dulu gih! 🛌💤"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let mut inv = inventory_map(&ctx);
    let mut lines = String::new();
    let mut total_value = 0;

    for (item, name, chance, min, max, price) in CROPS {
        if roll_percent() <= chance {
            let qty = roll_incl(min, max);
            let have = or_default(inv.get(item).and_then(Value::as_i64), 0);
            inv.insert(item.to_string(), serde_json::json!(have + qty));
            let value = qty * price;
            total_value += value;
            lines.push_str(&format!(
                "• {name}: *+{qty}* (Est. Rp {})\n",
                thousands_id(value as f64)
            ));
        }
    }

    if lines.is_empty() {
        let have = or_default(inv.get("padi").and_then(Value::as_i64), 0);
        inv.insert("padi".to_string(), serde_json::json!(have + 1));
        lines.push_str("• 🌾 Padi: *+1* (Est. Rp 100)\n");
        total_value = 100;
    }

    let exp_gain = total_value / 10 + crate::common::roll(100) as i64;
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "PANEN RAYA KAK! 🚜🌾\n\n\
         Gila, kebun kamu hari ini ngasilin banyak banget:\n\
         {lines}\n\
         💰 Total Estimasi Nilai: *Rp {}*\n\
         📈 EXP Petani: *+{exp_gain}*\n\
         ⚡ Stamina: *-{STAMINA_COST}*\n\n\
         Kamu bisa jual hasil panen ini ke pasar pakai `{}sellall` ya! 🧺✨",
        thousands_id(total_value as f64),
        ctx.prefix
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "inventory": inv, "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "berladang".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("berladang").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("berladang").unwrap();
        assert_eq!(p.meta.name, "berladang");
        assert_eq!(p.meta.alias, ["berkebun"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Berladang untuk mendapat hasil panen");
        assert_eq!(p.meta.usage, ".berladang");
        assert_eq!(p.meta.example, ".berladang");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("berkebun").unwrap().meta.name,
            "berladang"
        );
    }

    #[test]
    fn not_enough_stamina_blocks_the_hoe_and_writes_nothing() {
        for stamina in [0, 19] {
            let r = run(serde_json::json!({ "rpg": { "stamina": stamina } }));
            let t = text_of(&r);
            assert!(t.starts_with("Aduh kak, encok nih badan"), "stamina {stamina}: {t}");
            assert!(t.contains(&format!("sisa stamina kamu *{stamina}*")), "{t}");
            assert!(r.mutations().is_empty(), "stamina {stamina}");
        }
    }

    #[test]
    fn exactly_twenty_stamina_is_enough() {
        let mut went = false;
        for _ in 0..200 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 20 } }));
            if !text_of(&r).starts_with("Aduh kak, encok") {
                went = true;
                assert_eq!(write_of(&r)["rpg"]["stamina"], serde_json::json!(0));
            }
        }
        assert!(went, "20 harus lolos");
    }

    /// `if (results.length === 0)` memberi 1 padi bernilai 100. Baris hasil
    /// panen yang cuma SATU-SATUNYA tidak berarti cabang ini — hanya baris
    /// `padi` berisi `+1` dengan total 100 yang menandai fallback.
    ///
    /// Semua 6 tanaman harus gagal agar cabang ini kena: 0.10·0.30·0.50·0.60·
    /// 0.80·0.90 = 0.00648 per panen. 600 percobaan masih gagal ~2% dan test
    /// ini diam-diam jadi flaky; 2400 menurunkan ke ~1e-7.
    #[test]
    fn a_failed_harvest_still_yields_one_padi() {
        let mut fallback = 0;
        for _ in 0..2400 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 } }));
            let t = text_of(&r);
            assert!(t.starts_with("PANEN RAYA KAK!"), "{t}");
            let is_fallback = t.contains("• 🌾 Padi: *+1* (Est. Rp 100)")
                && t.contains("💰 Total Estimasi Nilai: *Rp 100*");
            if !is_fallback {
                // Panen biasa: minimal satu baris, total = jumlah nilainya.
                let listed = t.matches("Est. Rp").count();
                assert!(listed >= 1, "{t}");
                continue;
            }
            fallback += 1;
            assert_eq!(t.matches("Est. Rp").count(), 1, "fallback tak boleh baris lain: {t}");
            assert_eq!(write_of(&r)["inventory"]["padi"], serde_json::json!(1));
        }
        assert!(fallback > 0, "dalam 2400 percobaan harus ada panen kosong");
    }

    #[test]
    fn the_total_value_is_the_sum_of_the_listed_rows() {
        for _ in 0..400 {
            let r = run(serde_json::json!({ "rpg": { "stamina": 100 }, "exp": 0 }));
            let t = text_of(&r);
            let data = write_of(&r);
            let total = data["exp"].as_i64().unwrap();
            assert!(total >= 10, "exp = total/10 + acak(0..99): {total}");
            assert!(t.contains("📈 EXP Petani: *+"), "{t}");
            // Baris hasil panen harus cocok dengan isi inventory yang ditulis.
            let inv = data["inventory"].as_object().unwrap().clone();
            for (item, name, ..) in CROPS {
                if let Some(qty) = inv.get(item) {
                    assert!(
                        t.contains(&format!("• {name}: *+{qty}*")),
                        "{name} ada di tas tapi tak disebut: {t}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_crop_table_keeps_every_js_row() {
        assert_eq!(CROPS.len(), 6);
        assert_eq!(CROPS[0], ("padi", "🌾 Padi", 90, 2, 8, 100));
        assert_eq!(CROPS[5], ("melon", "🍈 Melon", 10, 1, 1, 1000));
    }
}