//! `plugins/owner/delenergi.js` — kurangi energi user.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::{jid_number, parse_int};

/// `user.energi === -1` — hasil `updateEnergi` juga `-1` tanpa mengubah apa
/// pun kalau user sudah unlimited (`database.js:630`).
const UNLIMITED: i64 = -1;

/// Ribuan bertitik, tanda minus ikut (`-1234` → `-1.234`).
fn thousands(n: i64) -> String {
    if n < 0 {
        format!("-{}", crate::tools::kalkulatormbg::thousands_id(n.unsigned_abs() as f64))
    } else {
        crate::tools::kalkulatormbg::thousands_id(n as f64)
    }
}

/// `formatNumber` (`delenergi.js:11-13`).
fn format_number(n: i64) -> String {
    if n == UNLIMITED {
        "∞ Unlimited".to_string()
    } else {
        thousands(n)
    }
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
        "⚡ *ᴅᴇʟ ᴇɴᴇʀɢɪ*\n\n\
         > `.{p}delenergi <jumlah>` - dari diri sendiri\n\
         > `.{p}delenergi <jumlah> @user` - dari user\n\n\
         `Contoh: {p}delenergi 50`",
        p = prefix
    )
}

/// Energi tersimpan untuk `jid`.
///
/// `ponytail:` `Ctx` hanya mengisi `user_data` untuk `sender` dan
/// `participants[i].data` untuk roster grup. JID di luar keduanya dianggap
/// "tidak ada di database" — sama dengan `db.getUser(jid)` yang `null`.
fn stored_energi(ctx: &Ctx, jid: &str) -> Option<i64> {
    let entry = if jid == ctx.sender {
        Some(&ctx.user_data)
    } else {
        ctx.roster
            .participants
            .iter()
            .find(|p| p.jid == jid)
            .map(|p| &p.data)
    };
    // `db.getUser` null kalau row-nya tidak ada; row yang ada tapi tanpa
    // field `energi` tetap dihitung 0 oleh `user.energi ?? 0` di updateEnergi.
    let entry = entry?;
    if entry.is_null() {
        return None;
    }
    Some(
        entry
            .get("energi")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
    )
}

fn detail(target: &str, amount: i64, new_energi: i64) -> String {
    format!(
        "✅ *ᴇɴᴇʀɢɪ ᴅɪᴋᴜʀᴀɴɢɪ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 👤 ᴜsᴇʀ: @{target}\n\
         ┃ ➖ ᴋᴜʀᴀɴɢ: *-{amount}*\n\
         ┃ ⚡ sɪsᴀ: *{left}*\n\
         ╰┈┈⬡",
        amount = thousands(amount),
        left = format_number(new_energi)
    )
}

#[plugin(
    name = "delenergi",
    alias = ["kurangenergi", "removeenergi", "hapusenergi", "delenergy"],
    category = "owner",
    description = "Kurangi energi user",
    usage = ".delenergi <jumlah> @user",
    example = ".delenergi 50 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn delenergi(ctx: Ctx) -> Reply {
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

    let amount_label = amount;
    if amount <= 0 {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Jumlah harus lebih dari 0");
    }

    let Some(current) = stored_energi(&ctx, &target) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan di database");
    };

    // `user.energi === -1` → `setUser(targetJid, { energi: 25 })` dulu, jadi
    // user unlimited bisa dikurangi juga (dan turun ke 25).
    let base = if current == UNLIMITED { 25 } else { current };
    let new_energi = (base - amount).max(0);

    Reply::text(detail(jid_number(&target), amount_label, new_energi))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: target,
        data: json!({ "energi": new_energi }),
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
            command: "delenergi".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("delenergi").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> (String, i64) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                (jid.clone(), data["energi"].as_i64().expect("energi angka"))
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delenergi").unwrap();
        assert_eq!(p.meta.name, "delenergi");
        assert_eq!(
            p.meta.alias,
            ["kurangenergi", "removeenergi", "hapusenergi", "delenergy"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Kurangi energi user");
        assert_eq!(p.meta.usage, ".delenergi <jumlah> @user");
        assert_eq!(p.meta.example, ".delenergi 50 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["kurangenergi", "removeenergi", "hapusenergi", "delenergy"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "delenergi");
        }
    }

    #[test]
    fn no_amount_shows_help_and_writes_nothing() {
        for text in ["", "@628999", "abc"] {
            let r = run(text, json!({}), Some(OTHER));
            assert!(text_of(&r).starts_with("⚡ *ᴅᴇʟ ᴇɴᴇʀɢɪ*"), "input {text:?}");
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn a_bare_amount_subtracts_from_the_sender() {
        let r = run("50", json!({ "energi": 200 }), None);
        assert_eq!(written(&r), (SENDER.to_string(), 150));
        let t = text_of(&r);
        assert!(t.contains("┃ 👤 ᴜsᴇʀ: @628111"), "{t}");
        assert!(t.contains("┃ ➖ ᴋᴜʀᴀɴɢ: *-50*"), "{t}");
        assert!(t.contains("┃ ⚡ sɪsᴀ: *150*"), "{t}");
    }

    #[test]
    fn subtracting_beyond_zero_clamps_at_zero() {
        let r = run("500", json!({ "energi": 10 }), None);
        assert_eq!(written(&r).1, 0);
        assert!(text_of(&r).contains("┃ ⚡ sɪsᴀ: *0*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unlimited_user_first_drops_to_twenty_five() {
        // JS: `energi === -1` → `setUser(..., {energi: 25})` sebelum pengurangan.
        let r = run("50", json!({ "energi": -1 }), None);
        assert_eq!(written(&r).1, 0, "25 - 50 dijepit ke 0");
    }

    #[test]
    fn a_user_absent_from_the_store_is_refused_and_writes_nothing() {
        // `db.getUser(targetJid)` null → guard "tidak ditemukan". Baris `{}`
        // ADA di store (object JS truthy), jadi bukan kasus ini.
        let r = run("50", json!(null), None);
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_row_without_the_field_counts_as_zero() {
        let r = run("50", json!({}), None);
        assert_eq!(written(&r), (SENDER.to_string(), 0));
    }

    #[test]
    fn a_mention_becomes_the_target_but_its_row_is_unreadable() {
        // `ponytail:` target mention di luar roster tidak punya row di `Ctx`,
        // jadi guard "user tidak ditemukan" dulu yang kena — sama seperti
        // `db.getUser(targetJid)` yang `null` untuk user yang tak pernah
        // terdaftar. Barisnya ditulis ke JID target, bukan ke sender.
        let r = run("50", json!({ "energi": 200 }), Some(OTHER));
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
        assert!(r.mutations().is_empty(), "tidak boleh menulis");
    }

    #[test]
    fn a_quoted_sender_becomes_the_target() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delenergi".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: "50".into(),
            args: vec!["50".into()],
            user_data: json!({ "energi": 200 }),
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
        let r = (oktz_plugin_api::find("delenergi").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("User tidak ditemukan di database"), "{}", text_of(&r));
    }

    #[test]
    fn the_applied_reply_reacts_and_is_paired_with_the_write() {
        let r = run("50", json!({ "energi": 100 }), None);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn thousands_separator_matches_the_js_regex() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(5_000), "5.000");
        assert_eq!(format_number(UNLIMITED), "∞ Unlimited");
    }
}