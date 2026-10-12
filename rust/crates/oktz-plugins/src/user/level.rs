//! `plugins/user/level.js` — baca `users[sender].exp`, kotak level + progres.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::int_field;
use crate::tools::kalkulatormbg::thousands_id;

pub(crate) use crate::user::exp::{EXP_PER_LEVEL, calculate_level, role_of};

/// `expForLevel(level)` = `(level - 1) * EXP_PER_LEVEL`.
pub(crate) fn exp_for_level(level: i64) -> i64 {
    (level - 1) * EXP_PER_LEVEL
}

/// `expToNextLevel(exp)` = `expForLevel(level + 1) - exp`.
pub(crate) fn exp_to_next_level(exp: i64) -> i64 {
    exp_for_level(calculate_level(exp) + 1) - exp
}

/// `getLevelBar` (`level.js:29-37`) — 10 slot, `Math.min` di atas jumlah slot
/// supaya bar tidak pernah melebihi 10.
pub(crate) fn level_bar(current: i64, target: i64) -> String {
    let filled = if target <= 0 {
        // `Math.floor(x / 0)` = Infinity, `Math.min(Infinity, 10)` = 10.
        10
    } else {
        ((current as f64 / target as f64 * 10.0).floor() as i64).min(10)
    };
    let filled = filled.clamp(0, 10) as usize;
    "▰".repeat(filled) + &"▱".repeat(10 - filled)
}

pub(crate) fn render(ctx: &Ctx) -> String {
    let target = crate::common::user_target(ctx);
    let exp = int_field(ctx, "exp");
    let level = calculate_level(exp);
    let role = role_of(level);

    let current_level_exp = exp_for_level(level);
    let next_level_exp = exp_for_level(level + 1);
    let exp_in_level = exp - current_level_exp;
    let exp_needed = next_level_exp - current_level_exp;
    let progress = level_bar(exp_in_level, exp_needed);

    let mut txt = String::from("╭━━━━━━━━━━━━━━━━━╮\n");
    txt.push_str("┃ 📊 *ʟᴇᴠᴇʟ ɪɴꜰᴏ*\n");
    txt.push_str("╰━━━━━━━━━━━━━━━━━╯\n\n");

    txt.push_str("╭┈┈⬡「 👤 *ᴜsᴇʀ* 」\n");
    txt.push_str(&format!("┃ 🏷️ Name: *{}*\n", target.name));
    txt.push_str(&format!(
        "┃ 🆔 Tag: @{}\n",
        crate::common::jid_number(&target.jid)
    ));
    txt.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    txt.push_str("╭┈┈⬡「 📈 *sᴛᴀᴛs* 」\n");
    txt.push_str(&format!("┃ 📊 Level: *{level}*\n"));
    txt.push_str(&format!("┃ {role}\n"));
    txt.push_str(&format!("┃ 🚄 Exp: *{}*\n", thousands_id(exp as f64)));
    txt.push_str("┃ 📊 Progress:\n");
    txt.push_str(&format!("┃ {progress}\n"));
    txt.push_str(&format!(
        "┃ {} / {}\n",
        thousands_id(exp_in_level as f64),
        thousands_id(exp_needed as f64)
    ));
    txt.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    txt.push_str(&format!(
        "> Next level: *{}* exp* lagi!",
        thousands_id(exp_to_next_level(exp) as f64)
    ));
    txt
}

#[plugin(
    name = "level",
    alias = ["lvl", "ceklevel"],
    category = "user",
    description = "Cek level user",
    usage = ".level [@user]",
    example = ".level",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn level(ctx: Ctx) -> Reply {
    let target = crate::common::user_target(&ctx);
    Reply::Text {
        text: render(&ctx),
        mentions: vec![target.jid],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: serde_json::Value, mentioned: Vec<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "level".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("level").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("level").unwrap();
        assert_eq!(p.meta.name, "level");
        assert_eq!(p.meta.alias, ["lvl", "ceklevel"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Cek level user");
        assert_eq!(p.meta.usage, ".level [@user]");
        assert_eq!(p.meta.example, ".level");
        assert_eq!(p.meta.cooldown, 3);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["lvl", "ceklevel"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "level");
        }
    }

    #[test]
    fn exp_for_level_and_exp_to_next_are_consistent() {
        assert_eq!(exp_for_level(1), 0);
        assert_eq!(exp_for_level(2), 10_000);
        assert_eq!(exp_to_next_level(0), 10_000);
        assert_eq!(exp_to_next_level(9_999), 1);
        assert_eq!(exp_to_next_level(10_000), 10_000);
        assert_eq!(exp_to_next_level(24_999), 5_001);
    }

    #[test]
    fn level_bar_has_ten_slots_and_never_overflows() {
        assert_eq!(level_bar(0, 10_000), "▱▱▱▱▱▱▱▱▱▱");
        assert_eq!(level_bar(5_000, 10_000), "▰▰▰▰▰▱▱▱▱▱");
        assert_eq!(level_bar(10_000, 10_000), "▰▰▰▰▰▰▰▰▰▰");
        // `Math.min` menahan nilai di atas target.
        assert_eq!(level_bar(99_999, 10_000), "▰▰▰▰▰▰▰▰▰▰");
        for (c, t) in [(0, 10_000), (1, 10_000), (3_333, 10_000), (9_999, 10_000)] {
            assert_eq!(level_bar(c, t).chars().count(), 10, "({c}, {t})");
        }
    }

    #[test]
    fn level_bar_survives_a_zero_target() {
        // `current / 0` = Infinity di JS, `Math.min(Infinity, 10)` = 10.
        assert_eq!(level_bar(0, 0), "▰▰▰▰▰▰▰▰▰▰");
    }

    #[test]
    fn absent_state_renders_level_one_with_an_empty_bar() {
        for user in [json!(null), json!({}), json!({"exp": null})] {
            let reply = run(user.clone(), vec![]);
            let out = text_of(&reply);
            assert!(out.starts_with("╭━━━━━━━━━━━━━━━━━╮\n┃ 📊 *ʟᴇᴠᴇʟ ɪɴꜰᴏ*\n╰━━━━━━━━━━━━━━━━━╯\n\n"), "{user}");
            assert!(out.contains("┃ 🏷️ Name: *Kamu*\n"));
            assert!(out.contains("┃ 🆔 Tag: @628111\n"));
            assert!(out.contains("┃ 📊 Level: *1*\n"));
            assert!(out.contains("┃ 🛡️ Warrior\n"));
            assert!(out.contains("┃ 🚄 Exp: *0*\n"));
            assert!(out.contains("┃ ▱▱▱▱▱▱▱▱▱▱\n"));
            assert!(out.contains("┃ 0 / 10.000\n"));
            assert!(out.ends_with("> Next level: *10.000* exp* lagi!"));
        }
    }

    #[test]
    fn progress_advances_with_stored_exp() {
        let reply = run(json!({"exp": 17_500}), vec![]);
        let out = text_of(&reply);
        assert!(out.contains("┃ 📊 Level: *2*\n"));
        assert!(out.contains("┃ 🚄 Exp: *17.500*\n"));
        assert!(out.contains("┃ ▰▰▰▰▰▰▰▱▱▱\n"));
        assert!(out.contains("┃ 7.500 / 10.000\n"));
        assert!(out.ends_with("> Next level: *2.500* exp* lagi!"));
    }

    #[test]
    fn a_mention_changes_the_name_the_tag_and_the_mention_list() {
        let r = run(json!({"exp": 0}), vec!["628999:5@s.whatsapp.net"]);
        let out = text_of(&r);
        assert!(out.contains("┃ 🏷️ Name: *628999:5*\n"), "`jid.split('@')[0]` di JS");
        assert!(out.contains("┃ 🆔 Tag: @628999:5\n"));
        match r {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &["628999:5@s.whatsapp.net"]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn both_boxes_open_and_close_exactly_once_each() {
        let reply = run(json!({"exp": 100}), vec![]);
        let out = text_of(&reply);
        assert_eq!(out.matches("╭┈┈⬡").count(), 2);
        assert_eq!(out.matches("╰┈┈┈┈┈┈┈┈⬡").count(), 2);
        assert_eq!(out.matches("╭━━━━━━━━━━━━━━━━━╮").count(), 1);
    }
}