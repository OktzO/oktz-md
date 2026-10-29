//! `plugins/user/birthday.js` — baca `users[sender].birthday`, hitung hari ke
//! ulang tahun berikutnya di Asia/Jakarta.
//!
//! `ponytail:` `Ctx::user_data` hanya diisi untuk `sender`, sedangkan JS membaca
//! `db.getUser(target)` setelah mention. Port ini karena itu selalu membaca dan
//! menampilkan birthday PENGIRIM — nama dan mention juga mengikuti pengirim
//! supaya tidak ada baris yang mencampur nama orang lain dengan tanggal orang
//! ini. Jalankan tanpa mention (`.birthday`) kalau butuh hasil yang persis
//! sama dengan JS. Perbaiki di `Ctx` lebih dulu kalau mention harus didukung.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{MONTHS_ID, days_from_civil, jakarta_parts};

const NO_SELF: &str = "❌ Kamu belum set birthday!\n\n";
const NO_OTHER: &str = "❌ User belum set birthday!";

/// `(hari, bulan)` dari `"DD-MM"` hasil `setbirthday`. `Number('xx')` = NaN di
/// JS, dan `new Date(y, NaN - 1, day)` jadi Invalid Date — tidak crash, tapi
/// semua perhitungan jadi NaN. Di sini yang tidak terbaca jadi `None` supaya
/// plugin tidak pernah mengetag nonsense nonsense.
pub(crate) fn parse_birthday(raw: &str) -> Option<(u32, u32)> {
    let (d, m) = raw.split_once('-')?;
    let day: u32 = d.parse().ok()?;
    let month: u32 = m.parse().ok()?;
    if !(1..=12).contains(&month) || day == 0 {
        return None;
    }
    Some((day, month))
}

/// `(next_bday_ms, diff_days, is_today)` untuk `now` dalam ms.
///
/// JS: `nextBday = new Date(y, month - 1, day)`; kalau sudah lewat, maju satu
/// tahun. `Math.ceil(diff / 86400000)` — jadi besok masih "1 hari lagi".
pub(crate) fn next_birthday(now: i64, day: u32, month: u32) -> (i64, i64, bool) {
    let (year, cur_month, cur_day, ..) = jakarta_parts(now);
    let is_today = cur_day == day && cur_month == month;

    // Semua dalam milidetik: `now` dari `Date.now()`, `midnight` dari
    // `days_from_civil(...)*86400*1000`. Mengcampur detik dan milidetik di sini
    // membuat hitung mundur meleset ribuan hari.
    let midnight =
        days_from_civil(year, month, day) * 86_400_000 - crate::common::JAKARTA_OFFSET_SECS * 1000;
    let start_of_today = days_from_civil(year, cur_month, cur_day) * 86_400_000
        - crate::common::JAKARTA_OFFSET_SECS * 1000;
    let next = if midnight < start_of_today {
        midnight + 365 * 86_400_000
    } else {
        midnight
    };

    // `Math.ceil(diff / 86400000)` — `div_ceil` butuh Rust 1.73, MSRV crate
    // ini lebih tua, jadi pembulatan ke atas ditulis eksplisit.
    let delta = next - now;
    let diff_days = delta.div_euclid(86_400_000) + i64::from(delta.rem_euclid(86_400_000) > 0);
    (next, diff_days, is_today)
}

pub(crate) fn render(ctx: &Ctx, now: i64) -> String {
    let raw = ctx.user_field("birthday").and_then(|v| v.as_str());
    let Some((day, month)) = raw.and_then(parse_birthday) else {
        return if ctx.mentioned.is_empty() {
            format!(
                "{NO_SELF}> Gunakan: {}setbirthday DD-MM\n> Contoh: {}setbirthday 25-12",
                ctx.prefix, ctx.prefix
            )
        } else {
            NO_OTHER.to_string()
        };
    };

    let (_, diff_days, is_today) = next_birthday(now, day, month);
    let mut text = String::from("🎂 *ʙɪʀᴛʜᴅᴀʏ ɪɴғᴏ*\n\n");
    text.push_str("╭┈┈⬡「 👤 *ᴜsᴇʀ* 」\n");
    text.push_str(&format!("┃ 🏷️ @{}\n", crate::common::jid_number(&ctx.sender)));
    text.push_str(&format!("┃ 📅 {day} {}\n", MONTHS_ID[(month - 1) as usize]));
    if is_today {
        text.push_str("┃ 🎉 *HARI INI ULTAH!*\n");
    } else {
        text.push_str(&format!("┃ 🕕 {diff_days} hari lagi\n"));
    }
    text.push_str("╰┈┈┈┈┈┈┈┈⬡");

    if is_today {
        text.push_str("\n\n🎊 *HAPPY BIRTHDAY!* 🎊\n");
        text.push_str("> Semoga panjang umur dan\n");
        text.push_str("> sukses selalu! 🎉🎂");
    }
    text
}

#[plugin(
    name = "birthday",
    alias = ["bday", "ultah", "ulangtahun"],
    category = "user",
    description = "Lihat ulang tahun member",
    usage = ".birthday [@user]",
    example = ".birthday @user",
    cooldown = 5,
    energi = 0,
    is_private = false
)]
pub fn birthday(ctx: Ctx) -> Reply {
    Reply::Text {
        text: render(&ctx, crate::common::now_ms()),
        mentions: vec![ctx.sender.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(user: serde_json::Value, mentioned: Vec<&str>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "birthday".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        }
    }

    /// `render` langsung supaya `now` bisa dikunci — handler sungguhan
    /// memakai jam sistem dan tidak bisa diuji pada tanggal tertentu.
    fn run(user: serde_json::Value, now: i64, mentioned: Vec<&str>) -> String {
        render(&ctx_of(user, mentioned), now)
    }

    #[test]
    fn the_registered_handler_answers_with_the_same_shape() {
        let entry = oktz_plugin_api::find("birthday").unwrap();
        // Tanpa birthday -> cabang error; dengan birthday -> kartu; rusak ->
        // kembali ke cabang error. Ketiganya harus tetap berupa teks.
        for (user, head) in [
            (serde_json::json!(null), "❌ Kamu belum set birthday!"),
            (serde_json::json!({}), "❌ Kamu belum set birthday!"),
            (serde_json::json!({"birthday": "25-12"}), "🎂 *ʙɪʀᴛʜᴅᴀʏ ɪɴғᴏ*"),
            (serde_json::json!({"birthday": "salah"}), "❌ Kamu belum set birthday!"),
        ] {
            let reply = (entry.handler)(ctx_of(user.clone(), vec![]));
            let body = reply.without_mutations().text_body().expect("harus ada teks");
            assert!(body.starts_with(head), "{user}: {body:?}");
            match reply {
                Reply::Text { mentions, .. } => assert_eq!(mentions, ["628111@s.whatsapp.net"]),
                other => panic!("harus Text, dapat {other:?}"),
            }
        }
    }

    /// 2026-06-15T00:00:00Z = 2026-06-15 07:00 WIB.
    const JUNE_15: i64 = 1_781_481_600_000;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("birthday").unwrap();
        assert_eq!(p.meta.name, "birthday");
        assert_eq!(p.meta.alias, ["bday", "ultah", "ulangtahun"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Lihat ulang tahun member");
        assert_eq!(p.meta.usage, ".birthday [@user]");
        assert_eq!(p.meta.example, ".birthday @user");
        assert_eq!(p.meta.cooldown, 5);
        assert!(!p.meta.is_private && !p.meta.is_group && p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["bday", "ultah", "ulangtahun"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "birthday");
        }
    }

    // -- parsing -----------------------------------------------------------

    #[test]
    fn parse_birthday_accepts_the_setbirthday_output_format() {
        assert_eq!(parse_birthday("25-12"), Some((25, 12)));
        assert_eq!(parse_birthday("01-01"), Some((1, 1)));
        assert_eq!(parse_birthday("29-02"), Some((29, 2)), "leap day ikut diterima");
        assert_eq!(parse_birthday(""), None);
        assert_eq!(parse_birthday("25"), None);
        assert_eq!(parse_birthday("25-13"), None);
        assert_eq!(parse_birthday("00-01"), None);
        assert_eq!(parse_birthday("abc-def"), None);
    }

    // -- cabang state tersimpan -------------------------------------------

    #[test]
    fn absent_birthday_for_self_asks_to_set_one_with_the_prefix() {
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"birthday": null}), serde_json::json!({"birthday": ""})] {
            let out = run(user.clone(), JUNE_15, vec![]);
            assert_eq!(
                out,
                "❌ Kamu belum set birthday!\n\n> Gunakan: .setbirthday DD-MM\n> Contoh: .setbirthday 25-12",
                "input {user}"
            );
        }
    }

    #[test]
    fn absent_birthday_for_a_mention_says_the_shorter_line() {
        let out = run(serde_json::json!({}), JUNE_15, vec!["628999@s.whatsapp.net"]);
        assert_eq!(out, "❌ User belum set birthday!");
    }

    // -- cabang hitung mundur ---------------------------------------------

    #[test]
    fn a_future_birthday_counts_down_in_whole_days() {
        let out = run(serde_json::json!({"birthday": "25-12"}), JUNE_15, vec![]);
        assert!(out.starts_with("🎂 *ʙɪʀᴛʜᴅᴀʏ ɪɴғᴏ*\n\n╭┈┈⬡「 👤 *ᴜsᴇʀ* 」\n┃ 🏷️ @628111\n┃ 📅 25 Desember\n"));
        assert!(out.contains("┃ 🕕 193 hari lagi\n"));
        assert!(out.ends_with("╰┈┈┈┈┈┈┈┈⬡"), "tanpa tambahan jika bukan hari ini");
        assert!(!out.contains("HAPPY BIRTHDAY"));
    }

    #[test]
    fn a_past_birthday_rolls_over_to_next_year() {
        let out = run(serde_json::json!({"birthday": "01-01"}), JUNE_15, vec![]);
        assert!(out.contains("┃ 📅 1 Januari\n"));
        assert!(out.contains("┃ 🕕 200 hari lagi\n"));
    }

    #[test]
    fn today_gets_the_celebration_branch_and_no_countdown() {
        let out = run(serde_json::json!({"birthday": "15-06"}), JUNE_15, vec![]);
        assert!(out.contains("┃ 🎉 *HARI INI ULTAH!*\n"));
        assert!(!out.contains("hari lagi"));
        assert!(out.ends_with("╰┈┈┈┈┈┈┈┈⬡\n\n🎊 *HAPPY BIRTHDAY!* 🎊\n> Semoga panjang umur dan\n> sukses selalu! 🎉🎂"));
    }

    #[test]
    fn tomorrow_still_counts_one_day() {
        // 2026-06-15T00:00:00Z = 2026-06-15 07:00 WIB; ultah berikutnya = 16 Juni.
        let out = run(serde_json::json!({"birthday": "16-06"}), JUNE_15, vec![]);
        assert!(out.contains("┃ 🕕 1 hari lagi\n"), "{out}");
        assert!(!out.contains("HARI INI ULTAH"));
    }

    #[test]
    fn the_month_name_uses_the_indonesian_long_form() {
        for (raw, nama) in [
            ("15-01", "Januari"),
            ("15-02", "Februari"),
            ("15-03", "Maret"),
            ("15-04", "April"),
            ("15-05", "Mei"),
            ("15-06", "Juni"),
            ("15-07", "Juli"),
            ("15-08", "Agustus"),
            ("15-09", "September"),
            ("15-10", "Oktober"),
            ("15-11", "November"),
            ("15-12", "Desember"),
        ] {
            let out = run(serde_json::json!({"birthday": raw}), JUNE_15, vec![]);
            assert!(out.contains(&format!("┃ 📅 15 {nama}\n")), "bulan {raw}");
        }
    }

    #[test]
    fn a_corrupt_birthday_falls_into_the_not_set_branch_instead_of_panicking() {
        let out = run(serde_json::json!({"birthday": "bukan-tanggal"}), JUNE_15, vec![]);
        assert!(out.starts_with("❌ Kamu belum set birthday!"));
    }

    #[test]
    fn next_birthday_is_stable_across_the_rollover_boundary() {
        // Tepat tengah malam WIB sehari sebelum ulang tahun.
        let (_, d, today) = next_birthday(JUNE_15 - 86_400_000, 15, 6);
        assert_eq!((d, today), (1, false));
        let (_, d, today) = next_birthday(JUNE_15, 15, 6);
        assert_eq!((d, today), (0, true));
    }
}