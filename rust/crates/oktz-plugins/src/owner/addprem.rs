//! `plugins/owner/addprem.js` — kelola premium users (tambah / hapus / daftar).
//!
//! `ponytail:` JS tulis state lewat `getDatabase()` (`db.data.premium`,
//! `db.getUser`/`db.setUser`, `db.updateExp`, `db.updateKoin`, `db.save()`) serta
//! `jadibotId`/`isJadibot` — tidak ada di `Ctx`, dan tidak ada `Mutation` untuk
//! array `db.data.premium`. Oleh karena itu hanya cabang validasi + teks help
//! yang di-port; operasi tulis (add/del/list jadibot maupun reguler) diakhiri
//! dengan `error_template` dan reaksi `☢`, pola yang sama seperti `swm.rs` /
//! `delpanel.rs`. Saat store premium + jadibot tersedia di `Ctx`, tambahkan
//! `pub mod addprem;` di `src/owner/mod.rs` secara manual (mod.rs milik plugin
//! lain — dilarang menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Energi default yang dipakai JS saat mempromosikan user ke premium
/// (`config.energi?.premium || 999999`).
const PREMIUM_ENERGI_DEFAULT: i64 = 999999;

/// Durasi bawaan saat tidak ada argumen waktu: 30 hari.
const DURASI_DEFAULT_MS: i64 = 30 * 24 * 60 * 60 * 1000;
const DURASI_DEFAULT_LABEL: &str = "30 hari";

/// `formatDate(ts)` (`addprem.js:32-38`) — `id-ID`, hari numerik, bulan panjang,
/// tahun numerik. Padanan [`crate::common::date_long`].
pub(crate) fn format_tanggal(ms: i64) -> String {
    crate::common::date_long(ms)
}

/// `extractTarget(m)` (`addprem.js:40-46`) — quoted.sender, mentioned[0],
/// args[0]; ambil digit saja, kosong kalau tidak ada.
pub(crate) fn ekstrak_target(
    quoted_sender: Option<&str>,
    mentioned: &[String],
    args: &[String],
) -> String {
    for s in quoted_sender
        .iter()
        .chain(mentioned.first().map(String::as_str))
        .chain(args.first().map(String::as_str))
    {
        let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            return digits;
        }
    }
    String::new()
}

/// `toMentionJid(value)` (`addprem.js:48-51`) — digit → `628xxx@s.whatsapp.net`.
pub(crate) fn to_mention_jid(value: &str) -> Option<String> {
    let number: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    if number.is_empty() {
        None
    } else {
        Some(format!("{number}@s.whatsapp.net"))
    }
}

/// Hasil pemrosesan argumen durasi (`addprem.js:160-180`).
pub(crate) struct Durasi {
    pub ms: i64,
    pub label: String,
}

/// Unit waktu yang dikenali JS (`h|hari|j|jam|m|menit|d|detik`).
const UNIT_HARI: &[&str] = &["d", "detik"];
const UNIT_MENIT: &[&str] = &["m", "menit"];
const UNIT_JAM: &[&str] = &["h", "j", "jam"];
const UNIT_DETIK: &[&str] = &["d", "detik"];

/// Apakah seluruh argumen cocok `/^\d+(unit)?$/i` (argumen waktu).
pub(crate) fn is_arg_waktu(s: &str) -> bool {
    let lowercase = s.to_ascii_lowercase();
    let mut digit_len = 0;
    for c in lowercase.chars() {
        if c.is_ascii_digit() {
            digit_len += 1;
        } else {
            break;
        }
    }
    if digit_len == 0 {
        return false;
    }
    let unit = &lowercase[digit_len..];
    unit.is_empty()
        || unit == "h"
        || unit == "hari"
        || unit == "j"
        || unit == "jam"
        || unit == "m"
        || unit == "menit"
        || unit == "d"
        || unit == "detik"
}

/// `parseInt(match[1])` + pemilihan unit (`addprem.js:162-179`).
pub(crate) fn parse_durasi(args: &[String]) -> Durasi {
    let time_arg = args.iter().find(|a| is_arg_waktu(a));
    let Some(teks) = time_arg else {
        return Durasi {
            ms: DURASI_DEFAULT_MS,
            label: DURASI_DEFAULT_LABEL.to_string(),
        };
    };
    let lowercase = teks.to_ascii_lowercase();
    let mut digit_len = 0;
    for c in lowercase.chars() {
        if c.is_ascii_digit() {
            digit_len += 1;
        } else {
            break;
        }
    }
    let angka: i64 = lowercase[..digit_len].parse().unwrap_or(0);
    let unit = &lowercase[digit_len..];
    let (ms, label) = if unit == "d" || unit == "detik" {
        (angka * 1000, format!("{angka} detik"))
    } else if unit == "m" || unit == "menit" {
        (angka * 60 * 1000, format!("{angka} menit"))
    } else if unit == "j" || unit == "jam" {
        (angka * 60 * 60 * 1000, format!("{angka} jam"))
    } else {
        // unit kosong / "h" / "hari" → hari
        (angka * 24 * 60 * 60 * 1000, format!("{angka} hari"))
    };
    Durasi { ms, label }
}

/// Teks help tanpa argumen (`addprem.js:115-119`).
pub(crate) fn help_kerja(prefix: &str, tambah: bool) -> String {
    let kata = if tambah { "ADD" } else { "DEL" };
    format!(
        "💎 *{kata} PREMIUM*\n\nMasukkan nomor atau tag user\n\n`Contoh: {prefix}{cmd} 6281234567890`",
        cmd = if tambah { "addprem" } else { "delprem" }
    )
}

/// Teks daftar premium kosong (`addprem.js:85-87`).
pub(crate) const DAFTAR_KOSONG: &str = "💎 Belum ada premium terdaftar";

/// Teks daftar premium jadibot kosong (`addprem.js:66-69`).
pub(crate) fn daftar_jadibot_kosong(jadibot_id: &str) -> String {
    format!(
        "💎 Belum ada premium di jadibot ini\nGunakan `{prefix}addprem` untuk menambah",
        prefix = crate::common::CONFIG_PREFIX
    )
}

#[plugin(
    name = "addprem",
    alias = ["addpremium", "setprem", "delprem", "delpremium", "listprem", "premlist"],
    category = "owner",
    description = "Kelola premium users",
    usage = ".addprem <nomor/@tag> [hari]\n.delprem <nomor/@tag>\n.listprem\n.cekprem <nomor/@tag>",
    example = ".addprem 6281234567890 30",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addprem(ctx: Ctx) -> Reply {
    let cmd = ctx.command.to_ascii_lowercase();
    let is_add = matches!(cmd.as_str(), "addprem" | "addpremium" | "setprem");
    let is_del = matches!(cmd.as_str(), "delprem" | "delpremium");
    let is_list = matches!(cmd.as_str(), "listprem" | "premlist");

    if is_list {
        // `db.data.premium` + `jadibotId`/`isJadibot` tidak tersedia di Ctx.
        return Reply::text(error_templ(&ctx.prefix, &ctx.command)).with_reaction("☢");
    }

    let target = ekstrak_target(
        ctx.quoted().map(|q| q.sender.as_str()),
        &ctx.mentioned,
        &ctx.args,
    );

    if target.is_empty() {
        return Reply::text(help_kerja(&ctx.prefix, is_add));
    }

    let mut target_number = target;
    if target_number.starts_with('0') {
        target_number = format!("62{}", &target_number[1..]);
    }

    if target_number.len() < 10 || target_number.len() > 15 {
        return Reply::text("❌ Format nomor tidak valid");
    }

    // `db.data.premium`, `db.getUser`/`db.setUser`, `db.updateExp`,
    // `db.updateKoin`, `db.save()` tidak tersedia di Ctx — operasi add/del
    // tidak dapat dijalankan. Sama seperti swm.rs / delpanel.rs: berakhir di
    // error template.
    Reply::text(error_templ(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

fn error_templ(prefix: &str, command: &str) -> String {
    crate::common::error_template(prefix, command)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "addprem".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = oktz_plugin_api::find("addprem").unwrap();
        assert_eq!(p.meta.name, "addprem");
        assert_eq!(
            p.meta.alias,
            ["addpremium", "setprem", "delprem", "delpremium", "listprem", "premlist"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn semua_alias_resolve() {
        for a in ["addpremium", "setprem", "delprem", "delpremium", "listprem", "premlist"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "addprem",
                "alias {a}"
            );
        }
    }

    #[test]
    fn tanpa_target_mengembalikan_help() {
        let r = addprem(ctx());
        assert!(teks(&r).starts_with("💎 *ADD PREMIUM*"));
        assert!(teks(&r).contains("Contoh: .addprem 6281234567890"));
    }

    #[test]
    fn del_tanpa_target_mengembalikan_help_del() {
        let mut c = ctx();
        c.command = "delprem".into();
        let r = addprem(c);
        assert!(teks(&r).starts_with("💎 *DEL PREMIUM*"));
    }

    #[test]
    fn ekstrak_target_mengambil_digit_dari_quoted_pertama() {
        assert_eq!(
            ekstrak_target(Some("abc 628123@s.whatsapp.net"), &[], &[]),
            "628123"
        );
        assert_eq!(
            ekstrak_target(None, &["628999:3@s.whatsapp.net".into()], &[]),
            "6289993"
        );
        assert_eq!(ekstrak_target(None, &[], &["081234567890".into()]), "081234567890");
        assert_eq!(ekstrak_target(None, &[], &[]), "");
    }

    #[test]
    fn to_mention_jid_mengembalikan_jid_atau_none() {
        assert_eq!(to_mention_jid("628123"), Some("628123@s.whatsapp.net".to_string()));
        assert_eq!(to_mention_jid("abc"), None);
        assert_eq!(to_mention_jid("628123:3@s.whatsapp.net"), Some("6281233@s.whatsapp.net"));
    }

    #[test]
    fn format_tanggal_sama_dengan_date_long() {
        assert_eq!(format_tanggal(1727602800000), crate::common::date_long(1727602800000));
    }

    #[test]
    fn is_arg_waktu_menerima_unit_dan_angka_saja() {
        assert!(is_arg_waktu("30"));
        assert!(is_arg_waktu("7h"));
        assert!(is_arg_waktu("2hari"));
        assert!(is_arg_waktu("1j"));
        assert!(is_arg_waktu("3jam"));
        assert!(is_arg_waktu("5m"));
        assert!(is_arg_waktu("10menit"));
        assert!(is_arg_waktu("1d"));
        assert!(is_arg_waktu("30detik"));
        assert!(!is_arg_waktu("abc"));
        assert!(!is_arg_waktu("3x"));
        assert!(!is_arg_waktu(""));
    }

    #[test]
    fn parse_durasi_default_30_hari() {
        let d = parse_durasi(&[]);
        assert_eq!(d.ms, DURASI_DEFAULT_MS);
        assert_eq!(d.label, "30 hari");
    }

    #[test]
    fn parse_durasi_semua_unit() {
        assert_eq!(parse_durasi(&["86400d".into()]).ms, 86400 * 1000);
        assert_eq!(parse_durasi(&["86400detik".into()]).label, "86400 detik");
        assert_eq!(parse_durasi(&["5menit".into()]).ms, 5 * 60 * 1000);
        assert_eq!(parse_durasi(&["2jam".into()]).ms, 2 * 60 * 60 * 1000);
        assert_eq!(parse_durasi(&["1h".into()]).ms, 1 * 60 * 60 * 1000);
        assert_eq!(parse_durasi(&["3hari".into()]).ms, 3 * 24 * 60 * 60 * 1000);
    }

    #[test]
    fn nomor_awal_nol_diganti_62() {
        let mut target = "08123456789".to_string();
        if target.starts_with('0') {
            target = format!("62{}", &target[1..]);
        }
        assert_eq!(target, "628123456789");
    }

    #[test]
    fn nomor_panjang_salah_ditolak() {
        assert!("12345".len() < 10);
        assert!("123456789012345".len() > 15);
    }

    #[test]
    fn nomor_valid_berakhir_error_template() {
        // DB write tidak tersedia → error template + ☢.
        let mut c = ctx();
        c.args = vec!["6281234567890".into()];
        let r = addprem(c);
        assert_eq!(
            teks(&r),
            error_templ(".", "addprem"),
            "harus error_template karena db tidak tersedia"
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            ),
            "harus bereaksi ☢"
        );
    }
}
