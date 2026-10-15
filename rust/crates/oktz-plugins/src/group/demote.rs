//! `plugins/group/demote.js` — turunkan admin menjadi member biasa.
//!
//! Yang dipindah: seluruh gerbang (ambil target, cari peserta, tolak yang
//! bukan admin, tolak owner). Penurunan sebenarnya
//! `sock.groupParticipantsUpdate(m.chat, [id], 'demote')` (`demote.js:63`)
//! tidak ada di API plugin, jadi setelah gerbang lolos plugin diam — bukan
//! mengarang "sekarang bukan admin lagi".

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::group::kick::find_participant;

/// `m.quoted ? m.quoted.sender : m.mentionedJid[0]` (`demote.js:24-28`).
fn demote_target(ctx: &Ctx) -> Option<String> {
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    ctx.mentioned.first().cloned()
}

/// `demote.js:31-35`.
fn no_target(prefix: &str) -> String {
    format!(
        "❌ *ᴛᴀʀɢᴇᴛ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n\
         > Reply pesan user atau mention!\n\
         > Contoh: `{prefix}demote @user`"
    )
}

const NOT_FOUND: &str = "❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan di grup!";
const NOT_ADMIN: &str = "❌ *ɢᴀɢᴀʟ*\n\n> User bukan admin!";
const IS_OWNER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak bisa demote owner grup!";

#[plugin(
    name = "demote",
    alias = ["unadmin", "turunkan"],
    category = "group",
    description = "Turunkan admin menjadi member biasa",
    usage = ".demote @user",
    example = ".demote @user",
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
pub fn demote(ctx: Ctx) -> Reply {
    let Some(target) = demote_target(&ctx) else {
        return Reply::text(no_target(&ctx.prefix));
    };

    // `findParticipantByNumber` di grup `addressing_mode=lid` library mengirim
    // { id: LID, phoneNumber: PN }; roster Rust hanya menyimpan satu JID per
    // peserta, jadi cocoknya hanya pada field itu.
    let Some(p) = find_participant(&ctx.roster.participants, &target) else {
        return Reply::text(NOT_FOUND);
    };

    if !p.is_admin {
        return Reply::text(NOT_ADMIN);
    }

    if p.is_owner {
        return Reply::text(IS_OWNER);
    }

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;
    use oktz_plugin_api::find;
    use oktz_plugin_api::registry::GroupRoster;
    use serde_json::Value;

    fn p(jid: &str, admin: bool, owner: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: owner,
            data: Value::Null,
        }
    }

    fn ctx_with(target: Option<&str>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "demote".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            mentioned: target.map(|t| vec![t.to_string()]).unwrap_or_default(),
            roster: GroupRoster {
                participants: vec![
                    p("628111@s.whatsapp.net", false, false),
                    p("628999@s.whatsapp.net", true, false),
                    p("628555@s.whatsapp.net", true, true),
                ],
                is_bot_admin: true,
            },
            ..Default::default()
        }
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn run(ctx: Ctx) -> Reply {
        (find("demote").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("demote").unwrap();
        assert_eq!(e.meta.name, "demote");
        assert_eq!(e.meta.alias, ["unadmin", "turunkan"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Turunkan admin menjadi member biasa");
        assert_eq!(e.meta.usage, ".demote @user");
        assert_eq!(e.meta.example, ".demote @user");
        assert_eq!(e.meta.cooldown, 5);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["unadmin", "turunkan"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("demote"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn without_quote_or_mention_it_asks_for_a_target() {
        let r = run(ctx_with(None));
        assert_eq!(
            body(&r),
            "❌ *ᴛᴀʀɢᴇᴛ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n\
             > Reply pesan user atau mention!\n\
             > Contoh: `.demote @user`"
        );
    }

    #[test]
    fn a_missing_participant_is_reported_before_the_admin_checks() {
        let r = run(ctx_with(Some("628777@s.whatsapp.net")));
        assert_eq!(body(&r), NOT_FOUND);
    }

    #[test]
    fn a_plain_member_cannot_be_demoted() {
        let r = run(ctx_with(Some("628111@s.whatsapp.net")));
        assert_eq!(body(&r), NOT_ADMIN);
    }

    #[test]
    fn the_group_owner_is_never_demoted() {
        let r = run(ctx_with(Some("628555@s.whatsapp.net")));
        assert_eq!(body(&r), IS_OWNER);
    }

    #[test]
    fn a_real_admin_passes_every_guard_and_then_goes_silent() {
        let r = run(ctx_with(Some("628999@s.whatsapp.net")));
        assert!(r.is_none(), "penurunan tidak di-port; jangan mengarang");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_target_follows_quote_first_then_mention() {
        let mut ctx = ctx_with(Some("628111@s.whatsapp.net"));
        ctx.quoted = Some(oktz_plugin_api::QuotedMessage {
            id: "M1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: "hai".into(),
            kind: oktz_plugin_api::QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        });
        let r = run(ctx);
        assert!(r.is_none(), "quoted yang dipakai, jadi lolos gerbang admin");
    }
}
