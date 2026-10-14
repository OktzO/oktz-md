//! `plugins/group/mutemember.js` — tambah JID ke `groups[chat].mutedMembers`.
//!
//! Teks meseinya dihapus `handler.js` lewat `isMutedMember()` yang diekspor
//! dari file ini di JS; yang diport di sini hanya perintahnya.
//!
//! `ponytail:` `lidToJid`/`findParticipantByNumber` di JS membaca
//! `lidCache` (pemetaan LID → nomor) yang tidak ada di `Ctx`. Yang dipakai di
//! sini cukup penggantian sufiks `@lid` dan pencocokan `jid` roster, yang
//! selalu berisi nomor telepon asli.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::{jid_number, string_list};

/// `isLid` (`lid.js:159`).
fn is_lid(jid: &str) -> bool {
    jid.ends_with("@lid")
}

/// `lidToJid` tanpa cache: `jid.replace("@lid", "@s.whatsapp.net")`.
fn lid_to_jid(jid: &str) -> String {
    jid.replacen("@lid", "@s.whatsapp.net", 1)
}

/// `raw.replace(/[^0-9]/g, '')` — hanya digit ASCII.
fn digits_only(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).collect()
}

/// `resolveTarget` — dipakai [`crate::group::unmutemember`] juga, jadi
/// bentuknya sama persis di kedua plugin.
pub(crate) fn resolve_target(ctx: &Ctx) -> String {
    let mut raw = String::new();
    if let Some(q) = ctx.quoted() {
        raw = q.sender.clone();
    } else if !ctx.mentioned.is_empty() {
        raw = ctx.mentioned[0].clone();
    } else if let Some(arg) = ctx.args.first() {
        raw = arg.clone();
    }
    if raw.is_empty() {
        return String::new();
    }

    if is_lid(&raw) {
        raw = lid_to_jid(&raw);
    }
    if !raw.contains('@') {
        raw = format!("{}@s.whatsapp.net", digits_only(&raw));
    }
    raw
}

/// `c === n || c.endsWith(n) || n.endsWith(c)` setelah kedua sisi dibuang
/// bagian `@...` — dipakai untuk mencocokkan nomor dengan/suffix, bukan JID
/// persis, karena grup bisa menyimpan nomor dengan/tanpa device.
pub(crate) fn same_number(a: &str, b: &str) -> bool {
    a == b || a.ends_with(b) || b.ends_with(a)
}

/// `mutedMembers || []`.
pub(crate) fn muted_members(ctx: &Ctx) -> Vec<String> {
    string_list(ctx.group_field("mutedMembers"))
}

/// `findParticipantByNumber(...)` — versi roster: `Participant.jid` sudah
/// nomor telepon, jadi cukup cocokkan bagian sebelum `@`.
fn is_target_admin(ctx: &Ctx, target_number: &str) -> bool {
    ctx.roster
        .participants
        .iter()
        .find(|p| jid_number(&p.jid) == target_number)
        .is_some_and(|p| p.is_admin)
}

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "🔇 *MUTE MEMBER*\n\n\
         > Bisukan member tertentu di grup ini\n\
         > Pesan member yang dimute akan dihapus oleh bot\n\n\
         `Contoh:`\n\
         > {prefix}mutemember @user\n\
         > {prefix}mutemember 6281234567890\n\
         > Reply pesan member + {prefix}mutemember"
    )
}

pub(crate) fn is_admin_error() -> String {
    "❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat mute admin grup".to_string()
}

pub(crate) fn already_muted(target_number: &str) -> String {
    format!("❌ *ɢᴀɢᴀʟ*\n\n> Member @{target_number} sudah dimute")
}

pub(crate) fn muted_detail(target_number: &str, total: usize, prefix: &str) -> String {
    format!(
        "🔇 *MEMBER DIMUTE*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 👤 ᴍᴇᴍʙᴇʀ: @{target_number}\n\
         ┃ 🔇 sᴛᴀᴛᴜs: `Muted`\n\
         ┃ 📊 ᴛᴏᴛᴀʟ ᴍᴜᴛᴇ: `{total}` ᴍᴇᴍʙᴇʀ\n\
         ╰┈┈⬡\n\n\
         > Semua pesan dari member ini akan dihapus otomatis\n\
         > Gunakan `{prefix}unmutemember` untuk unmute"
    )
}

#[plugin(
    name = "mutemember",
    alias = ["mutmember", "silentmember", "bisukanmember"],
    category = "group",
    description = "Bisukan member tertentu (pesan akan dihapus bot)",
    usage = ".mutemember <@tag/reply/nomor>",
    example = ".mutemember @user",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn mutemember(ctx: Ctx) -> Reply {
    let target = resolve_target(&ctx);
    if target.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }

    let target_number = jid_number(&target).to_string();

    if ctx.is_group && is_target_admin(&ctx, &target_number) {
        return Reply::text(is_admin_error());
    }

    let mut list = muted_members(&ctx);
    if list.iter().any(|jid| same_number(jid_number(jid), &target_number)) {
        return Reply::Text {
            text: already_muted(&target_number),
            mentions: vec![target],
        };
    }

    list.push(target.clone());
    let total = list.len();
    Reply::Text {
        text: muted_detail(&target_number, total, &ctx.prefix),
        mentions: vec![target],
    }
    .with_reaction("🔇")
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({
            "mutedMembers": list.into_iter().map(Value::String).collect::<Vec<_>>()
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::{Participant, QuotedMessage};
    use oktz_plugin_api::registry::GroupRoster;
    use serde_json::json;

    const ADMIN: &str = "628111@s.whatsapp.net";

    fn roster(jid: &str, admin: bool) -> GroupRoster {
        GroupRoster {
            participants: vec![Participant {
                jid: jid.into(),
                is_admin: admin,
                is_owner: false,
                data: Value::Null,
            }],
            is_bot_admin: true,
        }
    }

    fn run(group: Value, args: &[&str], mentioned: &[&str], quoted: Option<&str>, r: GroupRoster) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mutemember".into(),
            sender: ADMIN.into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            mentioned: mentioned.iter().map(|s| s.to_string()).collect(),
            quoted: quoted.map(|sender| QuotedMessage {
                id: "MSG1".into(),
                sender: sender.into(),
                chat: "120363@g.us".into(),
                text: String::new(),
                kind: oktz_plugin_api::QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            group_data: group,
            roster: r,
            ..Default::default()
        };
        (oktz_plugin_api::find("mutemember").unwrap().handler)(ctx)
    }

    fn plain(group: Value, args: &[&str], mentioned: &[&str]) -> Reply {
        run(group.clone(), args, mentioned, None, GroupRoster::default())
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
        let p = oktz_plugin_api::find("mutemember").unwrap();
        assert_eq!(p.meta.name, "mutemember");
        assert_eq!(p.meta.alias, ["mutmember", "silentmember", "bisukanmember"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Bisukan member tertentu (pesan akan dihapus bot)");
        assert_eq!(p.meta.usage, ".mutemember <@tag/reply/nomor>");
        assert_eq!(p.meta.example, ".mutemember @user");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mutmember", "silentmember", "bisukanmember"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mutemember");
        }
    }

    #[test]
    fn no_target_at_all_prints_the_usage_and_writes_nothing() {
        let r = plain(json!({}), &[], &[]);
        assert_eq!(
            text_of(&r),
            "🔇 *MUTE MEMBER*\n\n\
             > Bisukan member tertentu di grup ini\n\
             > Pesan member yang dimute akan dihapus oleh bot\n\n\
             `Contoh:`\n\
             > .mutemember @user\n\
             > .mutemember 6281234567890\n\
             > Reply pesan member + .mutemember"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_mention_is_stored_verbatim_and_counted() {
        let r = plain(json!({}), &[], &["628333@s.whatsapp.net"]);
        assert_eq!(group_write(&r), json!(["628333@s.whatsapp.net"]));
        assert_eq!(
            text_of(&r),
            "🔇 *MEMBER DIMUTE*\n\n\
             ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
             ┃ 👤 ᴍᴇᴍʙᴇʀ: @628333\n\
             ┃ 🔇 sᴛᴀᴛᴜs: `Muted`\n\
             ┃ 📊 ᴛᴏᴛᴀʟ ᴍᴜᴛᴇ: `1` ᴍᴇᴍʙᴇʀ\n\
             ╰┈┈⬡\n\n\
             > Semua pesan dari member ini akan dihapus otomatis\n\
             > Gunakan `.unmutemember` untuk unmute"
        );
        assert_eq!(mentions_of(&r), ["628333@s.whatsapp.net"]);
    }

    #[test]
    fn the_success_branch_reacts_with_a_bell() {
        match plain(json!({}), &[], &["628333@s.whatsapp.net"]).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "🔇"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn a_bare_number_argument_gets_the_whatsapp_server_suffix() {
        let r = plain(json!({}), &["62-812-345"], &[]);
        assert_eq!(group_write(&r), json!(["62812345@s.whatsapp.net"]));
        assert!(text_of(&r).contains("👤 ᴍᴇᴍʙᴇʀ: @62812345"), "{}", text_of(&r));
    }

    #[test]
    fn a_leading_zero_number_is_not_rewritten_to_62_here() {
        // `getpp` melakukan normalisasi `0` → `62`; `mutemember` tidak, dan
        // teks harus mengikuti apa adanya.
        let r = plain(json!({}), &["0812345"], &[]);
        assert_eq!(group_write(&r), json!(["0812345@s.whatsapp.net"]));
    }

    #[test]
    fn the_quote_wins_over_the_mention_and_the_argument() {
        let r = run(
            json!({}),
            &["628999@s.whatsapp.net"],
            &["628888@s.whatsapp.net"],
            Some("628777@s.whatsapp.net"),
            GroupRoster::default(),
        );
        assert_eq!(group_write(&r), json!(["628777@s.whatsapp.net"]));
    }

    #[test]
    fn a_lid_target_is_stored_as_a_phone_jid() {
        let r = plain(json!({}), &[], &["123456789@lid"]);
        assert_eq!(group_write(&r), json!(["123456789@s.whatsapp.net"]));
    }

    #[test]
    fn an_admin_cannot_be_muted() {
        let r = run(
            json!({}),
            &[],
            &["628333@s.whatsapp.net"],
            None,
            roster("628333@s.whatsapp.net", true),
        );
        assert_eq!(text_of(&r), "❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat mute admin grup");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_plain_member_is_not_blocked_by_the_admin_check() {
        let r = run(
            json!({}),
            &[],
            &["628333@s.whatsapp.net"],
            None,
            roster("628333@s.whatsapp.net", false),
        );
        assert_eq!(group_write(&r), json!(["628333@s.whatsapp.net"]));
    }

    #[test]
    fn an_already_muted_number_is_not_added_twice() {
        let r = plain(
            json!({"mutedMembers": ["628333@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        );
        assert_eq!(text_of(&r), already_muted("628333"));
        assert!(r.mutations().is_empty());
        assert_eq!(mentions_of(&r), ["628333@s.whatsapp.net"]);
    }

    #[test]
    fn the_same_number_with_a_device_suffix_counts_as_a_different_entry() {
        // `endsWith` di JS membandingkan string apa adanya, `628333:9` tidak
        // berakhir dengan `628333` dan sebaliknya.
        let r = plain(
            json!({"mutedMembers": ["628333@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        );
        assert_eq!(text_of(&r), already_muted("628333"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_different_number_is_appended_and_the_total_grows() {
        let r = plain(
            json!({"mutedMembers": ["628222@s.whatsapp.net"]}),
            &[],
            &["628333@s.whatsapp.net"],
        );
        assert_eq!(
            group_write(&r),
            json!(["628222@s.whatsapp.net", "628333@s.whatsapp.net"])
        );
        assert!(text_of(&r).contains("`2` ᴍᴇᴍʙᴇʀ"), "{}", text_of(&r));
    }

    #[test]
    fn a_non_array_stored_value_reads_as_an_empty_list() {
        let r = plain(json!({"mutedMembers": "bukan-array"}), &[], &["628333@s.whatsapp.net"]);
        assert_eq!(group_write(&r), json!(["628333@s.whatsapp.net"]));
    }
}