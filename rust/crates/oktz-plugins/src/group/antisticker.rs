//! `plugins/group/antisticker.js` — sakelar antiblokir stiker.
//!
//! `ponytail:` hanya handler `.antisticker` yang dipindah. `checkAntisticker`
//! (pemeriksaan per-pesan) butuh `sock.sendMessage` untuk menghapus pesan —
//! `Ctx` tidak punya itu. `gpMsg` yang membaca `config.groupProtection` ikut
//! tidak dipindah karena hanya dipakai `checkAntisticker`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *AntiSticker diaktifkan*";
pub(crate) const OFF: &str = "❌ *AntiSticker dinonaktifkan*";
pub(crate) const BAD_ACTION: &str = "❌ Gunakan `.antisticker on` atau `.antisticker off`";

fn write(ctx: &Ctx, message: &str, on: bool) -> Reply {
    Reply::text(message).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "antisticker": on }),
    })
}

#[plugin(
    name = "antisticker",
    alias = ["nosticker"],
    category = "group",
    description = "Mengatur antisticker di grup",
    usage = ".antisticker <on/off>",
    example = ".antisticker on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antisticker(ctx: Ctx) -> Reply {
    let action = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    if action.is_empty() {
        let on = crate::common::js_truthy(ctx.group_field("antisticker"));
        return Reply::text(format!(
            "🎭 *AntiSticker*\n\n> Status: *{}*\n\n> `.antisticker on/off`",
            if on { "✅ ON" } else { "❌ OFF" }
        ));
    }

    match action.as_str() {
        "on" => write(&ctx, ON, true).with_reaction("✅"),
        "off" => write(&ctx, OFF, false).with_reaction("❌"),
        _ => Reply::text(BAD_ACTION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antisticker".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antisticker").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antisticker").unwrap();
        assert_eq!(p.meta.name, "antisticker");
        assert_eq!(p.meta.alias, ["nosticker"]);
        assert_eq!(p.meta.description, "Mengatur antisticker di grup");
        assert_eq!(p.meta.usage, ".antisticker <on/off>");
        assert_eq!(p.meta.example, ".antisticker on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("nosticker").unwrap().meta.name,
            "antisticker"
        );
    }

    #[test]
    fn no_argument_reports_status_without_writing() {
        let r = run(serde_json::json!({}), "");
        assert!(
            text_of(&r).contains("> Status: *❌ OFF*"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());

        let on = run(serde_json::json!({"antisticker": true}), "");
        assert!(text_of(&on).contains("> Status: *✅ ON*"), "{}", text_of(&on));
        assert!(on.mutations().is_empty());
    }

    #[test]
    fn on_and_off_write_a_boolean() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "antisticker": true }));

        let off = run(serde_json::json!({"antisticker": true}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "antisticker": false }));
    }

    #[test]
    fn each_branch_carries_its_own_reaction() {
        for (text, emoji) in [("on", "✅"), ("off", "❌")] {
            match run(serde_json::json!({}), text).without_mutations() {
                Reply::Reaction { emoji: e, .. } => assert_eq!(e, emoji, "{text}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn an_unknown_action_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_ACTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_action_is_lowercased_before_matching() {
        assert_eq!(
            written(&run(serde_json::json!({}), "OFF")),
            &serde_json::json!({ "antisticker": false })
        );
    }
}