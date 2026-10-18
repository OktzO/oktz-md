//! `plugins/group/antiswgc.js` — sakelar deteksi status/group mention.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *AntiSWGC aktif*\n\n> Tipe SW group mention akan dihapus otomatis.";
pub(crate) const OFF: &str = "❌ *AntiSWGC nonaktif*";
pub(crate) const BAD_ACTION: &str = "❌ Gunakan: on atau off";

#[plugin(
    name = "antiswgc",
    alias = ["antiswgroup", "antiswmentiongc", "antiswtaggc"],
    category = "group",
    description = "Deteksi tipe SW group mention atau status mention yang masuk ke grup",
    usage = ".antiswgc <on/off>",
    example = ".antiswgc on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antiswgc(ctx: Ctx) -> Reply {
    let action = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if action.is_empty() {
        let status = ctx
            .group_field("antiswgc")
            .and_then(Value::as_str)
            .unwrap_or("off");
        return Reply::text(format!(
            "📡 *ᴀɴᴛɪsᴡɢᴄ*\n\n\
             > Status: *{}*\n\n\
             > Fitur ini mendeteksi tipe SW group mention seperti:\n\
             > • groupStatusMentionMessage\n\
             > • groupMentionedMessage\n\
             > • statusMentionMessage\n\
             > • groupStatusMessage\n\
             > • contextInfo.groupMentions\n\n\
             > Admin dan pesan dari bot sendiri tidak akan disentuh.\n\n\
             > `{}antiswgc on`\n\
             > `{}antiswgc off`",
            if status == "on" { "✅ Aktif" } else { "❌ Nonaktif" },
            ctx.prefix,
            ctx.prefix
        ));
    }

    let write = |message: &str, status: &str| {
        Reply::text(message).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "antiswgc": status }),
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
            command: "antiswgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antiswgc").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antiswgc").unwrap();
        assert_eq!(p.meta.name, "antiswgc");
        assert_eq!(
            p.meta.alias,
            ["antiswgroup", "antiswmentiongc", "antiswtaggc"]
        );
        assert_eq!(
            p.meta.description,
            "Deteksi tipe SW group mention atau status mention yang masuk ke grup"
        );
        assert_eq!(p.meta.usage, ".antiswgc <on/off>");
        assert_eq!(p.meta.example, ".antiswgc on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["antiswgroup", "antiswmentiongc", "antiswtaggc"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antiswgc");
        }
    }

    #[test]
    fn no_argument_lists_the_detected_types_and_writes_nothing() {
        let r = run(serde_json::json!({}), "");
        assert!(
            text_of(&r).contains("> Status: *❌ Nonaktif*"),
            "{}",
            text_of(&r)
        );
        assert!(
            text_of(&r).contains("• groupStatusMentionMessage"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn only_the_exact_string_on_counts_as_active() {
        let r = run(serde_json::json!({"antiswgc": "on"}), "");
        assert!(text_of(&r).contains("✅ Aktif"), "{}", text_of(&r));

        let r = run(serde_json::json!({"antiswgc": true}), "");
        assert!(text_of(&r).contains("❌ Nonaktif"), "{}", text_of(&r));
    }

    #[test]
    fn on_and_off_write_a_string_and_do_not_react() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "antiswgc": "on" }));
        assert!(
            !matches!(on.without_mutations(), Reply::Reaction { .. }),
            "tidak ada m.react di JS untuk plugin ini"
        );

        let off = run(serde_json::json!({"antiswgc": "on"}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "antiswgc": "off" }));
    }

    #[test]
    fn an_unknown_action_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_ACTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_action_is_lowercased_before_matching() {
        assert_eq!(written(&run(serde_json::json!({}), "ON")), &serde_json::json!({ "antiswgc": "on" }));
    }
}