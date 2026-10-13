//! `plugins/owner/delkoin.js` — kurangi koin user.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::{jid_number, parse_int};

/// `db.updateKoin` menjepit antara 0 dan 9 triliun.
const MAX_KOIN: i64 = 9_000_000_000_000;

/// `formatKoin` (`delkoin.js:11-17`) — T/B/M/K dua desimal, lalu ribuan titik.
/// Berbeda dari `addkoin.js`: tidak ada cabang `-1`, dan helper yang bernama
/// `formatNumber` di file JS itu sama sekali tidak dipakai.
fn format_koin(num: i64) -> String {
    let n = num as f64;
    if n >= 1_000_000_000_000.0 {
        return format!("{:.2}T", n / 1_000_000_000_000.0);
    }
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
        "💰 *ᴅᴇʟ ᴋᴏɪɴ*\n\n\
         > `.{p}delkoin <jumlah>` - dari diri sendiri\n\
         > `.{p}delkoin <jumlah> @user` - dari user\n\n\
         `Contoh: {p}delkoin 50000`",
        p = prefix
    )
}

/// Koin tersimpan untuk `jid` — `None` kalau row-nya tidak ada.
///
/// `ponytail:` hanya `sender` (dan anggota roster grup) yang row-nya terbaca
/// lewat `Ctx`; lihat catatan modul `addenergi`.
fn stored_koin(ctx: &Ctx, jid: &str) -> Option<i64> {
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
            .get("koin")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
    )
}

fn detail(target: &str, amount: i64, new_koin: i64) -> String {
    format!(
        "✅ *ᴋᴏɪɴ ᴅɪᴋᴜʀᴀɴɢɪ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 👤 ᴜsᴇʀ: @{target}\n\
         ┃ ➖ ᴋᴜʀᴀɴɢ: *-{amount}*\n\
         ┃ 💰 sɪsᴀ: *{left}*\n\
         ╰┈┈⬡",
        amount = format_koin(amount),
        left = format_koin(new_koin)
    )
}

#[plugin(
    name = "delkoin",
    alias = ["kurangkoin", "removekoin", "delcoin", "delmoney"],
    category = "owner",
    description = "Kurangi koin user",
    usage = ".delkoin <jumlah> @user",
    example = ".delkoin 50000 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn delkoin(ctx: Ctx) -> Reply {
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

    let Some(current) = stored_koin(&ctx, &target) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan di database");
    };
    // `koin === -1` (unlimited) tidak diubah oleh `updateKoin`.
    let new_koin = if current == -1 {
        -1
    } else {
        (current - amount).clamp(0, MAX_KOIN)
    };

    Reply::text(detail(jid_number(&target), amount, new_koin.max(0)))
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: target,
            data: json!({ "koin": new_koin }),
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
            command: "delkoin".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("delkoin").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> (String, i64) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                (jid.clone(), data["koin"].as_i64().expect("koin angka"))
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delkoin").unwrap();
        assert_eq!(p.meta.name, "delkoin");
        assert_eq!(
            p.meta.alias,
            ["kurangkoin", "removekoin", "delcoin", "delmoney"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Kurangi koin user");
        assert_eq!(p.meta.usage, ".delkoin <jumlah> @user");
        assert_eq!(p.meta.example, ".delkoin 50000 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kurangkoin", "removekoin", "delcoin", "delmoney"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "delkoin");
        }
    }

    #[test]
    fn no_amount_shows_help_and_writes_nothing() {
        for text in ["", "@628999", "abc"] {
            let r = run(text, json!({}), Some(OTHER));
            assert!(text_of(&r).starts_with("💰 *ᴅᴇʟ ᴋᴏɪɴ*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_bare_amount_subtracts_from_the_sender() {
        let r = run("50000", json!({ "koin": 1_000_000 }), None);
        assert_eq!(written(&r), (SENDER.to_string(), 950_000));
        let t = text_of(&r);
        assert!(t.contains("┃ 👤 ᴜsᴇʀ: @628111"), "{t}");
        assert!(t.contains("┃ ➖ ᴋᴜʀᴀɴɢ: *-50.00K*"), "{t}");
        assert!(t.contains("┃ 💰 sɪsᴀ: *950.00K*"), "{t}");
    }

    #[test]
    fn subtracting_beyond_zero_clamps_at_zero() {
        let r = run("50000", json!({ "koin": 10 }), None);
        assert_eq!(written(&r).1, 0);
        assert!(text_of(&r).contains("┃ 💰 sɪsᴀ: *0*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unlimited_koin_holder_is_left_alone() {
        // `user.koin === -1` → `updateKoin` mengembalikan `-1` tanpa mengubah.
        let r = run("50000", json!({ "koin": -1 }), None);
        assert_eq!(written(&r).1, -1);
        assert!(text_of(&r).contains("┃ 💰 sɪsᴀ: *0*"), "{}", text_of(&r));
    }

    #[test]
    fn a_user_absent_from_the_store_is_refused_and_writes_nothing() {
        let r = run("50000", json!(null), None);
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_row_without_the_field_counts_as_zero() {
        let r = run("50000", json!({}), None);
        assert_eq!(written(&r), (SENDER.to_string(), 0));
    }

    #[test]
    fn a_mention_becomes_the_target_but_its_row_is_unreadable() {
        // `ponytail:` row target di luar sender+roster tidak terbaca, jadi
        // guard "tidak ditemukan" dulu yang kena — bukan menulis ke sender.
        let r = run("50000", json!({ "koin": 1000 }), Some(OTHER));
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
        assert!(r.mutations().is_empty(), "tidak boleh menulis");
    }

    #[test]
    fn a_quoted_sender_becomes_the_target() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delkoin".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: "50000".into(),
            args: vec!["50000".into()],
            user_data: json!({ "koin": 1000 }),
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
        let r = (oktz_plugin_api::find("delkoin").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
    }

    #[test]
    fn the_applied_reply_reacts_and_is_paired_with_the_write() {
        let r = run("50000", json!({ "koin": 100 }), None);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn koin_format_switches_unit_at_each_threshold() {
        assert_eq!(format_koin(999), "999");
        assert_eq!(format_koin(1_000), "1.00K");
        assert_eq!(format_koin(2_500_000), "2.50M");
        assert_eq!(format_koin(3_000_000_000), "3.00B");
        assert_eq!(format_koin(4_000_000_000_000), "4.00T");
    }
}