//! `plugins/owner/delpremall.js` — lepas premium semua member grup.
//!
//! `db.data.premium` adalah store terpisah (`premium.json`), bukan `db.setting`,
//! dan `Ctx` tidak membawanya — sama seperti pada `addpremall.rs`. Yang dipakai
//! di sini adalah flag `isPremium` per user (`premium-db.js:isPremium` +
//! `database.js:202`), jadi "sudah premium" = record user-nya punya flag itu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const NO_MEMBER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ada member di grup ini";

fn report(total: usize, removed: usize, not_prem: usize, premium_total: usize, subject: &str) -> String {
    format!(
        "🗑️ *ᴅᴇʟ ᴘʀᴇᴍɪᴜᴍ ᴀʟʟ*\n\n\
         ╭┈┈⬡「 📋 *ʜᴀsɪʟ* 」\n\
         ┃ 👥 ᴛᴏᴛᴀʟ ᴍᴇᴍʙᴇʀ: `{total}`\n\
         ┃ ✅ ᴅɪʜᴀᴘᴜs: `{removed}`\n\
         ┃ ⏭️ ʙᴜᴋᴀɴ ᴘʀᴇᴍɪᴜᴍ: `{not_prem}`\n\
         ┃ 💎 sɪsᴀ ᴘʀᴇᴍɪᴜᴍ: `{premium_total}`\n\
         ╰┈┈⬡\n\n\
         > Grup: {subject}"
    )
}

#[plugin(
    name = "delpremall",
    alias = ["delpremiumall", "removepremall"],
    category = "owner",
    description = "Menghapus semua member grup dari premium",
    usage = ".delprem all",
    example = ".delprem all",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn delpremall(ctx: Ctx) -> Reply {
    if ctx.roster.is_empty() {
        return Reply::text(NO_MEMBER);
    }

    let total = ctx.roster.participants.len();
    let mut writes = Vec::new();
    let (mut removed, mut not_prem) = (0usize, 0usize);

    for p in &ctx.roster.participants {
        let digits: String = p.jid.chars().filter(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        if !crate::common::js_truthy(p.data.get("isPremium")) {
            not_prem += 1;
            continue;
        }
        // `db.getUser(jid)` = `null` kalau user belum pernah ada — JS tetap
        // menaikkan `removedCount`, hanya penulisan flag-nya yang dilewati.
        if !p.data.is_null() {
            writes.push(Mutation::SetUser {
                jid: format!("{digits}@s.whatsapp.net"),
                data: serde_json::json!({ "isPremium": false }),
            });
        }
        removed += 1;
    }

    // `db.data.premium.length` — store premium global tidak ada di `Mutation`,
    // jadi sisa di sini = member grup yang masih premium di store user.
    Reply::text(report(
        total,
        removed,
        not_prem,
        total - not_prem,
        ctx.group_field("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Grup"),
    ))
    .with_reaction("🕕")
    .with_reaction("🗑️")
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

    fn roster(members: Vec<Participant>) -> GroupRoster {
        GroupRoster {
            participants: members,
            is_bot_admin: false,
        }
    }

    fn run(r: GroupRoster) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "delprem".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: serde_json::json!({ "subject": "Grup Uji" }),
            roster: r,
            ..Default::default()
        };
        (oktz_plugin_api::find("delpremall").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("delpremall").unwrap();
        assert_eq!(p.meta.name, "delpremall");
        assert_eq!(p.meta.alias, ["delpremiumall", "removepremall"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Menghapus semua member grup dari premium"
        );
        assert_eq!(p.meta.usage, ".delprem all");
        assert_eq!(p.meta.example, ".delprem all");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner && p.meta.is_group);
        assert!(!p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delpremiumall", "removepremall"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "delpremall");
        }
    }

    #[test]
    fn an_empty_roster_is_refused_and_writes_nothing() {
        let r = run(roster(vec![]));
        assert_eq!(text_of(&r), NO_MEMBER);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_premium_member_loses_the_flag_and_is_counted_removed() {
        let r = run(roster(vec![member(
            "628111@s.whatsapp.net",
            serde_json::json!({ "isPremium": true }),
        )]));
        let w = writes(&r);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0, "628111@s.whatsapp.net");
        assert_eq!(w[0].1["isPremium"], serde_json::json!(false));
        assert!(text_of(&r).contains("┃ ✅ ᴅɪʜᴀᴘᴜs: `1`"), "{}", text_of(&r));
        assert!(
            text_of(&r).contains("┃ ⏭️ ʙᴜᴋᴀɴ ᴘʀᴇᴍɪᴜᴍ: `0`"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("┃ 💎 sɪsᴀ ᴘʀᴇᴍɪᴜᴍ: `1`"), "{}", text_of(&r));
    }

    #[test]
    fn a_non_premium_member_is_counted_but_never_written() {
        let r = run(roster(vec![
            member("628111@s.whatsapp.net", serde_json::json!({ "isPremium": true })),
            member("628222@s.whatsapp.net", serde_json::json!({ "isPremium": false })),
        ]));
        assert_eq!(writes(&r).len(), 1, "hanya member premium yang ditulis");
        assert!(text_of(&r).contains("┃ ✅ ᴅɪʜᴀᴘᴜs: `1`"), "{}", text_of(&r));
        assert!(
            text_of(&r).contains("┃ ⏭️ ʙᴜᴋᴀɴ ᴘʀᴇᴍɪᴜᴍ: `1`"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn a_member_with_no_stored_record_counts_as_not_premium() {
        // JS bisa punya nomor di `premium.json` tanpa record di `users.json`
        // dan tetap menghitungnya terhapus; dengan flag `isPremium` sebagai
        // pengganti, record yang tidak ada otomatis berarti bukan premium.
        let r = run(roster(vec![member(
            "628111@s.whatsapp.net",
            Value::Null,
        )]));
        assert!(r.mutations().is_empty());
        assert!(
            text_of(&r).contains("┃ ⏭️ ʙᴜᴋᴀɴ ᴘʀᴇᴍɪᴜᴍ: `1`"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn a_participant_without_digits_is_skipped_entirely() {
        let r = run(roster(vec![member("@lid", serde_json::json!({ "isPremium": true }))]));
        assert!(r.mutations().is_empty());
        assert!(text_of(&r).contains("┃ 👥 ᴛᴏᴛᴀʟ ᴍᴇᴍʙᴇʀ: `1`"), "{}", text_of(&r));
        assert!(
            text_of(&r).contains("┃ ✅ ᴅɪʜᴀᴘᴜs: `0`"),
            "{}",
            text_of(&r)
        );
        assert!(
            text_of(&r).contains("┃ ⏭️ ʙᴜᴋᴀɴ ᴘʀᴇᴍɪᴜᴍ: `0`"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn the_jid_suffix_is_stripped_the_way_the_js_regex_did() {
        let r = run(roster(vec![member(
            "628111:12@s.whatsapp.net",
            serde_json::json!({ "isPremium": true }),
        )]));
        assert_eq!(writes(&r)[0].0, "62811112@s.whatsapp.net");
    }

    #[test]
    fn the_report_names_the_group_and_falls_back_when_absent() {
        let r = run(roster(vec![member("1@s.whatsapp.net", serde_json::json!({}))]));
        assert!(text_of(&r).contains("> Grup: Grup Uji"));
        let ctx = Ctx {
            prefix: ".".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: roster(vec![member("1@s.whatsapp.net", serde_json::json!({}))]),
            ..Default::default()
        };
        let bare = (oktz_plugin_api::find("delpremall").unwrap().handler)(ctx);
        assert!(text_of(&bare).contains("> Grup: Grup"), "{}", text_of(&bare));
    }

    #[test]
    fn the_final_reaction_is_the_delete_one_from_the_js_order() {
        let r = run(roster(vec![member("1@s.whatsapp.net", serde_json::json!({}))]));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "🗑️"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}
