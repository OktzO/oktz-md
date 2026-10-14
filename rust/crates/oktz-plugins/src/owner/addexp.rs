//! `plugins/owner/addexp.js` — tambah exp user (plafon 9 miliar).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

use crate::common::{jid_number, parse_int};
use crate::user::exp::calculate_level;

const MAX_EXP: i64 = 9_000_000_000;

/// `formatNumber` (`addexp.js:22-27`) — B/M/K dua desimal, lalu ribuan titik.
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
        "⭐ *ᴀᴅᴅ ᴇxᴘ*\n\n\
         > `.{p}addexp <jumlah>` - ke diri sendiri\n\
         > `.{p}addexp <jumlah> @user` - ke user\n\
         > Max: 9.000.000.000 (9B)\n\n\
         `Contoh: {p}addexp 10000`",
        p = prefix
    )
}

#[plugin(
    name = "addexp",
    alias = ["tambahexp", "giveexp", "addxp"],
    category = "owner",
    description = "Tambah exp user (max 9 Miliar)",
    usage = ".addexp <jumlah> @user",
    example = ".addexp 10000 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addexp(ctx: Ctx) -> Reply {
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

    // Guard JS: `!targetJid || amount <= 0` (OR) — jadi mention tanpa jumlah
    // juga kena help, bukan lolos ke sender. Cabang `amount <= 0` kedua di
    // JS (balasan `ɢᴀɢᴀʟ`) mati karena guard pertama, jadi tidak diport.
    if amount <= 0 {
        return Reply::text(help(&ctx.prefix));
    }
    let amount = amount.min(MAX_EXP);

    // `addExpWithLevelCheck` menulis ke `m.sender`, bukan `targetJid`
    // (`level.js:234`) — bug di JS yang sengaja dibiarkan agar tidak
    // "diperbaiki" diam-diam. Kartu level-up (canvas + `sock.sendMedia`)
    // tidak ada di port ini; `checkAndNotifyLevelUp` yang tersisa efeknya
    // cuma kolom `rpg` di bawah.
    //
    // `ponytail:` `user` di JS adalah row TARGET, sedangkan `Ctx` hanya
    // membawa row pengirim. Exp yang dijumlahkan dan ditulis di sini milik
    // pengirim; tambahkan `user_entry(jid)` ke `Ctx` kalau router mau
    // menghitung dari row target.
    let old = ctx
        .user_field("exp")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let new_exp = if old == -1 {
        -1
    } else {
        (old + amount).clamp(0, MAX_EXP)
    };

    // `user.rpg = {}` dulu (`level.js:220`), lalu `user.rpg.level` +
    // empat stat turunan hanya kalau level benar-benar naik (`level.js:135`).
    let mut rpg = ctx
        .user_field("rpg")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    let new_level = calculate_level(new_exp);
    if new_level > calculate_level(old) {
        rpg.insert("level".into(), json!(new_level));
        rpg.insert("maxHealth".into(), json!(100 + (new_level - 1) * 10));
        rpg.insert("maxMana".into(), json!(100 + (new_level - 1) * 5));
        rpg.insert("maxStamina".into(), json!(100 + (new_level - 1) * 5));
        rpg.insert("health".into(), json!(100 + (new_level - 1) * 10));
        rpg.insert("mana".into(), json!(100 + (new_level - 1) * 5));
        rpg.insert("stamina".into(), json!(100 + (new_level - 1) * 5));
    }

    Reply::text(format!(
        "✅ Berhasil menambahkan exp *{}* ke *@{}*",
        format_exp(amount),
        jid_number(&target)
    ))
    .with_reaction("✅")
    .with_mutation(Mutation::SetUser {
        jid: ctx.sender.clone(),
        data: json!({ "exp": new_exp, "rpg": rpg }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedKind;
    use serde_json::Value;

    const SENDER: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    fn run(text: &str, user: Value, mentioned: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addexp".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("addexp").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> (String, i64) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert!(data.get("rpg").is_some(), "rpg ikut ditulis");
                (jid.clone(), data["exp"].as_i64().expect("exp angka"))
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    fn written_rpg(r: &Reply) -> Value {
        match &r.mutations()[0] {
            Mutation::SetUser { data, .. } => data["rpg"].clone(),
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addexp").unwrap();
        assert_eq!(p.meta.name, "addexp");
        assert_eq!(p.meta.alias, ["tambahexp", "giveexp", "addxp"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Tambah exp user (max 9 Miliar)");
        assert_eq!(p.meta.usage, ".addexp <jumlah> @user");
        assert_eq!(p.meta.example, ".addexp 10000 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tambahexp", "giveexp", "addxp"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addexp");
        }
    }

    #[test]
    fn no_amount_shows_help_and_writes_nothing() {
        for msg in ["", "@628999", "abc"] {
            let r = run(msg, json!({}), Some(OTHER));
            assert!(text_of(&r).starts_with("⭐ *ᴀᴅᴅ ᴇxᴘ*"), "input {msg:?}");
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn a_mention_without_an_amount_is_still_help() {
        let r = run("0", json!({}), Some(OTHER));
        assert!(text_of(&r).starts_with("⭐ *ᴀᴅᴅ ᴇxᴘ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_bare_amount_writes_to_the_sender() {
        let r = run("10000", json!({ "exp": 5_000 }), None);
        assert_eq!(written(&r), (SENDER.to_string(), 15_000));
        assert!(text_of(&r).contains("*10.00K*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("ke *@628111*"), "{}", text_of(&r));
    }

    #[test]
    fn a_quote_supplies_the_target_for_the_message_only() {
        let ctx = Ctx {
            quoted: Some(oktz_plugin_api::QuotedMessage {
                id: "q".into(),
                sender: OTHER.into(),
                chat: SENDER.into(),
                text: String::new(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            text: "250".into(),
            args: vec!["250".into()],
            sender: SENDER.into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("addexp").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("ke *@628999*"), "{}", text_of(&r));
        // JS tetap menulis ke `m.sender` (level.js:234) — dijaga supaya
        // tidak "dikoreksi" tanpa sengaja.
        assert_eq!(written(&r).0, SENDER);
    }

    #[test]
    fn the_cap_is_nine_billion_and_is_clamped_before_writing() {
        let r = run("99999999999", json!({ "exp": 0 }), None);
        assert_eq!(written(&r).1, MAX_EXP);
        assert!(text_of(&r).contains("*9.00B*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unlimited_exp_user_keeps_minus_one() {
        // `user.exp === -1` → `newExp = oldExp`, jadi tidak naik.
        let r = run("500", json!({ "exp": -1 }), None);
        assert_eq!(written(&r).1, -1);
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
    fn exp_format_switches_unit_at_each_threshold() {
        assert_eq!(format_exp(999), "999");
        assert_eq!(format_exp(1_000), "1.00K");
        assert_eq!(format_exp(1_500_000), "1.50M");
        assert_eq!(format_exp(2_000_000_000), "2.00B");
    }

    #[test]
    fn rpg_stats_are_written_only_when_the_level_actually_rises() {
        // exp 9.999 → 10.009 lewat 10.000: level 1 → 2, jadi stats ikut.
        let up = run("10", json!({ "exp": 9_999 }), None);
        let rpg = written_rpg(&up);
        assert_eq!(rpg["level"], json!(2));
        assert_eq!(rpg["maxHealth"], json!(110));
        assert_eq!(rpg["maxMana"], json!(105));
        assert_eq!(rpg["maxStamina"], json!(105));
        assert_eq!(rpg["health"], json!(110));
        assert_eq!(rpg["mana"], json!(105));
        assert_eq!(rpg["stamina"], json!(105));

        // Masih level 1 → `rpg` tetap object kosong, tanpa field level.
        let flat = run("10", json!({ "exp": 0 }), None);
        assert_eq!(written_rpg(&flat), json!({}));
    }

    #[test]
    fn existing_rpg_fields_survive_the_write() {
        let r = run("1", json!({ "exp": 0, "rpg": { "gold": 12 } }), None);
        let rpg = written_rpg(&r);
        assert_eq!(rpg["gold"], json!(12), "field lain tidak boleh hilang");
    }
}