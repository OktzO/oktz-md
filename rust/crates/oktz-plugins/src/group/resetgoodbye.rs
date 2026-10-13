//! `plugins/group/resetgoodbye.js` — `groups[chat].goodbyeMsg = null`.
//!
//! Simetris dengan [`crate::group::resetwelcome`]: guard-nya `!groupData?.goodbyeMsg`,
//! jadi falsy JS (`undefined`/`null`/`""`/`0`/`false`) sudah dianggap default.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_falsy;

pub(crate) const ALREADY_DEFAULT: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Goodbye message sudah default";

pub(crate) const DONE: &str = "✅ *ɢᴏᴏᴅʙʏᴇ ᴅɪʀᴇsᴇᴛ*\nKembali ke pesan default";

#[plugin(
    name = "resetgoodbye",
    alias = ["delgoodbye", "cleargoodbye"],
    category = "group",
    description = "Reset goodbye message ke default",
    usage = ".resetgoodbye",
    example = ".resetgoodbye",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn resetgoodbye(ctx: Ctx) -> Reply {
    if js_falsy(ctx.group_field("goodbyeMsg")) {
        return Reply::text(ALREADY_DEFAULT);
    }
    Reply::text(DONE)
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "goodbyeMsg": serde_json::Value::Null }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::{Value, json};

    fn run(group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resetgoodbye".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("resetgoodbye").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
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
        let p = oktz_plugin_api::find("resetgoodbye").unwrap();
        assert_eq!(p.meta.name, "resetgoodbye");
        assert_eq!(p.meta.alias, ["delgoodbye", "cleargoodbye"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Reset goodbye message ke default");
        assert_eq!(p.meta.usage, ".resetgoodbye");
        assert_eq!(p.meta.example, ".resetgoodbye");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin, "isBotAdmin tidak ada di pluginConfig JS");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delgoodbye", "cleargoodbye"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "resetgoodbye");
        }
    }

    #[test]
    fn an_absent_or_falsy_message_is_reported_as_default() {
        for g in [
            json!(null),
            json!({}),
            json!({"jid": "120363@g.us"}),
            json!({"goodbyeMsg": null}),
            json!({"goodbyeMsg": ""}),
            json!({"goodbyeMsg": 0}),
            json!({"goodbyeMsg": false}),
        ] {
            let r = run(g.clone());
            assert_eq!(text_of(&r), ALREADY_DEFAULT, "input {g}");
            assert!(r.mutations().is_empty(), "tidak boleh menulis: {g}");
        }
    }

    #[test]
    fn an_empty_object_is_truthy_in_js_so_the_reset_still_runs() {
        let r = run(json!({"goodbyeMsg": {}}));
        assert_eq!(text_of(&r), DONE);
        assert_eq!(group_write(&r)["goodbyeMsg"], Value::Null);
    }

    #[test]
    fn a_stored_message_is_cleared_to_null_not_deleted() {
        let r = run(json!({"goodbyeMsg": "Sampai jumpa"}));
        assert_eq!(group_write(&r), &json!({"goodbyeMsg": null}));
        assert_eq!(group_write(&r).as_object().unwrap().len(), 1);
        assert_eq!(text_of(&r), DONE);
    }

    #[test]
    fn the_reset_reacts_with_a_check_mark() {
        match run(json!({"goodbyeMsg": "x"})).without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }
}