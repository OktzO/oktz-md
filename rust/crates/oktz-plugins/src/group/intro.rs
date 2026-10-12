//! `plugins/group/intro.js` — `groups[chat].intro` + placeholder parsing.
//!
//! `ponytail:` `@group` dan `@desc` diambil dari `m.groupMetadata.subject` /
//! `.desc` — metadata WA, bukan `groups[chat]`. `Ctx` tidak membawa keduanya,
//! jadi kedua placeholder itu selalu jatuh ke nilai cadangan JS. Tambahkan
//! `group_subject` / `group_desc` ke `Ctx` kalau router mau keduanya; sisanya
//! (`@user`, `@count`, `@date`, `@time`, `@botname`) sudah tercermin.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::{date_long, jid_number, time_hm};

/// `config.bot.name` (`config.js:39`).
pub(crate) const BOT_NAME: &str = "Oktz Bot";

/// Nilai cadangan untuk `groupMeta?.subject` dan `groupMeta?.desc`.
pub(crate) const UNKNOWN_GROUP: &str = "Grup";
pub(crate) const NO_DESC: &str = "Tidak ada deskripsi";

pub(crate) const DEFAULT_INTRO: &str = "halo kak @user 🖐\n\nKenalan dulu yukk\n- Nama : \n- Umur : \n- Asal : \n- Hobi : \n- Status : \n\nSemoga betah yahh, di grup @group\n\n> Untuk Owner:\nganti intro bawaan dengan .setintro <text>";

/// Urutan token harus sama dengan `parsePlaceholders` (`intro.js:18-25`) —
/// JS menjalankan tujuh `.replace` berurutan, jadi `@user` ditulis sebelum
/// `@count`, dan nilai pengganti ikut bisa tersentuh pola berikutnya.
const TOKENS: [&str; 7] = [
    "@user", "@group", "@count", "@date", "@time", "@desc", "@botname",
];

/// `text.replace(/@token/gi, value)` untuk ketujuh token, satu kali lintasan.
pub(crate) fn parse_placeholders(
    text: &str,
    now: i64,
    member_count: usize,
    sender_jid: &str,
) -> String {
    let values = [
        format!("@{}", jid_number(sender_jid)),
        UNKNOWN_GROUP.to_string(),
        member_count.to_string(),
        date_long(now),
        time_hm(now),
        NO_DESC.to_string(),
        BOT_NAME.to_string(),
    ];

    let mut out = String::with_capacity(text.len());
    let bytes: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == '@'
            && let Some(hit) = TOKENS.iter().position(|t| {
                let tc: Vec<char> = t.chars().collect();
                bytes.len() - i >= tc.len()
                    && tc
                        .iter()
                        .enumerate()
                        .all(|(k, c)| bytes[i + k].eq_ignore_ascii_case(c))
            })
        {
            out.push_str(&values[hit]);
            i += TOKENS[hit].chars().count();
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// `groupData.intro || DEFAULT_INTRO`.
pub(crate) fn intro_text(ctx: &Ctx) -> String {
    match ctx.group_field("intro").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => DEFAULT_INTRO.to_string(),
    }
}

#[plugin(
    name = "intro",
    alias = ["perkenalan", "selamatdatang"],
    category = "group",
    description = "Tampilkan pesan intro grup",
    usage = ".intro",
    example = ".intro",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn intro(ctx: Ctx) -> Reply {
    Reply::Text {
        text: parse_placeholders(
            &intro_text(&ctx),
            crate::common::now_ms(),
            ctx.roster.participants.len(),
            &ctx.sender,
        ),
        mentions: vec![ctx.sender.clone()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::{GroupRoster, Participant};
    use serde_json::json;

    fn roster(n: usize) -> GroupRoster {
        GroupRoster {
            participants: (0..n)
                .map(|i| Participant {
                    jid: format!("62800{i}@s.whatsapp.net"),
                    is_admin: false,
                    is_owner: false,
                    data: Default::default(),
                })
                .collect(),
            is_bot_admin: false,
        }
    }

    fn run(group: serde_json::Value, sender: &str, n: usize) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "intro".into(),
            sender: sender.into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: roster(n),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("intro").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions_of(r: &Reply) -> &Vec<String> {
        match r {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("intro").unwrap();
        assert_eq!(p.meta.name, "intro");
        assert_eq!(p.meta.alias, ["perkenalan", "selamatdatang"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Tampilkan pesan intro grup");
        assert_eq!(p.meta.usage, ".intro");
        assert_eq!(p.meta.example, ".intro");
        assert_eq!(p.meta.cooldown, 10);
        assert!(p.meta.is_group && !p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["perkenalan", "selamatdatang"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "intro");
        }
    }

    // -- cabang state tersimpan -------------------------------------------

    #[test]
    fn absent_group_intro_falls_back_to_the_default_template() {
        for g in [json!(null), json!({}), json!({"intro": ""})] {
            let reply = run(g.clone(), "628111@s.whatsapp.net", 3);
            let out = text_of(&reply);
            assert!(out.starts_with("halo kak @628111 🖐\n\nKenalan dulu yukk"), "{g}");
            assert!(out.contains("> Untuk Owner:"), "{g}");
            assert!(!out.contains("@user"), "semua placeholder harus habis: {g}");
        }
    }

    #[test]
    fn a_custom_intro_replaces_the_default_entirely() {
        let reply = run(
            json!({"intro": "Halo @user! Kamu member ke-@count."}),
            "628111@s.whatsapp.net",
            7,
        );
        assert_eq!(text_of(&reply), "Halo @628111! Kamu member ke-7.");
    }

    // -- cabang placeholder ------------------------------------------------

    #[test]
    fn every_seven_placeholder_is_substituted() {
        let now = 1_767_225_600_000; // 2026-01-01T00:00:00Z = 07:00 WIB
        let out = parse_placeholders(
            "@user|@group|@count|@date|@time|@desc|@botname",
            now,
            42,
            "628111@s.whatsapp.net",
        );
        assert_eq!(
            out,
            "@628111|Grup|42|1 Januari 2026|07:00|Tidak ada deskripsi|Oktz Bot"
        );
    }

    #[test]
    fn placeholders_are_case_insensitive_and_replace_every_occurrence() {
        let now = 1_767_225_600_000;
        let out = parse_placeholders("@USER @user @User", now, 1, "628111@s.whatsapp.net");
        assert_eq!(out, "@628111 @628111 @628111");
        assert_eq!(
            parse_placeholders("@COUNT @count", now, 5, "628111@s.whatsapp.net"),
            "5 5",
            "flag `i` di regex JS"
        );
    }

    #[test]
    fn an_at_sign_that_starts_no_token_is_left_alone() {
        let now = 1_767_225_600_000;
        assert_eq!(
            parse_placeholders("@user @nope @", now, 1, "628111@s.whatsapp.net"),
            "@628111 @nope @"
        );
    }

    #[test]
    fn member_count_comes_from_the_roster_and_zero_for_an_empty_roster() {
        let full = run(json!({"intro": "jumlah @count"}), "628111@s.whatsapp.net", 12);
        assert_eq!(text_of(&full), "jumlah 12");
        let empty = run(json!({"intro": "jumlah @count"}), "628111@s.whatsapp.net", 0);
        assert_eq!(
            text_of(&empty),
            "jumlah 0",
            "roster kosong = `@count` 0, sama seperti `participants.length || '0'`"
        );
    }

    #[test]
    fn sender_is_always_mentioned_and_numbered_without_the_server() {
        let r = run(json!({"intro": "hai @user"}), "628111:3@s.whatsapp.net", 2);
        assert_eq!(text_of(&r), "hai @628111:3");
        assert_eq!(mentions_of(&r), &["628111:3@s.whatsapp.net"]);
    }

    #[test]
    fn the_default_template_mentions_every_placeholder_it_can_reach() {
        let out = parse_placeholders(
            DEFAULT_INTRO,
            1_767_225_600_000,
            5,
            "628111@s.whatsapp.net",
        );
        assert!(out.contains("@628111"));
        assert!(out.contains("di grup Grup"));
        assert!(!out.contains("@user"), "semua placeholder harus habis");
        assert!(!out.contains("@group"));
    }
}