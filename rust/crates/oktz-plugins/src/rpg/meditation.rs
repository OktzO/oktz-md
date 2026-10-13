//! `plugins/rpg/meditation.js` — istirahat untuk pulihkan HP, stamina, mana.
//!
//! Reaksi + pesan antara sebelum jeda `setTimeout` tidak masuk `Reply`.
//!
//! Catatan kursor: `maxHealth`/`maxMana` dihitung dari `user.level` (level di
//! akar row, bukan `user.rpg.level`) —-default 1, jadi 105 HP dan 53 mana.
//! Polanya berbeda dari `addexp.js` yang menulis `user.rpg.level`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value, json};

use crate::rpg::jualan::{coalesce, or_default, roll_incl, rpg_int, rpg_map};

pub(crate) const MAX_STAMINA: i64 = 100;

#[plugin(
    name = "meditation",
    alias = ["rest", "istirahat", "tidur", "sleep"],
    category = "rpg",
    description = "Istirahat untuk pulihkan HP dan stamina",
    usage = ".meditation",
    example = ".meditation",
    cooldown = 600,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn meditation(ctx: Ctx) -> Reply {
    let mut rpg = rpg_map(&ctx);

    // Tiga operator berbeda di JS: `?? 100` untuk stamina (nol tetap nol),
    // `|| 100` dan `|| 50` untuk health/mana (nol jadi default).
    let cur_stamina = coalesce(rpg_int(&rpg, "stamina"), MAX_STAMINA);
    let cur_health = or_default(rpg_int(&rpg, "health"), 100);
    let cur_mana = or_default(rpg_int(&rpg, "mana"), 50);

    let level = or_default(ctx.user_field("level").and_then(Value::as_i64), 1);
    let max_health = 100 + level * 5;
    let max_mana = 50 + level * 3;

    if cur_stamina >= MAX_STAMINA && cur_health >= max_health && cur_mana >= max_mana {
        return Reply::text(format!(
            "💤 *sᴜᴅᴀʜ ꜰᴜʟʟ*\n\n\
             > ⚡ Stamina: {cur_stamina}/{MAX_STAMINA}\n\
             > ❤️ Health: {cur_health}/{max_health}\n\
             > 💙 Mana: {cur_mana}/{max_mana}\n\n\
             💡 Kamu sudah dalam kondisi prima!"
        ));
    }

    let stamina_back = (MAX_STAMINA - cur_stamina).min(roll_incl(40, 59));
    let health_back = (max_health - cur_health).min(roll_incl(30, 49));
    let mana_back = (max_mana - cur_mana).min(roll_incl(25, 39));

    let new_stamina = (cur_stamina + stamina_back).min(MAX_STAMINA);
    let new_health = (cur_health + health_back).min(max_health);
    let new_mana = (cur_mana + mana_back).min(max_mana);

    rpg.insert("stamina".into(), json!(new_stamina));
    rpg.insert("health".into(), json!(new_health));
    rpg.insert("mana".into(), json!(new_mana));

    Reply::text(format!(
        "✨ *ɪsᴛɪʀᴀʜᴀᴛ sᴇʟᴇsᴀɪ!*\n\n\
         *💖 *ᴘᴜʟɪʜ:*
\n\
         > ⚡ Stamina: *+{stamina_back}* ({new_stamina}/{MAX_STAMINA})\n\
         > ❤️ Health: *+{health_back}* ({new_health}/{max_health})\n\
         > 💙 Mana: *+{mana_back}* ({new_mana}/{max_mana})\n\n\n\
         > Kamu merasa lebih segar! 🌟"
    ))
    .with_reaction("✨")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(user: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "meditation".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("meditation").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("meditation").unwrap();
        assert_eq!(p.meta.name, "meditation");
        assert_eq!(p.meta.alias, ["rest", "istirahat", "tidur", "sleep"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Istirahat untuk pulihkan HP dan stamina");
        assert_eq!(p.meta.usage, ".meditation");
        assert_eq!(p.meta.example, ".meditation");
        assert_eq!(p.meta.cooldown, 600);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["rest", "istirahat", "tidur", "sleep"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "meditation");
        }
    }

    #[test]
    fn the_rest_cap_is_a_no_op_and_writes_nothing() {
        let r = run(json!({"rpg": {"stamina": 100, "health": 105, "mana": 53}}));
        let t = text_of(&r);
        assert!(t.starts_with("💤 *sᴜᴅᴀʜ ꜰᴜʟʟ*"), "{t}");
        assert!(t.contains("> ⚡ Stamina: 100/100"), "{t}");
        assert!(t.contains("> ❤️ Health: 105/105"), "{t}");
        assert!(t.contains("> 💙 Mana: 53/53"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_user_with_no_rpg_row_is_already_at_the_cap_because_of_the_defaults() {
        // stamina `?? 100` = 100, health `|| 100` = 100 (cap 105), mana `|| 50`
        // = 50 (cap 53) — jadi health dan mana masih di bawah cap.
        let r = run(json!({}));
        let d = written(&r);
        let rpg = &d["rpg"];
        assert_eq!(rpg["stamina"], json!(100), "stamina sudah penuh");
        assert!(rpg["health"].as_i64().unwrap() > 100);
        assert!(rpg["mana"].as_i64().unwrap() > 50);
    }

    #[test]
    fn zero_health_and_zero_mana_are_read_as_full_because_the_js_uses_or() {
        // `|| 100` melihat 0 sebagai falsy, jadi pemulihan dihitung dari 100 —
        // bukan dari 0. Level 20 lifting cap (200 HP / 110 mana) supaya
        // pemulihan tidak pennies up ke batas dan titik mulainya terlihat.
        for _ in 0..20 {
            let r = run(json!({"level": 20, "rpg": {"stamina": 100, "health": 0, "mana": 0}}));
            let t = text_of(&r);
            let rpg = written(&r)["rpg"].clone();
            for (label, start, cap) in [("❤️ Health", 100i64, 200i64), ("💙 Mana", 50, 110)] {
                let line = t
                    .lines()
                    .find(|l| l.starts_with(&format!("> {label}:")))
                    .unwrap_or_else(|| panic!("tidak ada baris {label}: {t}"));
                let gain: i64 = line
                    .split("*+")
                    .nth(1)
                    .and_then(|s| s.split('*').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap();
                let field = if label.contains("Health") { "health" } else { "mana" };
                assert_eq!(
                    rpg[field].as_i64().unwrap(),
                    (start + gain).min(cap),
                    "harus dihitung dari {start}, bukan dari 0: {t}"
                );
                assert!(line.ends_with(&format!("/{cap})")), "{t}");
            }
        }
    }

    #[test]
    fn recovery_never_exceeds_the_cap() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 10, "health": 10, "mana": 10}}));
            let rpg = written(&r)["rpg"].clone();
            let s = rpg["stamina"].as_i64().unwrap();
            let h = rpg["health"].as_i64().unwrap();
            let m = rpg["mana"].as_i64().unwrap();
            assert!((50..=69).contains(&s), "stamina 10 + recover 40..59: {rpg}");
            assert!((40..=105).contains(&h), "health 10 + recover 30..49: {rpg}");
            assert!((35..=53).contains(&m), "mana 10 + recover 25..39: {rpg}");
        }
    }

    #[test]
    fn the_reported_gain_matches_the_actual_delta() {
        for _ in 0..300 {
            let r = run(json!({"rpg": {"stamina": 40, "health": 60, "mana": 20}}));
            let t = text_of(&r);
            let rpg = &written(&r)["rpg"];
            let gain = |label: &str| -> i64 {
                t.split(&format!("> {label}: *+"))
                    .nth(1)
                    .and_then(|s| s.split('*').next())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_else(|| panic!("tidak ada baris {label}: {t}"))
            };
            assert_eq!(gain("⚡ Stamina"), rpg["stamina"].as_i64().unwrap() - 40);
            assert_eq!(gain("❤️ Health"), rpg["health"].as_i64().unwrap() - 60);
            assert_eq!(gain("💙 Mana"), rpg["mana"].as_i64().unwrap() - 20);
        }
    }

    #[test]
    fn a_higher_level_raises_only_the_health_and_mana_caps() {
        // level 10 -> maxHealth 150, maxMana 80; maxStamina tetap 100.
        let r = run(json!({"level": 10, "rpg": {"stamina": 0, "health": 0, "mana": 0}}));
        let t = text_of(&r);
        let line = |label: &str| {
            t.lines()
                .find(|l| l.starts_with(&format!("> {label}:")))
                .unwrap_or_else(|| panic!("tidak ada baris {label}: {t}"))
                .to_string()
        };
        assert!(line("⚡ Stamina").ends_with("/100)"), "cap stamina tetap 100: {t}");
        assert!(line("❤️ Health").ends_with("/150)"), "maxHealth = 100 + 10*5: {t}");
        assert!(line("💙 Mana").ends_with("/80)"), "maxMana = 50 + 10*3: {t}");
    }
}
