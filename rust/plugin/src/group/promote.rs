//! `plugins/group/promote.js` — jadikan member sebagai admin.
//!
//! Yang dipindah: seluruh gerbang (ambil target, cari peserta, tolak yang
//! sudah admin). Pengangkatan sebenarnya
//! `sock.groupParticipantsUpdate(m.chat, [id], 'promote')` (`promote.js:58`)
//! tidak ada di API plugin, jadi setelah gerbang lolos plugin diam — bukan
//! mengarang "sekarang menjadi admin".

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::group::kick::find_participant;

/// `m.quoted ? m.quoted.sender : m.mentionedJid[0]` (`promote.js:24-28`).
fn promote_target(ctx: &Ctx) -> Option<String> {
    if let Some(q) = ctx.quoted() {
        return Some(q.sender.clone());
    }
    ctx.mentioned.first().cloned()
}

/// `promote.js:31-35`.
fn no_target(prefix: &str) -> String {
    format!(
        "❌ *ᴛᴀʀɢᴇᴛ ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n\
         > Reply pesan user atau mention!\n\
         > Contoh: `{prefix}promote @user`"
    )
}

const NOT_FOUND: &str = "❌ *ɢᴀɢᴀʟ*\n\n> User tidak ditemukan di grup!";
const ALREADY_ADMIN: &str = "❌ *ɢᴀɢᴀʟ*\n\n> User sudah menjadi admin!";

#[plugin(
    name = "promote",
    alias = ["jadiadmin", "admin"],
    category = "group",
    description = "Jadikan member sebagai admin",
    usage = ".promote @user",
    example = ".promote @user",
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
pub fn promote(ctx: Ctx) -> Reply {
    let Some(target) = promote_target(&ctx) else {
        return Reply::text(no_target(&ctx.prefix));
    };

    let Some(p) = find_participant(&ctx.roster.participants, &target) else {
        return Reply::text(NOT_FOUND);
    };

    if p.is_admin {
        return Reply::text(ALREADY_ADMIN);
    }

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Participant;
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

    fn ctx_with(target: Option<&str>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "promote".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            mentioned: target.map(|t| vec![t.to_string()]).unwrap_or_default(),
            roster: GroupRoster {
                participants: vec![
                    p("628111@s.whatsapp.net", false),
                    p("628999@s.whatsapp.net", true),
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
        (find("promote").unwrap().handler)(ctx)
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("promote").unwrap();
        assert_eq!(e.meta.name, "promote");
        assert_eq!(e.meta.alias, ["jadiadmin", "admin"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Jadikan member sebagai admin");
        assert_eq!(e.meta.usage, ".promote @user");
        assert_eq!(e.meta.example, ".promote @user");
        assert_eq!(e.meta.cooldown, 5);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["jadiadmin", "admin"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("promote"),
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
             > Contoh: `.promote @user`"
        );
    }

    #[test]
    fn a_missing_participant_is_reported_first() {
        let r = run(ctx_with(Some("628777@s.whatsapp.net")));
        assert_eq!(body(&r), NOT_FOUND);
    }

    #[test]
    fn an_existing_admin_is_refused() {
        // Owner juga admin — di JS `participant.admin` truthy juga, jadi
        // cabang "sudah admin" yang capturing, bukan ada cabang owner.
        let r = run(ctx_with(Some("628999@s.whatsapp.net")));
        assert_eq!(body(&r), ALREADY_ADMIN);
    }

    #[test]
    fn a_plain_member_passes_every_guard_and_then_goes_silent() {
        let r = run(ctx_with(Some("628111@s.whatsapp.net")));
        assert!(r.is_none(), "pengangkatan tidak di-port; jangan mengarang");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_reply_wins_over_a_mention() {
        let mut ctx = ctx_with(Some("628111@s.whatsapp.net"));
        ctx.quoted = Some(QuotedMessage {
            id: "M1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: "hai".into(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
            timestamp: 0,
        });
        let r = run(ctx);
        assert_eq!(body(&r), ALREADY_ADMIN, "quoted admin, bukan mention biasa");
    }
}
