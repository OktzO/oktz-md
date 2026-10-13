//! `plugins/rpg/steal.js` — mencuri dari NPC demi emas.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//!
//! Catatan kursor: tingkat kesalahan target disaring dari `user.level` (level di
//! akar row, bukan `user.rpg.level`) — default 1, jadi tanpa level yang tersimpan
//! tidak ada target yang boleh dibobol.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::common::int_field;
use crate::rpg::jualan::{
    apply_exp, coalesce, inventory_map, math_random, or_default, roll_excl, rpg_int, rpg_map,
    thousands_en,
};

pub(crate) const STAMINA_COST: i64 = 15;

/// `TARGETS` (`steal.js:20-27`) — `(nama, kesulitan, emas min, emas max,
/// peluang ketahuan)`. Syarat lolos: `userLevel >= difficulty * 3`.
pub(crate) const TARGETS: [(&str, i64, i64, i64, i64); 6] = [
    ("👨‍🌾 Petani Lengah", 1, 50, 150, 10),
    ("👨‍💼 Pedagang Keliling", 2, 100, 300, 20),
    ("🧙‍♂️ Penyihir Tua", 3, 200, 500, 30),
    ("⚔️ Ksatria Kerajaan", 4, 300, 800, 40),
    ("👑 Bangsawan Sombong", 5, 500, 1500, 50),
    ("🏰 Raja Tiran", 6, 1000, 3000, 60),
];

/// `items` untuk bonus jarahan (`steal.js:93`).
pub(crate) const BONUS_ITEMS: [&str; 4] = ["potion", "key", "gem", "ring"];

/// Target yang boleh approached user pada level tertentu — urutan `TARGETS`
/// utuh, jadi `random_index` di atasnya sama dengan `filter` di JS.
pub(crate) fn targets_for(level: i64) -> Vec<&'static (&'static str, i64, i64, i64, i64)> {
    TARGETS.iter().filter(|t| level >= t.1 * 3).collect()
}

/// `Math.max(5, target.catchChance - luck * 2)` — lantai 5 persen, jadi luck
/// besar tidak pernah membuat pencuri kebal tertangkap.
pub(crate) fn catch_chance(target_chance: i64, luck: i64) -> i64 {
    (target_chance - luck * 2).max(5)
}

#[plugin(
    name = "steal",
    alias = ["mencuri", "curi", "pickpocket"],
    category = "rpg",
    description = "Mencuri dari NPC untuk gold",
    usage = ".steal",
    example = ".steal",
    cooldown = 300,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn steal(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "Hadeh... mau jadi pencuri kok loyo? 😴\n\
             Butuh *{STAMINA_COST} Stamina* buat nyusup, lu cuma punya *{stamina}*. Istirahat gih!"
        ));
    }
    rpg.insert("stamina".into(), json!(stamina - STAMINA_COST));

    let koin = int_field(&ctx, "koin");
    let level = or_default(ctx.user_field("level").and_then(Value::as_i64), 1);
    let available = targets_for(level);

    // `db.save()` di cabang ini: stamina yang sudah dipotong tetap tersimpan
    // walau tidak ada yang bisa dibobol.
    if available.is_empty() {
        return Reply::text(format!(
            "Level lu masih bocil (Level {level}). Target paling rendahan aja minimal butuh *Level 3* biar lu nggak mati konyol! 😂"
        ))
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "rpg": rpg }),
        });
    }

    let &(name, difficulty, min_gold, max_gold, catch_chance_field) =
        available[crate::common::random_index(available.len())];

    let luck_bonus = or_default(rpg_int(&rpg, "luck"), 5);
    let adjusted = catch_chance(catch_chance_field, luck_bonus);

    if math_random() * 100.0 < adjusted as f64 {
        let gold_loss = koin / 10;
        let health_loss = 10 + difficulty * 5;
        let health = (or_default(rpg_int(&rpg, "health"), 100) - health_loss).max(1);
        rpg.insert("health".into(), json!(health));
        return Reply::text(format!(
            "SIAAALLLL!! KESANDUNG POT BUNGA!! 💥🚨\n\n\
             Si *{name}* langsung kebangun dan mukulin lu abis-abisan!\n\n\
             *Kerugian:* \n\
             💸 Duit berceceran: *-Rp {}*\n\
             ❤️ Kena Pukul: *-{health_loss} HP*\n\
             ⚡ Stamina Buat Kabur: *-{STAMINA_COST}*\n\n\
             *Tips:* Banyakin stat *Luck* biar langkah kaki lu nggak bersuara bro!",
            thousands_en(gold_loss)
        ))
        .with_reaction("💀")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: json!({ "koin": (koin - gold_loss).max(0), "rpg": rpg }),
        });
    }

    // `Math.floor(Math.random() * (max - min)) + min` — batas atas eksklusif.
    let gold = roll_excl(min_gold, max_gold);
    let exp_reward = 50 + difficulty * 30;
    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

    let mut inv = inventory_map(&ctx);
    let bonus_text = if math_random() > 0.7 {
        let item = BONUS_ITEMS[crate::common::random_index(BONUS_ITEMS.len())];
        let slot = inv.entry(item.to_string()).or_insert(json!(0));
        let have = slot.as_i64().unwrap_or(0);
        *slot = json!(have + 1);
        format!("\n📦 Bonus Jarahan: *{item} x1*")
    } else {
        String::new()
    };

    Reply::text(format!(
        "NINJA STRIKE BERHASIL! 🥷✨\n\n\
         Lu berhasil ngejarah rumah *{name}* tanpa ketahuan sama sekali!\n\n\
         *Hasil Jarahan:* \n\
         💵 Emas Batangan: *+Rp {}*\n\
         ✨ EXP Menyusup: *+{exp_reward}*{bonus_text}\n\
         ⚡ Stamina: *-{STAMINA_COST}*",
        thousands_en(gold)
    ))
    .with_reaction("💰")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "koin": koin + gold, "exp": new_exp, "rpg": rpg, "inventory": inv }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "steal".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("steal").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("steal").unwrap();
        assert_eq!(p.meta.name, "steal");
        assert_eq!(p.meta.alias, ["mencuri", "curi", "pickpocket"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Mencuri dari NPC untuk gold");
        assert_eq!(p.meta.usage, ".steal");
        assert_eq!(p.meta.example, ".steal");
        assert_eq!(p.meta.cooldown, 300);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mencuri", "curi", "pickpocket"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "steal");
        }
    }

    #[test]
    fn a_low_level_thief_is_turned_away_but_still_pays_the_stamina() {
        let r = run(json!({"level": 1, "rpg": {"stamina": 100}, "koin": 500}));
        let t = text_of(&r);
        assert!(t.starts_with("Level lu masih bocil (Level 1)."), "{t}");
        assert!(t.contains("minimal butuh *Level 3*"), "{t}");
        let d = written(&r);
        assert_eq!(d["rpg"]["stamina"], json!(85), "db.save() di cabang ini");
        assert!(d.get("koin").is_none());
    }

    #[test]
    fn low_stamina_is_refused_before_the_level_check() {
        let r = run(json!({"level": 1, "rpg": {"stamina": 14}}));
        assert!(text_of(&r).starts_with("Hadeh... mau jadi pencuri kok loyo?"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_level_gate_opens_one_target_per_three_levels() {
        assert_eq!(targets_for(1).len(), 0);
        assert_eq!(targets_for(3).len(), 1);
        assert_eq!(targets_for(6).len(), 2);
        assert_eq!(targets_for(18).len(), 6);
        assert_eq!(targets_for(0).len(), 0);
    }

    #[test]
    fn a_heist_reports_its_target_and_never_breaks_its_own_range() {
        for _ in 0..1000 {
            let r = run(json!({"level": 20, "rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            let d = written(&r);
            assert_eq!(d["rpg"]["stamina"], json!(85));
            if t.starts_with("SIAAALLLL") {
                continue;
            }
            let (_, difficulty, min, max, _) = TARGETS
                .iter()
                .find(|(name, ..)| t.contains(&format!("ngejarah rumah *{name}*")))
                .copied()
                .unwrap_or_else(|| panic!("target tak dikenal: {t}"));
            let koin = d["koin"].as_i64().unwrap();
            assert!((min..max).contains(&koin), "{koin} harus di {min}..{max}");
            assert_eq!(d["exp"], json!(50 + difficulty * 30));
        }
    }

    #[test]
    fn getting_caught_costs_a_tenth_of_the_purse_and_blood_but_no_exp() {
        let mut seen = 0;
        for _ in 0..6000 {
            let r = run(json!({"level": 18, "rpg": {"stamina": 100}, "koin": 1000, "exp": 42}));
            let d = written(&r);
            let t = text_of(&r);
            if !t.starts_with("SIAAALLLL") {
                continue;
            }
            seen += 1;
            let (_, difficulty, ..) = TARGETS
                .iter()
                .find(|(name, ..)| t.contains(&format!("Si *{name}* langsung")))
                .copied()
                .unwrap_or_else(|| panic!("target tak dikenal: {t}"));
            assert_eq!(d["koin"], json!(900), "10% dari 1000");
            assert_eq!(d["rpg"]["health"], json!(100 - (10 + difficulty * 5)), "{t}");
            assert!(d.get("exp").is_none(), "exp utuh");
        }
        assert!(seen > 0, "cabang ketahuan harus muncul");
    }

    #[test]
    fn a_streak_of_luck_never_pushes_the_catch_chance_below_five_percent() {
        assert_eq!(catch_chance(60, 5), 50, "luck default 5 -> bonus 10");
        assert_eq!(catch_chance(60, 0), 60, "`rpg.luck || 5`, nol jadi 5 bukan 0");
        assert_eq!(catch_chance(10, 100), 5, "dijepit di lantai 5");
        assert_eq!(catch_chance(10, 9999), 5, "luck gila tetap 5");
    }

    #[test]
    fn the_bonus_loot_only_lands_on_the_four_named_items() {
        let mut seen = false;
        for _ in 0..3000 {
            let r = run(json!({"level": 18, "rpg": {"stamina": 100}, "koin": 0, "exp": 0}));
            let t = text_of(&r);
            if t.contains("Bonus Jarahan") {
                seen = true;
                let item = t
                    .split("Bonus Jarahan: *")
                    .nth(1)
                    .and_then(|s| s.split(' ').next())
                    .unwrap();
                assert!(BONUS_ITEMS.contains(&item), "item {item}");
                assert_eq!(written(&r)["inventory"][item], json!(1));
            }
        }
        assert!(seen, "bonus 30% harus muncul dalam 3000 sampel");
    }
}
