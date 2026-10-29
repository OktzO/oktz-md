//! `plugins/group/autodl.js` — sakelar auto download link sosmed.
//!
//! `ponytail:` `pluginConfig.name` di JS adalah array
//! `['autodl', 'autodownload']`. `PluginMeta.name` hanya `String`, jadi
//! elemen pertama jadi `name` dan sisanya jadi `alias` — registry
//! `matches()` tetap menerima keduanya.
//!
//! Penegakan `m.text` dipindah tidak: filtering link dan unduhan berjalan di
//! middleware/handler lain, bukan di `.autodl`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *ᴀᴜᴛᴏ ᴅᴏᴡɴʟᴏᴀᴅ ᴀᴋᴛɪꜰ*\n\n> Kirim link sosmed dan bot akan auto download!\n> Support: TikTok, IG, FB, YouTube, Twitter/X";
pub(crate) const OFF: &str = "❌ *ᴀᴜᴛᴏ ᴅᴏᴡɴʟᴏᴀᴅ ɴᴏɴᴀᴋᴛɪꜰ*";
pub(crate) const BAD_ARG: &str = "❌ *ᴀʀɢᴜᴍᴇɴ ᴛɪᴅᴀᴋ ᴠᴀʟɪᴅ*\n\n> Gunakan: `on` atau `off`";

#[plugin(
    name = "autodl",
    alias = ["autodownload"],
    category = "group",
    description = "Toggle auto download link sosmed",
    usage = ".autodl on/off",
    example = ".autodl on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn autodl(ctx: Ctx) -> Reply {
    let arg = ctx
        .args
        .first()
        .map(|a| a.to_lowercase())
        .unwrap_or_default();

    let current = crate::common::js_truthy(ctx.group_field("autodl"));

    if arg.is_empty() || arg == "status" {
        return Reply::text(format!(
            "🔗 *ᴀᴜᴛᴏ ᴅᴏᴡɴʟᴏᴀᴅ*\n\n\
             > Status: {}\n\n\
             *Platform Support:*\n\
             > TikTok, Instagram, Facebook\n\
             > YouTube, Twitter/X\n\
             > Telegram, Discord\n\n\
             *Penggunaan:*\n\
             > `{}autodl on` - Aktifkan\n\
             > `{}autodl off` - Nonaktifkan",
            if current { "✅ Aktif" } else { "❌ Nonaktif" },
            ctx.prefix,
            ctx.prefix
        ));
    }

    let write = |message: &str, on: bool| {
        Reply::text(message).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "autodl": on }),
        })
    };

    match arg.as_str() {
        "on" => write(ON, true).with_reaction("✅"),
        "off" => write(OFF, false).with_reaction("❌"),
        _ => Reply::text(BAD_ARG),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autodl".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("autodl").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("autodl").unwrap();
        // `name` di JS adalah array `['autodl', 'autodownload']`.
        assert_eq!(p.meta.name, "autodl");
        assert_eq!(p.meta.alias, ["autodownload"]);
        assert_eq!(p.meta.description, "Toggle auto download link sosmed");
        assert_eq!(p.meta.usage, ".autodl on/off");
        assert_eq!(p.meta.example, ".autodl on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        assert!(!p.meta.is_bot_admin, "JS: isBotAdmin: false");
    }

    #[test]
    fn both_names_from_the_js_array_resolve() {
        for c in ["autodl", "autodownload"] {
            assert_eq!(oktz_plugin_api::find(c).unwrap().meta.name, "autodl");
        }
    }

    #[test]
    fn a_missing_group_reads_as_inactive_and_writes_nothing() {
        let r = run(serde_json::json!({}), "");
        assert!(
            text_of(&r).contains("> Status: ❌ Nonaktif"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_status_keyword_behaves_like_no_argument() {
        let a = run(serde_json::json!({"autodl": true}), "");
        let b = run(serde_json::json!({"autodl": true}), "status");
        assert_eq!(text_of(&a), text_of(&b));
        assert!(text_of(&a).contains("> Status: ✅ Aktif"), "{}", text_of(&a));
        assert!(b.mutations().is_empty());
    }

    #[test]
    fn on_and_off_write_a_boolean_and_react() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "autodl": true }));
        match on.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }

        let off = run(serde_json::json!({"autodl": true}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "autodl": false }));
        match off.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "❌"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn an_unknown_argument_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), BAD_ARG);
        assert!(r.mutations().is_empty());
        assert!(!matches!(r.without_mutations(), Reply::Reaction { .. }));
    }

    #[test]
    fn the_argument_is_lowercased_before_matching() {
        assert_eq!(written(&run(serde_json::json!({}), "ON")), &serde_json::json!({ "autodl": true }));
    }

    #[test]
    fn a_non_boolean_stored_value_counts_as_active_like_javascript() {
        let r = run(serde_json::json!({"autodl": "ya"}), "");
        assert!(text_of(&r).contains("> Status: ✅ Aktif"), "{}", text_of(&r));
    }
}