//! `plugins/owner/addpremall.js` — jadikan semua member grup premium.
//!
//! `db.data.premium` adalah store terpisah (`premium.json`), bukan
//! `db.setting`. `Ctx` tidak membawanya dan `Mutation` tidak punya operasi
//! untuk menuliskannya — jadi daftar premium global TIDAK bisa diubah dari
//! plugin ini. Yang ditulis: flag `isPremium` + energi per user, persis
//! bagian `db.setUser`/`updateExp`/`updateKoin` di JS.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

/// `config.limits?.premium || 100` — `config.limits` tidak ada di `config.js`.
const PREMIUM_LIMIT: i64 = 100;

pub(crate) const NO_MEMBER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ada member di grup ini";

fn report(total: usize, added: usize, already: usize, premium_total: usize, subject: &str) -> String {
    format!(
        "💎 *ᴀᴅᴅ ᴘʀᴇᴍɪᴜᴍ ᴀʟʟ*\n\n\
         ╭┈┈⬡「 📋 *ʜᴀsɪʟ* 」\n\
         ┃ 👥 ᴛᴏᴛᴀʟ ᴍᴇᴍʙᴇʀ: `{total}`\n\
         ┃ ✅ ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ: `{added}`\n\
         ┃ ⏭️ sᴜᴅᴀʜ ᴘʀᴇᴍɪᴜᴍ: `{already}`\n\
         ┃ 💎 ᴛᴏᴛᴀʟ ᴘʀᴇᴍɪᴜᴍ: `{premium_total}`\n\
         ╰┈┈⬡\n\n\
         > Grup: {subject}"
    )
}

#[plugin(
    name = "addpremall",
    alias = ["addpremiumall", "setpremall"],
    category = "owner",
    description = "Menambahkan semua member grup ke premium",
    usage = ".addprem all",
    example = ".addprem all",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn addpremall(ctx: Ctx) -> Reply {
    if ctx.roster.is_empty() {
        return Reply::text(NO_MEMBER);
    }

    let total = ctx.roster.participants.len();
    let mut writes = Vec::new();
    let (mut added, mut already) = (0usize, 0usize);

    for p in &ctx.roster.participants {
        let digits: String = p.jid.chars().filter(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        let jid = format!("{digits}@s.whatsapp.net");
        if p.data.get("isPremium").and_then(serde_json::Value::as_bool) == Some(true) {
            already += 1;
            continue;
        }
        let exp = p
            .data
            .get("exp")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let koin = p
            .data
            .get("koin")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        writes.push(Mutation::SetUser {
            jid,
            data: json!({
                "energi": PREMIUM_LIMIT,
                "isPremium": true,
                "exp": (exp + 200_000).clamp(0, 9_000_000_000),
                "koin": (koin + 20_000).clamp(0, 9_000_000_000_000),
            }),
        });
        added += 1;
    }

    // `db.data.premium.length` — store premium global tidak ada di `Mutation`,
    // jadi total di sini = member grup yang sudah premium (added + already).
    let premium_total = added + already;
    Reply::text(report(
        total,
        added,
        already,
        premium_total,
        ctx.group_field("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Grup"),
    ))
    .with_reaction("🕕")
    .with_reaction("💎")
    .mutating(writes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Participant, registry::GroupRoster};
    use serde_json::Value;

    fn member(jid: &str, data: Value) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: false,
            is_owner: false,
            data,
        }
    }

    fn run(roster: GroupRoster) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addprem".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: json!({ "subject": "Grup Uji" }),
            roster,
            ..Default::default()
        };
        (oktz_plugin_api::find("addpremall").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn writes(r: &Reply) -> Vec<(String, Value)> {
        r.mutations()
            .iter()
            .map(|m| match m {
                Mutation::SetUser { jid, data } => (jid.clone(), data.clone()),
                other => panic!("harus SetUser, dapat {other:?}"),
            })
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("addpremall").unwrap();
        assert_eq!(p.meta.name, "addpremall");
        assert_eq!(p.meta.alias, ["addpremiumall", "setpremall"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Menambahkan semua member grup ke premium"
        );
        assert_eq!(p.meta.usage, ".addprem all");
        assert_eq!(p.meta.example, ".addprem all");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner && p.meta.is_group);
        assert!(!p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["addpremiumall", "setpremall"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addpremall");
        }
    }

    #[test]
    fn an_empty_roster_is_refused_and_writes_nothing() {
        let r = run(GroupRoster::default());
        assert_eq!(text_of(&r), NO_MEMBER);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_fresh_member_gets_the_full_premium_package() {
        let roster = GroupRoster {
            participants: vec![member("628111@s.whatsapp.net", json!({ "exp": 0, "koin": 0 }))],
            is_bot_admin: false,
        };
        let w = writes(&run(roster));
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0, "628111@s.whatsapp.net");
        assert_eq!(w[0].1["energi"], json!(100));
        assert_eq!(w[0].1["isPremium"], json!(true));
        assert_eq!(w[0].1["exp"], json!(200_000));
        assert_eq!(w[0].1["koin"], json!(20_000));
    }

    #[test]
    fn bonuses_are_added_on_top_of_the_existing_balance() {
        let roster = GroupRoster {
            participants: vec![member(
                "628111@s.whatsapp.net",
                json!({ "exp": 1_000, "koin": 500 }),
            )],
            is_bot_admin: false,
        };
        let w = writes(&run(roster));
        assert_eq!(w[0].1["exp"], json!(201_000));
        assert_eq!(w[0].1["koin"], json!(20_500));
    }

    #[test]
    fn an_already_premium_member_is_counted_and_not_rewritten() {
        let roster = GroupRoster {
            participants: vec![
                member("1@s.whatsapp.net", json!({ "isPremium": true })),
                member("2@s.whatsapp.net", json!({})),
            ],
            is_bot_admin: false,
        };
        let r = run(roster);
        assert_eq!(writes(&r).len(), 1);
        assert!(text_of(&r).contains("┃ ⏭️ sᴜᴅᴀʜ ᴘʀᴇᴍɪᴜᴍ: `1`"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ ✅ ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ: `1`"), "{}", text_of(&r));
    }

    #[test]
    fn a_participant_without_digits_is_neither_written_nor_counted_as_premium() {
        let roster = GroupRoster {
            participants: vec![member("@lid", json!({}))],
            is_bot_admin: false,
        };
        let r = run(roster);
        assert!(r.mutations().is_empty());
        assert!(text_of(&r).contains("┃ ✅ ᴅɪᴛᴀᴍʙᴀʜᴋᴀɴ: `0`"), "{}", text_of(&r));
    }

    #[test]
    fn the_report_names_the_group_from_group_data() {
        let roster = GroupRoster {
            participants: vec![member("1@s.whatsapp.net", json!({}))],
            is_bot_admin: false,
        };
        assert!(text_of(&run(roster)).contains("> Grup: Grup Uji"));
    }

    #[test]
    fn the_final_reaction_is_the_one_from_the_js_order() {
        let roster = GroupRoster {
            participants: vec![member("1@s.whatsapp.net", json!({}))],
            is_bot_admin: false,
        };
        match run(roster).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "💎"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}