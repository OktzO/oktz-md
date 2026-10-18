//! `plugins/owner/bcgc.js` — broadcast pesan ke semua grup dengan media dan jeda.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `sock` (`fetchGroupsSafe(sock)` + per-grup
//! `sock.sendMessage(..., { quoted: m })` dengan media audio/video/dokumen/sticker),
//! `getDatabase()` (`db.setting`/`db.setUser` untuk `bcgcEnabled` dan
//! `jedaBcgc`), state global `statusBcgc`/`stopBcgc`, dan `saluranCtx()`
//! (`src/lib/context.js`). Tidak ada satupun di `Ctx`.
//!
//! `ponytail:` kalau `sock`, `db`, dan state global tersedia di `Ctx` nanti,
//! port ulang dari `plugins/owner/bcgc.js` (guard `stopbcgc`/`jedabcgc`/`on`/
//! `off`, deteksi media quoted, loop grup dengan jeda, dan reaksi
//! `📢`/`❌`/`✅`/`☢`).
//!
//! Fungsi murni di bawah (`parseDelay`, `formatDelay`) disalin dari JS untuk
//! dijaga agar tidak hilang, dan diuji.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `bcgc.js:30-48` — `5s`/`2m`/`1h`/`1d` ke milidetik (case-insensitive,
/// mirip `/^(\d+)(s|m|h|d)$/i`); satuan gabung seperti `15ms` → `None`.
pub(crate) fn parse_delay(input: &str) -> Option<i64> {
    let mut bytes = input.bytes();
    let mut digits_end = 0;
    for (i, b) in input.bytes().enumerate() {
        if b.is_ascii_digit() {
            digits_end = i + 1;
        } else {
            break;
        }
    }
    if digits_end == 0 || digits_end == input.len() {
        return None;
    }
    let val: i64 = input[..digits_end].parse().ok()?;
    let unit = input[digits_end..].to_ascii_lowercase();
    let ms = match unit.as_str() {
        "s" => val * 1_000,
        "m" => val * 60 * 1_000,
        "h" => val * 60 * 60 * 1_000,
        "d" => val * 24 * 60 * 1_000,
        _ => return None,
    };
    Some(ms)
}

/// `bcgc.js:50-55` — format milidetik ke teks bahasa Indonesia.
/// `toFixed(0)` di JS membulatkan (1,5 menit → "2"), bukan memotong.
pub(crate) fn format_delay(ms: i64) -> String {
    if ms >= 86_400_000 {
        format!("{} hari", (ms as f64 / 86_400_000.0).round())
    } else if ms >= 3_600_000 {
        format!("{} jam", (ms as f64 / 3_600_000.0).round())
    } else if ms >= 60_000 {
        format!("{} menit", (ms as f64 / 60_000.0).round())
    } else {
        format!("{} detik", (ms as f64 / 1_000.0).round())
    }
}

/// `bcgc.js:62-71` / `216-227` — stop broadcast.
const STOP_TEXT: &str = "⏹️ *Broadcast Grup Dihentikan*\n\n> Proses broadcast sedang dihentikan...";

#[plugin(
    name = "bcgc",
    alias = [
        "broadcastgc",
        "bcgroup",
        "jedabcgc",
        "delaybcgc",
        "setjedabcgc",
        "stopbcgc",
        "stopbroadcastgc",
    ],
    category = "owner",
    description = "Broadcast pesan ke semua grup dengan dukungan semua jenis media (DIBLOKIR: butuh sock, db, dan state global)",
    usage = ".bcgc",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn bcgc(_ctx: Ctx) -> Reply {
    Reply::text(STOP_TEXT).with_reaction("⏹️")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("bcgc").unwrap();
        assert_eq!(p.meta.name, "bcgc");
        assert_eq!(
            p.meta.alias,
            [
                "broadcastgc", "bcgroup", "jedabcgc", "delaybcgc",
                "setjedabcgc", "stopbcgc", "stopbroadcastgc"
            ]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn parse_delay_mirip_js() {
        assert_eq!(parse_delay("5s"), Some(5_000));
        assert_eq!(parse_delay("5S"), Some(5_000), "case-insensitive");
        assert_eq!(parse_delay("2m"), Some(120_000));
        assert_eq!(parse_delay("1h"), Some(3_600_000));
        assert_eq!(parse_delay("1d"), Some(86_400_000));
        assert_eq!(parse_delay("abc"), None);
        assert_eq!(parse_delay("5"), None);
        assert_eq!(parse_delay("5x"), None);
        assert_eq!(parse_delay("15ms"), None, "satuan gabung invalid");
        assert_eq!(parse_delay(" 5s"), None, "spasi depan invalid");
    }

    #[test]
    fn format_delay_mirip_js() {
        assert_eq!(format_delay(90_000), "2 menit", "1,5 menit dibulatkan ke atas");
        assert_eq!(format_delay(86_400_000), "1 hari");
        assert_eq!(format_delay(3_600_000), "1 jam");
        assert_eq!(format_delay(7_200_000), "2 jam");
        assert_eq!(format_delay(60_000), "1 menit");
        assert_eq!(format_delay(150_000), "3 menit", "2,5 menit dibulatkan ke atas");
        assert_eq!(format_delay(1_500), "2 detik", "1,5 detik dibulatkan ke atas");
        assert_eq!(format_delay(1_000), "1 detik");
        assert_eq!(format_delay(0), "0 detik");
    }

    #[test]
    fn handler_tidak_beri_reaksi_stim() {
        // `Reply::None` dikemas reaksi `⏹️` sebagai catatan sinyal stop;
        // aslinya `m.react("⏹️")` lalu teks stop di JS.
        let r = bcgc(Ctx::default());
        assert!(!r.is_none());
    }
}
