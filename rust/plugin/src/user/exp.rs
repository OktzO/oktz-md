//! `plugins/user/exp.js` — baca `users[sender].exp`.
//!
//! `calculateLevel` / `getRole` di JS diambil dari `src/lib/level.js`, yang
//! salinannya identik dengan `plugins/user/level.js` — dipakai dua file,
//! jadi didefinisikan sekali di sini.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{int_field, user_target};

pub(crate) const EXP_PER_LEVEL: i64 = 10_000;

/// `Math.floor(exp / EXP_PER_LEVEL) + 1`.
pub(crate) fn calculate_level(exp: i64) -> i64 {
    exp.div_euclid(EXP_PER_LEVEL) + 1
}

/// `getRole` (`level.js:19-27`) — ambang dari yang tertinggi.
pub(crate) fn role_of(level: i64) -> &'static str {
    if level >= 100 {
        "🐉 Mythic"
    } else if level >= 80 {
        "⚔️ Legend"
    } else if level >= 60 {
        "💜 Epic"
    } else if level >= 40 {
        "💪 Grandmaster"
    } else if level >= 20 {
        "🎖️ Master"
    } else if level >= 10 {
        "⭐ Elite"
    } else {
        "🛡️ Warrior"
    }
}

/// `formatNumber` (`exp.js:11-15`) — B/M/K dengan satu desimal, lalu ribuan
/// bertitik di bawah 1000.
pub(crate) fn format_exp(num: i64) -> String {
    let n = num as f64;
    if n >= 1_000_000_000.0 {
        return format!("{:.1}B", n / 1_000_000_000.0);
    }
    if n >= 1_000_000.0 {
        return format!("{:.1}M", n / 1_000_000.0);
    }
    if n >= 1_000.0 {
        return format!("{:.1}K", n / 1_000.0);
    }
    crate::tools::kalkulatormbg::thousands_id(n)
}

#[plugin(
    name = "exp",
    alias = ["cekexp", "myexp", "xp"],
    category = "user",
    description = "Cek exp user",
    usage = ".exp [@user]",
    example = ".exp",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn exp(ctx: Ctx) -> Reply {
    let target = user_target(&ctx);
    let user_exp = int_field(&ctx, "exp");
    let level = calculate_level(user_exp);

    let mut text = String::from("*〔 ⭐ EXP INFO 〕*\n\n");
    text.push_str(&format!("*〔 👤 User 〕* {}\n", target.name));
    text.push_str(&format!("*〔 ⭐ Exp 〕* {}\n", format_exp(user_exp)));
    text.push_str(&format!("*〔 🏆 Level 〕* {level}\n"));
    text.push_str(&format!("*〔 🎖️ Title 〕* {}\n", role_of(level)));
    Reply::text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `find("exp")` ambigu: `exp` juga alias plugin `expedition.js`, dan
    /// `find` mengembalikan siapa pun yang terdaftar lebih dulu di
    /// `distributed_slice`. Cari lewat `meta.name` supaya test ini selalu
    /// menguji plugin yang namanya `exp`.
    fn plugin() -> &'static oktz_plugin_api::registry::PluginEntry {
        oktz_plugin_api::all_plugins()
            .iter()
            .copied()
            .find(|e| e.meta.name == "exp")
            .expect("plugin exp harus terdaftar")
    }

    fn run(user: serde_json::Value, mentioned: Vec<&str>) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "exp".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (plugin().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = plugin();
        assert_eq!(p.meta.name, "exp");
        assert_eq!(p.meta.alias, ["cekexp", "myexp", "xp"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Cek exp user");
        assert_eq!(p.meta.usage, ".exp [@user]");
        assert_eq!(p.meta.example, ".exp");
        assert_eq!(p.meta.cooldown, 3);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["cekexp", "myexp", "xp"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "exp");
        }
    }

    #[test]
    fn level_is_floor_div_plus_one() {
        assert_eq!(calculate_level(0), 1);
        assert_eq!(calculate_level(9_999), 1);
        assert_eq!(calculate_level(10_000), 2);
        assert_eq!(calculate_level(99_999), 10);
        assert_eq!(calculate_level(999_999), 100);
    }

    #[test]
    fn negative_exp_floors_like_javascript_math_floor() {
        // `Math.floor(-1 / 10000)` = -1 -> level 0. `div_euclid` memang sama
        // untuk pembagi positif, tapi test ini menahan regresi ke truncate.
        assert_eq!(calculate_level(-1), 0);
        assert_eq!(calculate_level(-10_000), 0);
        assert_eq!(calculate_level(-10_001), -1);
        assert_eq!(role_of(calculate_level(-1)), "🛡️ Warrior");
    }

    #[test]
    fn role_bands_match_the_js_thresholds() {
        for (lvl, role) in [
            (0, "🛡️ Warrior"),
            (9, "🛡️ Warrior"),
            (10, "⭐ Elite"),
            (19, "⭐ Elite"),
            (20, "🎖️ Master"),
            (39, "🎖️ Master"),
            (40, "💪 Grandmaster"),
            (59, "💪 Grandmaster"),
            (60, "💜 Epic"),
            (79, "💜 Epic"),
            (80, "⚔️ Legend"),
            (99, "⚔️ Legend"),
            (100, "🐉 Mythic"),
            (1_000, "🐉 Mythic"),
        ] {
            assert_eq!(role_of(lvl), role, "level {lvl}");
        }
    }

    #[test]
    fn format_exp_switches_unit_at_each_threshold() {
        assert_eq!(format_exp(0), "0");
        assert_eq!(format_exp(999), "999");
        assert_eq!(format_exp(1_000), "1.0K");
        assert_eq!(format_exp(15_400), "15.4K");
        assert_eq!(format_exp(999_999), "1000.0K");
        assert_eq!(format_exp(1_000_000), "1.0M");
        assert_eq!(format_exp(2_500_000_000), "2.5B");
    }

    #[test]
    fn absent_state_shows_zero_exp_and_level_one() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"exp": null})] {
            let out = run(user.clone(), vec![]);
            assert_eq!(
                out,
                "*〔 ⭐ EXP INFO 〕*\n\n*〔 👤 User 〕* Kamu\n*〔 ⭐ Exp 〕* 0\n*〔 🏆 Level 〕* 1\n*〔 🎖️ Title 〕* 🛡️ Warrior\n",
                "input {user}"
            );
        }
    }

    #[test]
    fn stored_exp_drives_level_and_title_together() {
        let out = run(serde_json::json!({"exp": 999_999}), vec![]);
        assert!(out.contains("*〔 ⭐ Exp 〕* 1000.0K\n"));
        assert!(out.contains("*〔 🏆 Level 〕* 100\n"));
        assert!(out.contains("*〔 🎖️ Title 〕* 🐉 Mythic\n"));
    }

    #[test]
    fn a_mention_shows_the_target_number() {
        let out = run(serde_json::json!({"exp": 50_000}), vec!["628999@s.whatsapp.net"]);
        assert!(out.contains("*〔 👤 User 〕* 628999\n"));
        assert!(out.contains("*〔 🏆 Level 〕* 6\n"));
        assert!(out.contains("*〔 🎖️ Title 〕* 🛡️ Warrior\n"));
    }
}