//! `plugins/owner/cpuprofile.js` — CPU profiling via node:inspector.
//!
//! `ponytail:` `src/lib/profiler.js` TIDAK di-port. `startCpuProfile`/
//! `stopCpuProfile` butuh `node:inspector` dan menulis berkas `.cpuprofile`;
//! `process.memoryUsage()` juga tidak punya padanan di `Ctx`. Jadi tidak ada
//! profil yang benar-benar dibuat. Yang dipindah: parsing sub-perintah,
//! penghitungan durasi (termasuk batas atasnya), nama profil, dan teks help —
//! semuanya murni aritmetika dan string.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `CPU_MAX_DURATION_MS = 10 * 60 * 1000` (`profiler.js:188`) → 10 menit.
pub(crate) const MAX_DURATION_MS: i64 = 10 * 60 * 1000;
pub(crate) const MAX_MINUTES: f64 = 10.0;

/// `parseFloat(args[1]) || 5` — `parseFloat` membaca awalan angka saja
/// (`"5m"` → 5, `"2.5h"` → 2.5), dan hasil falsy (`NaN`, `0`, `-0`) jatuh
/// ke default 5.
pub(crate) fn parse_float_prefix(raw: &str) -> Option<f64> {
    let s = raw.trim_start();
    let bytes = s.as_bytes();
    let mut end = 0usize;
    if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    // `e`/`E` hanya bagian angka kalau diikuti tanda atau digit.
    if end < bytes.len() && (bytes[end] | 0x20) == b'e' {
        let mut exp = end + 1;
        if exp < bytes.len() && (bytes[exp] == b'+' || bytes[exp] == b'-') {
            exp += 1;
        }
        let digits_start = exp;
        while exp < bytes.len() && bytes[exp].is_ascii_digit() {
            exp += 1;
        }
        if exp > digits_start {
            end = exp;
        }
    }
    s[..end].parse().ok()
}

/// `Math.min(parseFloat(args[1]) || 5, CPU_MAX_DURATION_MS / 60000)`.
pub(crate) fn start_minutes(raw: Option<&str>) -> f64 {
    let parsed = raw.and_then(parse_float_prefix).filter(|v| *v != 0.0);
    parsed.unwrap_or(5.0).min(MAX_MINUTES)
}

/// `Math.round(minutes * 60 * 1000)`.
pub(crate) fn duration_ms(minutes: f64) -> i64 {
    (minutes * MAX_DURATION_MS as f64 / MAX_MINUTES).round() as i64
}

/// `` `cmd-${m.sender?.split("@")[0] || "owner"}` ``.
pub(crate) fn profile_name(ctx: &Ctx) -> String {
    let num = ctx.sender.split('@').next().unwrap_or(&ctx.sender);
    if num.is_empty() {
        "cmd-owner".to_string()
    } else {
        format!("cmd-{num}")
    }
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚙️ *CPU Profiler*\n\n\
         > *{prefix}cpuprofile start [menit]* — Mulai profiling (default 5, max {MAX_MINUTES})\n\
         > *{prefix}cpuprofile stop* — Stop profiling lebih awal\n\
         > *{prefix}cpuprofile status* — Cek status"
    )
}

pub(crate) fn unavailable(action: &str) -> String {
    format!(
        "⚠️ *CPU Profiler tidak di-port*\n\n\
         > `node:inspector` dan `process.memoryUsage()` tidak ada di `Ctx`,\n\
         > jadi tidak ada profil yang dibuat dan tidak ada status yang bisa dibaca.\n\
         > Aksi: `{action}`"
    )
}

#[plugin(
    name = "cpuprofile",
    alias = ["cpup", "cpuprof"],
    category = "owner",
    description = "CPU profiling via node:inspector (Owner Only)",
    usage = ".cpuprofile start [menit] | .cpuprofile stop | .cpuprofile status",
    example = ".cpuprofile start 5",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cpuprofile(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    match sub.as_str() {
        "start" => {
            let minutes = start_minutes(ctx.args.get(1).map(String::as_str));
            // Nama profil ikut dihitung supaya tetap terlihat benar walau
            // profiler-nya tidak ada.
            let name = profile_name(&ctx);
            Reply::text(format!(
                "{}\n> Durasi: {minutes} menit\n> Nama: {name}\n> Duration: {}ms",
                unavailable(&sub),
                duration_ms(minutes)
            ))
        }
        "stop" | "status" => Reply::text(unavailable(&sub)),
        "" => Reply::text(unavailable("status")),
        _ => Reply::text(help(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(args: &[&str]) -> String {
        (find("cpuprofile").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "cpuprofile".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        })
        .text_body()
        .expect("harus ada teks")
        .to_string()
    }

    fn with_sender(sender: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cpuprofile".into(),
            sender: sender.into(),
            chat: sender.into(),
            args: vec!["start".into()],
            ..Default::default()
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("cpuprofile").unwrap();
        assert_eq!(p.meta.name, "cpuprofile");
        assert_eq!(p.meta.alias, ["cpup", "cpuprof"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "CPU profiling via node:inspector (Owner Only)");
        assert_eq!(
            p.meta.usage,
            ".cpuprofile start [menit] | .cpuprofile stop | .cpuprofile status"
        );
        assert_eq!(p.meta.example, ".cpuprofile start 5");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["cpup", "cpuprof"] {
            assert_eq!(find(a).unwrap().meta.name, "cpuprofile", "alias {a}");
        }
    }

    #[test]
    fn parse_float_only_reads_the_leading_number() {
        assert_eq!(parse_float_prefix("5m"), Some(5.0));
        assert_eq!(parse_float_prefix("2.5h"), Some(2.5));
        assert_eq!(parse_float_prefix("1e2"), Some(100.0));
        assert_eq!(parse_float_prefix("  7 "), Some(7.0));
        assert_eq!(parse_float_prefix("abc"), None);
        assert_eq!(parse_float_prefix(""), None);
        assert_eq!(parse_float_prefix("-3"), Some(-3.0));
    }

    #[test]
    fn a_missing_or_unusable_duration_falls_back_to_five_minutes() {
        assert_eq!(start_minutes(None), 5.0);
        assert_eq!(start_minutes(Some("abc")), 5.0);
        assert_eq!(start_minutes(Some("0")), 5.0, "`0` falsy -> default");
        assert_eq!(start_minutes(Some("-0")), 5.0);
    }

    #[test]
    fn the_duration_is_capped_at_the_profiler_ceiling() {
        assert_eq!(start_minutes(Some("5")), 5.0);
        assert_eq!(start_minutes(Some("2.5")), 2.5);
        assert_eq!(start_minutes(Some("10")), 10.0);
        assert_eq!(start_minutes(Some("30")), MAX_MINUTES, "Math.min(x, 10)");
        assert_eq!(start_minutes(Some("9999")), MAX_MINUTES);
    }

    #[test]
    fn duration_ms_matches_the_js_rounding() {
        assert_eq!(duration_ms(5.0), 300_000);
        assert_eq!(duration_ms(10.0), MAX_DURATION_MS);
        assert_eq!(duration_ms(2.5), 150_000);
        assert_eq!(duration_ms(1.0 / 60.0), 1_000, "pembulatan ke detik terdekat");
    }

    #[test]
    fn the_profile_name_is_prefixed_with_cmd_and_the_bare_number() {
        assert_eq!(profile_name(&with_sender("628111@s.whatsapp.net")), "cmd-628111");
        assert_eq!(profile_name(&with_sender("628111@lid")), "cmd-628111");
        assert_eq!(profile_name(&with_sender("")), "cmd-owner", "falsy -> owner");
    }

    #[test]
    fn start_reports_the_computed_duration_and_name() {
        let out = run(&["start", "3"]);
        assert!(out.contains("Aksi: `start`"), "{out}");
        assert!(out.contains("> Durasi: 3 menit"), "{out}");
        assert!(out.contains("> Nama: cmd-"), "{out}");
        assert!(out.contains("> Duration: 180000ms"), "{out}");
    }

    #[test]
    fn stop_and_status_are_both_recognised_and_either_lowercases() {
        for a in ["stop", "STOP", "status", "Status"] {
            assert!(run(&[a]).contains("Aksi:"), "sub {a}");
        }
    }

    #[test]
    fn a_bare_command_falls_through_to_the_status_branch() {
        assert!(run(&[]).contains("Aksi: `status`"));
    }

    #[test]
    fn an_unknown_sub_command_prints_the_three_options() {
        let out = run(&["mulai"]);
        assert!(out.starts_with("⚙️ *CPU Profiler*"), "{out}");
        assert!(out.contains("*.cpuprofile start [menit]* — Mulai profiling (default 5, max 10)"), "{out}");
        assert!(out.contains("*.cpuprofile stop*"), "{out}");
        assert!(out.contains("*.cpuprofile status*"), "{out}");
    }
}
