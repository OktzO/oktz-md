//! `plugins/group/unmutemember.js` — buang satu JID dari
//! `groups[chat].mutedMembers`.
//!
//! `resolveTarget` dan pencocokan nomor yang dipakai di sini datang dari
//! [`crate::group::mutemember`] — di JS keduanya fungsi yang sama, disalin ke
//! dua file.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::jid_number;
use crate::group::mutemember::{muted_members, resolve_target, same_number};

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "🔊 *UNMUTE MEMBER*\n\n\
         > Membuka mute member tertentu\n\n\
         `Contoh:`\n\
         > {prefix}unmutemember @user\n\
         > {prefix}unmutemember 6281234567890\n\
         > Reply pesan member + {prefix}unmutemember"
    )
}

pub(crate) fn not_muted(target_number: &str) -> String {
    format!("❌ *ɢᴀɢᴀʟ*\n\n> Member @{target_number} tidak sedang dimute")
}

pub(crate) fn unmuted_detail(target_number: &str, left: usize) -> String {
    format!(
        "🔊 *MEMBER DIUNMUTE*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 👤 ᴍᴇᴍʙᴇʀ: @{target_number}\n\
         ┃ 🔊 sᴛᴀᴛᴜs: `Unmuted`\n\
         ┃ 📊 sɪsᴀ ᴍᴜᴛᴇ: `{left}` ᴍᴇᴍʙᴇʀ\n\
         ╰┈┈⬡"
    )
}

#[plugin(
    name = "unmutemember",
    alias = [
        "unmutmember",
        "unsilentmember",
        "unbisukanmember",
        "listmutemember",
        "listmute"
    ],
    category = "group",
    description = "Membuka mute member tertentu",
    usage = ".unmutemember <@tag/reply/nomor>",
    example = ".unmutemember @user",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn unmutemember(ctx: Ctx) -> Reply {
    let list = muted_members(&ctx);

    // `listmutemember`/`listmute` adalah alias yang jatuh ke cabang daftar,
    // bukan perintah daftar tersendiri.
    if ctx.command == "listmutemember" || ctx.command == "listmute" {
        if list.is_empty() {
            return Reply::text("🔇 *LIST MUTED MEMBERS*\n\n> Tidak ada member yang dimute di grup ini");
        }
        let mut txt = String::from("🔇 *LIST MUTED MEMBERS*\n\n╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ* 」\n");
        for (i, jid) in list.iter().enumerate() {
            txt.push_str(&format!("┃ {}. @{}\n", i + 1, jid_number(jid)));
        }
        txt.push_str(&format!(
            "╰┈┈⬡\n\n> Total: `{}` member dimute",
            list.len()
        ));
        return Reply::Text {
            text: txt,
            mentions: list,
        };
    }

    let target = resolve_target(&ctx);
    if target.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }

    let target_number = jid_number(&target).to_string();
    let Some(index) = list
        .iter()
        .position(|jid| same_number(jid_number(jid), &target_number))
    else {
        return Reply::Text {
            text: not_muted(&target_number),
            mentions: vec![target],
        };
    };

    let mut next = list;
    next.remove(index);
    let left = next.len();
    Reply::Text {
        text: unmuted_detail(&target_number, left),
        mentions: vec![target],
    }
    .with_reaction("🔊")
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({
            "mutedMembers": next.into_iter().map(Value::String).collect::<Vec<_>>()
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(command: &str, group: Value, args: &[&str], mentioned: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            mentioned: mentioned.iter().map(|s| s.to_string()).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("unmutemember").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Reply sukses dibungkus `Reaction`; mention-nya ada di reply dalam.
    fn mentions_of(r: &Reply) -> Vec<String> {
        let mut cur = r.without_mutations();
        while let Reply::Reaction { reply, .. } = cur {
            cur = reply.without_mutations();
        }
        match cur {
            Reply::Text { mentions, .. } => mentions.clone(),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn group_write(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data["mutedMembers"].clone()
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("unmutemember").unwrap();
        assert_eq!(p.meta.name, "unmutemember");
        assert_eq!(
            p.meta.alias,
            [
                "unmutmember",
                "unsilentmember",
                "unbisukanmember",
                "listmutemember",
                "listmute"
            ]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Membuka mute member tertentu");
        assert_eq!(p.meta.usage, ".unmutemember <@tag/reply/nomor>");
        assert_eq!(p.meta.example, ".unmutemember @user");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in [
            "unmutmember",
            "unsilentmember",
            "unbisukanmember",
            "listmutemember",
            "listmute",
        ] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "unmutemember");
        }
    }

    #[test]
    fn without_a_target_the_plugin_prints_the_usage() {
        let r = run("unmutemember", json!({}), &[], &[]);
        assert_eq!(
            text_of(&r),
            "🔊 *UNMUTE MEMBER*\n\n\
             > Membuka mute member tertentu\n\n\
             `Contoh:`\n\
             > .unmutemember @user\n\
             > .unmutemember 6281234567890\n\
             > Reply pesan member + .unmutemember"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_muted_member_is_removed_and_the_remainder_is_written_back() {
        let r = run(
            "unmutemember",
            json!({"mutedMembers": ["628222@s.whatsapp.net", "628333@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        );
        assert_eq!(group_write(&r), json!(["628222@s.whatsapp.net"]));
        assert_eq!(
            text_of(&r),
            "🔊 *MEMBER DIUNMUTE*\n\n\
             ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
             ┃ 👤 ᴍᴇᴍʙᴇʀ: @628333\n\
             ┃ 🔊 sᴛᴀᴛᴜs: `Unmuted`\n\
             ┃ 📊 sɪsᴀ ᴍᴜᴛᴇ: `1` ᴍᴇᴍʙᴇʀ\n\
             ╰┈┈⬡"
        );
        assert_eq!(mentions_of(&r), ["628333@s.whatsapp.net"]);
    }

    #[test]
    fn the_success_branch_reacts_with_a_speaker() {
        match run(
            "unmutemember",
            json!({"mutedMembers": ["628333@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        )
        .without_mutations()
        {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "🔊"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn only_the_first_matching_entry_is_removed() {
        // `findIndex` di JS — duplikat nama tidak dibersihkan semua.
        let r = run(
            "unmutemember",
            json!({"mutedMembers": ["628333@s.whatsapp.net", "628333:1@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        );
        assert_eq!(group_write(&r), json!(["628333:1@s.whatsapp.net"]));
    }

    #[test]
    fn a_member_who_is_not_muted_is_reported_without_writing() {
        for stored in [json!({}), json!({"mutedMembers": []})] {
            let r = run("unmutemember", stored.clone(), &[], &["628333@s.whatsapp.net"]);
            assert_eq!(text_of(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Member @628333 tidak sedang dimute", "input {stored}");
            assert_eq!(mentions_of(&r), ["628333@s.whatsapp.net"]);
            assert!(r.mutations().is_empty(), "input {stored}");
        }
    }

    #[test]
    fn the_list_aliases_print_the_table_and_never_write() {
        let r = run(
            "listmute",
            json!({"mutedMembers": ["628222@s.whatsapp.net", "628333:1@s.whatsapp.net"]}),
            &[],
            &[],
        );
        assert_eq!(
            text_of(&r),
            "🔇 *LIST MUTED MEMBERS*\n\n╭┈┈⬡「 📋 *ᴅᴀꜰᴛᴀʀ* 」\n\
             ┃ 1. @628222\n\
             ┃ 2. @628333:1\n\
             ╰┈┈⬡\n\n\
             > Total: `2` member dimute"
        );
        assert_eq!(mentions_of(&r), ["628222@s.whatsapp.net", "628333:1@s.whatsapp.net"]);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_list_branch_ignores_a_present_target() {
        // JS mengecek `m.command` DULUAN, sebelum `resolveTarget`.
        let r = run(
            "listmutemember",
            json!({"mutedMembers": ["628222@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        );
        assert!(text_of(&r).contains("LIST MUTED MEMBERS"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_empty_list_says_there_is_nobody_muted() {
        let r = run("listmutemember", json!({"mutedMembers": []}), &[], &[]);
        assert_eq!(
            text_of(&r),
            "🔇 *LIST MUTED MEMBERS*\n\n> Tidak ada member yang dimute di grup ini"
        );
        assert!(mentions_of(&r).is_empty());
        assert!(r.mutations().is_empty());
    }
}