//! `plugins/panel/cekjeda.js` — status cooldown panel create.
//!
//! Dua setting yang dibaca `jedacreate.rs`: `panelCreateJeda` (durasi) dan
//! `panelCreateLastUsed` (epoch ms create terakhir).
//!
//! `ponytail:` `formatTime` di sini BUKAN milik `jedacreate.rs` — JS punya dua
//! fungsi dengan nama sama dan bentuk berbeda: yang di `cekjeda.js` menyertakan
//! detik pada level jam dan menulis `"0 detik"` untuk nilai non-positif.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jakarta_parts, now_ms, pad2};
use crate::panel::jedacreate::{DEFAULT_JEDA, JEDA_KEY, LAST_USED_KEY};

pub(crate) const NO_ACCESS: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Kamu tidak memiliki akses ke CPanel!";

/// `formatTime(ms)` dari `cekjeda.js:22` — berbeda dari padanannya di
/// `jedacreate.rs`: jam tetap membawa detik, dan nilai non-positif jadi
/// `"0 detik"` (bukan `"Tanpa jeda"`).
pub(crate) fn format_time(ms: i64) -> String {
    if ms <= 0 {
        return "0 detik".to_string();
    }
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let hours = minutes / 60;
    if hours > 0 {
        format!("{hours} jam {} menit {} detik", minutes % 60, seconds % 60)
    } else if minutes > 0 {
        format!("{minutes} menit {} detik", seconds % 60)
    } else {
        format!("{seconds} detik")
    }
}

/// `timeHelper.fromTimestamp(ms, 'HH:mm:ss')` (`time.js:112`) — jam lokal
/// Asia/Jakarta, dua digit per bagian.
fn clock_hms(ms: i64) -> String {
    let (_, _, _, h, m, s) = jakarta_parts(ms);
    format!("{}:{}:{}", pad2(h), pad2(m), pad2(s))
}

/// Guard JS: `hasAccessToServer` membaca file role di `database/cpanel/`
/// (roles-cpanel.js, pakai `fs`) — tidak ada padanannya di `Ctx`. Yang
/// tersisa adalah cabang owner; lihat catatan batch.
fn has_access(ctx: &Ctx) -> bool {
    ctx.is_owner
}

/// Badan balasan, dipisah dari handler supaya setting bisa diuji langsung.
pub(crate) fn status_text(jeda_ms: i64, last_used: i64, now: i64, is_owner: bool, prefix: &str) -> String {
    let remaining = (jeda_ms - (now - last_used)).max(0);

    let (status, status_desc) = if jeda_ms == 0 {
        ("⚡ *NO JEDA*", "Jeda dinonaktifkan, bebas create!".to_string())
    } else if remaining > 0 {
        ("🕕 *COOLDOWN*", format!("Tunggu {} lagi", format_time(remaining)))
    } else {
        ("✅ *READY*", "Bisa create panel sekarang!".to_string())
    };

    let mut text = String::from("⏱️ *sᴛᴀᴛᴜs ᴊᴇᴅᴀ ᴘᴀɴᴇʟ*\n\n");
    text.push_str("╭┈┈⬡「 📊 *sᴛᴀᴛᴜs* 」\n");
    text.push_str(&format!("┃ {status}\n"));
    text.push_str(&format!("┃ {status_desc}\n"));
    text.push_str("╰┈┈⬡\n\n");

    // `jedaMs === 0 ? 'OFF' : formatTime(jedaMs)` — `OFF` tidak diformat ulang.
    let jeda_label = if jeda_ms == 0 {
        "OFF".to_string()
    } else {
        format_time(jeda_ms)
    };
    text.push_str("╭┈┈⬡「 ⚙️ *ᴋᴏɴꜰɪɢ* 」\n");
    text.push_str(&format!("┃ ◦ Jeda: *{jeda_label}*\n"));
    text.push_str("┃ ◦ Default: *5 menit*\n");

    if last_used > 0 {
        text.push_str(&format!("┃ ◦ Last create: *{}*\n", clock_hms(last_used)));
    }
    if remaining > 0 {
        text.push_str(&format!("┃ ◦ Sisa: *{}*\n", format_time(remaining)));
    }
    text.push_str("╰┈┈⬡\n\n");

    if is_owner {
        text.push_str(&format!(
            "> _Owner: gunakan `{prefix}jedacreate` untuk setting_"
        ));
    }

    text
}

#[plugin(
    name = "cekjeda",
    alias = ["jedastatus", "statusjeda"],
    category = "panel",
    description = "Cek status jeda panel create",
    usage = ".cekjeda",
    example = ".cekjeda",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cekjeda(ctx: Ctx) -> Reply {
    if !has_access(&ctx) {
        return Reply::text(NO_ACCESS);
    }

    // `db.setting('panelCreateJeda') ?? DEFAULT_JEDA` — `??` hanya usherkan
    // null/undefined, jadi `0` (jeda dimatikan) tetap `0`.
    let jeda_ms = match ctx.setting(JEDA_KEY) {
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0),
        _ => DEFAULT_JEDA,
    };
    let last_used = ctx.setting_i64(LAST_USED_KEY).unwrap_or(0);

    Reply::text(status_text(
        jeda_ms,
        last_used,
        now_ms(),
        ctx.is_owner,
        &ctx.prefix,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn ctx(settings: Value, is_owner: bool) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "cekjeda".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            settings,
            is_owner,
            ..Default::default()
        }
    }

    fn run(settings: Value, is_owner: bool) -> Reply {
        (find("cekjeda").unwrap().handler)(ctx(settings, is_owner))
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("plugin harus membalas teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("cekjeda").unwrap().meta;
        assert_eq!(m.name, "cekjeda");
        assert_eq!(m.alias, ["jedastatus", "statusjeda"]);
        assert_eq!(m.category, "panel");
        assert_eq!(m.description, "Cek status jeda panel create");
        assert_eq!(m.usage, ".cekjeda");
        assert_eq!(m.example, ".cekjeda");
        assert_eq!(m.cooldown, 3);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["jedastatus", "statusjeda"] {
            assert_eq!(find(a).unwrap().meta.name, "cekjeda", "alias {a}");
        }
    }

    #[test]
    fn a_sender_without_cpanel_access_is_refused() {
        // Role owner/ceo/reseller dibaca dari file `database/cpanel/*.json`;
        // tanpa akses itu, hanya owner yang lolos — sama seperti JS saat semua
        // file role kosong.
        let r = run(Value::Null, false);
        assert_eq!(text_of(&r), NO_ACCESS);
        assert!(r.mutations().is_empty(), "cek status tidak boleh menulis store");
    }

    #[test]
    fn a_missing_setting_falls_back_to_the_five_minute_default() {
        let t = text_of(&run(Value::Null, true));
        assert!(t.contains("┃ ◦ Jeda: *5 menit 0 detik*"), "{t}");
        assert!(t.contains("┃ ✅ *READY*"), "{t}");
        assert!(t.contains("Bisa create panel sekarang!"), "{t}");
        assert!(t.starts_with("⏱️ *sᴛᴀᴛᴜs ᴊᴇᴅᴀ ᴘᴀɴᴇʟ*"), "{t}");
    }

    #[test]
    fn a_zero_jeda_shows_no_jeda_and_hides_the_remaining_line() {
        let t = text_of(&run(serde_json::json!({ JEDA_KEY: 0 }), true));
        assert!(t.contains("┃ ⚡ *NO JEDA*"), "{t}");
        assert!(t.contains("Jeda dinonaktifkan, bebas create!"), "{t}");
        assert!(t.contains("┃ ◦ Jeda: *OFF*"), "{t}");
        assert!(!t.contains("◦ Sisa:"), "jeda 0 tidak boleh punya sisa: {t}");
    }

    #[test]
    fn an_active_cooldown_reports_the_remaining_time_twice() {
        let now = 1_700_000_000_000;
        let jeda = 300_000;
        let last_used = now - 100_000;
        let t = status_text(jeda, last_used, now, false, ".");
        assert!(t.contains("┃ 🕕 *COOLDOWN*"), "{t}");
        assert!(t.contains("┃ ◦ Jeda: *5 menit 0 detik*"), "{t}");
        // sisa 200000 ms -> 3 menit 20 detik
        assert!(t.contains("Tunggu 3 menit 20 detik lagi"), "{t}");
        assert!(t.contains("┃ ◦ Sisa: *3 menit 20 detik*"), "{t}");
    }

    #[test]
    fn the_last_create_line_only_appears_when_the_timestamp_is_positive() {
        let now = 1_700_000_000_000;
        assert!(!status_text(300_000, 0, now, false, ".").contains("Last create"));
        // 2023-11-14T22:13:20Z = 2023-11-15 05:13:20 WIB
        assert!(
            status_text(300_000, 1_700_000_000_000, now, false, ".")
                .contains("Last create: *05:13:20*")
        );
    }

    #[test]
    fn only_the_owner_gets_the_jedacreate_hint_with_the_live_prefix() {
        let owner = status_text(300_000, 0, 1_700_000_000_000, true, "!");
        assert!(owner.contains("> _Owner: gunakan `!jedacreate` untuk setting_"), "{owner}");
        let user = status_text(300_000, 0, 1_700_000_000_000, false, "!");
        assert!(!user.contains("jedacreate"), "{user}");
    }

    #[test]
    fn format_time_is_the_cekjeda_variant_with_seconds_under_hours() {
        assert_eq!(format_time(0), "0 detik");
        assert_eq!(format_time(-1), "0 detik");
        assert_eq!(format_time(45_000), "45 detik");
        assert_eq!(format_time(90_000), "1 menit 30 detik");
        assert_eq!(format_time(3_660_000), "1 jam 1 menit 0 detik");
        assert_eq!(format_time(7_200_000), "2 jam 0 menit 0 detik");
    }
}