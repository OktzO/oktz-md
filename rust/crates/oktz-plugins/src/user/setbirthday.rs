//! `plugins/user/setbirthday.js` — `users[sender].birthday` (format `DD-MM`).
//!
//! Validasi memakai tabel hari-per-bulan dengan Februari = 29 SELALU:
//! `daysInMonth` di JS tidak tahu tahun, jadi `29-02` diterima setiap tahun.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::MONTHS_ID;

/// `daysInMonth` (`setbirthday.js:57`) — indeks 0 = Januari.
pub(crate) const DAYS_IN_MONTH: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// `^(\d{1,2})[-/](\d{1,2})$` → `(day, month)`. Pemisah `-` atau `/`.
pub(crate) fn parse_date(input: &str) -> Option<(u32, u32)> {
    let input = input.trim();
    let sep = if input.contains('-') { '-' } else { '/' };
    let (d, m) = input.split_once(sep)?;
    // `\d{1,2}` — satu atau dua digit, TIDAK lebih.
    if d.is_empty() || d.len() > 2 || !d.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if m.is_empty() || m.len() > 2 || !m.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((d.parse().ok()?, m.parse().ok()?))
}

pub(crate) fn help(prefix: &str, current: Option<&str>) -> String {
    let mut text = String::from("🎂 *sᴇᴛ ʙɪʀᴛʜᴅᴀʏ*\n\n");
    if let Some(b) = current {
        text.push_str(&format!("> Birthday kamu: *{b}*\n\n"));
    }
    text.push_str(&format!(
        "╭┈┈⬡「 📋 *ғᴏʀᴍᴀᴛ* 」\n┃ {prefix}setbirthday DD-MM\n╰┈┈⬡\n\n\
         *Contoh:*\n> {prefix}setbirthday 25-12\n> {prefix}setbirthday 01-01"
    ));
    text
}

#[plugin(
    name = "setbirthday",
    alias = ["setbday", "setultah", "settgl"],
    category = "user",
    description = "Set tanggal ulang tahun",
    usage = ".setbirthday <DD-MM>",
    example = ".setbirthday 25-12",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn setbirthday(ctx: Ctx) -> Reply {
    let current = ctx.user_field("birthday").and_then(|v| v.as_str());
    let Some(input) = ctx.args.first() else {
        return Reply::text(help(&ctx.prefix, current));
    };

    let Some((day, month)) = parse_date(input) else {
        return Reply::text(format!(
            "❌ Format salah! Gunakan: DD-MM\n\n> Contoh: {}setbirthday 25-12",
            ctx.prefix
        ));
    };

    if !(1..=12).contains(&month) {
        return Reply::text("❌ Bulan tidak valid! (1-12)");
    }
    if day < 1 || day > DAYS_IN_MONTH[(month - 1) as usize] {
        return Reply::text(format!(
            "❌ Tanggal tidak valid untuk bulan {month}!"
        ));
    }

    // `day.toString().padStart(2,'0')` — selalu dua digit.
    let stored = format!("{day:02}-{month:02}");
    let month_name = MONTHS_ID[(month - 1) as usize];

    Reply::Text {
        text: format!(
            "✅ *ʙɪʀᴛʜᴅᴀʏ ᴅɪsɪᴍᴘᴀɴ!*\n\n\
             ╭┈┈⬡「 🎂 *ᴅᴇᴛᴀɪʟ* 」\n\
             ┃ 📅 Tanggal: *{day} {month_name}*\n\
             ┃ 👤 User: @{sender}\n\
             ╰┈┈⬡\n\n\
             > Bot akan mengucapkan selamat\n\
             > ulang tahun di hari spesialmu! 🎉",
            sender = crate::common::jid_number(&ctx.sender)
        ),
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: serde_json::json!({ "birthday": stored }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setbirthday".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("setbirthday").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> String {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data["birthday"].as_str().expect("harus string").to_string()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    fn mentions_of(r: &Reply) -> &Vec<String> {
        // Reply ini dibungkus `Mutate`, jadi `mentions` ada di balasan yang
        // sudah dilepas mutasinya.
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setbirthday").unwrap();
        assert_eq!(p.meta.name, "setbirthday");
        assert_eq!(p.meta.alias, ["setbday", "setultah", "settgl"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Set tanggal ulang tahun");
        assert_eq!(p.meta.usage, ".setbirthday <DD-MM>");
        assert_eq!(p.meta.example, ".setbirthday 25-12");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setbday", "setultah", "settgl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "setbirthday");
        }
    }

    #[test]
    fn no_argument_shows_the_format_and_writes_nothing() {
        for user in [json!(null), json!({}), json!({"birthday": "01-01"})] {
            let r = run(user.clone(), "");
            assert!(text_of(&r).starts_with("🎂 *sᴇᴛ ʙɪʀᴛʜᴅᴀʏ*"), "input {user}");
            assert!(text_of(&r).contains(".setbirthday DD-MM"), "{}", text_of(&r));
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn a_stored_birthday_is_shown_in_the_help_branch() {
        let r = run(json!({"birthday": "25-12"}), "");
        assert!(text_of(&r).contains("> Birthday kamu: *25-12*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_valid_date_is_stored_zero_padded() {
        assert_eq!(stored(&run(json!({}), "25-12")), "25-12");
        assert_eq!(stored(&run(json!({}), "1-1")), "01-01", "padStart(2,'0')");
        assert_eq!(stored(&run(json!({}), "05-09")), "05-09");
    }

    #[test]
    fn a_slash_separator_is_also_accepted_and_normalised_to_a_dash() {
        assert_eq!(stored(&run(json!({}), "07/03")), "07-03");
    }

    #[test]
    fn the_confirm_reply_names_the_day_the_month_and_mentions_the_sender() {
        let r = run(json!({}), "25-12");
        let t = text_of(&r);
        assert!(t.starts_with("✅ *ʙɪʀᴛʜᴅᴀʏ ᴅɪsɪᴍᴘᴀɴ!*"), "{t}");
        assert!(t.contains("┃ 📅 Tanggal: *25 Desember*"), "{t}");
        assert!(t.contains("┃ 👤 User: @628111"), "@ sebelum @ server");
        assert_eq!(mentions_of(&r), &["628111@s.whatsapp.net"]);
    }

    #[test]
    fn an_invalid_month_is_rejected_before_the_day_is_checked() {
        for bad in ["25-00", "25-13", "01-99"] {
            let r = run(json!({}), bad);
            assert_eq!(text_of(&r), "❌ Bulan tidak valid! (1-12)", "input {bad}");
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn a_day_beyond_the_month_length_is_rejected() {
        for bad in ["32-01", "31-04", "31-06", "31-09", "00-01"] {
            let r = run(json!({}), bad);
            assert!(
                text_of(&r).starts_with("❌ Tanggal tidak valid"),
                "input {bad}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn february_always_allows_29_because_javascript_ignores_the_year() {
        // `daysInMonth[1] = 29` tanpa lookup tahun — 29-02 diterima setiap
        // tahun, termasuk tahun bukan tahun kabisat.
        assert_eq!(stored(&run(json!({}), "29-02")), "29-02");
        assert_eq!(stored(&run(json!({}), "28-02")), "28-02");
    }

    #[test]
    fn a_malformed_date_is_rejected() {
        for bad in ["abc", "25", "25-12-01", "255-12", "25_12", "-12", "25-"] {
            let r = run(json!({}), bad);
            assert!(text_of(&r).starts_with("❌ Format salah!"), "input {bad}: {}", text_of(&r));
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn the_format_error_carries_the_live_prefix() {
        let r = run(json!({}), "abc");
        assert!(
            text_of(&r).contains("Contoh: .setbirthday 25-12"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn the_success_reply_is_paired_with_the_write() {
        let r = run(json!({}), "25-12");
        assert!(text_of(&r).contains("ᴅɪsɪᴍᴘᴀɴ"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_parser_contracts_hold() {
        assert_eq!(parse_date("25-12"), Some((25, 12)));
        assert_eq!(parse_date("1/2"), Some((1, 2)));
        assert_eq!(parse_date(""), None);
        assert_eq!(parse_date("123-1"), None, "tiga digit ditolak");
        assert_eq!(parse_date("1-123"), None);
        assert_eq!(parse_date("ab-cd"), None);
        assert_eq!(parse_date("2024-12-25"), None, "tahun tidak diterima");
    }
}