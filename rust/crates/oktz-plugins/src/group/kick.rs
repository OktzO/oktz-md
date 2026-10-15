//! `plugins/group/kick.js` — keluarkan member dari grup.
//!
//! Yang dipindah: seluruh gerbang (ambil target, cocokkan peserta, tolak
//! admin). Pengeluaran sebenarnya
//! `sock.groupParticipantsUpdate(m.chat, [id], 'remove')` (`kick.js:84`) tidak
//! ada di API plugin, jadi setelah gerbang lolos plugin diam — bukan
//! mengarang "telah dikeluarkan dari grup".
//!
//! `ponytail:` dua gerbang "tidak bisa kick bot sendiri" (`kick.js:46` dan
//! `:71-77`) membandingkan target dengan `sock.user?.id` dan `sock.user?.lid`;
//! `Ctx` tidak punya JID bot, jadi keduanya dilewati. Karena mengeluarkan
//! member juga tidak di-port, tidak ada siapa pun yang bisa ikut terpental.

use oktz_plugin_api::{Ctx, Participant, Reply, plugin};

use crate::group::delete::{is_same_group_member, is_same_participant};

/// `findParticipantByNumber` (`lid.js:484-508`) — dicocokkan pada bagian
/// sebelum `@`; roster Rust hanya menyimpan satu JID per peserta, jadi hanya
/// `p.jid` yang bisa dibandingkan (JS juga mencoba `p.id`/`p.lid`/
/// `p.phoneNumber`).
pub(crate) fn find_participant<'a>(
    roster: &'a [Participant],
    target: &str,
) -> Option<&'a Participant> {
    let want = target.split('@').next().unwrap_or(target);
    roster
        .iter()
        .find(|p| p.jid.split('@').next().unwrap_or(&p.jid) == want)
}

/// `m.quoted ? m.quoted.sender : m.mentionedJid[0]` (`kick.js:28-32`).
fn kick_target(ctx: &Ctx) -> Option<String> {
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    ctx.mentioned.first().cloned()
}

/// `kick.js:35-39`.
fn no_target(prefix: &str) -> String {
    format!(
        "❌ *ᴛᴀʀɢᴇᴛ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n\
         > Reply pesan user atau mention!\n\
         > Contoh: `{prefix}kick @user`"
    )
}

const CANT_KICK_SELF: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak bisa kick diri sendiri!";
const NOT_IN_GROUP: &str = "❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan dalam grup!";
const CANT_KICK_ADMIN: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak bisa kick admin grup!";

#[plugin(
    name = "kick",
    alias = ["remove", "tendang"],
    category = "group",
    description = "Kick member dari grup",
    usage = ".kick @user",
    example = ".kick @user",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn kick(ctx: Ctx) -> Reply {
    let Some(target) = kick_target(&ctx) else {
        return Reply::text(no_target(&ctx.prefix));
    };

    if is_same_participant(&target, &ctx.sender) {
        return Reply::text(CANT_KICK_SELF);
    }

    let Some(p) = find_participant(&ctx.roster.participants, &target) else {
        return Reply::text(NOT_IN_GROUP);
    };

    // `isSameGroupMember` menutup kasus PN di satu sisi, LID di sisi lain
    // (`kick.js:67-70`); daftar participant = kamusnya.
    if is_same_group_member(&ctx.roster.participants, &target, &ctx.sender) {
        return Reply::text(CANT_KICK_SELF);
    }

    if p.is_admin {
        return Reply::text(CANT_KICK_ADMIN);
    }

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedKind;
    use oktz_plugin_api::QuotedMessage;
    use oktz_plugin_api::find;
    use oktz_plugin_api::registry::GroupRoster;
    use serde_json::Value;

    fn p(jid: &str, admin: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: false,
            data: Value::Null,
        }
    }

    fn ctx_with(target: Option<&str>, target_admin: bool) -> Ctx {
        let roster = GroupRoster {
            participants: vec![
                p("628111@s.whatsapp.net", false),
                p("628999@s.whatsapp.net", target_admin),
            ],
            is_bot_admin: true,
        };
        Ctx {
            prefix: ".".into(),
            command: "kick".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            mentioned: target.map(|t| vec![t.to_string()]).unwrap_or_default(),
            roster,
            ..Default::default()
        }
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("kick").unwrap();
        assert_eq!(e.meta.name, "kick");
        assert_eq!(e.meta.alias, ["remove", "tendang"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Kick member dari grup");
        assert_eq!(e.meta.usage, ".kick @user");
        assert_eq!(e.meta.example, ".kick @user");
        assert_eq!(e.meta.cooldown, 5);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["remove", "tendang"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("kick"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn without_quote_or_mention_it_asks_for_a_target() {
        let r = (find("kick").unwrap().handler)(ctx_with(None, false));
        assert_eq!(
            body(&r),
            "❌ *ᴛᴀʀɢᴇᴛ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n\
             > Reply pesan user atau mention!\n\
             > Contoh: `.kick @user`"
        );
    }

    #[test]
    fn the_usage_example_follows_the_live_prefix() {
        let mut ctx = ctx_with(None, false);
        ctx.prefix = "!".into();
        let r = (find("kick").unwrap().handler)(ctx);
        assert!(body(&r).contains("`!kick @user`"), "{}", body(&r));
    }

    #[test]
    fn kicking_yourself_is_refused_before_the_roster_lookup() {
        let r = (find("kick").unwrap().handler)(ctx_with(Some("628111@s.whatsapp.net"), false));
        assert_eq!(body(&r), CANT_KICK_SELF);
        // Device yang berbeda tapi orang sama.
        let r = (find("kick").unwrap().handler)(ctx_with(Some("628111:5@s.whatsapp.net"), false));
        assert_eq!(body(&r), CANT_KICK_SELF, "LID/PN device harus tetap kena");
    }

    #[test]
    fn a_target_outside_the_roster_is_reported_as_not_in_the_group() {
        let r = (find("kick").unwrap().handler)(ctx_with(Some("628777@s.whatsapp.net"), false));
        assert_eq!(body(&r), NOT_IN_GROUP);
    }

    #[test]
    fn an_admin_target_is_refused() {
        let r = (find("kick").unwrap().handler)(ctx_with(Some("628999@s.whatsapp.net"), true));
        assert_eq!(body(&r), CANT_KICK_ADMIN);
    }

    #[test]
    fn a_plain_member_passes_every_guard_and_then_goes_silent() {
        let r = (find("kick").unwrap().handler)(ctx_with(Some("628999@s.whatsapp.net"), false));
        assert!(r.is_none(), "peng kick-an tidak di-port; jangan mengarang");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_reply_wins_over_a_mention_just_like_the_js_order() {
        let ctx = Ctx {
            quoted: Some(QuotedMessage {
                id: "M1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: "hai".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            mentioned: vec!["628777@s.whatsapp.net".into()],
            ..ctx_with(Some("628777@s.whatsapp.net"), false)
        };
        let r = (find("kick").unwrap().handler)(ctx);
        assert!(
            r.is_none(),
            "quoted yang dipakai, jadi bukan 'tidak ditemukan'"
        );
    }

    #[test]
    fn participant_lookup_ignores_the_server_part() {
        let roster = vec![p("628999@s.whatsapp.net", false), p("628000@lid", true)];
        assert_eq!(
            find_participant(&roster, "628999@lid").map(|p| p.jid.as_str()),
            Some("628999@s.whatsapp.net")
        );
        assert_eq!(
            find_participant(&roster, "628000@s.whatsapp.net").map(|p| p.jid.as_str()),
            Some("628000@lid")
        );
        assert!(find_participant(&roster, "628777@s.whatsapp.net").is_none());
        assert!(find_participant(&[], "628999@s.whatsapp.net").is_none());
        assert!(
            find_participant(&roster, "").is_none(),
            "target kosong tidak cocok"
        );
    }
}
