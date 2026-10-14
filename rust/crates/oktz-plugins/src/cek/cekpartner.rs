//! `plugins/cek/cekpartner.js` — detail status partner dari daftar `partner`.
//!
//! `ponytail:` `db.data.partner` adalah store top-level, bukan
//! `db.data.settings`. `Ctx` hanya membawa settings, jadi daftarnya dibaca dari
//! `ctx.setting_list("partner")` — sama seperti `owner/delpartner.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::cek::cekowner::{energi, koin, normalize, resolve_target};
use crate::common::{date_long, now_ms};

const DAY_MS: f64 = 1000.0 * 60.0 * 60.0 * 24.0;

/// `new Date(ts).toLocaleDateString('id-ID', {day, month, year})`.
///
/// Timestamp yang bukan angka menghasilkan `Invalid Date` di JS, dan itu
/// diteruskan apa adanya — bukan diganti tanggal Epoch.
pub(crate) fn format_date(v: Option<&serde_json::Value>) -> String {
    match v.and_then(serde_json::Value::as_i64) {
        Some(ms) => date_long(ms),
        None => "Invalid Date".to_string(),
    }
}

/// `Math.ceil((expired - from) / 86400000)` — `NaN` jadi 0, seperti `?? '?'`
/// tidak pernah terpakai di JS tapi hasil render tetap harus berupa angka.
pub(crate) fn days_between(expired: Option<i64>, from: i64) -> i64 {
    match expired {
        Some(e) => ((e - from) as f64 / DAY_MS).ceil() as i64,
        None => 0,
    }
}

/// `totalDays` — `'?'` kalau `addedAt` kosong.
pub(crate) fn total_days(info: &serde_json::Value) -> String {
    match (
        info.get("expired").and_then(serde_json::Value::as_i64),
        info.get("addedAt").and_then(serde_json::Value::as_i64),
    ) {
        (Some(expired), Some(added)) => days_between(Some(expired), added).to_string(),
        (Some(_), None) => "?".to_string(),
        (None, _) => "?".to_string(),
    }
}

/// `info.addedAt ? formatDate(info.addedAt) : 'Unknown'` — nilai kosong
/// dicek sebelum `formatDate`, jadi tidak pernah jadi `Invalid Date`.
pub(crate) fn added_at(info: &serde_json::Value) -> String {
    match info.get("addedAt") {
        Some(v) if !v.is_null() && v.as_i64().is_some() => format_date(Some(v)),
        _ => "Unknown".to_string(),
    }
}

/// `info.name || 'Unknown'`.
fn name_of(info: &serde_json::Value) -> &str {
    info.get("name")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("Unknown")
}

#[plugin(
    name = "cekpartner",
    alias = ["partnerinfo"],
    category = "cek",
    description = "Cek detail status partner user",
    usage = ".cekpartner @user",
    example = ".cekpartner",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_premium = true
)]
pub fn cekpartner(ctx: Ctx) -> Reply {
    let (_, raw) = resolve_target(&ctx);
    let number = normalize(raw);
    let jid = format!("{number}@s.whatsapp.net");

    let info = ctx
        .setting_list("partner")
        .iter()
        .find(|p| p.get("id").and_then(serde_json::Value::as_str) == Some(number.as_str()))
        .cloned();

    let Some(info) = info else {
        return Reply::Text {
            text: format!("❌ @{number} bukan partner"),
            mentions: vec![jid],
        };
    };

    let now = now_ms();
    let expired = info.get("expired").and_then(serde_json::Value::as_i64);
    let remaining = days_between(expired, now);

    let mut txt = String::from("🤝 *DETAIL PARTNER*\n\n");
    txt.push_str(&format!("👤 User: @{number}\n"));
    txt.push_str(&format!("📛 Nama: *{}*\n", name_of(&info)));
    txt.push_str(&format!(
        "📅 Mulai: *{}*\n",
        added_at(&info)
    ));
    txt.push_str(&format!("⏳ Expired: *{}*\n", format_date(info.get("expired"))));
    txt.push_str(&format!("🗓️ Durasi: *{} hari*\n", total_days(&info)));
    txt.push_str(&format!(
        "📊 Sisa: *{}*\n",
        if remaining > 0 {
            format!("{remaining} hari")
        } else {
            "⚠️ Expired".to_string()
        }
    ));

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
    }

    Reply::Text {
        text: txt,
        mentions: vec![jid],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const ME: &str = "628111@s.whatsapp.net";

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("cekpartner").unwrap().handler)(ctx)
    }

    fn base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cekpartner".into(),
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cekpartner").unwrap();
        assert_eq!(p.meta.name, "cekpartner");
        assert_eq!(p.meta.alias, ["partnerinfo"]);
        assert_eq!(p.meta.category, "cek");
        assert_eq!(p.meta.description, "Cek detail status partner user");
        assert_eq!(p.meta.usage, ".cekpartner @user");
        assert_eq!(p.meta.example, ".cekpartner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin && p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for key in ["cekpartner", "partnerinfo"] {
            assert_eq!(
                oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()),
                Some("cekpartner"),
                "key {key} salah resolve"
            );
        }
    }

    #[test]
    fn a_number_outside_the_list_takes_the_short_error_branch() {
        for settings in [json!({}), json!({"partner": []}), json!({"partner": "x"})] {
            let ctx = Ctx {
                settings: settings.clone(),
                ..base()
            };
            let (text, mentions) = text_of(&run(ctx.clone()));
            assert_eq!(text, "❌ @628111 bukan partner", "{settings}");
            assert_eq!(mentions, vec![ME.to_string()], "{settings}");
        }
    }

    #[test]
    fn the_normal_branch_reports_the_stored_partner_detail() {
        let added = now_ms() - 10 * 86_400_000;
        let expired = now_ms() + 20 * 86_400_000;
        let ctx = Ctx {
            settings: json!({
                "partner": [{"id": "628111", "name": "Zann", "addedAt": added, "expired": expired}]
            }),
            ..base()
        };
        let (text, mentions) = text_of(&run(ctx.clone()));
        assert!(text.starts_with("🤝 *DETAIL PARTNER*\n\n"), "{text}");
        assert!(text.contains("👤 User: @628111"), "{text}");
        assert!(text.contains("📛 Nama: *Zann*"), "{text}");
        assert!(text.contains("🗓️ Durasi: *30 hari*"), "{text}");
        assert!(text.contains("📊 Sisa: *20 hari*"), "{text}");
        assert!(text.contains(&format!("📅 Mulai: *{}*", date_long(added))), "{text}");
        assert!(text.contains(&format!("⏳ Expired: *{}*", date_long(expired))), "{text}");
        assert_eq!(mentions, vec![ME.to_string()]);
    }

    #[test]
    fn an_expired_partner_reports_the_warning_instead_of_a_negative_count() {
        let ctx = Ctx {
            settings: json!({
                "partner": [{"id": "628111", "addedAt": 0, "expired": 1}]
            }),
            ..base()
        };
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("📊 Sisa: *⚠️ Expired*"), "{text}");
    }

    #[test]
    fn an_entry_without_a_name_or_start_date_uses_the_js_fallbacks() {
        let ctx = Ctx {
            settings: json!({"partner": [{"id": "628111"}]}),
            ..base()
        };
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("📛 Nama: *Unknown*"), "{text}");
        assert!(text.contains("📅 Mulai: *Unknown*"), "{text}");
        assert!(text.contains("⏳ Expired: *Invalid Date*"), "{text}");
        assert!(text.contains("🗓️ Durasi: *? hari*"), "{text}");
    }

    #[test]
    fn a_stored_user_appends_energi_and_koin() {
        let ctx = Ctx {
            settings: json!({"partner": [{"id": "628111", "addedAt": 0, "expired": 0}]}),
            user_data: json!({"energi": -1, "koin": 2500}),
            ..base()
        };
        let (text, _) = text_of(&run(ctx.clone()));
        assert!(text.contains("⚡ Energi: *∞*"), "{text}");
        assert!(text.contains("💰 Koin: *2.500*"), "{text}");
    }

    #[test]
    fn day_math_rounds_up_like_math_ceil() {
        assert_eq!(days_between(Some(86_400_000), 0), 1);
        assert_eq!(days_between(Some(86_400_001), 0), 2, "harus bulat ke atas");
        assert_eq!(days_between(Some(0), 0), 0);
        assert_eq!(days_between(Some(-1), 0), 0, "ceil(-0.0000116) == -0 -> 0");
        assert_eq!(days_between(Some(-86_400_000 - 1), 0), -1);
        assert_eq!(days_between(None, 0), 0);
    }

    #[test]
    fn the_plugin_only_reads() {
        let ctx = Ctx {
            settings: json!({"partner": [{"id": "628111", "addedAt": 0, "expired": 0}]}),
            ..base()
        };
        assert!(run(ctx.clone()).mutations().is_empty());
    }

    #[test]
    fn a_leading_zero_target_is_normalized_before_the_lookup() {
        let ctx = Ctx {
            sender: "0811@s.whatsapp.net".into(),
            settings: json!({"partner": [{"id": "62811", "addedAt": 0, "expired": 0}]}),
            ..base()
        };
        let (text, mentions) = text_of(&run(ctx.clone()));
        assert!(text.contains("👤 User: @62811"), "{text}");
        assert_eq!(mentions, vec!["62811@s.whatsapp.net".to_string()]);
    }

    #[test]
    fn format_date_passes_through_a_non_numeric_timestamp() {
        assert_eq!(format_date(Some(&Value::Null)), "Invalid Date");
        assert_eq!(format_date(None), "Invalid Date");
        assert_eq!(format_date(Some(&json!("nope"))), "Invalid Date");
    }
}
