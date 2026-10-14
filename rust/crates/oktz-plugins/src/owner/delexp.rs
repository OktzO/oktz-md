//! `plugins/owner/delexp.js` — kurangi exp user.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::{jid_number, parse_int};

/// `db.updateExp` menjepit di 9 miliar dan tidak pernah turun ke bawah 0.
const MAX_EXP: i64 = 9_000_000_000;

/// `formatNumber` (`delexp.js:11-16`) — B/M/K dua desimal, lalu ribuan titik.
fn format_exp(num: i64) -> String {
    let n = num as f64;
    if n >= 1_000_000_000.0 {
        return format!("{:.2}B", n / 1_000_000_000.0);
    }
    if n >= 1_000_000.0 {
        return format!("{:.2}M", n / 1_000_000.0);
    }
    if n >= 1_000.0 {
        return format!("{:.2}K", n / 1_000.0);
    }
    crate::tools::kalkulatormbg::thousands_id(n)
}

/// `args.find(a => !isNaN(a) && !a.startsWith('@'))`.
fn numeric_arg(ctx: &Ctx) -> Option<i64> {
    ctx.args
        .iter()
        .find(|a| !a.starts_with('@') && a.parse::<f64>().is_ok())
        .and_then(|a| parse_int(a))
}

fn help(prefix: &str) -> String {
    format!(
        "⭐ *ᴅᴇʟ ᴇxᴘ*\n\n\
         > `.{p}delexp <jumlah>` - dari diri sendiri\n\
         > `.{p}delexp <jumlah> @user` - dari user\n\n\
         `Contoh: {p}delexp 5000`",
        p = prefix
    )
}

/// Exp tersimpan untuk `jid` — `None` kalau row-nya tidak ada.
///
/// `ponytail:` hanya `sender` (dan anggota roster grup) yang row-nya terbaca
/// lewat `Ctx`; lihat catatan modul `addenergi`.
fn stored_exp(ctx: &Ctx, jid: &str) -> Option<i64> {
    let entry = if jid == ctx.sender {
        Some(&ctx.user_data)
    } else {
        ctx.roster
            .participants
            .iter()
            .find(|p| p.jid == jid)
            .map(|p| &p.data)
    };
    let entry = entry?;
    if entry.is_null() {
        return None;
    }
    Some(
        entry
            .get("exp")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
    )
}

fn detail(target: &str, amount: i64, new_exp: i64) -> String {
    format!(
        "✅ *ᴇxᴘ ᴅɪᴋᴜʀᴀɴɢɪ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 👤 ᴜsᴇʀ: @{target}\n\
         ┃ ➖ ᴋᴜʀᴀɴɢ: *-{amount}*\n\
         ┃ ⭐ sɪsᴀ: *{left}*\n\
         ╰┈┈⬡",
        amount = format_exp(amount),
        left = format_exp(new_exp)
    )
}

#[plugin(
    name = "delexp",
    alias = ["kurangexp", "removeexp", "delxp"],
    category = "owner",
    description = "Kurangi exp user",
    usage = ".delexp <jumlah> @user",
    example = ".delexp 5000 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn delexp(ctx: Ctx) -> Reply {
    let amount = numeric_arg(&ctx).unwrap_or(0);

    let target = match ctx.quoted() {
        Some(q) => Some(q.sender.clone()),
        None => ctx.mentioned.first().cloned(),
    };
    let target = match target {
        Some(jid) if !jid.is_empty() => jid,
        _ if amount > 0 => ctx.sender.clone(),
        _ => return Reply::text(help(&ctx.prefix)),
    };

    // `!targetJid || amount <= 0` — OR, jadi mention tanpa jumlah juga kena
    // help, bukan lolos ke sender.
    if amount <= 0 {
        return Reply::text(help(&ctx.prefix));
    }

    if amount <= 0 {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Jumlah harus lebih dari 0");
    }

    let Some(current) = stored_exp(&ctx, &target) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan di database");
    };
    // `exp === -1` (unlimited) tidak diubah sama sekali oleh `updateExp`.
    let new_exp = if current == -1 {
        -1
    } else {
        (current - amount).clamp(0, MAX_EXP)
    };

    Reply::text(detail(jid_number(&target), amount, new_exp.max(0)))
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: target,
            data: json!({ "exp": new_exp }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn run(text: &str, user: Value, mentioned: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delexp".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("delexp").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> (String, i64) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                (jid.clone(), data["exp"].as_i64().expect("exp angka"))
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delexp").unwrap();
        assert_eq!(p.meta.name, "delexp");
        assert_eq!(p.meta.alias, ["kurangexp", "removeexp", "delxp"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Kurangi exp user");
        assert_eq!(p.meta.usage, ".delexp <jumlah> @user");
        assert_eq!(p.meta.example, ".delexp 5000 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kurangexp", "removeexp", "delxp"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "delexp");
        }
    }

    #[test]
    fn no_amount_shows_help_and_writes_nothing() {
        for text in ["", "@628999", "abc"] {
            let r = run(text.clone(), json!({}), Some(OTHER));
            assert!(text_of(&r).starts_with("⭐ *ᴅᴇʟ ᴇxᴘ*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_bare_amount_subtracts_from_the_sender() {
        let r = run("5000", json!({ "exp": 100_000 }), None);
        assert_eq!(written(&r), (SENDER.to_string(), 95_000));
        let t = text_of(&r);
        assert!(t.contains("┃ 👤 ᴜsᴇʀ: @628111"), "{t}");
        assert!(t.contains("┃ ➖ ᴋᴜʀᴀɴɢ: *-5.00K*"), "{t}");
        assert!(t.contains("┃ ⭐ sɪsᴀ: *95.00K*"), "{t}");
    }

    #[test]
    fn subtracting_beyond_zero_clamps_at_zero() {
        let r = run("5000", json!({ "exp": 10 }), None);
        assert_eq!(written(&r).1, 0);
        assert!(text_of(&r).contains("┃ ⭐ sɪsᴀ: *0*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unlimited_exp_user_is_left_alone() {
        // `user.exp === -1` → `updateExp` mengembalikan `-1` tanpa mengubah.
        let r = run("5000", json!({ "exp": -1 }), None);
        assert_eq!(written(&r).1, -1);
        assert!(text_of(&r).contains("┃ ⭐ sɪsᴀ: *0*"), "{}", text_of(&r));
    }

    #[test]
    fn a_user_absent_from_the_store_is_refused_and_writes_nothing() {
        let r = run("5000", json!(null), None);
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_row_without_the_field_counts_as_zero() {
        let r = run("5000", json!({}), None);
        assert_eq!(written(&r), (SENDER.to_string(), 0));
    }

    #[test]
    fn a_mention_becomes_the_target_but_its_row_is_unreadable() {
        // `ponytail:` row target di luar sender+roster tidak terbaca, jadi
        // guard "tidak ditemukan" dulu yang kena — bukan menulis ke sender.
        let r = run("5000", json!({ "exp": 1000 }), Some(OTHER));
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
        assert!(r.mutations().is_empty(), "tidak boleh menulis");
    }

    #[test]
    fn a_quoted_sender_becomes_the_target() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delexp".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: "5000".into(),
            args: vec!["5000".into()],
            user_data: json!({ "exp": 1000 }),
            quoted: Some(oktz_plugin_api::QuotedMessage {
                id: "q".into(),
                sender: OTHER.into(),
                chat: SENDER.into(),
                text: String::new(),
                kind: oktz_plugin_api::QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("delexp").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
    }

    #[test]
    fn the_applied_reply_reacts_and_is_paired_with_the_write() {
        let r = run("5000", json!({ "exp": 100 }), None);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn exp_format_switches_unit_at_each_threshold() {
        assert_eq!(format_exp(999), "999");
        assert_eq!(format_exp(1_000), "1.00K");
        assert_eq!(format_exp(2_500_000), "2.50M");
        assert_eq!(format_exp(3_000_000_000), "3.00B");
    }
}