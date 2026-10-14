//! `plugins/panel/jedacreate.js` — set jeda global untuk semua panel create.
//!
//! Dua setting global: `panelCreateJeda` (durasi) dan `panelCreateLastUsed`
//! (reset ke 0 setiap kali jeda diubah).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

pub(crate) const JEDA_KEY: &str = "panelCreateJeda";
pub(crate) const LAST_USED_KEY: &str = "panelCreateLastUsed";

/// `DEFAULT_JEDA = 5 * 60 * 1000`.
pub(crate) const DEFAULT_JEDA: i64 = 5 * 60 * 1000;

/// `parseTime(input)` — regex `^(\d+)(s|m|h)?$` dengan flag `i`.
///
/// `""` dan `"0"` → 0 (jeda mati, sebelum regex dijalankan). Tidak cocok regex
/// → `None` (format salah). Angka yang melimpah `i64` tetap dianggap valid —
/// JS tidak punya batas integer — jadi hasilnya disaturasi, bukan ditolak.
pub(crate) fn parse_time(input: &str) -> Option<i64> {
    let t = input.trim();
    if t.is_empty() || t == "0" {
        return Some(0);
    }
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    // Unit TIDAK boleh di-`trim`: regex `^(\d+)(s|m|h)?$` menolak spasi di
    // tengah, jadi `"5 m"` format salah — bukan `5m`.
    let unit = &t[digits.len()..];
    let factor = match unit.to_ascii_lowercase().as_str() {
        "" | "s" => 1_000,
        "m" => 60 * 1_000,
        "h" => 60 * 60 * 1_000,
        _ => return None,
    };
    let value: i64 = digits.parse().unwrap_or(i64::MAX);
    Some(value.saturating_mul(factor))
}

/// `formatTime(ms)` dari `jedacreate.js:35` — bentuknya sendiri: jam tidak
/// menyertakan detik (bandingkan `cekjeda.js`, yang tidak ikut dipakai di sini).
pub(crate) fn format_time(ms: i64) -> String {
    if ms <= 0 {
        return "Tanpa jeda".to_string();
    }
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        format!("{hours} jam {} menit", minutes % 60)
    } else if minutes > 0 {
        format!("{minutes} menit {} detik", seconds % 60)
    } else {
        format!("{seconds} detik")
    }
}

pub(crate) fn info(current: i64, prefix: &str) -> String {
    format!(
        "⏱️ *ᴊᴇᴅᴀ ᴘᴀɴᴇʟ ᴄʀᴇᴀᴛᴇ*\n\n\
         ╭┈┈⬡「 📋 *ɪɴꜰᴏ* 」\n\
         ┃ ◦ Jeda saat ini: *{}*\n\
         ┃ ◦ Default: *5 menit*\n\
         ╰┈┈⬡\n\n\
         > Gunakan: `{prefix}jedacreate <waktu>`\n\
         > Contoh: `{prefix}jedacreate 5m` (5 menit)\n\
         > Untuk nonaktifkan: `{prefix}jedacreate 0`\n\n\
         *Format waktu:*\n\
         • `30s` = 30 detik\n\
         • `5m` = 5 menit\n\
         • `1h` = 1 jam",
        format_time(current)
    )
}

pub(crate) const INVALID: &str = "❌ Format waktu tidak valid!\n\n> Contoh: 30s, 5m, 1h";

pub(crate) fn disabled() -> String {
    "✅ *ᴊᴇᴅᴀ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Panel create sekarang tanpa jeda".to_string()
}

pub(crate) fn saved(ms: i64) -> String {
    format!(
        "✅ *ᴊᴇᴅᴀ ᴅɪsᴇᴛ*\n\n\
         ╭┈┈⬡「 ⏱️ *ᴋᴏɴꜰɪɢ* 」\n\
         ┃ ◦ Jeda: *{}*\n\
         ╰┈┈⬡\n\n\
         > Setelah panel dibuat, SEMUA user harus menunggu {} sebelum bisa create lagi.",
        format_time(ms),
        format_time(ms)
    )
}

#[plugin(
    name = "jedacreate",
    alias = ["setjeda", "paneljeda", "jedapanel"],
    category = "panel",
    description = "Set jeda waktu untuk semua panel create command",
    usage = ".jedacreate <waktu>",
    example = ".jedacreate 5m",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn jedacreate(ctx: Ctx) -> Reply {
    let input = ctx.text.trim().to_string();

    // `db.setting('panelCreateJeda') ?? DEFAULT_JEDA` — `??` hanya usherkan
    // `null`/`undefined`, jadi `0` (jeda dimatikan) tetap `0`.
    let current = match ctx.setting(JEDA_KEY) {
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0),
        _ => DEFAULT_JEDA,
    };

    if input.is_empty() {
        return Reply::text(info(current, &ctx.prefix));
    }

    let Some(ms) = parse_time(&input) else {
        return Reply::text(INVALID);
    };

    let text = if ms == 0 { disabled() } else { saved(ms) };
    Reply::text(text)
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: JEDA_KEY.to_string(),
            value: serde_json::json!(ms),
        })
        .with_mutation(Mutation::SetSetting {
            key: LAST_USED_KEY.to_string(),
            value: serde_json::json!(0),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "jedacreate".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("jedacreate").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Varian dimiliki: untuk `owned(run(..))` di dalam assert.
    fn owned(r: Reply) -> String {
        r.text_body().map(str::to_string).unwrap_or_default()
    }

    fn settings_written(r: &Reply) -> Vec<(String, Value)> {
        r.mutations()
            .iter()
            .filter_map(|m| match m {
                Mutation::SetSetting { key, value } => Some((key.clone(), value.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("jedacreate").unwrap();
        assert_eq!(p.meta.name, "jedacreate");
        assert_eq!(p.meta.alias, ["setjeda", "paneljeda", "jedapanel"]);
        assert_eq!(p.meta.category, "panel");
        assert_eq!(
            p.meta.description,
            "Set jeda waktu untuk semua panel create command"
        );
        assert_eq!(p.meta.usage, ".jedacreate <waktu>");
        assert_eq!(p.meta.example, ".jedacreate 5m");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_owner, "satu-satunya plugin owner di batch ini");
        assert!(!p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setjeda", "paneljeda", "jedapanel"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "jedacreate");
        }
    }

    #[test]
    fn without_an_argument_the_current_setting_is_shown_and_nothing_is_written() {
        for (settings, expected) in [
            (Value::Null, "5 menit 0 detik"),
            (Value::Object(serde_json::Map::new()), "5 menit 0 detik"),
            (serde_json::json!({ JEDA_KEY: 300_000 }), "5 menit 0 detik"),
            (serde_json::json!({ JEDA_KEY: 0 }), "Tanpa jeda"),
            (serde_json::json!({ JEDA_KEY: 90_000 }), "1 menit 30 detik"),
            (serde_json::json!({ JEDA_KEY: 3_600_000 }), "1 jam 0 menit"),
            (serde_json::json!({ JEDA_KEY: 45_000 }), "45 detik"),
        ] {
            let r = run("", settings.clone());
            let t = text_of(&r);
            assert!(t.starts_with("⏱️ *ᴊᴇᴅᴀ ᴘᴀɴᴇʟ ᴄʀᴇᴀᴛᴇ*"), "{settings}: {t}");
            assert!(t.contains(&format!("Jeda saat ini: *{expected}*")), "{settings}: {t}");
            assert!(t.contains("Gunakan: `.jedacreate <waktu>`"), "{t}");
            assert!(r.mutations().is_empty(), "{settings}");
        }
    }

    #[test]
    fn a_zero_setting_is_not_replaced_by_the_default() {
        // `??` hanya usherkan null/undefined — `0` berarti "jeda dimatikan".
        let t = owned(run("", serde_json::json!({ JEDA_KEY: 0 })));
        assert!(t.contains("Jeda saat ini: *Tanpa jeda*"), "{t}");
    }

    #[test]
    fn a_valid_duration_is_stored_together_with_the_last_used_reset() {
        for (input, expected) in [
            ("5m", 300_000),
            ("30s", 30_000),
            ("1h", 3_600_000),
            ("45", 45_000),
            ("10M", 600_000),
            ("2H", 7_200_000),
        ] {
            let r = run(input, Value::Null);
            let w = settings_written(&r);
            assert_eq!(w.len(), 2, "input {input}");
            assert_eq!(w[0], (JEDA_KEY.to_string(), serde_json::json!(expected)), "input {input}");
            assert_eq!(w[1], (LAST_USED_KEY.to_string(), serde_json::json!(0)), "input {input}");
            match r.without_mutations() {
                Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅", "input {input}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn zero_disables_the_cooldown_with_its_own_message() {
        let r = run("0", Value::Null);
        assert_eq!(text_of(&r), disabled());
        assert!(!text_of(&r).contains("Jeda: *"), "{}", text_of(&r));
        assert_eq!(settings_written(&r)[0].1, serde_json::json!(0));
    }

    #[test]
    fn an_empty_whitespace_input_is_the_info_branch_not_an_error() {
        // `!input` menang sebelum `parseTime`, jadi `"   "` tidak jadi format salah.
        let r = run("   ", Value::Null);
        assert!(text_of(&r).starts_with("⏱️ *ᴊᴇᴅᴀ ᴘᴀɴᴇʟ ᴄʀᴇᴀᴛᴇ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_malformed_duration_is_refused_and_writes_nothing() {
        for bad in ["5x", "m", "-5m", "5 m", "1.5h", "abc"] {
            let r = run(bad, Value::Null);
            assert_eq!(text_of(&r), INVALID, "input {bad:?}");
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn an_overlong_number_is_accepted_and_saturated() {
        // JS `parseInt` tidak punya batas integer, jadi angka sepanjang ini
        // tetap FORMAT VALID — hanya nilainya yang tidak muat i64.
        let r = run("99999999999999999999", Value::Null);
        assert!(!text_of(&r).starts_with("❌"), "{}", text_of(&r));
        assert_eq!(
            settings_written(&r)[0].1,
            serde_json::json!(i64::MAX),
            "disaturasi, bukan ditolak"
        );
    }

    #[test]
    fn the_helper_contract_matches_the_js_regex() {
        assert_eq!(parse_time(""), Some(0));
        assert_eq!(parse_time("0"), Some(0));
        assert_eq!(parse_time("7"), Some(7_000));
        assert_eq!(parse_time("7S"), Some(7_000));
        assert_eq!(parse_time("7m"), Some(420_000));
        assert_eq!(parse_time("2h"), Some(7_200_000));
        assert_eq!(parse_time("5x"), None);
        assert_eq!(parse_time("-1"), None);
        assert_eq!(parse_time("5 m"), None, "spasi di tengah tidak lolos regex");
        assert_eq!(parse_time("1 000"), None);
        assert_eq!(parse_time("m5"), None);
    }

    #[test]
    fn format_time_drops_seconds_once_hours_are_reached() {
        // Bentuk `jedacreate.js`, bukan `cekjeda.js`: jam tidak bawa detik.
        assert_eq!(format_time(0), "Tanpa jeda");
        assert_eq!(format_time(-5), "Tanpa jeda");
        assert_eq!(format_time(45_000), "45 detik");
        assert_eq!(format_time(90_000), "1 menit 30 detik");
        assert_eq!(format_time(3_660_000), "1 jam 1 menit");
        assert_eq!(format_time(7_200_000), "2 jam 0 menit");
    }
}
