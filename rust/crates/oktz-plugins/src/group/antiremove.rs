//! `plugins/group/antiremove.js` — sakelar anti hapus pesan.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *AntiRemove diaktifkan*\n> Pesan yang dihapus akan di-forward ulang.";
pub(crate) const OFF: &str = "❌ *AntiRemove dinonaktifkan*";
pub(crate) const BAD_ACTION: &str = "❌ Gunakan `.antiremove on` atau `.antiremove off`";

#[plugin(
    name = "antiremove",
    alias = ["antidelete", "antihapus"],
    category = "group",
    description = "Mengaktifkan/menonaktifkan anti hapus pesan di grup",
    usage = ".antiremove <on/off>",
    example = ".antiremove on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn antiremove(ctx: Ctx) -> Reply {
    let action = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if action.is_empty() {
        let status = ctx
            .group_field("antiremove")
            .and_then(Value::as_str)
            .unwrap_or("off");
        return Reply::text(format!(
            "🗑️ *AntiRemove*\n\n> Status: *{}*\n\n> `.antiremove on/off`",
            if status == "on" { "✅ Aktif" } else { "❌ Nonaktif" }
        ));
    }

    // `db.setGroup(m.chat, { ...group, antiremove })` di JS menyalin grup
    // utuh; `Mutation::SetGroup` sudah berupa patch sebagian, jadi field lain
    // tidak perlu ditulis ulang.
    let write = |message: &str, status: &str| {
        Reply::text(message).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "antiremove": status }),
        })
    };

    match action.as_str() {
        "on" => write(ON, "on").with_reaction("✅"),
        "off" => write(OFF, "off").with_reaction("❌"),
        _ => Reply::text(BAD_ACTION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antiremove".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antiremove").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
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
        let p = oktz_plugin_api::find("antiremove").unwrap();
        assert_eq!(p.meta.name, "antiremove");
        assert_eq!(p.meta.alias, ["antidelete", "antihapus"]);
        assert_eq!(
            p.meta.description,
            "Mengaktifkan/menonaktifkan anti hapus pesan di grup"
        );
        assert_eq!(p.meta.usage, ".antiremove <on/off>");
        assert_eq!(p.meta.example, ".antiremove on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        assert!(
            !p.meta.is_bot_admin,
            "pluginConfig hanya isBotAdmin: false"
        );
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["antidelete", "antihapus"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antiremove");
        }
    }

    #[test]
    fn a_missing_setting_reads_as_off_without_writing() {
        let r = run(json!({}), "");
        assert!(
            text_of(&r).contains("> Status: *❌ Nonaktif*"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn only_the_exact_string_on_counts_as_active() {
        // JS `status === 'on'` — nilai lain (termasuk boolean `true`) jatuh ke
        // cabang nonaktif, bukan dianggap aktif.
        assert!(
            text_of(&run(json!({"antiremove": "on"}), "")).contains("✅ Aktif"),
            "{}",
            text_of(&run(json!({"antiremove": "on"}), ""))
        );
        for group in [json!({"antiremove": true}), json!({"antiremove": "ON"})] {
            let r = run(group.clone(), "");
            assert!(
                text_of(&r).contains("❌ Nonaktif"),
                "{group}: {}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn on_and_off_write_a_string_not_a_boolean() {
        let on = run(json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &json!({ "antiremove": "on" }));

        let off = run(json!({"antiremove": "on"}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &json!({ "antiremove": "off" }));
    }

    #[test]
    fn each_branch_carries_its_own_reaction() {
        for (text, emoji) in [("on", "✅"), ("off", "❌")] {
            match run(json!({}), text).without_mutations() {
                Reply::Reaction { emoji: e, .. } => assert_eq!(e, emoji, "{text}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn an_unknown_action_is_refused_and_nothing_is_written() {
        let r = run(json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_ACTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn other_group_fields_are_left_alone_by_the_patch() {
        let r = run(json!({"antiremove": "off", "welcome": true}), "on");
        assert_eq!(written(&r), &json!({ "antiremove": "on" }));
    }
}