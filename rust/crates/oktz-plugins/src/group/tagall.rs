//! `plugins/group/tagall.js` — roster, tanpa state tersimpan.
//!
//! `getParticipantJid` (`src/lib/lid.js`) mengembalikan JID wire peserta apa
//! adanya; `Participant::jid` sudah persis bentuk itu.

use oktz_plugin_api::registry::Participant;
use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::jid_number;

/// `m.text || 'Tag All Members'` (`tagall.js:20`) — string kosong itu falsy.
pub(crate) const DEFAULT_TEXT: &str = "Tag All Members";

pub(crate) const NO_MEMBER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ada member di grup ini.";
pub(crate) const NO_OTHER: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ada member lain yang bisa di-tag.";

/// Peserta selain pengirim, urutan roster apa adanya — `participants.filter(
/// p => getParticipantJid(p) !== m.sender)`.
pub(crate) fn tag_targets<'a>(
    participants: &'a [Participant],
    sender: &str,
) -> Vec<&'a Participant> {
    participants.iter().filter(|p| p.jid != sender).collect()
}

/// `@<nomor>` per baris, dipisah newline — `map(p => '@' + jid.split('@')[0])`.
pub(crate) fn member_list(targets: &[&Participant]) -> String {
    targets
        .iter()
        .map(|p| format!("@{}", jid_number(&p.jid)))
        .collect::<Vec<_>>()
        .join("\n")
}

#[plugin(
    name = "tagall",
    alias = ["all", "everyone"],
    category = "group",
    description = "Tag semua member grup",
    usage = ".tagall <pesan>",
    example = ".tagall Halo semua!",
    cooldown = 30,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = false
)]
pub fn tagall(ctx: Ctx) -> Reply {
    let text = if ctx.text.is_empty() {
        DEFAULT_TEXT
    } else {
        ctx.text.as_str()
    };

    if ctx.roster.is_empty() {
        return Reply::text(NO_MEMBER);
    }

    let targets = tag_targets(&ctx.roster.participants, &ctx.sender);
    if targets.is_empty() {
        return Reply::text(NO_OTHER);
    }

    Reply::Text {
        text: format!(
            "*Pesan:* {text}\n\n```━━━ {} MEMBER TOTAL ━━━```\n{}",
            targets.len(),
            member_list(&targets)
        ),
        mentions: targets.iter().map(|p| p.jid.clone()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::GroupRoster;

    fn p(jid: &str, admin: bool, owner: bool) -> Participant {
        Participant {
            jid: jid.into(),
            is_admin: admin,
            is_owner: owner,
        }
    }

    fn ctx(participants: Vec<Participant>) -> Ctx {
        Ctx {
            text: "Halo semua!".into(),
            prefix: ".".into(),
            command: "tagall".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: GroupRoster {
                participants,
                is_bot_admin: true,
            },
            ..Default::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions_of(r: &Reply) -> &Vec<String> {
        match r {
            Reply::Text { mentions, .. } => mentions,
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    fn run(c: Ctx) -> Reply {
        (oktz_plugin_api::find("tagall").unwrap().handler)(c)
    }

    // -- registrasi & meta ------------------------------------------------

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tagall").unwrap();
        assert_eq!(p.meta.name, "tagall");
        assert_eq!(p.meta.alias, ["all", "everyone"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Tag semua member grup");
        assert_eq!(p.meta.usage, ".tagall <pesan>");
        assert_eq!(p.meta.example, ".tagall Halo semua!");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_enabled);
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private);
        // `isAdmin: true` tapi `isBotAdmin: false` — bot TIDAK boleh auto-tag.
        assert!(p.meta.is_admin);
        assert!(!p.meta.is_bot_admin);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.skip_registration);
    }

    #[test]
    fn aliases_resolve_to_tagall() {
        for a in ["all", "everyone"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "tagall");
        }
        assert!(!oktz_plugin_api::find("tagall").unwrap().meta.matches("all2"));
    }

    // -- cabang roster -----------------------------------------------------

    #[test]
    fn empty_roster_takes_the_no_member_branch() {
        for bot_admin in [true, false] {
            let mut c = ctx(Vec::new());
            c.roster.is_bot_admin = bot_admin;
            let r = run(c);
            assert_eq!(text_of(&r), NO_MEMBER);
            assert!(mentions_of(&r).is_empty(), "tidak ada yang bisa di-tag");
        }
    }

    #[test]
    fn roster_of_only_the_sender_takes_the_no_other_branch() {
        let r = run(ctx(vec![p("628111@s.whatsapp.net", false, false)]));
        assert_eq!(text_of(&r), NO_OTHER);
        assert!(mentions_of(&r).is_empty());
    }

    #[test]
    fn sender_is_excluded_from_but_mentions_cover_everyone_else() {
        let r = run(ctx(vec![
            p("628111@s.whatsapp.net", true, true),
            p("628222@s.whatsapp.net", false, false),
            p("628333@lid", false, false),
        ]));
        assert_eq!(
            text_of(&r),
            "*Pesan:* Halo semua!\n\n```━━━ 2 MEMBER TOTAL ━━━```\n@628222\n@628333"
        );
        assert_eq!(mentions_of(&r), &["628222@s.whatsapp.net", "628333@lid"]);
    }

    #[test]
    fn admin_and_member_flags_do_not_change_who_gets_tagged() {
        // JS tidak memfilter admin; yang penting bukan pengirim.
        let base = ctx(vec![
            p("628111@s.whatsapp.net", false, false),
            p("628222@s.whatsapp.net", false, false),
        ]);
        let member = run(base);
        let admin = run(ctx(vec![
            p("628111@s.whatsapp.net", false, false),
            p("628222@s.whatsapp.net", true, true),
        ]));
        assert_eq!(text_of(&member), text_of(&admin));
    }

    #[test]
    fn is_bot_admin_false_still_tags_the_same_way() {
        // `isBotAdmin: false` itu gate router, bukan gate plugin — handler
        // tidak pernah membacanya. Menandainya di sini memastikan port tidak
        // diam-diam menambah exemption.
        let mut c = ctx(vec![
            p("628111@s.whatsapp.net", false, false),
            p("628222@s.whatsapp.net", false, false),
        ]);
        c.roster.is_bot_admin = false;
        let r = run(c);
        assert!(text_of(&r).contains("1 MEMBER TOTAL"));
        assert_eq!(mentions_of(&r), &["628222@s.whatsapp.net"]);
    }

    // -- cabang teks -------------------------------------------------------

    #[test]
    fn empty_text_falls_back_to_the_js_default() {
        let mut c = ctx(vec![
            p("628111@s.whatsapp.net", false, false),
            p("628222@s.whatsapp.net", false, false),
        ]);
        c.text = String::new();
        assert!(text_of(&run(c)).starts_with("*Pesan:* Tag All Members\n\n"));
        assert_eq!(DEFAULT_TEXT, "Tag All Members");
    }

    #[test]
    fn text_is_echoed_verbatim_inside_the_header() {
        let mut c = ctx(vec![
            p("628111@s.whatsapp.net", false, false),
            p("628222@s.whatsapp.net", false, false),
        ]);
        c.text = "mulai rapat *sekarang*".into();
        assert!(text_of(&run(c))
            .starts_with("*Pesan:* mulai rapat *sekarang*\n\n```━━━ 1 MEMBER TOTAL"));
    }

    #[test]
    fn device_suffix_is_kept_in_the_displayed_number_like_js_split() {
        let r = run(ctx(vec![
            p("628111@s.whatsapp.net", false, false),
            p("628999:12@s.whatsapp.net", false, false),
        ]));
        assert!(
            text_of(&r).ends_with("@628999:12"),
            "`jid.split('@')[0]` di JS juga menyimpan `:device`"
        );
        assert_eq!(mentions_of(&r), &["628999:12@s.whatsapp.net"]);
    }

    #[test]
    fn helper_contracts() {
        assert!(tag_targets(&[], "a@s.whatsapp.net").is_empty());
        assert_eq!(member_list(&[]), "");
        let ps = vec![
            p("a@s.whatsapp.net", false, false),
            p("b@lid", false, false),
        ];
        assert_eq!(member_list(&tag_targets(&ps, "a@s.whatsapp.net")), "@b");
        assert_eq!(member_list(&tag_targets(&ps, "b@lid")), "@a");
    }
}