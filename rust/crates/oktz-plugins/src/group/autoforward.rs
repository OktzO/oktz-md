//! `plugins/group/autoforward.js` — sakelar auto forward pesan masuk ke grup.
//!
//! `ponytail:` penerusan pesan nyata ada di middleware dan butuh
//! `sock.sendMessage`/`sock.sendMessage` grup lain, yang tidak ada di `Ctx`.
//! Yang dipindah hanya sakelar `.autoforward`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const BAD_OPTION: &str = "❌ Gunakan: on atau off";

#[plugin(
    name = "autoforward",
    alias = ["autofw", "autofwd"],
    category = "group",
    description = "Auto forward pesan yang masuk ke grup ke grup ini",
    usage = ".autoforward <on/off>",
    example = ".autoforward on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn autoforward(ctx: Ctx) -> Reply {
    let option = ctx.text.to_lowercase().trim().to_string();

    if option.is_empty() {
        let on = crate::common::js_truthy(ctx.group_field("autoforward"));
        return Reply::text(format!(
            "🔄 *ᴀᴜᴛᴏ ꜰᴏʀᴡᴀʀᴅ*\n\n\
             ╭┈┈⬡「 📋 *ɪɴꜰᴏ* 」\n\
             ┃ ◦ Status: *{}*\n\
             ╰┈┈⬡\n\n\
             > Gunakan `{}autoforward on/off`\n\n\
             _Fitur ini akan meneruskan semua pesan ke grup ini_",
            if on { "✅ ON" } else { "❌ OFF" },
            ctx.prefix
        ));
    }

    match option.as_str() {
        "on" => Reply::text(
            "🔄 *ᴀᴜᴛᴏ ꜰᴏʀᴡᴀʀᴅ*\n\n\
             ╭┈┈⬡「 ✅ *ᴀᴋᴛɪꜰ* 」\n\
             ┃ ◦ Status: *ON*\n\
             ╰┈┈⬡\n\n\
             > _Semua pesan akan di-forward_",
        )
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "autoforward": true }),
        }),
        "off" => Reply::text(
            "🔄 *ᴀᴜᴛᴏ ꜰᴏʀᴡᴀʀᴅ*\n\n\
             ╭┈┈⬡「 ❌ *ɴᴏɴᴀᴋᴛɪꜰ* 」\n\
             ┃ ◦ Status: *OFF*\n\
             ╰┈┈⬡",
        )
        .with_reaction("❌")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "autoforward": false }),
        }),
        _ => Reply::text(BAD_OPTION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autoforward".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("autoforward").unwrap().handler)(ctx)
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

    fn reaction(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autoforward").unwrap();
        assert_eq!(p.meta.name, "autoforward");
        assert_eq!(p.meta.alias, ["autofw", "autofwd"]);
        assert_eq!(
            p.meta.description,
            "Auto forward pesan yang masuk ke grup ke grup ini"
        );
        assert_eq!(p.meta.usage, ".autoforward <on/off>");
        assert_eq!(p.meta.example, ".autoforward on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        assert!(!p.meta.is_bot_admin, "tidak ada di pluginConfig");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["autofw", "autofwd"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "autoforward");
        }
    }

    #[test]
    fn no_argument_reports_status_without_writing() {
        let off = run(serde_json::json!({}), "");
        assert!(text_of(&off).contains("┃ ◦ Status: *❌ OFF*"), "{}", text_of(&off));
        assert!(off.mutations().is_empty());

        let on = run(serde_json::json!({"autoforward": true}), "");
        assert!(text_of(&on).contains("┃ ◦ Status: *✅ ON*"), "{}", text_of(&on));
        assert!(on.mutations().is_empty());
    }

    #[test]
    fn a_non_boolean_stored_value_counts_as_active_like_javascript() {
        let r = run(serde_json::json!({"autoforward": "ya"}), "");
        assert!(text_of(&r).contains("┃ ◦ Status: *✅ ON*"), "{}", text_of(&r));
    }

    #[test]
    fn on_writes_true_and_reacts_with_a_check() {
        let r = run(serde_json::json!({}), "on");
        assert_eq!(written(&r), &serde_json::json!({ "autoforward": true }));
        assert_eq!(reaction(&r), "✅");
        assert!(text_of(&r).contains("✅ *ᴀᴋᴛɪꜰ*"), "{}", text_of(&r));
    }

    #[test]
    fn off_writes_false_and_reacts_with_a_cross() {
        let r = run(serde_json::json!({"autoforward": true}), "off");
        assert_eq!(written(&r), &serde_json::json!({ "autoforward": false }));
        assert_eq!(reaction(&r), "❌");
        assert!(text_of(&r).contains("❌ *ɴᴏɴᴀᴋᴛɪꜰ*"), "{}", text_of(&r));
    }

    #[test]
    fn an_unknown_option_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_OPTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn options_are_matched_after_lowercase_and_trim() {
        assert_eq!(
            written(&run(serde_json::json!({}), "  ON  ")),
            &serde_json::json!({ "autoforward": true })
        );
    }
}