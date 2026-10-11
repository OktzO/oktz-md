//! `plugins/user/energi.js` — baca `users[sender].energi` + `isPremium`.
//!
//! `ponytail:` `db.setting('energi')` adalah toggle GLOBAL di store, dan `Ctx`
//! hanya membawa `users[sender]` + `groups[chat]`. Handler ini karena itu
//! selalu memakai cabang "energi aktif" (`config.energi.enabled === true`).
//! Cabang "OFF" tetap diuji lewat [`render`], yang menerima flag-nya sebagai
//! argumen — begitu router expose settings, cukup teruskan nilainya.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::user_target;
use crate::tools::kalkulatormbg::thousands_id;

/// `config.energi` (`config.js:112-117`).
pub(crate) const ENERGI_ENABLED: bool = true;
pub(crate) const ENERGI_DEFAULT: i64 = 99_999;
pub(crate) const ENERGI_PREMIUM: i64 = 99_999_999;
pub(crate) const UNLIMITED: i64 = -1;

/// `formatNumber` (`energi.js:16-19`) — `-1` jadi `∞ Unlimited`.
pub(crate) fn format_energi(num: i64) -> String {
    if num == UNLIMITED {
        "∞ Unlimited".to_string()
    } else {
        thousands_id(num as f64)
    }
}

/// `!energiEnabled || isOwner` → `-1`; premium → saldo sendiri atau tier
/// premium; selain itu saldo sendiri atau tier default.
pub(crate) fn final_energi(ctx: &Ctx, energi_enabled: bool, is_owner: bool) -> i64 {
    if !energi_enabled || is_owner {
        return UNLIMITED;
    }
    let tier = if ctx.user_field("isPremium").and_then(Value::as_bool) == Some(true) {
        ENERGI_PREMIUM
    } else {
        ENERGI_DEFAULT
    };
    ctx.user_field("energi")
        .and_then(Value::as_i64)
        .unwrap_or(tier)
}

/// `Free` / `Premium` / `Owner`, plus sufiks `(Energi OFF)` kalau toggle mati.
pub(crate) fn user_status(is_owner: bool, is_premium: bool, energi_enabled: bool) -> String {
    let mut s = if is_owner {
        "Owner".to_string()
    } else if is_premium {
        "Premium".to_string()
    } else {
        "Free".to_string()
    };
    if !energi_enabled {
        s.push_str(" (Energi OFF)");
    }
    s
}

pub(crate) fn render(ctx: &Ctx, energi_enabled: bool) -> String {
    let target = user_target(ctx);
    let is_premium = ctx.user_field("isPremium").and_then(Value::as_bool) == Some(true);
    let is_owner = ctx.is_owner;
    let is_self = target.jid == ctx.sender;

    let final_energi = final_energi(ctx, energi_enabled, is_owner);
    let is_unlimited = final_energi == UNLIMITED;

    let mut text = String::from("*〔 ⚡ ENERGI INFO 〕*\n\n");
    text.push_str(&format!("*〔 👤 User 〕* {}\n", target.name));
    text.push_str(&format!(
        "*〔 ⚡ Energi 〕* {}\n",
        format_energi(final_energi)
    ));
    text.push_str(&format!(
        "*〔 💎 Status 〕* {}\n\n",
        user_status(is_owner, is_premium, energi_enabled)
    ));

    if !energi_enabled {
        text.push_str("🔌 Sistem energi dinonaktifkan — semua command gratis");
    } else if is_self && !is_unlimited && final_energi < 10 {
        text.push_str("⚠️ Energi hampir habis!\n");
        text.push_str("Gunakan `.buyenergi` untuk beli");
    } else if is_unlimited {
        text.push_str("✨ Energi unlimited aktif!");
    }
    text
}

#[plugin(
    name = "energi",
    alias = ["cekenergi", "myenergi", "energy", "limit", "ceklimit"],
    category = "user",
    description = "Cek energi user",
    usage = ".energi [@user]",
    example = ".energi",
    cooldown = 3,
    energi = 0,
    is_private = false
)]
pub fn energi(ctx: Ctx) -> Reply {
    Reply::text(render(&ctx, ENERGI_ENABLED))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(user: serde_json::Value, mentioned: Vec<&str>, is_owner: bool) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "energi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            is_owner,
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("energi").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn ctx_with(user: serde_json::Value, is_owner: bool) -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            is_owner,
            user_data: user,
            ..Default::default()
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("energi").unwrap();
        assert_eq!(p.meta.name, "energi");
        assert_eq!(
            p.meta.alias,
            ["cekenergi", "myenergi", "energy", "limit", "ceklimit"]
        );
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Cek energi user");
        assert_eq!(p.meta.usage, ".energi [@user]");
        assert_eq!(p.meta.example, ".energi");
        assert_eq!(p.meta.cooldown, 3);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["cekenergi", "myenergi", "energy", "limit", "ceklimit"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "energi");
        }
    }

    // -- cabang toggle global --------------------------------------------

    #[test]
    fn the_disabled_flag_wins_over_everything_and_marks_the_status() {
        for (user, status) in [
            (json!({}), "Free (Energi OFF)"),
            (json!({"energi": 5}), "Free (Energi OFF)"),
            (json!({"energi": -1}), "Free (Energi OFF)"),
            (json!({"isPremium": true}), "Premium (Energi OFF)"),
        ] {
            let out = render(&ctx_with(user.clone(), false), false);
            assert!(out.contains("*〔 ⚡ Energi 〕* ∞ Unlimited\n"), "{user}");
            assert!(
                out.contains(&format!("*〔 💎 Status 〕* {status}\n")),
                "{user}"
            );
            assert!(out.ends_with("🔌 Sistem energi dinonaktifkan — semua command gratis"));
        }
        // Owner juga tidak lolos: `!energiEnabled` dicek lebih dulu.
        assert!(render(&ctx_with(json!({}), true), false)
            .contains("*〔 💎 Status 〕* Owner (Energi OFF)\n"));
    }

    #[test]
    fn the_disabled_flag_is_the_only_thing_that_reaches_that_branch() {
        // `ENERGI_ENABLED` mengikat apa yang dilihat handler sungguhan; kalau
        // nilainya dibalik ke false, cabang OFF harus muncul di output nyata.
        let out = render(&ctx_with(json!({"energi": 3}), false), ENERGI_ENABLED);
        assert!(!out.contains("Energi OFF"));
        assert!(!out.contains("🔌 Sistem energi dinonaktifkan"));
        assert!(out.ends_with("⚠️ Energi hampir habis!\nGunakan `.buyenergi` untuk beli"));
    }

    // -- cabang state tersimpan -------------------------------------------

    #[test]
    fn an_absent_balance_uses_the_config_tier() {
        assert_eq!(final_energi(&ctx_with(json!(null), false), true, false), 99_999);
        assert_eq!(
            final_energi(&ctx_with(json!({"isPremium": true}), false), true, false),
            99_999_999
        );
    }

    #[test]
    fn a_stored_balance_overrides_the_tier_including_zero() {
        assert_eq!(final_energi(&ctx_with(json!({"energi": 0}), false), true, false), 0);
        assert_eq!(
            final_energi(&ctx_with(json!({"energi": 42, "isPremium": true}), false), true, false),
            42
        );
        assert_eq!(
            final_energi(&ctx_with(json!({"energi": -1}), false), true, false),
            UNLIMITED,
            "`-1` di store berarti unlimited, bukan saldo negatif"
        );
    }

    #[test]
    fn owner_is_always_unlimited_regardless_of_tier() {
        assert_eq!(
            final_energi(&ctx_with(json!({"energi": 7, "isPremium": true}), true), true, true),
            UNLIMITED
        );
        assert!(render(&ctx_with(json!({"energi": 7}), true), true)
            .contains("*〔 💎 Status 〕* Owner\n"));
    }

    #[test]
    fn status_line_maps_each_tier_and_the_disabled_suffix() {
        assert_eq!(user_status(false, false, true), "Free");
        assert_eq!(user_status(false, true, true), "Premium");
        assert_eq!(user_status(true, false, true), "Owner");
        assert_eq!(user_status(true, true, true), "Owner", "owner menang");
        assert_eq!(user_status(true, true, false), "Owner (Energi OFF)");
    }

    // -- format ------------------------------------------------------------

    #[test]
    fn minus_one_renders_as_infinity_and_numbers_are_dotted() {
        assert_eq!(format_energi(-1), "∞ Unlimited");
        assert_eq!(format_energi(0), "0");
        assert_eq!(format_energi(99_999), "99.999");
        assert_eq!(format_energi(99_999_999), "99.999.999");
    }

    // -- cabang pesan bawah ----------------------------------------------

    #[test]
    fn low_balance_warns_only_the_owner_of_the_account() {
        let low = render(&ctx_with(json!({"energi": 3}), false), true);
        assert!(low.ends_with("⚠️ Energi hampir habis!\nGunakan `.buyenergi` untuk beli"));

        // Akun yang sama, tapi dilihat dari mention orang lain — `isSelf` mati.
        let mut seen_by_other = ctx_with(json!({"energi": 3}), false);
        seen_by_other.mentioned = vec!["628777@s.whatsapp.net".into()];
        let other = render(&seen_by_other, true);
        assert!(!other.contains("⚠️"), "branch `isSelf` hanya untuk target == sender");
        assert!(other.contains("*〔 👤 User 〕* 628777\n"));
        assert!(other.ends_with("*〔 💎 Status 〕* Free\n\n"));
    }

    #[test]
    fn unlimited_reports_the_unlimited_line() {
        assert!(render(&ctx_with(json!({"energi": -1}), false), true)
            .ends_with("✨ Energi unlimited aktif!"));
        assert!(render(&ctx_with(json!({}), true), true).ends_with("✨ Energi unlimited aktif!"));
    }

    #[test]
    fn a_healthy_balance_adds_no_footer_at_all() {
        let out = run(json!({"energi": 50}), vec![], false);
        assert!(out.ends_with("*〔 💎 Status 〕* Free\n\n"));
        assert_eq!(out.matches('\n').count(), 6, "tanpa baris tambahan apa pun");
    }

    #[test]
    fn a_mention_shows_the_target_number_and_hides_the_self_footer() {
        let ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            mentioned: vec!["628999@s.whatsapp.net".into()],
            user_data: json!({"energi": 2}),
            ..Default::default()
        };
        let out = render(&ctx, true);
        assert!(out.contains("*〔 👤 User 〕* 628999\n"));
        assert!(!out.contains("⚠️"), "footer hanya untuk `isSelf`");
    }
}