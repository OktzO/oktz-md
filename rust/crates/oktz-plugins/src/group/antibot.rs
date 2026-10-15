//! `plugins/group/antibot.js` — sakelar deteksi-kick bot per grup.
//!
//! Hanya handler `.antibot` yang dipindah. Dua export lain tidak:
//! - `isBotMessage`/`analyzeBotMessage` memeriksa `m.key`, `m.message`,
//!   `contextInfo`, dan `pushName` — `Ctx` tidak membawa field itu.
//! - `detectBot` butuh `sock.sendMessage({delete})` (:267, :269),
//!   `sock.groupParticipantsUpdate` (:278), dan `sock.user.id` (:242).
//! - `gpMsg` hanya dipakai `detectBot`, jadi ikut tidak dipindah.
//!
//! `pluginConfig.name` di JS berupa array `["antibot", "botdetect"]` dengan
//! `alias: []`; elemen pertama jadi nama, sisanya jadi alias.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const ON: &str = "✅ *AntiBot diaktifkan*";
const OFF: &str = "❌ *AntiBot dinonaktifkan*";
const BAD_ACTION: &str = "❌ Gunakan `.antibot on` atau `.antibot off`";

fn status(current: bool) -> String {
    format!(
        "🤖 *AntiBot*\n\n> Status: {}\n\n> Deteksi: *Smart Heuristic*\n\n> `.antibot on/off`",
        if current { "✅ Aktif" } else { "❌ Nonaktif" }
    )
}

fn write(ctx: &Ctx, message: &str, on: bool) -> Reply {
    Reply::text(message).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "antibot": on }),
    })
}

#[plugin(
    name = "antibot",
    alias = ["botdetect"],
    category = "group",
    description = "Deteksi dan kick bot WhatsApp (baileys) dari grup",
    usage = ".antibot <on/off>",
    example = ".antibot on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antibot(ctx: Ctx) -> Reply {
    let arg = ctx.args.first().map(|a| a.to_lowercase());
    // `groupData.antibot || false` — `||`, jadi nilai falsy apa pun jadi false.
    let current = crate::common::js_truthy(ctx.group_field("antibot"));

    match arg.as_deref() {
        None => Reply::text(status(current)),
        Some("status") => Reply::text(status(current)),
        Some("on") => write(&ctx, ON, true).with_reaction("✅"),
        Some("off") => write(&ctx, OFF, false).with_reaction("❌"),
        Some(_) => Reply::text(BAD_ACTION),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antibot".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antibot").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antibot").unwrap();
        assert_eq!(p.meta.name, "antibot");
        assert_eq!(p.meta.alias, ["botdetect"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Deteksi dan kick bot WhatsApp (baileys) dari grup"
        );
        assert_eq!(p.meta.usage, ".antibot <on/off>");
        assert_eq!(p.meta.example, ".antibot on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn the_second_entry_of_the_js_name_array_is_the_alias() {
        assert_eq!(
            oktz_plugin_api::find("botdetect").unwrap().meta.name,
            "antibot"
        );
    }

    #[test]
    fn no_argument_and_status_both_only_report() {
        for text in ["", "status"] {
            let r = run(json!({}), text);
            assert_eq!(text_of(&r), status(false), "input {text:?}");
            assert!(r.mutations().is_empty(), "status tidak boleh menulis");
        }
    }

    #[test]
    fn status_reflects_the_stored_flag() {
        let r = run(json!({ "antibot": true }), "");
        assert!(text_of(&r).contains("> Status: ✅ Aktif"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_falsy_stored_value_reports_nonaktif() {
        // `groupData.antibot || false` — `0`, `""`, dan `null` semuanya falsy.
        for stored in [json!(null), json!(false), json!(0), json!("")] {
            let r = run(json!({ "antibot": stored }), "");
            assert!(
                text_of(&r).contains("> Status: ❌ Nonaktif"),
                "stored {stored} harus nonaktif: {}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn on_and_off_write_a_boolean_and_react() {
        let on = run(json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &json!({ "antibot": true }));

        let off = run(json!({ "antibot": true }), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &json!({ "antibot": false }));
    }

    #[test]
    fn each_write_branch_carries_its_own_reaction() {
        for (text, emoji) in [("on", "✅"), ("off", "❌")] {
            match run(json!({}), text).without_mutations() {
                Reply::Reaction { emoji: e, .. } => assert_eq!(e, emoji, "{text}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn the_action_is_lowercased_before_matching() {
        assert_eq!(
            written(&run(json!({}), "ON")),
            &json!({ "antibot": true })
        );
        assert_eq!(
            written(&run(json!({}), "Off")),
            &json!({ "antibot": false })
        );
    }

    #[test]
    fn an_unknown_action_is_refused_and_nothing_is_written() {
        let r = run(json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_ACTION);
        assert!(r.mutations().is_empty());
    }
}