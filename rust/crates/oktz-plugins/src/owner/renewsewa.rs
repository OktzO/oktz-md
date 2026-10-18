//! `plugins/owner/renewsewa.js` — perpanjang durasi sewa grup.
//!
//! `ponytail:` Tidak ada API database (`db.db.data.sewa`, `db.db.write()`) dan
//! API grup (`sock.groupGetInviteInfo`, `sock.sendText`) yang diekspos ke
//! plugin di registry Rust. Bagian penyimpanan dan pengiriman pesan ke grup
//! tidak dapat dicerminkan. Handler di bawah hanya mem-port parsing argumen,
//! validasi durasi, dan pemformatan, lalu jatuh ke error saat operasi DB/grup
//! harus dieksekusi.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::error_template;

const LIFETIME_KATA: &[&str] = &["lifetime", "permanent", "forever", "unlimited"];

/// `parseDurationMs(str)` (`renewsewa.js:21-40`).
pub(crate) fn parse_duration_ms(s: &str) -> Option<i64> {
    if LIFETIME_KATA.iter().any(|k| s.to_lowercase() == *k) {
        return Some(i64::MAX);
    }
    let caps = s
        .as_bytes()
        .iter()
        .position(|b| !b.is_ascii_digit());
    let Some(idx) = caps else {
        return None;
    };
    let angka: &str = &s[..idx];
    let unit: &str = &s[idx..];
    if unit.len() != 1 {
        return None;
    }
    let value: i64 = angka.parse().ok()?;
    let multiplier = match unit.to_ascii_lowercase().as_str() {
        "i" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        "m" => 2_592_000_000,
        "y" => 31_536_000_000,
        _ => return None,
    };
    Some(value * multiplier)
}

/// `formatDuration(str)` (`renewsewa.js:42-53`).
pub(crate) fn format_duration(s: &str) -> String {
    if LIFETIME_KATA.iter().any(|k| s.to_lowercase() == *k) {
        return "Permanent".to_string();
    }
    let caps = s
        .as_bytes()
        .iter()
        .position(|b| !b.is_ascii_digit());
    let Some(idx) = caps else {
        return s.to_string();
    };
    let angka: &str = &s[..idx];
    let unit: &str = &s[idx..];
    let units = [
        ("i", "menit"),
        ("h", "jam"),
        ("d", "hari"),
        ("m", "bulan"),
        ("y", "tahun"),
    ];
    let nama = units
        .iter()
        .find(|(u, _)| unit.to_ascii_lowercase() == *u)
        .map(|(_, n)| *n)
        .unwrap_or(unit);
    format!("{angka} {nama}")
}

/// Kartu pakai saat argumen kurang.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📝 *PERPANJANG SEWA*\n\nFormat: *{prefix}renewsewa <link/id> <durasi>*\n\n*FORMAT DURASI:*\n• 30i = 30 menit\n• 12h = 12 jam\n• 7d = 7 hari\n• 1m = 1 bulan\n• 1y = 1 tahun\n• lifetime = Permanent\n\n*CONTOH:*\n• {prefix}renewsewa https://chat.whatsapp.com/xxx 30d\n• {prefix}renewsewa 120363xxx 1m\n\n💡 Durasi ditambahkan ke sisa waktu yang ada, bukan di-reset"
    )
}

#[plugin(
    name = "renewsewa",
    alias = ["perpanjangsewa", "extendsewa"],
    category = "owner",
    description = "Perpanjang durasi sewa grup",
    usage = ".renewsewa <link/id grup> <durasi>",
    example = ".renewsewa https://chat.whatsapp.com/xxx 30d",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn renewsewa(ctx: Ctx) -> Reply {
    if ctx.args.len() < 2 {
        return Reply::text(help(&ctx.prefix));
    }

    let duration_str = &ctx.args[1];
    let duration_ms = parse_duration_ms(duration_str);

    let Some(duration_ms) = duration_ms else {
        return Reply::text("❌ Format durasi tidak valid\nContoh: 7d, 1m, 1y, lifetime");
    };

    // `ponytail:` `resolveGroupId` (butuh `sock.groupGetInviteInfo`) dan
    // operasi `db.db.data.sewa` / `db.db.write()` tidak tersedia di registry,
    // jadi penyimpanan sewa dan pengiriman notifikasi ke grup tidak dapat
    // dieksekusi.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn lifetime_kata_mengembalikan_infinity() {
        for k in LIFETIME_KATA {
            assert_eq!(parse_duration_ms(k), Some(i64::MAX));
            assert_eq!(parse_duration_ms(&k.to_uppercase()), Some(i64::MAX));
        }
    }

    #[test]
    fn durasi_angka_unit_terjadi_perkalian() {
        assert_eq!(parse_duration_ms("30i"), Some(30 * 60_000));
        assert_eq!(parse_duration_ms("12h"), Some(12 * 3_600_000));
        assert_eq!(parse_duration_ms("7d"), Some(7 * 86_400_000));
        assert_eq!(parse_duration_ms("1m"), Some(1 * 2_592_000_000));
        assert_eq!(parse_duration_ms("1y"), Some(1 * 31_536_000_000));
        assert_eq!(parse_duration_ms("2d"), Some(2 * 86_400_000));
    }

    #[test]
    fn durasi_tidak_valid_mengembalikan_none() {
        assert_eq!(parse_duration_ms("abc"), None);
        assert_eq!(parse_duration_ms("1x"), None);
        assert_eq!(parse_duration_ms(""), None);
        assert_eq!(parse_duration_ms("12"), None);
    }

    #[test]
    fn format_duration_mengikuti_js() {
        assert_eq!(format_duration("lifetime"), "Permanent");
        assert_eq!(format_duration("LIFETIME"), "Permanent");
        assert_eq!(format_duration("30i"), "30 menit");
        assert_eq!(format_duration("12h"), "12 jam");
        assert_eq!(format_duration("7d"), "7 hari");
        assert_eq!(format_duration("1m"), "1 bulan");
        assert_eq!(format_duration("1y"), "1 tahun");
        assert_eq!(format_duration("abc"), "abc");
    }

    #[test]
    fn argumen_kurang_menghasilkan_kartu_pakai() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "renewsewa".into(),
            args: vec!["https://chat.whatsapp.com/xxx".into()],
            ..Ctx::default()
        };
        let r = renewsewa(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), help("."));
    }

    #[test]
    fn durasi_tidak_valid_menghasilkan_error() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "renewsewa".into(),
            args: vec!["https://chat.whatsapp.com/xxx".into(), "xyz".into()],
            ..Ctx::default()
        };
        let r = renewsewa(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            "❌ Format durasi tidak valid\nContoh: 7d, 1m, 1y, lifetime"
        );
    }

    #[test]
    fn operasi_db_grup_tanpa_api_jatuh_ke_error() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "renewsewa".into(),
            args: vec!["https://chat.whatsapp.com/xxx".into(), "7d".into()],
            ..Ctx::default()
        };
        let r = renewsewa(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "renewsewa")
        );
        assert!(matches!(r.without_mutations(), Reply::Reaction { .. }));
    }
}
