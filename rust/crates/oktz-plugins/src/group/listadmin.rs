//! `plugins/group/listadmin.js` — roster admin/superadmin.

use oktz_plugin_api::registry::Participant;
use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::jid_number;

pub(crate) const NO_ADMIN: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Tidak ada admin di grup ini.";
const HEAD: &str = "👑 *ʟɪsᴛ ᴀᴅᴍɪɴ*\n\n";

/// `admins.find(a => a.admin === 'superadmin')` — yang PERTAMA, dan `find`
/// mengembalikan `undefined` kalau tidak ada.
pub(crate) fn owner_of<'a>(admins: &[&'a Participant]) -> Option<&'a Participant> {
    admins.iter().copied().find(|a| a.is_owner)
}

/// `admins.filter(a => a.admin === 'admin')` — superadmin TIDAK ikut, karena
/// di wire `admin` itu string berbeda (`'admin'` vs `'superadmin'`), bukan
/// boolean yang menumpuk.
pub(crate) fn regular_admins<'a>(admins: &[&'a Participant]) -> Vec<&'a Participant> {
    admins
        .iter()
        .copied()
        .filter(|a| a.is_admin && !a.is_owner)
        .collect()
}

pub(crate) fn render_admin_list(participants: &[Participant]) -> String {
    let admins: Vec<&Participant> = participants.iter().filter(|a| a.is_admin).collect();
    if admins.is_empty() {
        return NO_ADMIN.to_string();
    }

    let mut txt = String::from(HEAD);
    if let Some(owner) = owner_of(&admins) {
        txt.push_str("```━━━ ᴏᴡɴᴇʀ ━━━```\n");
        txt.push_str(&format!("```👑 @{}```\n\n", jid_number(&owner.jid)));
    }

    let regular = regular_admins(&admins);
    if !regular.is_empty() {
        txt.push_str("```━━━ ᴀᴅᴍɪɴ ━━━```\n");
        for (i, admin) in regular.iter().enumerate() {
            txt.push_str(&format!("```{}. @{}```\n", i + 1, jid_number(&admin.jid)));
        }
    }
    txt.push_str(&format!("\n`Total Admin: {}`", admins.len()));
    txt
}

#[plugin(
    name = "listadmin",
    alias = ["admins", "adminlist"],
    category = "group",
    description = "Menampilkan daftar admin grup",
    usage = ".listadmin",
    example = ".listadmin",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn listadmin(ctx: Ctx) -> Reply {
    if !ctx.roster.admins().any(|_| true) {
        return Reply::text(NO_ADMIN);
    }
    Reply::Text {
        text: render_admin_list(&ctx.roster.participants),
        mentions: ctx
            .roster
            .admins()
            .map(|a| a.jid.clone())
            .collect(),
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
            data: Default::default(),
        }
    }

    fn run(participants: Vec<Participant>, bot_admin: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "listadmin".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            roster: GroupRoster {
                participants,
                is_bot_admin: bot_admin,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("listadmin").unwrap().handler)(ctx)
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

    // -- registrasi & meta ------------------------------------------------

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listadmin").unwrap();
        assert_eq!(p.meta.name, "listadmin");
        assert_eq!(p.meta.alias, ["admins", "adminlist"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Menampilkan daftar admin grup");
        assert_eq!(p.meta.usage, ".listadmin");
        assert_eq!(p.meta.example, ".listadmin");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        // `isAdmin: false` dan `isBotAdmin: false` — siapa pun boleh melihat.
        assert!(!p.meta.is_admin);
        assert!(!p.meta.is_bot_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn aliases_resolve_to_listadmin() {
        for a in ["admins", "adminlist"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "listadmin");
        }
    }

    // -- cabang roster -----------------------------------------------------

    #[test]
    fn empty_roster_takes_the_no_admin_branch() {
        for bot_admin in [true, false] {
            let r = run(Vec::new(), bot_admin);
            assert_eq!(text_of(&r), NO_ADMIN);
            assert!(mentions_of(&r).is_empty());
        }
    }

    #[test]
    fn roster_of_only_members_takes_the_no_admin_branch() {
        let r = run(
            vec![
                p("628111@s.whatsapp.net", false, false),
                p("628222@s.whatsapp.net", false, false),
            ],
            true,
        );
        assert_eq!(text_of(&r), NO_ADMIN);
    }

    #[test]
    fn owner_only_roster_lists_the_owner_block_and_counts_one() {
        let r = run(
            vec![
                p("628111@s.whatsapp.net", false, false),
                p("628999@s.whatsapp.net", true, true),
            ],
            true,
        );
        assert_eq!(
            text_of(&r),
            "👑 *ʟɪsᴛ ᴀᴅᴍɪɴ*\n\n```━━━ ᴏᴡɴᴇʀ ━━━```\n```👑 @628999```\n\n\n`Total Admin: 1`"
        );
        assert_eq!(mentions_of(&r), &["628999@s.whatsapp.net"]);
    }

    #[test]
    fn admins_only_roster_lists_the_admin_block_numbered_from_one() {
        let r = run(
            vec![
                p("628222@s.whatsapp.net", true, false),
                p("628333@s.whatsapp.net", true, false),
            ],
            false,
        );
        assert_eq!(
            text_of(&r),
            "👑 *ʟɪsᴛ ᴀᴅᴍɪɴ*\n\n```━━━ ᴀᴅᴍɪɴ ━━━```\n```1. @628222```\n```2. @628333```\n\n`Total Admin: 2`"
        );
        assert_eq!(
            mentions_of(&r),
            &["628222@s.whatsapp.net", "628333@s.whatsapp.net"]
        );
    }

    #[test]
    fn mixed_roster_shows_owner_first_then_numbered_admins() {
        let r = run(
            vec![
                p("628111@s.whatsapp.net", false, false),
                p("628222@s.whatsapp.net", true, false),
                p("628999@s.whatsapp.net", true, true),
                p("628333@s.whatsapp.net", true, false),
            ],
            true,
        );
        assert_eq!(
            text_of(&r),
            concat!(
                "👑 *ʟɪsᴛ ᴀᴅᴍɪɴ*\n\n",
                "```━━━ ᴏᴡɴᴇʀ ━━━```\n```👑 @628999```\n\n",
                "```━━━ ᴀᴅᴍɪɴ ━━━```\n```1. @628222```\n```2. @628333```\n",
                "\n`Total Admin: 3`"
            )
        );
        assert_eq!(
            mentions_of(&r),
            &[
                "628222@s.whatsapp.net",
                "628999@s.whatsapp.net",
                "628333@s.whatsapp.net"
            ],
            "mention ikut urutan roster, bukan urutan blok"
        );
    }

    #[test]
    fn is_bot_admin_does_not_affect_the_listing() {
        let ps = vec![p("628222@s.whatsapp.net", true, false)];
        assert_eq!(text_of(&run(ps.clone(), true)), text_of(&run(ps, false)));
    }

    #[test]
    fn only_the_first_superadmin_becomes_the_owner() {
        let ps = vec![
            p("628111@s.whatsapp.net", true, true),
            p("628222@s.whatsapp.net", true, true),
        ];
        let out = render_admin_list(&ps);
        assert_eq!(out.matches("👑 @").count(), 1, "owner hanya yang pertama");
        assert!(out.contains("👑 @628111"));
        // Superadmin kedua TIDAK jadi admin biasa: di wire `admin`-nya string
        // `'superadmin'`, dan filter JS cuma mengambil `'admin'`.
        assert!(!out.contains("```1. @628222```"));
        assert!(!out.contains("━━━ ᴀᴅᴍɪɴ ━━━"), "blok admin kosong di-lewati");
        assert!(out.ends_with("`Total Admin: 2`"));
    }

    #[test]
    fn helper_contracts() {
        assert!(owner_of(&[]).is_none());
        assert_eq!(owner_of(&[]).map(|p| p.is_owner), None);
        let ps = [p("a@s.whatsapp.net", true, true), p("b@s.whatsapp.net", true, false)];
        let admins: Vec<&Participant> = ps.iter().collect();
        assert_eq!(owner_of(&admins).unwrap().jid, "a@s.whatsapp.net");
        assert_eq!(regular_admins(&admins).len(), 1);
        assert_eq!(regular_admins(&admins)[0].jid, "b@s.whatsapp.net");
        assert_eq!(render_admin_list(&[]), NO_ADMIN);
    }
}