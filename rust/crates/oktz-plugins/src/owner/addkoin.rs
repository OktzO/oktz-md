//! `plugins/owner/addkoin.js` — tambah koin user (plafon 9 triliun).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::{jid_number, parse_int};

const MAX_KOIN: i64 = 9_000_000_000_000;

/// `formatKoin` (`addkoin.js:16-22`) — `-1` jadi `∞ Unlimited`, lalu T/B/M/K.
fn format_koin(num: i64) -> String {
    let n = num as f64;
    if num == -1 {
        return "∞ Unlimited".to_string();
    }
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
        "💰 *ᴀᴅᴅ ᴋᴏɪɴ*\n\n\
         > `.{p}addkoin <jumlah>` - ke diri sendiri\n\
         > `.{p}addkoin <jumlah> @user` - ke orang lain\n\
         > Max: 9.000.000.000.000 (9T)\n\n\
         `Contoh: {p}addkoin 100000`",
        p = prefix
    )
}

/// `ponytail:` `Ctx` hanya membawa row pengirim (lihat `oktz-wa` pipeline);
/// koin user lain tidak terbaca, jadi `db.updateKoin` di sini dijumlahkan dari
/// `0` — sama seperti user yang belum punya koin. Tambahkan
/// `user_entry(jid)` ke `Ctx` kalau router mau membaca row sembarang.
fn stored_koin(ctx: &Ctx) -> i64 {
    ctx.user_field("koin")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0)
}

#[plugin(
    name = "addkoin",
    alias = ["tambahkoin", "givekoin", "addcoin", "adddcoin"],
    category = "owner",
    description = "Tambah koin user (max 9 Triliun)",
    usage = ".addkoin <jumlah> @user",
    example = ".addkoin 100000 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addkoin(ctx: Ctx) -> Reply {
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
    // cabang help, bukan lolos ke sender.
    if amount <= 0 {
        return Reply::text(help(&ctx.prefix));
    }

    let amount = amount.min(MAX_KOIN);

    if stored_koin(&ctx) == -1 {
        return Reply::Text {
            text: format!(
                "💰 *INFORMATION*\n@{} sudah memiliki koin *∞ Unlimited*\nTidak perlu menambahkan koin lagi",
                jid_number(&target)
            ),
            mentions: vec![target],
        };
    }

    let new_koin = (stored_koin(&ctx) + amount).clamp(0, MAX_KOIN);
    Reply::text(format!(
        "✅ Berhasil menambahkan koin *@{}* sebanyak *{}*",
        jid_number(&target),
        format_koin(amount)
    ))
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
            command: "addkoin".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("addkoin").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("addkoin").unwrap();
        assert_eq!(p.meta.name, "addkoin");
        assert_eq!(
            p.meta.alias,
            ["tambahkoin", "givekoin", "addcoin", "adddcoin"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Tambah koin user (max 9 Triliun)");
        assert_eq!(p.meta.usage, ".addkoin <jumlah> @user");
        assert_eq!(p.meta.example, ".addkoin 100000 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tambahkoin", "givekoin", "addcoin", "adddcoin"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addkoin");
        }
    }

    #[test]
    fn no_amount_shows_help_and_writes_nothing() {
        for msg in ["", "@628999", "abc"] {
            let r = run(msg, json!({}), Some(OTHER));
            assert!(text_of(&r).starts_with("💰 *ᴀᴅᴅ ᴋᴏɪɴ*"), "input {msg:?}");
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn a_bare_amount_adds_to_the_sender_and_reports_the_added_value() {
        let r = run("100000", json!({ "koin": 5 }), None);
        assert_eq!(written(&r), (SENDER.to_string(), 100_005));
        assert!(text_of(&r).contains("sebanyak *100.00K*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("*@628111*"), "{}", text_of(&r));
    }

    #[test]
    fn a_mention_redirects_the_write_to_that_jid() {
        assert_eq!(
            written(&run("1000", json!({}), Some(OTHER))),
            (OTHER.to_string(), 1_000)
        );
    }

    #[test]
    fn the_cap_is_nine_trillion() {
        let r = run("99999999999999", json!({ "koin": 0 }), None);
        assert_eq!(written(&r).1, MAX_KOIN);
        assert!(text_of(&r).contains("*9.00T*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unlimited_koin_holder_is_refused_and_nothing_is_written() {
        let r = run("100", json!({ "koin": -1 }), None);
        assert!(text_of(&r).starts_with("💰 *INFORMATION*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &[SENDER.to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn the_success_reply_reacts_and_is_paired_with_the_write() {
        let r = run("100", json!({}), None);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn koin_format_switches_unit_at_each_threshold() {
        assert_eq!(format_koin(0), "0");
        assert_eq!(format_koin(999), "999");
        assert_eq!(format_koin(1_000), "1.00K");
        assert_eq!(format_koin(2_500_000), "2.50M");
        assert_eq!(format_koin(3_000_000_000), "3.00B");
        assert_eq!(format_koin(4_000_000_000_000), "4.00T");
        assert_eq!(format_koin(-1), "∞ Unlimited");
    }
}