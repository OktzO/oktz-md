//! `plugins/group/resetrulesgrup.js` — `groups[chat].groupRules = null`.
//!
//! Tanpa cabang: plugin ini SELALU menulis dan SELALU melaporkan sukses.
//! Quietly berbeda dari pasangan `setwelcome`/`resetwelcome` yang memeriksa
//! state lebih dulu — itu perilaku JS, bukan kelalaian port.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) fn confirm(prefix: &str) -> String {
    format!(
        "✅ *ɢʀᴜᴘ ʀᴜʟᴇs ᴅɪʀᴇsᴇᴛ*\n\
         Rules grup berhasil direset ke default!\n\
         Ketik `{prefix}rulesgrup` untuk melihat."
    )
}

#[plugin(
    name = "resetrulesgrup",
    alias = ["resetgrouprules"],
    category = "group",
    description = "Reset rules grup ke default (admin only)",
    usage = ".resetrulesgrup",
    example = ".resetrulesgrup",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn resetrulesgrup(ctx: Ctx) -> Reply {
    Reply::text(confirm(&ctx.prefix)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "groupRules": serde_json::Value::Null }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use serde_json::json;

    fn run(group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resetrulesgrup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("resetrulesgrup").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("resetrulesgrup").unwrap();
        assert_eq!(p.meta.name, "resetrulesgrup");
        assert_eq!(p.meta.alias, ["resetgrouprules"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Reset rules grup ke default (admin only)");
        assert_eq!(p.meta.usage, ".resetrulesgrup");
        assert_eq!(p.meta.example, ".resetrulesgrup");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("resetgrouprules").unwrap().meta.name,
            "resetrulesgrup"
        );
    }

    #[test]
    fn it_always_writes_even_when_nothing_was_set() {
        // Tidak ada pemeriksaan state di JS — reset tetap menulis.
        for g in [json!(null), json!({}), json!({"groupRules": null}), json!({"groupRules": "lama"})] {
            let r = run(g.clone());
            assert_eq!(group_write(&r)["groupRules"], Value::Null, "input {g}");
            assert!(text_of(&r).contains("berhasil direset"), "input {g}");
        }
    }

    #[test]
    fn only_group_rules_is_written() {
        let r = run(json!({"groupRules": "lama", "game": true}));
        let data = group_write(&r);
        assert_eq!(data.as_object().unwrap().len(), 1);
        assert_eq!(data["groupRules"], Value::Null);
    }

    #[test]
    fn the_confirm_reply_names_the_live_prefix_and_the_reader_command() {
        assert!(
            confirm("~").contains("`~rulesgrup`"),
            "{}",
            confirm("~")
        );
    }

    #[test]
    fn the_success_reply_is_always_paired_with_the_write() {
        for g in [json!({}), json!({"groupRules": "x"})] {
            let label = g.to_string();
            let r = run(g);
            assert!(text_of(&r).contains("berhasil direset ke default"));
            assert_eq!(r.mutations().len(), 1, "input {label}");
        }
    }

    #[test]
    fn set_and_reset_rules_keep_separate_keys() {
        for (name, keys) in [
            (
                "setrulesgrup",
                &["setrulesgrup", "setgrouprules", "setaturangrup"][..],
            ),
            ("resetrulesgrup", &["resetrulesgrup", "resetgrouprules"][..]),
        ] {
            for k in keys {
                let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(k))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(owners, [name], "key {k} diklaim oleh {owners:?}");
            }
        }
    }
}