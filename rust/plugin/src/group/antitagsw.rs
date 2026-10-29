//! `plugins/group/antitagsw.js` — sakelar anti tag status.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str =
    "✅ *ᴀɴᴛɪᴛᴀɢsᴡ ᴀᴋᴛɪꜰ*\n\n> Anti tag status berhasil diaktifkan!\n> Pesan tag status akan dihapus otomatis.";
pub(crate) const OFF: &str = "❌ *ᴀɴᴛɪᴛᴀɢsᴡ ɴᴏɴᴀᴋᴛɪꜰ*\n\n> Anti tag status berhasil dinonaktifkan.";
pub(crate) const BAD_ACTION: &str = "❌ *ᴘɪʟɪʜᴀɴ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n> Gunakan: on atau off";

#[plugin(
    name = "antitagsw",
    alias = ["antitag", "antistatustag"],
    category = "group",
    description = "Mengaktifkan/menonaktifkan anti tag status di grup",
    usage = ".antitagsw <on/off>",
    example = ".antitagsw on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antitagsw(ctx: Ctx) -> Reply {
    let action = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if action.is_empty() {
        let status = ctx
            .group_field("antitagsw")
            .and_then(Value::as_str)
            .unwrap_or("off");
        return Reply::text(format!(
            "📢 *ᴀɴᴛɪᴛᴀɢsᴡ sᴇᴛᴛɪɴɢs*\n\n\
             > Status: *{}*\n\n\
             > Fitur ini menghapus pesan tag status\n\
             > (groupStatusMentionMessage)\n\n\
             ```━━━ ᴘɪʟɪʜᴀɴ ━━━```\n\
             > `{}antitagsw on` → Aktifkan\n\
             > `{}antitagsw off` → Nonaktifkan",
            if status == "on" { "✅ Aktif" } else { "❌ Nonaktif" },
            ctx.prefix,
            ctx.prefix
        ));
    }

    let write = |message: &str, status: &str| {
        Reply::text(message).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "antitagsw": status }),
        })
    };

    match action.as_str() {
        "on" => write(ON, "on"),
        "off" => write(OFF, "off"),
        _ => Reply::text(BAD_ACTION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antitagsw".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antitagsw").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antitagsw").unwrap();
        assert_eq!(p.meta.name, "antitagsw");
        assert_eq!(p.meta.alias, ["antitag", "antistatustag"]);
        assert_eq!(
            p.meta.description,
            "Mengaktifkan/menonaktifkan anti tag status di grup"
        );
        assert_eq!(p.meta.usage, ".antitagsw <on/off>");
        assert_eq!(p.meta.example, ".antitagsw on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["antitag", "antistatustag"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antitagsw");
        }
    }

    #[test]
    fn no_argument_reports_status_and_writes_nothing() {
        let r = run(serde_json::json!({}), "");
        assert!(
            text_of(&r).contains("> Status: *❌ Nonaktif*"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("antitagsw on"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn only_the_exact_string_on_counts_as_active() {
        assert!(text_of(&run(serde_json::json!({"antitagsw": "on"}), "")).contains("✅ Aktif"));
        assert!(text_of(&run(serde_json::json!({"antitagsw": true}), "")).contains("❌ Nonaktif"));
    }

    #[test]
    fn on_and_off_write_a_string_and_do_not_react() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "antitagsw": "on" }));
        assert!(!matches!(on.without_mutations(), Reply::Reaction { .. }));

        let off = run(serde_json::json!({"antitagsw": "on"}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "antitagsw": "off" }));
    }

    #[test]
    fn an_unknown_action_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_ACTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_action_is_lowercased_before_matching() {
        assert_eq!(written(&run(serde_json::json!({}), "Off")), &serde_json::json!({ "antitagsw": "off" }));
    }
}