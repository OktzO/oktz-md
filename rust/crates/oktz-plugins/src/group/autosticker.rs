//! `plugins/group/autosticker.js` — sakelar ubah gambar/video jadi sticker.
//!
//! Hanya handler `.autosticker` yang dipindah. `autoStickerHandler` (:61)
//! tidak: butuh `m.download()` (:89), `sock.sendImageAsSticker` (:95),
//! `sock.sendVideoAsSticker` (:104), dan `config.sticker` (:96, :103).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const ALREADY_ON: &str = "🖼️ *ᴀᴜᴛᴏsᴛɪᴄᴋᴇʀ*\n\n> Sudah aktif!";
const ALREADY_OFF: &str = "🖼️ *ᴀᴜᴛᴏsᴛɪᴄᴋᴇʀ*\n\n> Sudah nonaktif!";
const TURNED_ON: &str =
    "🖼️ *ᴀᴜᴛᴏsᴛɪᴄᴋᴇʀ*\n\n> ✅ Berhasil diaktifkan!\n> Gambar/video akan otomatis jadi sticker";
const TURNED_OFF: &str = "🖼️ *ᴀᴜᴛᴏsᴛɪᴄᴋᴇʀ*\n\n> ❌ Berhasil dinonaktifkan!";

fn write(ctx: &Ctx, text: &str, on: bool) -> Reply {
    Reply::text(text).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "autosticker": on }),
    })
}

#[plugin(
    name = "autosticker",
    alias = ["autostiker", "as"],
    category = "group",
    description = "Toggle auto sticker - otomatis jadikan gambar/video jadi sticker",
    usage = ".autosticker on/off",
    example = ".autosticker on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn autosticker(ctx: Ctx) -> Reply {
    // `groupData.autosticker ?? false` — `??`, bukan `||`, tapi untuk flag
    // boolean keduanya berakhir sama karena `current` dipakai sebagai kondisi.
    let current = crate::common::js_truthy(ctx.group_field("autosticker"));
    let arg = ctx.args.first().map(|a| a.to_lowercase());

    match arg.as_deref() {
        None => Reply::text(format!(
            "🖼️ *ᴀᴜᴛᴏsᴛɪᴄᴋᴇʀ*\n\n\
             > Status: {}\n\n\
             > Gunakan:\n\
             > `{prefix}autosticker on` - aktifkan\n\
             > `{prefix}autosticker off` - nonaktifkan\n\n\
             > _Otomatis jadikan gambar/video jadi sticker_",
            if current { "✅ Aktif" } else { "❌ Nonaktif" },
            prefix = ctx.prefix
        )),
        Some("on" | "1" | "aktif") => {
            if current {
                Reply::text(ALREADY_ON)
            } else {
                write(&ctx, TURNED_ON, true)
            }
        }
        Some("off" | "0" | "nonaktif") => {
            if current {
                write(&ctx, TURNED_OFF, false)
            } else {
                Reply::text(ALREADY_OFF)
            }
        }
        Some(_) => Reply::text(format!(
            "❌ Gunakan: `{}autosticker on/off`",
            ctx.prefix
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autosticker".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("autosticker").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("autosticker").unwrap();
        assert_eq!(p.meta.name, "autosticker");
        assert_eq!(p.meta.alias, ["autostiker", "as"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Toggle auto sticker - otomatis jadikan gambar/video jadi sticker"
        );
        assert_eq!(p.meta.usage, ".autosticker on/off");
        assert_eq!(p.meta.example, ".autosticker on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
        // `isBotAdmin` tidak ada di pluginConfig JS.
        assert!(!p.meta.is_bot_admin);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["autostiker", "as"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "autosticker"
            );
        }
    }

    #[test]
    fn no_argument_reports_status_and_never_writes() {
        let off = run(serde_json::json!({}), "");
        assert!(
            text_of(&off).contains("> Status: ❌ Nonaktif"),
            "{}",
            text_of(&off)
        );
        assert!(off.mutations().is_empty());

        let on = run(serde_json::json!({ "autosticker": true }), "");
        assert!(
            text_of(&on).contains("> Status: ✅ Aktif"),
            "{}",
            text_of(&on)
        );
        assert!(on.mutations().is_empty());
    }

    #[test]
    fn the_help_text_embeds_the_matched_prefix() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "autosticker".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("autosticker").unwrap().handler)(ctx);
        assert!(
            text_of(&r).contains("`!autosticker on`"),
            "{}",
            text_of(&r)
        );
        assert!(!text_of(&r).contains("`.autosticker"));
    }

    #[test]
    fn every_on_spelling_writes_true_once() {
        for input in ["on", "1", "aktif"] {
            let r = run(serde_json::json!({}), input);
            assert_eq!(written(&r), &serde_json::json!({ "autosticker": true }), "{input}");
            assert_eq!(text_of(&r), TURNED_ON, "{input}");
        }
    }

    #[test]
    fn every_off_spelling_writes_false_once() {
        for input in ["off", "0", "nonaktif"] {
            let r = run(serde_json::json!({ "autosticker": true }), input);
            assert_eq!(written(&r), &serde_json::json!({ "autosticker": false }), "{input}");
            assert_eq!(text_of(&r), TURNED_OFF, "{input}");
        }
    }

    #[test]
    fn turning_on_an_already_on_group_writes_nothing() {
        for input in ["on", "1", "aktif"] {
            let r = run(serde_json::json!({ "autosticker": true }), input);
            assert_eq!(text_of(&r), ALREADY_ON, "{input}");
            assert!(r.mutations().is_empty(), "{input}");
        }
    }

    #[test]
    fn turning_off_an_already_off_group_writes_nothing() {
        for input in ["off", "0", "nonaktif"] {
            let r = run(serde_json::json!({}), input);
            assert_eq!(text_of(&r), ALREADY_OFF, "{input}");
            assert!(r.mutations().is_empty(), "{input}");
        }
    }

    #[test]
    fn the_spellings_are_lowercased_before_matching() {
        assert_eq!(written(&run(serde_json::json!({}), "ON")), &serde_json::json!({ "autosticker": true }));
        assert_eq!(
            written(&run(serde_json::json!({ "autosticker": true }), "Nonaktif")),
            &serde_json::json!({ "autosticker": false })
        );
    }

    #[test]
    fn an_unknown_word_is_refused_with_the_prefix() {
        let r = run(serde_json::json!({}), "mungkin");
        assert_eq!(text_of(&r), "❌ Gunakan: `.autosticker on/off`");
        assert!(r.mutations().is_empty());
    }
}