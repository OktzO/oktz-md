//! `plugins/group/resetwelcome.js` — `groups[chat].welcomeMsg = null`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_falsy;

/// `if (!groupData?.welcomeMsg)` — `undefined`, `null`, dan `""` falsy, jadi
/// ketiganya masuk cabang "sudah default". `js_falsy` juga menandai `0` dan
/// `false`, sama seperti `!0`; objek dan array tetap truthy karena `!{}` =
/// `false` di JS.
pub(crate) fn is_default(ctx: &Ctx) -> bool {
    js_falsy(ctx.group_field("welcomeMsg"))
}

pub(crate) fn already_default() -> &'static str {
    "❌ *ɢᴀɢᴀʟ*\n\n> Welcome message sudah default"
}

pub(crate) const DONE: &str = "✅ *ᴡᴇʟᴄᴏᴍᴇ ᴅɪʀᴇsᴇᴛ*\n\n> Kembali ke pesan default";

#[plugin(
    name = "resetwelcome",
    alias = ["delwelcome", "clearwelcome"],
    category = "group",
    description = "Reset welcome message ke default",
    usage = ".resetwelcome",
    example = ".resetwelcome",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn resetwelcome(ctx: Ctx) -> Reply {
    if is_default(&ctx) {
        return Reply::text(already_default());
    }
    Reply::text(DONE)
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "welcomeMsg": serde_json::Value::Null }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resetwelcome".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("resetwelcome").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("resetwelcome").unwrap();
        assert_eq!(p.meta.name, "resetwelcome");
        assert_eq!(p.meta.alias, ["delwelcome", "clearwelcome"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Reset welcome message ke default");
        assert_eq!(p.meta.usage, ".resetwelcome");
        assert_eq!(p.meta.example, ".resetwelcome");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["delwelcome", "clearwelcome"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "resetwelcome");
        }
    }

    #[test]
    fn an_absent_or_empty_welcome_is_reported_as_already_default() {
        for g in [json!(null), json!({}), json!({"welcomeMsg": null}), json!({"welcomeMsg": ""})] {
            let r = run(g.clone());
            assert_eq!(text_of(&r), already_default(), "input {g}");
            assert!(r.mutations().is_empty(), "tidak boleh menulis: {g}");
        }
    }

    #[test]
    fn a_group_row_that_exists_but_has_no_welcome_key_is_also_default() {
        let r = run(json!({"jid": "120363@g.us", "welcome": true}));
        assert_eq!(text_of(&r), already_default());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn js_falsiness_decides_the_branch_not_the_json_type() {
        // `0` falsy → sudah default. `{}` TRUTHY di JS (`!{}` = false) →
        // plugin mencoba mereset, dan menulis `null` di atasnya. Mengira
        // "bukan string = tidak ada" akan salah untuk objek.
        let zero = run(json!({"welcomeMsg": 0}));
        assert_eq!(text_of(&zero), already_default(), "0 falsy di JS");
        assert!(zero.mutations().is_empty());

        let obj = run(json!({"welcomeMsg": {}}));
        assert_eq!(text_of(&obj), DONE, "objek kosong truthy di JS");
        assert_eq!(group_write(&obj)["welcomeMsg"], Value::Null);

        let empty_arr = run(json!({"welcomeMsg": []}));
        assert_eq!(text_of(&empty_arr), DONE, "array kosong truthy di JS");

        let empty_obj = run(json!({"welcomeMsg": ""}));
        assert_eq!(text_of(&empty_obj), already_default(), "string kosong falsy");

        for falsy in [json!(false), json!(null)] {
            let r = run(json!({ "welcomeMsg": falsy.clone() }));
            assert_eq!(text_of(&r), already_default(), "input {falsy}");
        }
    }

    #[test]
    fn a_stored_message_is_cleared_to_null_not_deleted() {
        let r = run(json!({"welcomeMsg": "Halo {user}"}));
        let data = group_write(&r);
        assert_eq!(
            data["welcomeMsg"],
            Value::Null,
            "JS menulis null, bukan menghapus key"
        );
        assert_eq!(
            data.as_object().unwrap().len(),
            1,
            "hanya welcomeMsg yang disentuh"
        );
        assert_eq!(text_of(&r), DONE);
    }

    #[test]
    fn the_reset_reply_is_paired_with_the_write() {
        let r = run(json!({"welcomeMsg": "x"}));
        assert!(text_of(&r).contains("ᴅɪʀᴇsᴇᴛ"));
        assert_eq!(r.mutations().len(), 1);
    }
}