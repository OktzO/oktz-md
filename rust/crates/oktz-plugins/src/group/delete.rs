//! `plugins/group/delete.js` — hapus pesan yang di-reply.
//!
//! Yang dipindah: seluruh gerbang izin. `m.quoted.key.fromMe` →
//! `QuotedMessage::from_me`, `m.isBotAdmin` → `ctx.roster.is_bot_admin`,
//! `m.isAdmin` → `ctx.roster.is_admin(sender)`, dan
//! `isSameGroupMember(groupMembers, quotedSender, sender)` → pencocokan
//! roster (`lid.js:454-482`).
//!
//! `ponytail:` `sock.sendMessage(m.chat, { delete: key })` tidak ada di API
//! plugin, jadi `m.react('✅')` setelahnya juga tidak terjadi — plugin diam
//! setelah gerbang lolos, bukan melapor penghapusan yang tidak terjadi.
//! `botJids` dari `sock.user` juga tidak terbaca, jadi `isBotMessage` hanya
//! bisa `true` lewat `fromMe`.

use oktz_plugin_api::{Ctx, Participant, Reply, plugin};

pub(crate) const NO_QUOTE: &str = "⚠️ *Reply pesan yang ingin dihapus!*";
pub(crate) const BOT_NOT_ADMIN: &str = "⚠️ *Bot harus jadi admin untuk hapus pesan orang lain!*";
pub(crate) const NOT_ADMIN: &str = "⚠️ *Hanya admin yang bisa hapus pesan orang lain!*";

/// `normalizeComparableJid` (`lid.js:438-452`) — buang device `:N` hanya
/// untuk JID `@s.whatsapp.net`.
fn normalize(jid: &str) -> String {
    if let Some(base) = jid.trim().strip_suffix("@s.whatsapp.net")
        && let Some((head, _)) = base.split_once(':')
    {
        return format!("{head}@s.whatsapp.net");
    }
    jid.trim().to_string()
}

fn digits(jid: &str) -> String {
    jid.chars().filter(char::is_ascii_digit).collect()
}

/// `isSameParticipant` (`lid.js:454-475`).
pub(crate) fn is_same_participant(a: &str, b: &str) -> bool {
    let (a, b) = (normalize(a), normalize(b));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let (na, nb) = (digits(&a), digits(&b));
    if na.is_empty() || nb.is_empty() {
        return false;
    }
    na == nb || na.ends_with(&nb) || nb.ends_with(&na)
}

/// `findParticipantByNumber` (`lid.js:484-506`) — dicocokkan pada bagian
/// sebelum `@`; roster Rust hanya menyimpan satu JID per peserta.
fn find_by_number<'a>(roster: &'a [Participant], target: &str) -> Option<&'a Participant> {
    let want = target.split('@').next().unwrap_or(target);
    roster
        .iter()
        .find(|p| p.jid.split('@').next().unwrap_or(&p.jid) == want)
}

/// `isSameGroupMember` (`lid.js:477-482`).
pub(crate) fn is_same_group_member(roster: &[Participant], a: &str, b: &str) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if is_same_participant(a, b) {
        return true;
    }
    match (find_by_number(roster, a), find_by_number(roster, b)) {
        (Some(x), Some(y)) => x.jid == y.jid,
        _ => false,
    }
}

#[plugin(
    name = "delete",
    alias = ["del", "hapus", "d"],
    category = "group",
    description = "Hapus pesan dengan reply",
    usage = ".delete (reply pesan)",
    example = ".delete",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn delete(ctx: Ctx) -> Reply {
    let Some(quoted) = ctx.quoted() else {
        return Reply::text(NO_QUOTE);
    };

    let is_own_message =
        quoted.from_me || is_same_group_member(&ctx.roster.participants, &quoted.sender, &ctx.sender);
    // `botJids.includes(...)` butuh `sock.user` — lihat `ponytail:` modul.
    let is_bot_message = quoted.from_me;

    if !is_own_message && !is_bot_message {
        if !ctx.roster.is_bot_admin {
            return Reply::text(BOT_NOT_ADMIN);
        }
        if !ctx.roster.is_admin(&ctx.sender) && !ctx.is_owner {
            return Reply::text(NOT_ADMIN);
        }
    }

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::GroupRoster;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};
    use serde_json::json;

    fn p(jid: &str, admin: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: false,
            data: json!({}),
        }
    }

    struct Opts {
        quoted_sender: Option<&'static str>,
        from_me: bool,
        bot_admin: bool,
        sender_admin: bool,
        is_owner: bool,
    }

    fn ctx_with(o: Opts) -> Ctx {
        let roster = GroupRoster {
            participants: vec![
                p("628111@s.whatsapp.net", o.sender_admin),
                p("628999@s.whatsapp.net", false),
            ],
            is_bot_admin: o.bot_admin,
        };
        Ctx {
            prefix: ".".into(),
            command: "delete".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: o.is_owner,
            roster,
            quoted: o.quoted_sender.map(|s| QuotedMessage {
                id: "MSG1".into(),
                sender: s.into(),
                chat: "120363@g.us".into(),
                text: "halo".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: o.from_me,
            }),
            ..Default::default()
        }
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("delete").unwrap();
        assert_eq!(p.meta.name, "delete");
        assert_eq!(p.meta.alias, ["del", "hapus", "d"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Hapus pesan dengan reply");
        assert_eq!(p.meta.usage, ".delete (reply pesan)");
        assert_eq!(p.meta.example, ".delete");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["del", "hapus", "d"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("delete")
            );
        }
    }

    #[test]
    fn without_a_reply_it_asks_for_one() {
        let r = (oktz_plugin_api::find("delete").unwrap().handler)(Ctx {
            prefix: ".".into(),
            chat: "120363@g.us".into(),
            ..Default::default()
        });
        assert_eq!(body(&r), NO_QUOTE);
    }

    #[test]
    fn replying_to_your_own_message_needs_no_admin_rights() {
        let r = (oktz_plugin_api::find("delete").unwrap().handler)(ctx_with(Opts {
            quoted_sender: Some("628111@s.whatsapp.net"),
            from_me: true,
            bot_admin: false,
            sender_admin: false,
            is_owner: false,
        }));
        assert!(r.is_none(), "gerbang lolos; penghapusan tidak di-port");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_member_who_is_also_the_roster_sender_passes_the_own_check() {
        // `fromMe` false, tapi JID sama persis dengan pengirim.
        let r = (oktz_plugin_api::find("delete").unwrap().handler)(ctx_with(Opts {
            quoted_sender: Some("628111@s.whatsapp.net"),
            from_me: false,
            bot_admin: false,
            sender_admin: false,
            is_owner: false,
        }));
        assert!(r.is_none(), "pesan sendiri tetap boleh dihapus");
    }

    #[test]
    fn deleting_someone_elses_message_needs_a_bot_that_is_admin() {
        let r = (oktz_plugin_api::find("delete").unwrap().handler)(ctx_with(Opts {
            quoted_sender: Some("628999@s.whatsapp.net"),
            from_me: false,
            bot_admin: false,
            sender_admin: true,
            is_owner: true,
        }));
        assert_eq!(body(&r), BOT_NOT_ADMIN);
    }

    #[test]
    fn with_the_bot_as_admin_the_caller_must_be_admin_or_owner() {
        let guest = (oktz_plugin_api::find("delete").unwrap().handler)(ctx_with(Opts {
            quoted_sender: Some("628999@s.whatsapp.net"),
            from_me: false,
            bot_admin: true,
            sender_admin: false,
            is_owner: false,
        }));
        assert_eq!(body(&guest), NOT_ADMIN);

        for is_owner in [true, false] {
            let allowed = (oktz_plugin_api::find("delete").unwrap().handler)(ctx_with(Opts {
                quoted_sender: Some("628999@s.whatsapp.net"),
                from_me: false,
                bot_admin: true,
                sender_admin: true,
                is_owner,
            }));
            assert!(allowed.is_none(), "is_owner={is_owner} harusnya lolos");
        }
    }

    #[test]
    fn participant_matching_follows_the_js_number_rules() {
        let roster = vec![p("628999@lid", false)];
        assert!(is_same_participant("628999@s.whatsapp.net", "628999@lid"));
        assert!(is_same_participant("628999:5@s.whatsapp.net", "628999@s.whatsapp.net"));
        assert!(!is_same_participant("628999@s.whatsapp.net", "628111@s.whatsapp.net"));
        assert!(!is_same_participant("", "628999@s.whatsapp.net"));
        // Berbeda di roster tapi nomornya sama → tetap member yang sama.
        assert!(is_same_group_member(&roster, "628999@s.whatsapp.net", "628999:3@s.whatsapp.net"));
        assert!(!is_same_group_member(&roster, "628999@s.whatsapp.net", "628111@s.whatsapp.net"));
        // Roster kosong tidak mengubah apa pun: `isSameParticipant` memutuskan
        // lebih dulu, persis seperti JS sebelum menyentuh participants.
        assert!(is_same_group_member(&[], "628999@s.whatsapp.net", "628999@lid"));
        assert!(!is_same_group_member(&[], "628999@s.whatsapp.net", ""));
    }
}