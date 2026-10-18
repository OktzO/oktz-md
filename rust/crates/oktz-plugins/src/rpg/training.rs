//! `plugins/rpg/training.js` — latihan untuk meningkatkan stats.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

use crate::rpg::jualan::{
    apply_exp, coalesce, or_default, roll_incl, rpg_int, rpg_map,
};

/// `TRAINING_TYPES` (`training.js:19-25`) — `(kunci, nama, stat, bonus min,
/// bonus max, exp, biaya stamina)`.
pub(crate) const TRAINING: [(&str, &str, &str, i64, i64, i64, i64); 5] = [
    ("attack", "⚔️ Attack Training", "attack", 1, 3, 80, 20),
    ("defense", "🛡️ Defense Training", "defense", 1, 2, 70, 15),
    ("health", "❤️ Health Training", "health", 5, 15, 90, 25),
    ("speed", "💨 Speed Training", "speed", 1, 2, 75, 18),
    ("luck", "🍀 Luck Training", "luck", 1, 2, 85, 22),
];

pub(crate) fn find_training(key: &str) -> Option<&'static (&'static str, &'static str, &'static str, i64, i64, i64, i64)> {
    TRAINING.iter().find(|t| t.0 == key)
}

/// Nilai stat di papan Thermal — literal `|| x` per baris di `training.js:41-46`.
/// BEDA dari [`base_stat`]: speed di papan 10, tapi jadi 5 kalau belum pernah
/// dilatih (`currentStat`).
pub(crate) fn board_stat(stat: &str) -> i64 {
    match stat {
        "attack" => 10,
        "health" => 100,
        "speed" => 10,
        _ => 5,
    }
}

/// Nilai awal stat saat belum pernah dilatih — `training.stat === "health"
/// ? 100 : stat === "attack" ? 10 : 5`. `|| 0` juga ikut memperlakukan
/// `0` sebagai belum ada, jadi zeroes ikut di-backfill.
pub(crate) fn base_stat(stat: &str) -> i64 {
    match stat {
        "health" => 100,
        "attack" => 10,
        _ => 5,
    }
}

#[plugin(
    name = "training",
    alias = ["train", "latihan", "workout"],
    category = "rpg",
    description = "Latihan untuk meningkatkan stats",
    usage = ".training <attack/defense/health>",
    example = ".training attack",
    cooldown = 180,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn training(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);

    let Some(key) = ctx.args.first().map(|a| a.to_lowercase()) else {
        let stat = |key: &str| or_default(rpg_int(&rpg, key), board_stat(key));
        let mut txt = String::from("🏋️ *ᴛʀᴀɪɴɪɴɢ sʏsᴛᴇᴍ*\n\n> Latihan untuk meningkatkan stats!\n\n*📊 *sᴛᴀᴛs ᴋᴀᴍᴜ:*\n\n");
        txt += &format!("> ⚔️ Attack: *{}*\n", stat("attack"));
        txt += &format!("> 🛡️ Defense: *{}*\n", stat("defense"));
        txt += &format!("> ❤️ Health: *{}*\n", stat("health"));
        txt += &format!("> 💨 Speed: *{}*\n", stat("speed"));
        txt += &format!("> 🍀 Luck: *{}*\n", stat("luck"));
        // Baris stamina tetap `?? 100` — nol tidak di-backfill di sini.
        txt += &format!(
            "> ⚡ Stamina: *{}*\n\n\n*🏋️ *ᴛʀᴀɪɴɪɴɢ:*\n\n",
            coalesce(rpg_int(&rpg, "stamina"), 100)
        );
        for (key, name, _, _, _, _, cost) in TRAINING {
            txt += &format!(
                "> {name}\n> ⚡ Stamina: {cost}\n> → `{}training {key}`\n> \n",
                ctx.prefix
            );
        }
        return Reply::text(txt);
    };

    let Some(&(_, name, stat, min, max, exp_gain, cost)) = find_training(&key) else {
        return Reply::text(format!(
            "❌ Training tidak ditemukan!\n\n> Ketik `{}training` untuk melihat daftar.",
            ctx.prefix
        ));
    };

    let stamina = coalesce(rpg_int(&rpg, "stamina"), 100);
    if stamina < cost {
        return Reply::text(format!(
            "⚡ *sᴛᴀᴍɪɴᴀ ᴋᴜʀᴀɴɢ*\n\n> Butuh: {cost}\n> Punya: {stamina}\n\n\
             💡 Gunakan `{}rest` atau makan makanan",
            ctx.prefix
        ));
    }
    rpg.insert("stamina".into(), serde_json::json!(stamina - cost));

    let bonus = roll_incl(min, max);
    let current = or_default(rpg_int(&rpg, stat), base_stat(stat));
    let next = current + bonus;
    rpg.insert(stat.to_string(), serde_json::json!(next));

    let old_exp = or_default(ctx.user_field("exp").and_then(Value::as_i64), 0);
    let new_exp = apply_exp(&mut rpg, old_exp, exp_gain);

    Reply::text(format!(
        "💪 *ᴛʀᴀɪɴɪɴɢ sᴇʟᴇsᴀɪ!*\n\n\
         *📊 *ʀᴇsᴜʟᴛ:*
\n\
         > 🏋️ Training: *{name}*\n\
         > 📈 {stat}: *{current} → {next}* (+{bonus})\n\
         > ⚡ Stamina: *-{cost}*\n\
         > ✨ EXP: *+{exp_gain}*\n"
    ))
    .with_reaction("💪")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(args: &[&str], user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "training".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("training").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn data(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { data, .. } => data,
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("training").unwrap();
        assert_eq!(p.meta.name, "training");
        assert_eq!(p.meta.alias, ["train", "latihan", "workout"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Latihan untuk meningkatkan stats");
        assert_eq!(p.meta.usage, ".training <attack/defense/health>");
        assert_eq!(p.meta.example, ".training attack");
        assert_eq!(p.meta.cooldown, 180);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["train", "latihan", "workout"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "training");
        }
    }

    #[test]
    fn the_board_shows_the_five_backfilled_stats_and_the_menu() {
        let r = run(&[], serde_json::json!({}));
        let t = text_of(&r);
        assert!(t.starts_with("🏋️ *ᴛʀᴀɪɴɪɴɢ sʏsᴛᴇᴍ*"), "{t}");
        assert!(t.contains("> ⚔️ Attack: *10*\n> 🛡️ Defense: *5*\n> ❤️ Health: *100*"), "{t}");
        assert!(t.contains("> 💨 Speed: *10*\n> 🍀 Luck: *5*\n> ⚡ Stamina: *100*"), "{t}");
        for (key, name, _, _, _, _, cost) in TRAINING {
            assert!(t.contains(&format!("> {name}\n> ⚡ Stamina: {cost}\n")), "{key}: {t}");
            assert!(t.contains(&format!("`.training {key}`")), "{key}: {t}");
        }
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_zero_stat_is_backfilled_because_the_board_uses_or() {
        let r = run(&[], serde_json::json!({"rpg": {"attack": 0, "stamina": 0}}));
        let t = text_of(&r);
        assert!(t.contains("> ⚔️ Attack: *10*"), "`|| 10` mengembalikan 0 ke 10: {t}");
        assert!(t.contains("> ⚡ Stamina: *0*"), "`?? 100` tidak ada di baris ini: {t}");
    }

    #[test]
    fn an_unknown_discipline_is_rejected() {
        let r = run(&["swim"], serde_json::json!({"rpg": {"stamina": 100}}));
        let t = text_of(&r);
        assert!(t.starts_with("❌ Training tidak ditemukan!"), "{t}");
        assert!(t.contains("`.training` untuk melihat daftar."), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn every_discipline_adds_a_bonus_inside_its_own_band_and_costs_its_stamina() {
        for (key, name, stat, min, max, exp_gain, cost) in TRAINING {
            for _ in 0..200 {
                let r = run(&[key], serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
                let d = data(&r);
                let t = text_of(&r);
                assert!(t.contains(&format!("> 🏋️ Training: *{name}*")), "{t}");
                let line = t
                    .lines()
                    .find(|l| l.starts_with(&format!("> 📈 {stat}:")))
                    .unwrap_or_else(|| panic!("tidak ada baris {stat}: {t}"));
                let (from, to) = parse_arrow(line);
                assert!((min..=max).contains(&(to - from)), "{key}: bonus {to} - {from}");
                assert_eq!(d["rpg"][stat], serde_json::json!(to), "{key}: stat harus sama dengan baris");
                assert_eq!(d["rpg"]["stamina"], serde_json::json!(100 - cost), "{key}");
                assert_eq!(d["exp"], serde_json::json!(exp_gain), "{key}");
            }
        }
    }

    fn parse_arrow(line: &str) -> (i64, i64) {
        let body = line.split("*").nth(1).unwrap();
        let (a, b) = body.split_once("→").unwrap();
        (
            a.trim().parse().unwrap(),
            b.trim_start_matches(|c: char| !c.is_ascii_digit()).parse().unwrap(),
        )
    }

    #[test]
    fn the_first_session_starts_from_the_table_default_not_zero() {
        let r = run(&["defense"], serde_json::json!({"rpg": {"stamina": 100}, "exp": 0}));
        let d = data(&r);
        let defense = d["rpg"]["defense"].as_i64().unwrap();
        assert!((6..=7).contains(&defense), "5 + bonus 1..2 = {defense}");
    }

    #[test]
    fn an_exhausted_bar_blocks_the_session_and_keeps_the_stat() {
        let r = run(
            &["health"],
            serde_json::json!({"rpg": {"stamina": 24, "health": 200}}),
        );
        let t = text_of(&r);
        assert!(t.starts_with("⚡ *sᴛᴀᴍɪɴᴀ ᴋᴜʀᴀɴɢ*"), "{t}");
        assert!(t.contains("> Butuh: 25\n> Punya: 24"), "{t}");
        assert!(r.mutations().is_empty());
    }
}
