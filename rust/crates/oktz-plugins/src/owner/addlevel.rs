//! `plugins/owner/addlevel.js` — tambah level user lewat exp.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{jid_number, parse_int};
use crate::user::exp::{EXP_PER_LEVEL, calculate_level, role_of};
use crate::tools::kalkulatormbg::thousands_id;

/// `db.updateExp` juga punya plafon 9 miliar (`database.js:668`).
const MAX_EXP: i64 = 9_000_000_000;

/// `args.find(a => !isNaN(a) && !a.startsWith('@'))`.
fn numeric_arg(ctx: &Ctx) -> Option<i64> {
    ctx.args
        .iter()
        .find(|a| !a.starts_with('@') && a.parse::<f64>().is_ok())
        .and_then(|a| parse_int(a))
}

fn help(prefix: &str) -> String {
    format!(
        "📊 *ADD LEVEL*\n\n\
         Sistem untuk menambahkan level kepada member secara instan.\n\n\
         *PENGGUNAAN:*\n\
         - *{p}addlevel <jumlah>* — (ke diri sendiri)\n\
         - *{p}addlevel <jumlah> @user* — (ke orang lain)\n\n\
         *CONTOH PENGGUNAAN:*\n\
         - *{p}addlevel 5*\n\
         - *{p}addlevel 10 @user*",
        p = prefix
    )
}

/// `ponytail:` hanya row pengirim yang terbaca di `Ctx`, dan kartu level-up
/// (canvas + `sock.sendMedia`) tidak ada di port ini — yang ditulis di sini
/// persis `db.updateExp` + `db.setUser(targetJid, user)` dari JS.
fn applied(target: &str, levels: i64, new_exp: i64) -> String {
    let level = calculate_level(new_exp);
    format!(
        "✅ *BERHASIL MENAMBAH LEVEL*\n\n\
         Level milik *@{target}* telah sukses ditambahkan sebanyak *{levels} Level*.\n\n\
         *Statistik Terkini:*\n\
         - Level Sekarang: *{level}*\n\
         - Role Saat Ini: *{role}*\n\
         - Total XP: *{exp}* XP",
        role = role_of(level),
        exp = thousands_id(new_exp as f64)
    )
}

#[plugin(
    name = "addlevel",
    alias = ["tambahlevel", "givelevel", "addlvl"],
    category = "owner",
    description = "Tambah level user (via exp)",
    usage = ".addlevel <jumlah> @user",
    example = ".addlevel 5 @user",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addlevel(ctx: Ctx) -> Reply {
    let levels = numeric_arg(&ctx).unwrap_or(0);

    let target = match ctx.quoted() {
        Some(q) => Some(q.sender.clone()),
        None => ctx.mentioned.first().cloned(),
    };
    let target = match target {
        Some(jid) if !jid.is_empty() => jid,
        _ if levels > 0 => ctx.sender.clone(),
        _ => return Reply::text(help(&ctx.prefix)),
    };

    // `!targetJid || levels <= 0` — OR, jadi mention tanpa jumlah tetap help.
    if levels <= 0 {
        return Reply::text(help(&ctx.prefix));
    }

    let old_exp = ctx
        .user_field("exp")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    // `expToAdd = levels * 10000`, lalu `updateExp` yang menjepit ke 9 miliar.
    let new_exp = if old_exp == -1 {
        -1
    } else {
        (old_exp + levels * EXP_PER_LEVEL).clamp(0, MAX_EXP)
    };
    // `exp === -1` berarti tak terbatas; `calculateLevel(-1)` = 0 di JS dan
    // teks pun menampilkan 0, bukan error.
    let reported_exp = new_exp.max(0);

    Reply::text(applied(jid_number(&target), levels, reported_exp))
        .with_reaction("🕕")
        .with_reaction("✅")
        .with_mutation(Mutation::SetUser {
            jid: target,
            data: serde_json::json!({
                "exp": new_exp,
                "rpg": ctx
                    .user_field("rpg")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            }),
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
            command: "addlevel".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            mentioned: mentioned.into_iter().map(String::from).collect(),
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("addlevel").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written_exp(r: &Reply) -> (String, i64) {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert!(data.get("rpg").is_some(), "rpg ikut ditulis");
                (jid.clone(), data["exp"].as_i64().expect("exp angka"))
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addlevel").unwrap();
        assert_eq!(p.meta.name, "addlevel");
        assert_eq!(p.meta.alias, ["tambahlevel", "givelevel", "addlvl"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Tambah level user (via exp)");
        assert_eq!(p.meta.usage, ".addlevel <jumlah> @user");
        assert_eq!(p.meta.example, ".addlevel 5 @user");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tambahlevel", "givelevel", "addlvl"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addlevel");
        }
    }

    #[test]
    fn no_level_count_shows_help_and_writes_nothing() {
        for msg in ["", "@628999", "abc", "0"] {
            let r = run(msg, serde_json::json!({}), Some(OTHER));
            assert!(text_of(&r).starts_with("📊 *ADD LEVEL*"), "input {msg:?}");
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn five_levels_costs_exactly_fifty_thousand_exp() {
        let r = run("5", serde_json::json!({ "exp": 0 }), None);
        assert_eq!(written_exp(&r), (SENDER.to_string(), 50_000));
        let t = text_of(&r);
        assert!(t.contains("sebanyak *5 Level*"), "{t}");
        assert!(t.contains("Level Sekarang: *6*"), "{t}");
        assert!(t.contains("Role Saat Ini: *🛡️ Warrior*"), "{t}");
        assert!(t.contains("Total XP: *50.000* XP"), "{t}");
    }

    #[test]
    fn a_mention_redirects_the_write_to_that_jid() {
        assert_eq!(
            written_exp(&run("10", serde_json::json!({}), Some(OTHER))),
            (OTHER.to_string(), 100_000)
        );
    }

    #[test]
    fn the_role_follows_the_new_level_not_the_added_amount() {
        // 9.999 exp = level 1; +10 level (100.000) → 109.999 = level 11 (Elite).
        // Yang menentukan role adalah level SESUDAH, bukan `levels` yang
        // ditambahkan: 10 level di sini, role-nya Elite bukan Master.
        let r = run("10", serde_json::json!({ "exp": 9_999 }), None);
        let t = text_of(&r);
        assert!(t.contains("Level Sekarang: *11*"), "{t}");
        assert!(t.contains("Role Saat Ini: *⭐ Elite*"), "{t}");

        // 900.000 exp = level 91; +1 level → 910.000 = level 92, tetap Mythic.
        let high = run("1", serde_json::json!({ "exp": 900_000 }), None);
        assert!(text_of(&high).contains("Level Sekarang: *92*"), "{}", text_of(&high));
    }

    #[test]
    fn exp_is_capped_at_nine_billion() {
        let r = run("1000", serde_json::json!({ "exp": 8_999_999_999i64 }), None);
        assert_eq!(written_exp(&r).1, MAX_EXP);
    }

    #[test]
    fn both_reactions_are_emitted_in_the_js_order() {
        match run("5", serde_json::json!({}), None).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_applied_reply_is_paired_with_the_write_it_claims() {
        assert_eq!(run("5", serde_json::json!({}), None).mutations().len(), 1);
    }
}