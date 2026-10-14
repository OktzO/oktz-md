//! `plugins/user/birthdaylist.js` — daftar ulang tahun member grup.
//!
//! Sumbernya `m.groupMetadata.participants`, lalu `db.getUser(jid)` per
//! peserta. Di `Ctx` baris user hanya ada untuk `sender` dan anggota roster,
//! jadi member yang TIDAK ada di store tidak bisa punya `birthday` — daftar
//! bisa lebih pendek dari roster, persis seperti di port `owner/addenergi.rs`.
//!
//! `ponytail:` `ctx.roster.participants[i].data` dipakai, bukan `db.getUser`:
//! pipeline mengisi `data` hanya untuk baris yang ada di store, jadi
//! `Value::Null` = "user belum pernah ada" seperti `db.getUser` mengembalikan
//! `undefined`.
//!
//! Pengurutan bukan "belum ulang tahun dulu" sederhana: `aNext`/`bNext` di JS
//! menandai ulang tahun yang sudahLewat tahun ini, jadi yang lewat masih
//! ikut — hanya tertinggal di bawah yang belum. Comparing dua-duanya menghasilkan
//! urutan yang sama seperti pengurutan JS, dan itu yang dipakai di sini.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{jid_number, jakarta_parts, now_ms};

/// `months` (`birthdaylist.js:60`) — bulan 3 huruf, berbeda dari
/// `common::MONTHS_ID` yang memanjang.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun", "Jul", "Agu", "Sep", "Okt", "Nov", "Des",
];

/// Batas baris yang dicetak sebelum jadi "… dan N lainnya".
const SHOWN: usize = 15;

struct Birthday {
    jid: String,
    day: u32,
    month: u32,
}

const NONE: &str = "❌ *ᴛɪᴅᴀᴋ ᴀᴅᴀ ᴅᴀᴛᴀ*\n\n\
     > Belum ada member yang set birthday\n\n\
     > Gunakan: .setbirthday DD-MM";

/// `user.birthday.split('-').map(Number)` → `(day, month)`. Field yang rusak
/// (bukan `DD-MM`) menghasilkan `NaN` di JS, yang membuat baris itu tidak
/// pernah sama dengan `currentDay/currentMonth` — di sini diabaikan, hasil
///_visible identik karena baris rusak tidak akan pernah ditandai "HARI INI".
fn parse(value: Option<&str>) -> Option<(u32, u32)> {
    let v = value?;
    let (d, m) = v.split_once('-')?;
    Some((d.trim().parse().ok()?, m.trim().parse().ok()?))
}

/// `aNext`/`bNext` di JS, lalu `month`, lalu `day`. `sort` di JS stabil,
/// jadi urutan roster dipertahankan untuk kunci yang sama.
fn collect(ctx: &Ctx) -> (Vec<Birthday>, u32, u32) {
    let (_, month_now, day_now, ..) = jakarta_parts(now_ms());
    let mut out = Vec::new();
    for p in &ctx.roster.participants {
        if let Some((day, month)) = parse(p.data.get("birthday").and_then(|v| v.as_str())) {
            out.push(Birthday {
                jid: p.jid.clone(),
                day,
                month,
            });
        }
    }
    let next = |b: &Birthday| b.month > month_now || (b.month == month_now && b.day >= day_now);
    out.sort_by(|a, b| {
        match (next(a), next(b)) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.month.cmp(&b.month).then(a.day.cmp(&b.day)),
        }
    });
    (out, day_now, month_now)
}

#[plugin(
    name = "birthdaylist",
    alias = ["bdaylist", "listultah", "ultahlist"],
    category = "user",
    description = "Lihat daftar ulang tahun member",
    usage = ".birthdaylist",
    example = ".birthdaylist",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn birthdaylist(ctx: Ctx) -> Reply {
    let (list, day_now, month_now) = collect(&ctx);
    if list.is_empty() {
        return Reply::text(NONE);
    }

    let mut text = String::from(
        "╭━━━━━━━━━━━━━━━━━╮\n┃  🎂 *ᴅᴀғᴛᴀʀ ᴜʟᴛᴀʜ*\n╰━━━━━━━━━━━━━━━━━╯\n\n",
    );
    text.push_str(&format!(
        "╭┈┈⬡「 📋 *{} ᴍᴇᴍʙᴇʀ* 」\n",
        list.len()
    ));

    let mut mentions = Vec::with_capacity(list.len().min(SHOWN));
    for b in list.iter().take(SHOWN) {
        let is_today = b.day == day_now && b.month == month_now;
        let emoji = if is_today { "🎉" } else { "🎂" };
        text.push_str(&format!(
            "┃ {emoji} {} {} - @{}{}\n",
            b.day,
            MONTHS[(b.month - 1) as usize],
            jid_number(&b.jid),
            if is_today { " *HARI INI!*" } else { "" }
        ));
        mentions.push(b.jid.clone());
    }
    if list.len() > SHOWN {
        text.push_str(&format!("┃ ... dan {} lainnya\n", list.len() - SHOWN));
    }
    text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n> Set birthday: .setbirthday DD-MM");

    Reply::Text { text, mentions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;
    use oktz_plugin_api::registry::GroupRoster;
    use serde_json::{Value, json};

    fn run(participants: Vec<(&str, Value)>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "birthdaylist".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: GroupRoster {
                participants: participants
                    .into_iter()
                    .map(|(jid, data)| Participant {
                        jid: jid.into(),
                        data,
                        ..Default::default()
                    })
                    .collect(),
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("birthdaylist").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap().to_string()
    }

    fn mentioned(r: &Reply) -> Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions.clone(),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn with_bday<'a>(jid: &'a str, b: &'a str) -> (&'a str, Value) {
        (
            jid,
            json!({"name": "Someone", "birthday": b}),
        )
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("birthdaylist").unwrap();
        assert_eq!(p.meta.name, "birthdaylist");
        assert_eq!(p.meta.alias, ["bdaylist", "listultah", "ultahlist"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Lihat daftar ulang tahun member");
        assert_eq!(p.meta.usage, ".birthdaylist");
        assert_eq!(p.meta.example, ".birthdaylist");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn all_three_aliases_resolve_back_to_this_plugin() {
        for a in ["bdaylist", "listultah", "ultahlist"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("birthdaylist")
            );
        }
    }

    #[test]
    fn a_roster_without_any_birthday_says_there_is_no_data() {
        let r = run(vec![
            ("628111@s.whatsapp.net", json!({"name": "Tanpa"})),
            ("628222@s.whatsapp.net", Value::Null),
        ]);
        assert_eq!(text_of(&r), NONE);
        assert!(mentioned(&r).is_empty());
    }

    #[test]
    fn an_empty_roster_also_reports_no_data() {
        assert_eq!(text_of(&run(vec![])), NONE);
    }

    #[test]
    fn one_entry_shows_the_short_month_and_mentions_the_member() {
        let r = run(vec![with_bday("628111@s.whatsapp.net", "25-12")]);
        let t = text_of(&r);
        assert!(t.starts_with("╭━━━━━━━━━━━━━━━━━╮"), "{t}");
        assert!(t.contains("1 ᴍᴇᴍʙᴇʀ"), "{t}");
        assert!(t.contains("@628111"), "{t}");
        assert!(t.contains("25 Des - @628111"), "{t}");
        assert!(t.contains("> Set birthday: .setbirthday DD-MM"), "{t}");
        assert_eq!(mentioned(&r), ["628111@s.whatsapp.net"]);
    }

    // `birthdays.sort` di JS (birthdaylist.js:49) menaruh yang BELUM ulang tahun
    // tahun ini di depan, baru bulan lalu tanggal. Karena tiap baris punya
    // status "belum" sendiri, urutannya memang bergantung tanggal hari test
    // jalan — jadi harapan dihitung dari helper yang sama, bukan angka tebakan.
    #[test]
    fn entries_are_sorted_upcoming_first_then_by_month_then_day() {
        let (_, month_now, day_now, ..) = jakarta_parts(now_ms());
        let upcoming = |m: u32, d: u32| m > month_now || (m == month_now && d >= day_now);

        // (bulan, hari, jid, nilai `birthday`)
        let rows: [(u32, u32, &str, &str); 3] = [
            (1, 1, "2@s.whatsapp.net", "01-01"),
            (1, 15, "3@s.whatsapp.net", "15-01"),
            (2, 3, "1@s.whatsapp.net", "03-02"),
        ];
        let mut want: Vec<&str> = rows.iter().map(|r| r.2).collect();
        want.sort_by_key(|j| {
            let r = rows.iter().find(|r| r.2 == *j).unwrap();
            (!upcoming(r.0, r.1), r.0, r.1)
        });

        let r = run(rows.iter().map(|r| with_bday(r.2, r.3)).collect());
        assert_eq!(mentioned(&r), want);

        let t = text_of(&r);
        let lines: Vec<&str> = t.lines().filter(|l| l.contains(" - @")).collect();
        assert_eq!(lines.len(), rows.len(), "{t}");
        for (line, jid) in lines.iter().zip(&want) {
            let (m, d, _, _) = rows.iter().find(|r| r.2 == *jid).unwrap();
            let mon = MONTHS[(m - 1) as usize];
            let today = *d == day_now && *m == month_now;
            assert_eq!(
                line.trim(),
                format!(
                    "┃ {} {d} {mon} - @{}{}",
                    if today { "🎉" } else { "🎂" },
                    jid_number(jid),
                    if today { " *HARI INI!*" } else { "" }
                ),
                "baris `{line}`"
            );
        }
    }

    #[test]
    fn only_the_first_fifteen_entries_are_printed_and_the_rest_counted() {
        let participants: Vec<(String, Value)> = (0..20)
            .map(|i| {
                (
                    format!("628{:03}@s.whatsapp.net", i),
                    json!({"birthday": format!("{:02}-01", i + 1)}),
                )
            })
            .collect();
        let t = text_of(&run(
            participants
                .into_iter()
                .map(|(j, d)| (Box::leak(j.into_boxed_str()) as &str, d))
                .collect(),
        ));
        assert!(t.contains("20 ᴍᴇᴍʙᴇʀ"), "jumlah di header tetap total: {t}");
        assert!(t.contains("┃ ... dan 5 lainnya"), "{t}");
        assert_eq!(
            t.lines().filter(|l| l.contains(" - @")).count(),
            SHOWN,
            "hanya 15 baris member"
        );
    }

    #[test]
    fn a_malformed_birthday_value_is_skipped_instead_of_panicking() {
        let r = run(vec![
            with_bday("1@s.whatsapp.net", "25-12"),
            ("2@s.whatsapp.net", json!({"birthday": "bukan-angka"})),
            ("3@s.whatsapp.net", json!({"birthday": "25"})),
            ("4@s.whatsapp.net", json!({"birthday": 2512})),
        ]);
        let t = text_of(&r);
        assert!(t.contains("1 ᴍᴇᴍʙᴇʀ"), "hanya yang valid: {t}");
        assert!(t.contains("@1"), "{t}");
    }

    #[test]
    fn the_current_date_marked_as_today_uses_the_party_emoji() {
        // Ambil tanggal hari ini dari helper yang sama dengan plugin supaya
        // test tidak meng-hardcode tanggal yang basi.
        let (_, m, d, ..) = jakarta_parts(now_ms());
        let today = format!("{:02}-{:02}", d, m);
        let __balasan = run(vec![with_bday("1@s.whatsapp.net", &today)]);
        let t = text_of(&__balasan);
        assert!(t.contains("🎉"), "emoji hari ini: {t}");
        assert!(t.contains("HARI INI!"), "{t}");
    }

    #[test]
    fn a_date_that_is_not_today_uses_the_cake_emoji() {
        // 29-02 hanya "hari ini" di tahun kabisat; aman dipakai untuk kasus negatif
        // pada tahun yang tidak kabisat, tapi tidak diuji sebagai negatif mutlak —
        // cukup pastikan barisnya tidak ditandai HARI INI kalau bukan tanggalnya.
        let (_, m, d, ..) = jakarta_parts(now_ms());
        let other = if (d, m) != (29, 2) { "29-02" } else { "30-02" };
        let __balasan = run(vec![with_bday("1@s.whatsapp.net", other)]);
        let t = text_of(&__balasan);
        assert!(!t.contains("🎉"), "bukan hari ini: {t}");
    }

    #[test]
    fn the_short_month_names_are_the_js_ones() {
        assert_eq!(MONTHS[0], "Jan");
        assert_eq!(MONTHS[4], "Mei");
        assert_eq!(MONTHS[7], "Agu", "Agustus disingkat jadi Agu di JS");
        assert_eq!(MONTHS[9], "Okt");
        assert_eq!(MONTHS[11], "Des");
    }

    #[test]
    fn a_day_month_pair_is_read_from_the_stored_dd_mm_format() {
        assert_eq!(parse(Some("25-12")), Some((25, 12)));
        assert_eq!(parse(Some(" 1 - 1 ")), Some((1, 1)));
        assert_eq!(parse(Some("25")), None);
        assert_eq!(parse(Some("-12")), None);
        assert_eq!(parse(None), None);
    }
}
