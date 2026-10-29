//! `plugins/rpg/boss.js` — raid boss, 15 ronde, crit 10%.
//!
//! Kursor yang dipertahankan:
//!
//! 1. `availableBosses` disaring `user.level` (field ATAS, bukan `rpg.level`),
//!    jadi pemain baru benar-benar terkunci dari SELURUH boss.
//! 2. `Math.max(1, health - 50)` saat kalah — HP tidak pernah menyentuh 0,
//!    tidak seperti `adventure`.
//! 3. `toLocaleString()` tanpa locale memakai koma (`5,000`), bukan titik.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::heal::{
    apply_exp, coalesce, inventory_map, math_random, or_default, rpg_int, rpg_map, thousands_en,
    user_level,
};

const STAMINA_COST: i64 = 50;
const MAX_ROUNDS: i64 = 15;
const CRIT_THRESHOLD: f64 = 0.9;

/// `(name, hp, attack, minLevel, exp, gold, drops)`.
type Boss = (
    &'static str,
    i64,
    i64,
    i64,
    i64,
    i64,
    &'static [&'static str],
);

const BOSSES: [Boss; 7] = [
    ("🐉 Elder Dragon", 500, 50, 10, 2000, 5000, &["dragonscale", "dragonbone"]),
    ("👹 Demon Lord", 400, 60, 15, 2500, 7000, &["demonsoul", "cursedgem"]),
    ("🧟 Undead King", 350, 45, 8, 1500, 4000, &["soulstone", "ancientbone"]),
    ("🦑 Kraken", 600, 40, 12, 2200, 6000, &["krakententacle", "seagem"]),
    ("🌋 Volcanic Titan", 700, 55, 20, 3000, 10000, &["titancore", "lavagem"]),
    ("❄️ Frost Queen", 450, 50, 18, 2800, 8000, &["frostheart", "icecrown"]),
    ("⚡ Thunder God", 550, 65, 25, 4000, 15000, &["thunderstone", "divinecore"]),
];

fn available(level: i64) -> Vec<&'static Boss> {
    BOSSES.iter().filter(|(_, _, _, min, ..)| level >= *min).collect()
}

/// `BOSSES.reduce((a, b) => (a.minLevel < b.minLevel ? a : b))` — tabel sudah
/// urut, jadi `min_by_key` memberi hasil yang sama.
fn lowest() -> &'static Boss {
    BOSSES
        .iter()
        .min_by_key(|(_, _, _, min, ..)| *min)
        .expect("tabel tidak kosong")
}

/// `Math.floor((user.koin || 0) * 0.15)` — penalti koin saat kalah.
fn loss_gold(koin: i64) -> i64 {
    (koin as f64 * 0.15).floor() as i64
}

/// `Math.max(1, health - 50)` — lantai 1 HP, bukan 0.
fn loss_health(health: i64) -> i64 {
    (health - 50).max(1)
}

#[allow(dead_code)] // `rounds` dipakai balasan; field lain dibaca lewat `fight()`
struct Fight {
    my_hp: i64,
    boss_hp: i64,
    rounds: i64,
    log: Vec<String>,
}

/// `while (userHp > 0 && bossHp > 0 && round < 15)` — crit memakai
/// `Math.random()` yang SAMA untuk multiplier dan penanda log.
fn fight(user_attack: i64, user_defense: i64, max_hp: i64, boss: &Boss) -> Fight {
    let (mut user_hp, mut boss_hp, mut round) = (max_hp, boss.1, 0);
    let mut log = Vec::new();
    while user_hp > 0 && boss_hp > 0 && round < MAX_ROUNDS {
        round += 1;
        let player_dmg = (user_attack + crate::common::roll(20) as i64 - 5).max(10);
        let crit = math_random() > CRIT_THRESHOLD;
        let dealt = if crit { player_dmg * 2 } else { player_dmg };
        boss_hp -= dealt;
        log.push(if crit {
            format!("> 💥 *CRITICAL HIT!!* Serangan mematikanmu masuk: *-{dealt} HP*")
        } else {
            format!("> ⚔️ Kamu menebas boss: *-{dealt} HP*")
        });
        if boss_hp <= 0 {
            break;
        }
        let boss_dmg = (boss.2 - user_defense + crate::common::roll(15) as i64).max(10);
        user_hp -= boss_dmg;
        log.push(format!("> 👹 Boss mengamuk dan memukul mundur: *-{boss_dmg} HP*"));
    }
    Fight { my_hp: user_hp, boss_hp, rounds: round, log }
}

fn log_block(f: &Fight) -> String {
    f.log
        .iter()
        .skip(f.log.len().saturating_sub(6))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

#[plugin(
    name = "boss",
    alias = ["raidboss", "bigboss"],
    category = "rpg",
    description = "Lawan boss untuk hadiah besar",
    usage = ".boss",
    example = ".boss",
    cooldown = 600,
    energi = 3,
    is_group = false,
    is_private = false
)]
pub fn boss(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);
    let level = user_level(&ctx);
    let pool = available(level);

    if pool.is_empty() {
        let lowest_boss = lowest();
        return Reply::text(format!(
            "Aduh kak, level kamu masih terlalu rendah buat ikutan Raid Boss! 😭\n\n\
             Level kamu sekarang: *{level}*\n\
             Minimal level yang dibutuhkan: *{}*\n\n\
             💡 _Tips: Rajin-rajin farming EXP dari `.dungeon`, `.fishing`, atau `.mining` dulu yuk kak!_",
            lowest_boss.3
        ));
    }

    // `user.rpg.stamina ?? 100` — `??`, bukan `||`.
    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < STAMINA_COST {
        return Reply::text(format!(
            "⚡ Yahh stamina kamu habis kak!\n\n\
             Butuh *{STAMINA_COST} Stamina* untuk melawan boss.\n\
             Stamina kamu sisa: *{stamina}*"
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - STAMINA_COST));

    let chosen = pool[crate::common::random_index(pool.len())];
    let user_attack = or_default(rpg_int(&rpg, "attack"), 10) + level * 3;
    let user_defense = or_default(rpg_int(&rpg, "defense"), 5) + level * 2;
    let max_hp = or_default(rpg_int(&rpg, "health"), 100) + level * 5;

    let f = fight(user_attack, user_defense, max_hp, chosen);

    if f.boss_hp <= 0 {
        let exp_reward = chosen.4 + crate::common::roll(500) as i64;
        let gold_reward = chosen.5 + crate::common::roll(2000) as i64;

        let mut inv = inventory_map(&ctx);
        let mut dropped = Vec::new();
        for drop in chosen.6 {
            if math_random() > 0.5 {
                let qty = crate::common::roll(3) as i64 + 1;
                let have = or_default(inv.get(*drop).and_then(Value::as_i64), 0);
                inv.insert((*drop).to_string(), serde_json::json!(have + qty));
                dropped.push(format!("{drop} (x{qty})"));
            }
        }

        let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
        let new_exp = apply_exp(&mut rpg, old_exp, exp_reward);

        let mut txt = format!(
            "🏆 *BOSS BERHASIL DIKALAHKAN!!* 🎉\n\n\
             Wahh gila, kamu berhasil numbangin monster raksasa *{}* kak!\n\n\
             *🎁 Harta Karun Boss:*\n\
             ✨ EXP: *+{}*\n\
             💰 Koin Emas: *+Rp {}*\n",
            chosen.0,
            thousands_en(exp_reward),
            thousands_en(gold_reward)
        );
        if !dropped.is_empty() {
            txt.push_str(&format!("📦 Item Loot: *{}*\n", dropped.join(", ")));
        }
        txt.push_str(&format!(
            "\n> ❤️ Sisa HP Kamu: *{}/{max_hp}*",
            f.my_hp.max(0)
        ));

        let mut data = serde_json::json!({ "koin": crate::common::int_field(&ctx, "koin") + gold_reward,
                               "exp": new_exp,
                               "rpg": rpg });
        if !dropped.is_empty() {
            data["inventory"] = Value::Object(inv);
        }

        return Reply::text(format!(
            "⚔️ *Sengitnya Pertarungan...*\n\n{}\n\n{txt}",
            log_block(&f)
        ))
        .with_reaction("🏆")
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data,
        });
    }

    // Kalah: `Math.max(1, (health || 100) - 50)` — HP tidak boleh 0.
    let gold_loss = loss_gold(crate::common::int_field(&ctx, "koin"));
    let left_hp = loss_health(or_default(rpg_int(&rpg, "health"), 100));
    rpg.insert("health".into(), serde_json::json!(left_hp));

    Reply::text(format!(
        "⚔️ *Sengitnya Pertarungan...*\n\n{}\n\n\
         💀 *YAH... KAMU TERPURUK...* 💔\n\n\
         Tenaga *{}* ternyata masih terlalu besar buat kamu kak!\n\n\
         *Penalti Kekalahan:*\n\
         💸 Koin Terjatuh: *-Rp {}*\n\
         ❤️ HP Berkurang: *-50 HP*\n\n\
         > 💡 _Tips: Coba tingkatkan level dan perbaiki senjatamu sebelum nantangin dia lagi ya kak!_",
        log_block(&f),
        chosen.0,
        thousands_en(gold_loss)
    ))
    .with_reaction("💀")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "koin": (crate::common::int_field(&ctx, "koin") - gold_loss).max(0),
                      "rpg": rpg }),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "boss".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("boss").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("boss").unwrap();
        assert_eq!(p.meta.name, "boss");
        assert_eq!(p.meta.alias, ["raidboss", "bigboss"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Lawan boss untuk hadiah besar");
        assert_eq!(p.meta.usage, ".boss");
        assert_eq!(p.meta.example, ".boss");
        assert_eq!(p.meta.cooldown, 600);
        assert_eq!(p.meta.energi, 3);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["raidboss", "bigboss"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "boss");
        }
    }

    /// `user.level` default 1 dan `minLevel` terkecil 8 — level 1 gagal total.
    #[test]
    fn a_new_player_is_locked_out_of_every_boss() {
        for user in [
            serde_json::json!({}),
            serde_json::json!({ "level": 1 }),
            serde_json::json!({ "level": 7, "rpg": { "stamina": 100 } }),
        ] {
            let r = run(user.clone());
            let t = text_of(&r);
            assert!(t.starts_with("Aduh kak, level kamu masih terlalu rendah"), "input {user}");
            assert!(t.contains("Minimal level yang dibutuhkan: *8*"), "{t}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn the_gate_opens_exactly_at_the_lowest_min_level() {
        assert!(available(7).is_empty());
        assert_eq!(available(8).len(), 1);
        assert_eq!(available(10).len(), 2);
        assert_eq!(available(25).len(), 7);
    }

    #[test]
    fn a_player_without_stamina_is_turned_away_after_unlocking_the_gate() {
        for stamina in [0, 49] {
            let r = run(serde_json::json!({ "level": 10, "rpg": { "stamina": stamina } }));
            let t = text_of(&r);
            assert!(t.starts_with("⚡ Yahh stamina kamu habis"), "stamina {stamina}: {t}");
            assert!(t.contains(&format!("sisa: *{stamina}*")), "{t}");
            assert!(r.mutations().is_empty(), "stamina {stamina}");
        }
    }

    #[test]
    fn a_win_pays_the_boss_rewards_plus_random_rolls_and_marks_a_reaction() {
        let mut won = 0;
        for _ in 0..400 {
            let r = run(serde_json::json!({ "level": 30, "rpg": { "stamina": 100, "attack": 5_000, "defense": 5_000, "health": 100 }, "koin": 0, "exp": 0 }));
            let t = text_of(&r);
            if !t.contains("🏆 *BOSS BERHASIL DIKALAHKAN!!*") {
                continue;
            }
            won += 1;
            let data = write_of(&r);
            assert!(t.contains("⚔️ *Sengitnya Pertarungan..."), "{t}");
            assert!(data["koin"].as_i64().unwrap() >= 4000);
            assert!(data["exp"].as_i64().unwrap() >= 1500);
            assert_eq!(data["rpg"]["stamina"], serde_json::json!(50));
        }
        assert!(won > 0, "dalam 400 percobaan harus ada kemenangan");
    }

    #[test]
    fn a_loss_leaves_at_least_one_hp_and_fifteen_percent_of_the_purse() {
        // Penalti dihitung fungsi, bukan satu duel acak yang kebetulan kalah.
        assert_eq!(loss_health(100), 50, "Math.max(1, 100 - 50)");
        assert_eq!(loss_gold(10_000), 1_500, "floor(15% dari 10.000)");
        assert_eq!(loss_gold(999), 149, "floor, bukan round");
        assert_eq!(loss_gold(0), 0);

        // Level 8 = satu-satunya boss yang terbuka (Undead King, atk 45):
        // serangan pemain mustahil menghabiskan 350 HP dalam 15 ronde, jadi
        // kalah bisa terjadi dan cabang penalti ikut teruji.
        let r = run(serde_json::json!({ "level": 8, "rpg": { "stamina": 100, "attack": 1, "defense": 1, "health": 100 }, "koin": 10_000, "exp": 0 }));
        let t = text_of(&r);
        assert!(t.contains("💀 *YAH... KAMU TERPURUK...*"), "{t}");
        assert!(t.contains("💸 Koin Terjatuh: *-Rp 1,500*"), "{t}");
        let data = write_of(&r);
        assert_eq!(data["rpg"]["health"], serde_json::json!(50));
        assert_eq!(data["koin"], serde_json::json!(8_500), "15% dari 10.000");
        assert_eq!(data["rpg"]["stamina"], serde_json::json!(50), "stamina tetap kepotong saat kalah");
        assert!(data.get("exp").is_none(), "kalah tak memberi exp");
        assert!(data.get("inventory").is_none(), "kalah tak melepas loot");
    }

    /// `Math.max(1, ...)` — HP 0 masih naik ke 1, bukan 0.
    #[test]
    fn the_loss_floor_is_one_hp_not_zero() {
        assert_eq!(loss_health(1), 1, "1 - 50 -> max(1, -49)");
        assert_eq!(loss_health(0), 1, "0 - 50 -> max(1, -50)");
        assert_eq!(loss_health(49), 1);
        assert_eq!(loss_health(50), 1, "tepat 50 tidak nol");
        assert_eq!(loss_health(51), 1);

        // health 1 = `max_hp` 1 + level 8 * 5, jadi bossPasti kill di ronde
        // pertama — lantai 1 HP benar-benar tertulis ke store.
        // (`attack` 1 + level 8 * 3 = 25, damage minimal 20 per ronde, jadi
        // 350 HP boss mustahil habis dalam 15 ronde.)
        let r = run(serde_json::json!({ "level": 8, "rpg": { "stamina": 100, "attack": 1, "defense": 1, "health": 1 }, "koin": 0 }));
        assert!(text_of(&r).contains("TERPURUK"), "{}", text_of(&r));
        assert_eq!(write_of(&r)["rpg"]["health"], serde_json::json!(1));
    }

    #[test]
    fn the_battle_stops_at_fifteen_rounds() {
        let f = fight(10, 0, 100_000, &BOSSES[0]);
        assert!(f.rounds <= MAX_ROUNDS);
        let log_lines = f.log.len();
        assert!(log_lines <= 2 * MAX_ROUNDS as usize);
    }

    #[test]
    fn the_boss_table_keeps_every_js_row() {
        assert_eq!(BOSSES.len(), 7);
        assert_eq!(BOSSES[0], ("🐉 Elder Dragon", 500, 50, 10, 2000, 5000,
            &["dragonscale", "dragonbone"] as &[&str]));
        assert_eq!(BOSSES[6], ("⚡ Thunder God", 550, 65, 25, 4000, 15000,
            &["thunderstone", "divinecore"] as &[&str]));
        assert_eq!(lowest().3, 8, "Undead King adalah minLevel paling rendah");
    }
}