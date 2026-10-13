//! `plugins/group/unmute.js` — `groups[chat].mute = false`.
//!
//! Simetris dengan [`crate::group::mute`]: nama grup diambil dari
//! `groups[chat].name`, lihat catatan `ponytail:` di sana.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{jid_number, js_truthy};
use crate::group::mute::group_name;

pub(crate) const NOT_MUTED: &str = "❌ Grup tidak sedang di-mute.";

#[plugin(
    name = "unmute",
    alias = ["unbisukan"],
    category = "group",
    description = "Membuka mute grup",
    usage = ".unmute",
    example = ".unmute",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn unmute(ctx: Ctx) -> Reply {
    if !js_truthy(ctx.group_field("mute")) {
        return Reply::text(NOT_MUTED);
    }
    Reply::Text {
        text: format!(
            "✅ Grup *{}* berhasil di-unmute oleh @{}\n\nSemua member sekarang bisa mengirim pesan.",
            group_name(&ctx),
            jid_number(&ctx.sender)
        ),
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "mute": false }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(group: serde_json::Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "unmute".into(),
            sender: "628111:3@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("unmute").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &serde_json::Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("unmute").unwrap();
        assert_eq!(p.meta.name, "unmute");
        assert_eq!(p.meta.alias, ["unbisukan"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Membuka mute grup");
        assert_eq!(p.meta.usage, ".unmute");
        assert_eq!(p.meta.example, ".unmute");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("unbisukan").unwrap().meta.name, "unmute");
    }

    #[test]
    fn unmuting_writes_false_and_mentions_the_sender() {
        let r = run(json!({"name": "Grup Diskusi", "mute": true}));
        assert_eq!(
            text_of(&r),
            "✅ Grup *Grup Diskusi* berhasil di-unmute oleh @628111:3\n\n\
             Semua member sekarang bisa mengirim pesan."
        );
        assert_eq!(group_write(&r), &json!({"mute": false}));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &["628111:3@s.whatsapp.net".to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_group_that_is_not_muted_is_left_alone() {
        for falsy in [json!({}), json!({"mute": false}), json!({"mute": 0}), json!({"mute": ""})] {
            let r = run(falsy.clone());
            assert_eq!(text_of(&r), NOT_MUTED, "input {falsy}");
            assert!(r.mutations().is_empty(), "tidak boleh menulis: {falsy}");
        }
    }

    #[test]
    fn only_the_mute_flag_travels_to_the_store() {
        let r = run(json!({"mute": true, "antilink": true, "warnings": {}}));
        assert_eq!(
            group_write(&r).as_object().unwrap().len(),
            1,
            "field lain tetap di store, jadi tidak ikut dikirim"
        );
    }
}