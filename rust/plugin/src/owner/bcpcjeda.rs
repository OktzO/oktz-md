//! `plugins/owner/bcpcjeda.js` — setelan jeda broadcast private chat.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const DEFAULT_DELAY_MS: i64 = 5_000;

/// `^(\d+)(s|m|h|d)$/i` → milidetik. `None` kalau tidak cocok.
pub(crate) fn parse_delay(input: &str) -> Option<i64> {
    let (digits, unit) = input.split_at(input.len().checked_sub(1)?);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let value = digits.parse::<i64>().ok()?;
    let ms = match unit.to_ascii_lowercase().as_str() {
        "s" => value * 1000,
        "m" => value * 60 * 1000,
        "h" => value * 60 * 60 * 1000,
        "d" => value * 24 * 60 * 60 * 1000,
        _ => return None,
    };
    Some(ms)
}

/// `formatDelay` — satuan terbesar yang masih muat, `toFixed(0)` jadi
/// `round-half-up` di JS.
pub(crate) fn format_delay(ms: i64) -> String {
    let n = ms as f64;
    if ms >= 86_400_000 {
        return format!("{:.0} hari", n / 86_400_000.0);
    }
    if ms >= 3_600_000 {
        return format!("{:.0} jam", n / 3_600_000.0);
    }
    if ms >= 60_000 {
        return format!("{:.0} menit", n / 60_000.0);
    }
    format!("{:.0} detik", n / 1000.0)
}

fn help(ctx: &Ctx, current: i64) -> String {
    format!(
        "⏱️ *JEDA BROADCAST PRIVATE*\n\n\
         Jeda saat ini: *{cur}* ({current}ms)\n\n\
         *CARA PAKAI:*\n\
         > `{p}bcpcjeda <angka><satuan>`\n\n\
         *SATUAN:*\n\
         • `s` — detik\n• `m` — menit\n• `h` — jam\n• `d` — hari\n\n\
         *CONTOH:*\n\
         > `{p}bcpcjeda 5s` → 5 detik\n\
         > `{p}bcpcjeda 2m` → 2 menit\n\
         > `{p}bcpcjeda 1h` → 1 jam",
        cur = format_delay(current),
        p = ctx.prefix
    )
}

pub(crate) const BAD_FORMAT: &str =
    "❌ Format salah. Contoh: `5s`, `2m`, `1h`, `1d`";

#[plugin(
    name = "bcpcjeda",
    alias = ["delaybcpc", "jedabcpc", "setjedabcpc"],
    category = "owner",
    description = "Atur jeda broadcast private chat",
    usage = ".bcpcjeda <waktu> (contoh: 5s, 2m, 1h)",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bcpcjeda(ctx: Ctx) -> Reply {
    // `db.setting('jedaBcpc') || 5000` — nilai 0 ikut menjadi 5000 karena
    // `||`, bukan `??`.
    let stored = ctx.setting_i64("jedaBcpc").unwrap_or(0);
    let current = if stored == 0 { DEFAULT_DELAY_MS } else { stored };

    let input = ctx.text.trim();
    if input.is_empty() {
        return Reply::text(help(&ctx, current));
    }

    let ms = match parse_delay(input) {
        // `!ms || ms < 1000` — `!ms` juga menangkap `0`, yang mustahil dari
        // `parseDelay` karena `\d+` minimal satu digit.
        Some(ms) if ms >= 1000 => ms,
        _ => return Reply::text(BAD_FORMAT),
    };

    Reply::text(format!(
        "✅ *Jeda broadcast private diubah*\n\n\
         Sebelumnya: *{}*\n\
         Sekarang: *{}*",
        format_delay(current),
        format_delay(ms)
    ))
    .with_mutation(Mutation::SetSetting {
        key: "jedaBcpc".into(),
        value: serde_json::json!(ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "bcpcjeda".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("bcpcjeda").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> i64 {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "jedaBcpc");
                value.as_i64().expect("harus angka")
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bcpcjeda").unwrap();
        assert_eq!(p.meta.name, "bcpcjeda");
        assert_eq!(p.meta.alias, ["delaybcpc", "jedabcpc", "setjedabcpc"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Atur jeda broadcast private chat");
        assert_eq!(p.meta.usage, ".bcpcjeda <waktu> (contoh: 5s, 2m, 1h)");
        // `pluginConfig` JS tidak punya `example`, jadi default router yang
        // dipakai — bukan tebakan.
        assert_eq!(p.meta.example, "");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delaybcpc", "jedabcpc", "setjedabcpc"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "bcpcjeda");
        }
    }

    #[test]
    fn no_argument_shows_the_current_delay_and_writes_nothing() {
        let r = run("", serde_json::json!({}));
        assert!(
            text_of(&r).starts_with("⏱️ *JEDA BROADCAST PRIVATE*"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("5 detik"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_zero_falls_back_to_the_default_like_js_or_does() {
        // `|| 5000`, bukan `?? 5000` — 0 yang tersimpan jadi 5 detik.
        assert!(text_of(&run("", serde_json::json!({"jedaBcpc": 0}))).contains("5 detik"));
    }

    #[test]
    fn every_unit_is_parsed() {
        assert_eq!(parse_delay("5s"), Some(5_000));
        assert_eq!(parse_delay("2m"), Some(120_000));
        assert_eq!(parse_delay("1h"), Some(3_600_000));
        assert_eq!(parse_delay("1d"), Some(86_400_000));
        assert_eq!(parse_delay("10S"), Some(10_000), "regex punya flag i");
    }

    #[test]
    fn a_malformed_value_is_refused_and_writes_nothing() {
        for bad in ["5", "5x", "s", "-5s", "5 s", "1.5m"] {
            let r = run(bad, serde_json::json!({}));
            assert_eq!(text_of(&r), BAD_FORMAT, "input {bad:?}");
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn an_interval_below_one_second_is_refused() {
        let r = run("0s", serde_json::json!({}));
        assert_eq!(text_of(&r), BAD_FORMAT, "0s itu 0 ms");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn setting_a_delay_writes_it_and_names_both_sides() {
        let r = run("2m", serde_json::json!({}));
        assert_eq!(stored(&r), 120_000);
        let t = text_of(&r);
        assert!(t.contains("Sebelumnya: *5 detik*"), "{t}");
        assert!(t.contains("Sekarang: *2 menit*"), "{t}");
    }

    #[test]
    fn formatting_picks_the_largest_fitting_unit() {
        assert_eq!(format_delay(999), "1 detik");
        assert_eq!(format_delay(1_000), "1 detik");
        assert_eq!(format_delay(60_000), "1 menit");
        assert_eq!(format_delay(3_600_000), "1 jam");
        assert_eq!(format_delay(86_400_000), "1 hari");
    }
}