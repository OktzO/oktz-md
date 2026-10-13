//! `plugins/owner/dellevel.js` — kurangi level user lewat exp.
//!
//! `ponytail:` JS baca `db.getUser(targetJid)` untuk JID yang bukan sender.
//! `Ctx` hanyabringakan data sender, jadi user lain diambil dari roster
//! (`ctx.roster.participants[].data`) — entry yang tidak ada di sana dianggap
//! user baru (`exp = 0`), sama seperti `db.getUser(x) || db.setUser(x)`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::jid_number;
use crate::tools::kalkulatormbg::thousands_id;
use crate::user::exp::{EXP_PER_LEVEL, calculate_level, role_of};

/// Dua baris "Cara pakai" di-hardcode `.dellevel` di JS; hanya baris contoh
/// yang memakai `m.prefix`.
pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "📊 *ᴅᴇʟ ʟᴇᴠᴇʟ*\n\n\
         ╭┈┈⬡「 📋 *ᴜsᴀɢᴇ* 」\n\
         ┃ > `.dellevel <jumlah>` - ke diri sendiri\n\
         ┃ > `.dellevel <jumlah> @user` - ke orang lain\n\
         ╰┈┈┈┈┈┈┈┈⬡\n\n\
         > Contoh: `{prefix}dellevel 5`"
    )
}

/// `args.find(a => !isNaN(a) && !a.startsWith('@'))` — `isNaN` dipakai pada
/// string, jadi bentuk `"5px"` TIDAK lolos (Number("5px") = NaN), berbeda dari
/// `parseInt` yang memotong di karakter non-digit pertama.
pub(crate) fn levels_from(args: &[String]) -> Option<i64> {
    args.iter().find(|a| {
        if a.starts_with('@') {
            return false;
        }
        let t = a.trim();
        // `Number("inf")` = NaN di JS; `f64::from_str` justru menerimanya,
        // jadi nama non-numerik harus dikunci sebelum parse.
        let numeric_start =
            matches!(t.chars().next(), Some(c) if c.is_ascii_digit() || c == '+' || c == '-' || c == '.');
        numeric_start && t.parse::<f64>().is_ok()
    })
    .and_then(|a| crate::common::parse_int(a))
}

/// `extractTarget` — reply lebih dulu dari mention, tapi keduanya kalah dari
/// `levels > 0` yang membuat target mundur ke sender.
pub(crate) fn target_of(ctx: &Ctx) -> Option<String> {
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    ctx.mentioned.first().cloned()
}

/// `db.getUser(targetJid)` — data user lain hanya ada di roster; di luar
/// situ JS akan membuat user baru dengan exp 0.
pub(crate) fn user_exp(ctx: &Ctx, target: &str) -> i64 {
    if target == ctx.sender {
        return crate::common::int_field(ctx, "exp");
    }
    let number = jid_number(target);
    ctx.roster
        .participants
        .iter()
        .find(|p| p.jid == target || jid_number(&p.jid) == number)
        .and_then(|p| p.data.get("exp"))
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
}

/// `Math.max(0, exp - levels * EXP_PER_LEVEL)`.
pub(crate) fn exp_after(exp: i64, levels: i64) -> i64 {
    (exp - levels * EXP_PER_LEVEL).max(0)
}

pub(crate) fn detail(target: &str, levels: i64, removed: i64, old: i64, new: i64) -> String {
    format!(
        "✅ *ʟᴇᴠᴇʟ ᴅɪᴋᴜʀᴀɴɢɪ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 👤 User: @{number}\n\
         ┃ ➖ Kurang: *-{levels} Level*\n\
         ┃ 🚄 Exp Removed: *-{removed}*\n\
         ┃ 📊 Level: *{old} → {new}*\n\
         ┃ {role}\n\
         ╰┈┈┈┈┈┈┈┈⬡",
        number = jid_number(target),
        removed = thousands_id(removed as f64),
        role = role_of(new),
    )
}

#[plugin(
    name = "dellevel",
    alias = ["kuranglevel", "removelevel", "dellvl"],
    category = "owner",
    description = "Kurangi level user (via exp)",
    usage = ".dellevel <jumlah> @user",
    example = ".dellevel 5 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dellevel(ctx: Ctx) -> Reply {
    let levels = levels_from(&ctx.args).unwrap_or(0);
    let mut target = target_of(&ctx);
    if target.is_none() && levels > 0 {
        target = Some(ctx.sender.clone());
    }
    let Some(target) = target else {
        return Reply::text(usage(&ctx.prefix));
    };
    if levels <= 0 {
        return Reply::text(usage(&ctx.prefix));
    }

    let old_exp = user_exp(&ctx, &target);
    let old_level = calculate_level(old_exp);
    let removed = levels * EXP_PER_LEVEL;
    let new_exp = exp_after(old_exp, levels);
    let new_level = calculate_level(new_exp);

    Reply::Text {
        text: detail(&target, levels, removed, old_level, new_level),
        mentions: vec![target.clone()],
    }
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: target,
        data: serde_json::json!({ "exp": new_exp }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, QuotedKind, QuotedMessage};
    use serde_json::{Value, json};

    fn quoted(sender: &str) -> Option<QuotedMessage> {
        Some(QuotedMessage {
            id: "MSG1".into(),
            sender: sender.into(),
            chat: "120363@g.us".into(),
            text: "hi".into(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        })
    }

    fn member(jid: &str, exp: i64) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: false,
            is_owner: false,
            data: json!({ "exp": exp }),
        }
    }

    fn run(args: &[&str], mentioned: &[&str], quoted_from: Option<&str>, exp: i64) -> Reply {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: "dellevel".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: args.join(" "),
            args,
            mentioned: mentioned.iter().map(|s| s.to_string()).collect(),
            quoted: quoted_from.and_then(quoted),
            user_data: json!({ "exp": exp }),
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![member("628999@s.whatsapp.net", 250_000)],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("dellevel").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written_exp(r: &Reply) -> (String, Value) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => (jid.clone(), data["exp"].clone()),
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("dellevel").unwrap();
        assert_eq!(p.meta.name, "dellevel");
        assert_eq!(p.meta.alias, ["kuranglevel", "removelevel", "dellvl"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Kurangi level user (via exp)");
        assert_eq!(p.meta.usage, ".dellevel <jumlah> @user");
        assert_eq!(p.meta.example, ".dellevel 5 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium && !p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn the_level_drop_is_written_to_the_quoted_sender() {
        let r = run(&["5"], &[], Some("628999@s.whatsapp.net"), 0);
        let (jid, exp) = written_exp(&r);
        assert_eq!(jid, "628999@s.whatsapp.net");
        assert_eq!(exp, json!(200_000), "250.000 - 5 * 10.000");
        // calculateLevel = floor(exp / 10.000) + 1, jadi 250.000 -> 26.
        assert!(text_of(&r).contains("*26 → 21*"));
        assert!(text_of(&r).contains("*-50.000*"), "exp pakai format id-ID");
    }

    #[test]
    fn a_mention_wins_when_nothing_is_quoted() {
        let r = run(&["2"], &["628999@s.whatsapp.net"], None, 0);
        assert_eq!(written_exp(&r).0, "628999@s.whatsapp.net");
    }

    #[test]
    fn without_a_mention_the_sender_is_the_target() {
        let r = run(&["1"], &[], None, 20_000);
        let (jid, exp) = written_exp(&r);
        assert_eq!(jid, "628111@s.whatsapp.net");
        assert_eq!(exp, json!(10_000));
        assert!(text_of(&r).contains("@628111"));
    }

    #[test]
    fn exp_never_goes_below_zero() {
        assert_eq!(exp_after(0, 5), 0);
        assert_eq!(exp_after(3_000, 1), 0);
        assert_eq!(exp_after(30_000, 1), 20_000);
    }

    #[test]
    fn a_non_numeric_or_zero_level_shows_usage_and_writes_nothing() {
        for args in [vec![], vec!["abc"], vec!["0"], vec!["-3"]] {
            let r = run(&args, &[], None, 20_000);
            assert!(text_of(&r).starts_with("📊 *ᴅᴇʟ ʟᴇᴠᴇʟ*"), "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn a_number_prefixed_by_an_at_sign_is_not_a_level() {
        // `.dellevel @user 5` — mention ditulis sebagai argumen biasa, bukan
        // `mentionedJid`, jadi tidak boleh jadi jumlah.
        assert_eq!(levels_from(&["@628999".to_string()]), None);
        assert_eq!(levels_from(&["5".to_string()]), Some(5));
        assert_eq!(levels_from(&["5px".to_string()]), None, "isNaN, bukan parseInt");
    }

    #[test]
    fn a_reply_mentions_the_target_jid() {
        let r = run(&["1"], &[], Some("628999@s.whatsapp.net"), 0);
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => match reply.as_ref() {
                Reply::Text { mentions, .. } => {
                    assert_eq!(mentions, &["628999@s.whatsapp.net".to_string()])
                }
                other => panic!("harus Text, dapat {other:?}"),
            },
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}