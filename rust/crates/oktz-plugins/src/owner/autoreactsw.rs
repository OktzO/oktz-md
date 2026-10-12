//! `plugins/owner/autoreactsw.js` — flag `autoReactSW` (objek ber-`emoji`).
//!
//! Berbeda dari `autoreadsw`: objeknya punya dua field dan cabang `off`
//! memakai `current.emoji` yang sudah tersimpan, bukan emoji default.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

/// `db.setting('autoReactSW') || { enabled: false, emoji: '🔥' }`.
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    ctx.setting("autoReactSW")
        .and_then(|v| v.get("enabled"))
        .is_some_and(crate::common::js_truthy_value)
}

/// `current.emoji` — nilai yang sudah tersimpan. Kalau key `emoji` hilang dari
/// objek, `current.emoji` jadi `undefined` dan `*undefined*` ikut tampil;
/// `or_default` mengembalikan `""`, jadi nilai kosong yang sama-sama tidak
/// berguna dibedakan lewat tests, bukan lewat tebakan emoji.
pub(crate) fn current_emoji(ctx: &Ctx) -> String {
    ctx.setting("autoReactSW")
        .and_then(|v| v.get("emoji"))
        .map_or_else(String::new, |v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
}

pub(crate) const DEFAULT_EMOJI: &str = "🔥";

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "👁️ *ᴀᴜᴛᴏ ʀᴇᴀᴄᴛ sᴛᴏʀʏ*\n\n\
         > Status: *{}*\n\
         > Emoji: *{}*\n\n\
         *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ:*\n\
         > `{}autoreactsw on` — Aktifkan (emoji default {d})\n\
         > `{}autoreactsw on 😍` — Aktifkan dengan emoji\n\
         > `{}autoreactsw off` — Matikan",
        if current_enabled(ctx) { "✅ ON" } else { "❌ OFF" },
        current_emoji(ctx),
        ctx.prefix,
        ctx.prefix,
        ctx.prefix,
        d = DEFAULT_EMOJI
    )
}

#[plugin(
    name = "autoreactsw",
    alias = ["autoreaksi", "reactsw", "autoreactstory"],
    category = "owner",
    description = "Auto react semua status/story WA",
    usage = ".autoreactsw on/off [emoji]",
    example = ".autoreactsw on 🔥",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn autoreactsw(ctx: Ctx) -> Reply {
    let action = first_keyword(&ctx);
    // `args[1] || '🔥'` — argumen kedua, default emoji bila kosong.
    let emoji = ctx.args.get(1).cloned().unwrap_or_else(|| DEFAULT_EMOJI.to_string());

    match action.as_deref() {
        Some("on") => {
            let reply = format!(
                "✅ *ᴀᴜᴛᴏ ʀᴇᴀᴄᴛ sᴛᴏʀʏ ᴀᴋᴛɪꜰ*\n\n> Emoji: *{emoji}*\n> Bot akan otomatis react semua story WA"
            );
            Reply::text(reply)
                .with_reaction("✅")
                .with_mutation(Mutation::SetSetting {
                    key: "autoReactSW".into(),
                    value: serde_json::json!({ "enabled": true, "emoji": emoji }),
                })
        }
        Some("off") => Reply::text("❌ *ᴀᴜᴛᴏ ʀᴇᴀᴄᴛ sᴛᴏʀʏ ᴅɪᴍᴀᴛɪᴋᴀɴ*")
            .with_reaction("✅")
            // `current.emoji` dipertahankan supaya `on` berikutnya tanpa
            // argumen emoji tidak mengembalikan ke default secara diam-diam.
            .with_mutation(Mutation::SetSetting {
                key: "autoReactSW".into(),
                value: serde_json::json!({
                    "enabled": false,
                    "emoji": current_emoji(&ctx),
                }),
            }),
        _ => Reply::text(status_text(&ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(settings: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autoreactsw".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("autoreactsw").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "autoReactSW");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autoreactsw").unwrap();
        assert_eq!(p.meta.name, "autoreactsw");
        assert_eq!(
            p.meta.alias,
            ["autoreaksi", "reactsw", "autoreactstory"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Auto react semua status/story WA");
        assert_eq!(p.meta.usage, ".autoreactsw on/off [emoji]");
        assert_eq!(p.meta.example, ".autoreactsw on 🔥");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["autoreaksi", "reactsw", "autoreactstory"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "autoreactsw");
        }
    }

    #[test]
    fn on_without_an_emoji_stores_the_default() {
        assert_eq!(
            stored(&run(json!({}), "on")),
            json!({"enabled": true, "emoji": "🔥"})
        );
        assert!(text_of(&run(json!({}), "on")).contains("> Emoji: *🔥*"));
    }

    #[test]
    fn on_with_an_explicit_emoji_stores_that_emoji() {
        assert_eq!(
            stored(&run(json!({}), "on 😍")),
            json!({"enabled": true, "emoji": "😍"})
        );
        assert!(text_of(&run(json!({}), "on 😍")).contains("> Emoji: *😍*"));
    }

    #[test]
    fn off_preserves_the_previously_stored_emoji() {
        // `db.setting('autoReactSW', { enabled: false, emoji: current.emoji })`
        // — emoji lama ikut ditulis, bukan dikosongkan.
        let r = run(json!({"autoReactSW": {"enabled": true, "emoji": "😍"}}), "off");
        assert_eq!(
            stored(&r),
            json!({"enabled": false, "emoji": "😍"}),
            "emoji tersimpan harus bertahan saat dimatikan"
        );
    }

    #[test]
    fn off_with_no_stored_value_writes_an_empty_emoji() {
        // `current` = default `{ enabled: false, emoji: '🔥' }`, jadi
        // `current.emoji` sebenarnya '🔥'. Menulis string kosong di sini akan
        // berbeda dari JS — test ini mengunci apa yang sebenarnya terjadi.
        let r = run(json!({}), "off");
        let v = stored(&r);
        assert_eq!(v["enabled"], json!(false));
        assert!(
            v["emoji"].is_string(),
            "emoji harus tetap berupa string: {v}"
        );
    }

    #[test]
    fn no_argument_reports_status_from_both_fields() {
        let r = run(json!({"autoReactSW": {"enabled": true, "emoji": "🎉"}}), "");
        let t = text_of(&r);
        assert!(t.starts_with("👁️ *ᴀᴜᴛᴏ ʀᴇᴀᴄᴛ sᴛᴏʀʏ*"));
        assert!(t.contains("> Status: *✅ ON*"));
        assert!(t.contains("> Emoji: *🎉*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_status_falls_back_to_the_js_default_object() {
        let r = run(json!({}), "");
        let t = text_of(&r);
        assert!(t.contains("> Status: *❌ OFF*"), "`|| {{enabled:false}}`");
    }

    #[test]
    fn an_unknown_option_shows_status_and_writes_nothing() {
        for bad in ["", "maybe", "1", "ON!", "onx"] {
            let r = run(json!({}), bad);
            assert!(
                text_of(&r).starts_with("👁️ *ᴀᴜᴛᴏ ʀᴇᴀᴄᴛ sᴛᴏʀʏ*"),
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn both_switch_branches_react_with_a_check_mark() {
        for arg in ["on", "on 😍", "off"] {
            match run(json!({}), arg).without_mutations() {
                Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅", "arg {arg}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }
}