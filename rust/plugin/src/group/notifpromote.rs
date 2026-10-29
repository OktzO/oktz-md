//! `plugins/group/notifpromote.js` — toggle `groups[chat].notifPromote`.
//!
//! Tubuh bersama dengan tiga plugin `notif*` lain ada di
//! [`crate::group::notifclosegroup`].

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::group::notifclosegroup::{NotifToggle, notif_toggle};

#[plugin(
    name = "notifpromote",
    alias = [],
    category = "group",
    description = "Toggle notifikasi saat ada yang dijadikan admin",
    usage = ".notifpromote on/off",
    example = ".notifpromote on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn notifpromote(ctx: Ctx) -> Reply {
    notif_toggle(
        &ctx,
        &NotifToggle {
            field: "notifPromote",
            title: "ɴᴏᴛɪꜰ ᴘʀᴏᴍᴏᴛᴇ",
            emoji: "👑",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::group::notifclosegroup::ADMIN_ONLY;
    use oktz_plugin_api::{Mutation, Participant};
    use serde_json::{Value};

    fn run(group: Value, arg: Option<&str>, owner: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notifpromote".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: owner,
            args: arg.into_iter().map(String::from).collect(),
            group_data: group,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![Participant {
                    jid: "628111@s.whatsapp.net".into(),
                    is_admin: false,
                    is_owner: false,
                    data: Value::Null,
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("notifpromote").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { data, .. } => data,
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("notifpromote").unwrap();
        assert_eq!(p.meta.name, "notifpromote");
        assert!(p.meta.alias.is_empty(), "alias di JS kosong");
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Toggle notifikasi saat ada yang dijadikan admin");
        assert_eq!(p.meta.usage, ".notifpromote on/off");
        assert_eq!(p.meta.example, ".notifpromote on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn the_status_header_uses_the_promote_title_and_crown() {
        let r = run(serde_json::json!({"notifPromote": false}), None, true);
        assert!(
            text_of(&r).starts_with("👑 *ɴᴏᴛɪꜰ ᴘʀᴏᴍᴏᴛᴇ*\n\n> Status: ❌ Nonaktif"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn on_and_off_use_this_plugin_own_wording() {
        assert_eq!(
            text_of(&run(serde_json::json!({}), Some("on"), true)),
            "✅ *ɴᴏᴛɪꜰ ᴘʀᴏᴍᴏᴛᴇ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*"
        );
        assert_eq!(group_write(&run(serde_json::json!({}), Some("on"), true)), &serde_json::json!({"notifPromote": true}));
        assert_eq!(
            text_of(&run(serde_json::json!({}), Some("off"), true)),
            "❌ *ɴᴏᴛɪꜰ ᴘʀᴏᴍᴏᴛᴇ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*"
        );
        assert_eq!(group_write(&run(serde_json::json!({}), Some("off"), true)), &serde_json::json!({"notifPromote": false}));
    }

    #[test]
    fn a_plain_member_is_rejected() {
        let r = run(serde_json::json!({}), Some("on"), false);
        assert_eq!(text_of(&r), ADMIN_ONLY);
        assert!(r.mutations().is_empty());
    }
}