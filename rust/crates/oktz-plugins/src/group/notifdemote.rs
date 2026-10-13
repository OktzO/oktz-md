//! `plugins/group/notifdemote.js` — toggle `groups[chat].notifDemote`.
//!
//! Tubuh bersama dengan tiga plugin `notif*` lain ada di
//! [`crate::group::notifclosegroup`].

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::group::notifclosegroup::{NotifToggle, notif_toggle};

#[plugin(
    name = "notifdemote",
    alias = [],
    category = "group",
    description = "Toggle notifikasi saat ada yang dicopot dari admin",
    usage = ".notifdemote on/off",
    example = ".notifdemote on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn notifdemote(ctx: Ctx) -> Reply {
    notif_toggle(
        &ctx,
        &NotifToggle {
            field: "notifDemote",
            title: "ɴᴏᴛɪꜰ ᴅᴇᴍᴏᴛᴇ",
            emoji: "👤",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::group::notifclosegroup::ADMIN_ONLY;
    use oktz_plugin_api::{Mutation, Participant};
    use serde_json::Value;
    use serde_json::json;

    fn run(group: Value, arg: Option<&str>, admin: bool) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notifdemote".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: arg.into_iter().map(String::from).collect(),
            group_data: group,
            roster: oktz_plugin_api::registry::GroupRoster {
                participants: vec![Participant {
                    jid: "628111@s.whatsapp.net".into(),
                    is_admin: admin,
                    is_owner: false,
                    data: Value::Null,
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        };
        (oktz_plugin_api::find("notifdemote").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("notifdemote").unwrap();
        assert_eq!(p.meta.name, "notifdemote");
        assert!(p.meta.alias.is_empty(), "alias di JS kosong");
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Toggle notifikasi saat ada yang dicopot dari admin");
        assert_eq!(p.meta.usage, ".notifdemote on/off");
        assert_eq!(p.meta.example, ".notifdemote on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn only_the_own_field_name_differs_between_the_four_toggles() {
        let r = run(json!({}), Some("on"), true);
        assert_eq!(group_write(&r), &json!({"notifDemote": true}));
    }

    #[test]
    fn the_status_header_uses_the_demote_title_and_avatar() {
        let r = run(json!({"notifDemote": true}), None, true);
        assert!(
            text_of(&r).starts_with("👤 *ɴᴏᴛɪꜰ ᴅᴇᴍᴏᴛᴇ*\n\n> Status: ✅ Aktif"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("`.notifdemote on` - Aktifkan"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn on_and_off_use_this_plugin_own_wording() {
        assert_eq!(
            text_of(&run(json!({}), Some("on"), true)),
            "✅ *ɴᴏᴛɪꜰ ᴅᴇᴍᴏᴛᴇ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*"
        );
        assert_eq!(
            text_of(&run(json!({}), Some("off"), true)),
            "❌ *ɴᴏᴛɪꜰ ᴅᴇᴍᴏᴛᴇ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*"
        );
        assert_eq!(group_write(&run(json!({}), Some("off"), true)), &json!({"notifDemote": false}));
    }

    #[test]
    fn a_plain_member_is_rejected() {
        let r = run(json!({}), Some("on"), false);
        assert_eq!(text_of(&r), ADMIN_ONLY);
        assert!(r.mutations().is_empty());
    }
}