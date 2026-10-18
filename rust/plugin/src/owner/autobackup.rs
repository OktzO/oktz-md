//! `plugins/owner/autobackup.js` — kelola sistem auto backup.
//!
//! `ponytail:` modul `src/lib/auto-backup.js` TIDAK di-port. Ia menyimpan
//! state di berkas sendiri, menjadwalkan timer, dan mengirim zip lewat `sock`
//! — tidak ada padanannya di `Ctx`, dan mengarang tempatnya akan membuat
//! `.autobackup status` melaporkan angka palsu. Yang dipindah: parsing aksi,
//! validasi interval (`parseInterval`, murni dan punya batas angka), serta
//! seluruh teks statis. Status/jadwal/win/lokal yang butuh modul itu berhenti
//! di balasan "tidak tersedia".

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) const MIN_INTERVAL_MS: i64 = 60_000;
pub(crate) const MAX_INTERVAL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

pub(crate) const BAD_INTERVAL: &str =
    "❌ *ɢᴀɢᴀʟ*\n\n> Format interval tidak valid. Contoh: 5m, 1h, 2d";

/// `parseInterval(str)` — `/^(\d+)(m|h|d)$/i`, lalu ditolak kalau di bawah
/// 1 menit atau di atas 7 hari. Mengembalikan `(ms, str)` seperti JS.
/// Regex JS tidak meng-trim, jadi `" 5h"` juga ditolak.
pub(crate) fn parse_interval(raw: &str) -> Option<(i64, String)> {
    let digits_end = raw
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(raw.len());
    if digits_end == 0 {
        return None;
    }
    let (head, tail) = raw.split_at(digits_end);
    let value: i64 = head.parse().ok()?;
    let unit = match tail.to_lowercase().as_str() {
        "m" => 60_000,
        "h" => 60 * 60_000,
        "d" => 24 * 60 * 60_000,
        _ => return None,
    };
    let ms = value.checked_mul(unit)?;
    if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&ms) {
        return None;
    }
    Some((ms, format!("{value}{}", tail.to_lowercase())))
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🗂️ *ᴀᴜᴛᴏ ʙᴀᴄᴋᴜᴘ sʏsᴛᴇᴍ*\n\n\
         > ⚠️ Status (`getBackupStatus()`) tidak di-port: state backup disimpan\n\
         > di berkas modul, bukan di store yang dibuka `Ctx`.\n\n\
         *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ:*\n\
         > `{prefix}autobackup on <interval>`\n\
         > `{prefix}autobackup off`\n\
         > `{prefix}autobackup status`\n\
         > `{prefix}autobackup now`\n\n\
         *ꜰᴏʀᴍᴀᴛ ɪɴᴛᴇʀᴠᴀʟ:*\n\
         > • `5m` = 5 menit\n\
         > • `1h` = 1 jam\n\
         > • `6h` = 6 jam\n\
         > • `1d` = 1 hari\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > `{prefix}autobackup on 6h` - backup setiap 6 jam"
    )
}

pub(crate) fn interval_required(prefix: &str) -> String {
    format!(
        "⚠️ *ɪɴᴛᴇʀᴠᴀʟ ᴅɪʙᴜᴛᴜʜᴋᴀɴ*\n\n\
         > `{prefix}autobackup on <interval>`\n\n\
         *ᴄᴏɴᴛᴏʜ:*\n\
         > `{prefix}autobackup on 30m` - tiap 30 menit\n\
         > `{prefix}autobackup on 6h` - tiap 6 jam\n\
         > `{prefix}autobackup on 1d` - tiap 1 hari"
    )
}

pub(crate) fn invalid_action(prefix: &str) -> String {
    format!(
        "⚠️ *ᴀᴄᴛɪᴏɴ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n\
         > Pilih: `on`, `off`, `status`, atau `now`\n\
         > Contoh: `{prefix}autobackup on 6h`"
    )
}

pub(crate) fn unavailable(action: &str) -> String {
    format!(
        "⚠️ *ᴀᴜᴛᴏ ʙᴀᴄᴋᴜᴘ ᴛɪᴅᴀᴋ ᴅɪᴘᴏʀᴛ*\n\n\
         > Modul `src/lib/auto-backup.js` butuh timer + `sock` yang tidak ada\n\
         > di `Ctx`, jadi tidak ada backup yang dibuat atau dijadwalkan.\n\
         > Aksi: `{action}`"
    )
}

#[plugin(
    name = "autobackup",
    alias = ["ab"],
    category = "owner",
    description = "Kelola sistem auto backup",
    usage = ".autobackup <on/off/status/now> [interval]",
    example = ".autobackup on 5h",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn autobackup(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    match action.as_str() {
        "" => Reply::text(help(&ctx.prefix)),
        // JS: `enableAutoBackup` menolak interval buruk dengan `{success:false}`
        // dan handler membalas `❌ *ɢᴀɢᴀʟ*`. Cabang ini benar-benar hidup.
        "on" | "enable" | "start" => match ctx.args.get(1) {
            None => Reply::text(interval_required(&ctx.prefix)),
            Some(raw) => match parse_interval(raw) {
                None => Reply::text(BAD_INTERVAL),
                Some((_, normalized)) => Reply::text(unavailable(&normalized)),
            },
        },
        "off" | "disable" | "stop" | "status" | "info" | "now" | "manual" | "trigger" => {
            Reply::text(unavailable(&action))
        }
        _ => Reply::text(invalid_action(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(args: &[&str]) -> String {
        (find("autobackup").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "autobackup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        })
        .text_body()
        .expect("harus ada teks")
        .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("autobackup").unwrap();
        assert_eq!(p.meta.name, "autobackup");
        assert_eq!(p.meta.alias, ["ab"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Kelola sistem auto backup");
        assert_eq!(p.meta.usage, ".autobackup <on/off/status/now> [interval]");
        assert_eq!(p.meta.example, ".autobackup on 5h");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(find("ab").unwrap().meta.name, "autobackup");
    }

    #[test]
    fn minutes_hours_and_days_scale_their_own_unit() {
        assert_eq!(parse_interval("5m"), Some((300_000, "5m".into())));
        assert_eq!(parse_interval("6h"), Some((21_600_000, "6h".into())));
        assert_eq!(parse_interval("1d"), Some((86_400_000, "1d".into())));
    }

    #[test]
    fn the_unit_is_case_insensitive_but_the_echo_is_lowercased() {
        assert_eq!(parse_interval("5H"), Some((18_000_000, "5h".into())));
    }

    #[test]
    fn the_ends_of_the_range_are_inclusive_one_minute_and_seven_days() {
        assert_eq!(parse_interval("1m").unwrap().0, MIN_INTERVAL_MS);
        assert_eq!(parse_interval("7d").unwrap().0, MAX_INTERVAL_MS);
        assert_eq!(
            parse_interval("10080m").unwrap().0,
            MAX_INTERVAL_MS,
            "10080 menit = tepat 7 hari, masih diterima"
        );
        // 59 detik bukan satuan yang dikenali; 10081 menit = 7 hari 1 menit.
        assert_eq!(parse_interval("59s"), None);
        assert_eq!(parse_interval("10081m"), None);
    }

    #[test]
    fn junk_intervals_are_rejected_the_way_the_js_regex_does() {
        for bad in ["", "m", "h5", "5x", "-5m", "5 m", "5.5h", "0m", " 5h", "5h "] {
            assert_eq!(parse_interval(bad), None, "interval {bad:?}");
        }
    }

    #[test]
    fn no_action_prints_the_usage_block_with_the_prefix_interpolated() {
        let out = run(&[]);
        assert!(out.contains("> `.autobackup on <interval>`"), "{out}");
        assert!(out.contains("> `.autobackup on 6h` - backup setiap 6 jam"));
        assert!(!out.contains("{prefix}"), "prefix harus sudah diganti");
    }

    #[test]
    fn on_without_an_interval_asks_for_one() {
        let out = run(&["on"]);
        assert!(out.starts_with("⚠️ *ɪɴᴛᴇʀᴠᴀʟ ᴅɪʙᴜᴛᴜʜᴋᴀɴ*"), "{out}");
        assert!(out.contains("> `.autobackup on <interval>`"));
    }

    #[test]
    fn on_with_a_bad_interval_returns_the_javascript_failure_text() {
        assert_eq!(run(&["on", "5x"]), BAD_INTERVAL);
        assert_eq!(run(&["enable", "0m"]), BAD_INTERVAL);
        assert_eq!(run(&["start", "9d"]), BAD_INTERVAL);
    }

    #[test]
    fn on_with_a_good_interval_reaches_the_unported_scheduler() {
        assert!(run(&["on", "6h"]).contains("Aksi: `6h`"));
    }

    #[test]
    fn every_other_switch_arm_is_recognised() {
        for a in ["off", "disable", "stop", "status", "info", "now", "manual", "trigger"] {
            assert!(run(&[a]).contains(&format!("Aksi: `{a}`")), "aksi {a}");
        }
        assert!(run(&["OFF"]).contains("Aksi: `off`"), "lowercase dulu");
    }

    #[test]
    fn an_unknown_action_lists_the_four_valid_ones() {
        let out = run(&["mulai"]);
        assert!(out.starts_with("⚠️ *ᴀᴄᴛɪᴏɴ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*"), "{out}");
        assert!(out.contains("> Pilih: `on`, `off`, `status`, atau `now`"), "{out}");
    }
}
