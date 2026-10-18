//! `plugins/cek/cekprem.js` — detail status premium dari daftar `premium`.
//!
//! `ponytail:` `config.isPremium/isOwner` membaca `config.js`, yang tidak ada
//! di `Ctx`. Gantinya: daftar `owner`/`partner`/`premium` dari settings.
//! Sifatnya dipertahankan: di `config.js`, owner selalu dianggap premium dan
//! partner selalu dianggap premium, jadi daftar yang-shortcut itu ikut
//! ditiru agar plug-in ini tidak memberi jawaban lebih longgar dari JS.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::cek::cekowner::{energi, koin, level, normalize, resolve_target, thousands_id};
use crate::cek::cekpartner::{added_at, days_between, format_date, total_days};
use crate::common::{joined_strings, now_ms};

/// `config.isOwner` → hanya daftar owner.
fn is_owner(ctx: &Ctx, number: &str) -> bool {
    joined_strings(ctx.setting_list("owner")).iter().any(|n| n == number)
}

/// `config.isPartner` — owner otomatis partner (`config.js:466`).
fn is_partner(ctx: &Ctx, number: &str) -> bool {
    is_owner(ctx, number) || joined_strings(ctx.setting_list("partner")).iter().any(|n| n == number)
}

/// `config.isPremium` — owner dan partner otomatis premium (`config.js:406`).
fn is_premium(ctx: &Ctx, number: &str) -> bool {
    is_partner(ctx, number) || joined_strings(ctx.setting_list("premium")).iter().any(|n| n == number)
}

/// `db.data.premium.find(p => typeof p === 'string' ? p === n : p.id === n)`.
fn find_premium(ctx: &Ctx, number: &str) -> Option<serde_json::Value> {
    ctx.setting_list("premium")
        .iter()
        .find(|p| match p {
            serde_json::Value::String(s) => s == number,
            other => other.get("id").and_then(serde_json::Value::as_str) == Some(number),
        })
        .cloned()
}

/// `user.exp ?? 0` dengan `toLocaleString('id-ID')`.
fn exp_of(user: &serde_json::Value) -> String {
    thousands_id(
        user.get("exp")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
    )
}

#[plugin(
    name = "cekprem",
    alias = ["cekpremium", "preminfo"],
    category = "cek",
    description = "Cek detail status premium user",
    usage = ".cekprem @user",
    example = ".cekprem",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_premium = true
)]
pub fn cekprem(ctx: Ctx) -> Reply {
    let (_, raw) = resolve_target(&ctx);
    let number = normalize(raw);
    let jid = format!("{number}@s.whatsapp.net");

    let prem = find_premium(&ctx, &number);
    let owner = is_owner(&ctx, &number);

    if prem.is_none() && !is_premium(&ctx, &number) {
        return Reply::Text {
            text: format!("❌ @{number} bukan premium"),
            mentions: vec![jid],
        };
    }

    let mut txt = String::from("💎 *DETAIL PREMIUM*\n\n");
    txt.push_str(&format!("👤 User: @{number}\n"));

    let permanent = matches!(&prem, Some(serde_json::Value::String(_)))
        || prem
            .as_ref()
            .and_then(|p| p.get("expired"))
            .is_none_or(|v| v.is_null());

    if owner {
        txt.push_str("🏷️ Role: *👑 Owner (Permanent)*\n");
    } else if permanent {
        txt.push_str("🏷️ Role: *💎 Premium (Permanent)*\n");
    } else {
        let info = prem.as_ref().expect("dijamin Some di cabang ini");
        let expired = info.get("expired").and_then(serde_json::Value::as_i64);
        let remaining = days_between(expired, now_ms());
        txt.push_str(&format!(
            "📛 Nama: *{}*\n",
            info.get("name")
                .and_then(serde_json::Value::as_str)
                .filter(|s| !s.is_empty())
                .unwrap_or("Unknown")
        ));
        txt.push_str(&format!("📅 Mulai: *{}*\n", added_at(info)));
        txt.push_str(&format!(
            "⏳ Expired: *{}*\n",
            format_date(info.get("expired"))
        ));
        txt.push_str(&format!("🗓️ Durasi: *{} hari*\n", total_days(info)));
        txt.push_str(&format!(
            "📊 Sisa: *{}*\n",
            if remaining > 0 {
                format!("{remaining} hari")
            } else {
                "⚠️ Expired".to_string()
            }
        ));
    }

    let user = if ctx.sender == jid {
        (!ctx.user_data.is_null()).then_some(&ctx.user_data)
    } else {
        ctx.roster
            .participants
            .iter()
            .find(|p| p.jid == jid)
            .map(|p| &p.data)
            .filter(|d| !d.is_null())
    };
    if let Some(user) = user {
        txt.push_str(&format!("⚡ Energi: *{}*\n", energi(user)));
        txt.push_str(&format!("💰 Koin: *{}*\n", koin(user)));
        txt.push_str(&format!("⭐ Exp: *{}*\n", exp_of(user)));
        txt.push_str(&format!("📊 Level: *{}*\n", level(user)));
    }

    Reply::Text {
        text: txt,
        mentions: vec![jid],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    const ME: &str = "628111@s.whatsapp.net";

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("cekprem").unwrap().handler)(ctx)
    }

    fn base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cekprem".into(),
            sender: ME.into(),
            chat: ME.into(),
            ..Default::default()
        }
    }

    fn text_of(r: &Reply) -> (String, Vec<String>) {
        match r {
            Reply::Text { text, mentions } => (text.clone(), mentions.clone()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn ctx_with(settings: Value, user_data: Value) -> Ctx {
        Ctx {
            settings,
            user_data,
            ..base()
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cekprem").unwrap();
        assert_eq!(p.meta.name, "cekprem");
        assert_eq!(p.meta.alias, ["cekpremium", "preminfo"]);
        assert_eq!(p.meta.category, "cek");
        assert_eq!(p.meta.description, "Cek detail status premium user");
        assert_eq!(p.meta.usage, ".cekprem @user");
        assert_eq!(p.meta.example, ".cekprem");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin && p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cekprem", "cekpremium", "preminfo"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cekprem"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn a_plain_number_is_not_premium() {
        for settings in [serde_json::json!({}), serde_json::json!({"premium": []}), serde_json::json!({"premium": ["628999"]})] {
            let ctx = ctx_with(settings.clone(), serde_json::json!({}));
            let (text, mentions) = text_of(&run(ctx.clone()));
            assert_eq!(text, "❌ @628111 bukan premium", "{settings}");
            assert_eq!(mentions, vec![ME.to_string()], "{settings}");
        }
    }

    #[test]
    fn a_bare_string_entry_is_reported_as_permanent() {
        let ctx = ctx_with(serde_json::json!({ "premium": ["628111"] }), serde_json::json!({}));
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.starts_with("💎 *DETAIL PREMIUM*\n\n"), "{text}");
        assert!(text.contains("👤 User: @628111"), "{text}");
        assert!(text.contains("🏷️ Role: *💎 Premium (Permanent)*"), "{text}");
        assert!(!text.contains("📊 Sisa"), "permanent tidak punya hitung mundur: {text}");
    }

    #[test]
    fn an_entry_without_an_expiry_is_also_permanent() {
        for entry in [serde_json::json!({"id": "628111"}), serde_json::json!({"id": "628111", "expired": null})] {
            let ctx = ctx_with(serde_json::json!({ "premium": [entry.clone()] }), serde_json::json!({}));
            let (text, _) = text_of(&run(ctx.clone()));
            assert!(
                text.contains("🏷️ Role: *💎 Premium (Permanent)*"),
                "{entry} -> {text}"
            );
        }
    }

    #[test]
    fn a_dated_entry_reports_the_whole_window() {
        let added = now_ms() - 10 * 86_400_000;
        let expired = now_ms() + 20 * 86_400_000;
        let ctx = ctx_with(
            serde_json::json!({"premium": [{"id": "628111", "name": "Lyoraaa", "addedAt": added, "expired": expired}]}),
            serde_json::json!({}),
        );
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("📛 Nama: *Lyoraaa*"), "{text}");
        assert!(text.contains("🗓️ Durasi: *30 hari*"), "{text}");
        assert!(text.contains("📊 Sisa: *20 hari*"), "{text}");
        assert!(!text.contains("Permanent"), "{text}");
    }

    #[test]
    fn an_owner_always_reports_the_permanent_owner_role_first() {
        // `isConfigOwner` diperiksa sebelum `premData`, dan owner selalu
        // lolos guard walau tidak ada di daftar premium.
        let ctx = ctx_with(serde_json::json!({ "owner": ["628111"] }), serde_json::json!({}));
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("🏷️ Role: *👑 Owner (Permanent)*"), "{text}");
        assert!(!text.contains("💎 Premium (Permanent)"), "{text}");
    }

    #[test]
    fn a_partner_is_premium_without_being_in_the_premium_list() {
        let ctx = ctx_with(serde_json::json!({ "partner": ["628111"] }), serde_json::json!({}));
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("🏷️ Role: *💎 Premium (Permanent)*"), "{text}");
    }

    #[test]
    fn a_stored_user_appends_energi_koin_exp_and_level() {
        let ctx = ctx_with(
            serde_json::json!({ "premium": ["628111"] }),
            serde_json::json!({"energi": -1, "koin": -1, "exp": 25000, "level": 7}),
        );
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("⚡ Energi: *∞*"), "{text}");
        assert!(text.contains("💰 Koin: *∞*"), "{text}");
        assert!(text.contains("⭐ Exp: *25.000*"), "{text}");
        assert!(text.contains("📊 Level: *7*"), "{text}");
    }

    #[test]
    fn missing_store_fields_use_the_js_defaults() {
        let ctx = ctx_with(serde_json::json!({ "premium": ["628111"] }), serde_json::json!({}));
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("⚡ Energi: *0*"), "{text}");
        assert!(text.contains("💰 Koin: *0*"), "{text}");
        assert!(text.contains("⭐ Exp: *0*"), "{text}");
        assert!(text.contains("📊 Level: *1*"), "{text}");
    }

    #[test]
    fn the_plugin_only_reads() {
        let ctx = ctx_with(serde_json::json!({ "premium": ["628111"] }), serde_json::json!({}));
        assert!(run(ctx.clone()).mutations().is_empty());
    }

    #[test]
    fn find_premium_accepts_both_entry_shapes() {
        let ctx = ctx_with(serde_json::json!({ "premium": ["628111", {"id": "628222"}] }), serde_json::json!({}));
        assert!(matches!(
            find_premium(&ctx, "628111"),
            Some(Value::String(_))
        ));
        assert!(matches!(
            find_premium(&ctx, "628222"),
            Some(Value::Object(_))
        ));
        assert!(find_premium(&ctx, "628333").is_none());
    }
}
