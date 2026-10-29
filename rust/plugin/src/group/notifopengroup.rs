//! `plugins/group/notifopengroup.js` — toggle `groups[chat].notifOpenGroup`.
//!
//! Tubuh bersama dengan tiga plugin `notif*` lain ada di
//! [`crate::group::notifclosegroup`].

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::group::notifclosegroup::{NotifToggle, notif_toggle};

#[plugin(
    name = "notifopengroup",
    alias = ["notifopen"],
    category = "group",
    description = "Toggle notifikasi saat grup dibuka",
    usage = ".notifopengroup on/off",
    example = ".notifopengroup on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn notifopengroup(ctx: Ctx) -> Reply {
    notif_toggle(
        &ctx,
        &NotifToggle {
            field: "notifOpenGroup",
            title: "ɴᴏᴛɪꜰ ᴏᴘᴇɴ ɢʀᴏᴜᴘ",
            emoji: "🔓",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::group::notifclosegroup::ADMIN_ONLY;
    use oktz_plugin_api::{Mutation, Participant};
    use serde_json::{Value};

    fn run(group: Value, arg: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notifopengroup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: true,
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
        (oktz_plugin_api::find("notifopengroup").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("notifopengroup").unwrap();
        assert_eq!(p.meta.name, "notifopengroup");
        assert_eq!(p.meta.alias, ["notifopen"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Toggle notifikasi saat grup dibuka");
        assert_eq!(p.meta.usage, ".notifopengroup on/off");
        assert_eq!(p.meta.example, ".notifopengroup on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("notifopen").unwrap().meta.name, "notifopengroup");
    }

    #[test]
    fn the_status_header_uses_the_open_title_and_unlock() {
        let r = run(serde_json::json!({"notifOpenGroup": true}), None);
        assert!(
            text_of(&r).starts_with("🔓 *ɴᴏᴛɪꜰ ᴏᴘᴇɴ ɢʀᴏᴜᴘ*\n\n> Status: ✅ Aktif"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("`.notifopengroup off` - Nonaktifkan"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn on_and_off_use_this_plugin_own_wording() {
        assert_eq!(
            text_of(&run(serde_json::json!({}), Some("on"))),
            "✅ *ɴᴏᴛɪꜰ ᴏᴘᴇɴ ɢʀᴏᴜᴘ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*"
        );
        assert_eq!(group_write(&run(serde_json::json!({}), Some("on"))), &serde_json::json!({"notifOpenGroup": true}));
        assert_eq!(
            text_of(&run(serde_json::json!({}), Some("off"))),
            "❌ *ɴᴏᴛɪꜰ ᴏᴘᴇɴ ɢʀᴏᴜᴘ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*"
        );
        assert_eq!(group_write(&run(serde_json::json!({}), Some("off"))), &serde_json::json!({"notifOpenGroup": false}));
    }

    #[test]
    fn a_stored_non_bool_never_reports_aktif() {
        let r = run(serde_json::json!({"notifOpenGroup": "true"}), None);
        assert!(text_of(&r).contains("> Status: ❌ Nonaktif"), "{}", text_of(&r));
    }

    #[test]
    fn the_admin_guard_is_shared_with_the_other_three() {
        let ctx = Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: vec!["on".into()],
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("notifopengroup").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), ADMIN_ONLY);
        assert!(r.mutations().is_empty());
    }
}