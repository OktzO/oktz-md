//! `plugins/owner/autoreadsw.js` — flag `autoReadSW`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

/// `db.setting('autoReadSW') || { enabled: false }` — default-nya objek dengan
/// `enabled: false`, BUKAN `false`. Jadi `current.enabled` selalu ada begitu
/// key ini pernah ditulis dengan bentuk apa pun.
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    ctx.setting("autoReadSW")
        .and_then(|v| v.get("enabled"))
        .is_some_and(crate::common::js_truthy_value)
}

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "👁️ *ᴀᴜᴛᴏ ʀᴇᴀᴅ sᴛᴏʀʏ*\n\n\
         > Status: *{}*\n\n\
         *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ:*\n\
         > `{}autoreadsw on` — Aktifkan\n\
         > `{}autoreadsw off` — Matikan",
        if current_enabled(ctx) { "✅ ON" } else { "❌ OFF" },
        ctx.prefix,
        ctx.prefix
    )
}

#[plugin(
    name = "autoreadsw",
    alias = ["autoreadstory", "readstory", "bacasw"],
    category = "owner",
    description = "Auto read semua status/story WA",
    usage = ".autoreadsw on/off",
    example = ".autoreadsw on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn autoreadsw(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        Some("on") => Reply::text(
            "✅ *ᴀᴜᴛᴏ ʀᴇᴀᴅ sᴛᴏʀʏ ᴀᴋᴛɪꜰ*\n\n> Bot akan otomatis membaca semua story WA",
        )
        .with_reaction("✅")
        .with_mutation(Mutation::SetSetting {
            key: "autoReadSW".into(),
            value: serde_json::json!({ "enabled": true }),
        }),
        Some("off") => Reply::text("❌ *ᴀᴜᴛᴏ ʀᴇᴀᴅ sᴛᴏʀʏ ᴅɪᴍᴀᴛɪᴋᴀɴ*")
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "autoReadSW".into(),
                value: serde_json::json!({ "enabled": false }),
            }),
        _ => Reply::text(status_text(&ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "autoreadsw".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("autoreadsw").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "autoReadSW");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autoreadsw").unwrap();
        assert_eq!(p.meta.name, "autoreadsw");
        assert_eq!(p.meta.alias, ["autoreadstory", "readstory", "bacasw"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Auto read semua status/story WA");
        assert_eq!(p.meta.usage, ".autoreadsw on/off");
        assert_eq!(p.meta.example, ".autoreadsw on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["autoreadstory", "readstory", "bacasw"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "autoreadsw");
        }
    }

    #[test]
    fn on_and_off_write_a_whole_replacement_object() {
        assert_eq!(stored(&run(serde_json::json!({}), "on")), serde_json::json!({"enabled": true}));
        assert_eq!(stored(&run(serde_json::json!({}), "off")), serde_json::json!({"enabled": false}));
    }

    #[test]
    fn no_argument_reports_status_and_writes_nothing() {
        for s in [
            serde_json::json!({}),
            serde_json::json!({"autoReadSW": {"enabled": true}}),
            serde_json::json!({"autoReadSW": {"enabled": false}}),
        ] {
            let r = run(s.clone(), "");
            assert!(text_of(&r).starts_with("👁️ *ᴀᴜᴛᴏ ʀᴇᴀᴅ sᴛᴏʀʏ*"), "input {s}");
            assert!(r.mutations().is_empty(), "input {s}");
        }
    }

    #[test]
    fn the_status_line_reads_the_nested_enabled_flag() {
        assert!(text_of(&run(serde_json::json!({"autoReadSW": {"enabled": true}}), "")).contains("✅ ON"));
        assert!(text_of(&run(serde_json::json!({"autoReadSW": {"enabled": false}}), "")).contains("❌ OFF"));
        // `|| { enabled: false }` — key hilang tetap OFF.
        assert!(text_of(&run(serde_json::json!({}), "")).contains("❌ OFF"));
    }

    #[test]
    fn a_stored_value_without_the_enabled_field_reads_as_off() {
        assert!(text_of(&run(serde_json::json!({"autoReadSW": {}}), "")).contains("❌ OFF"));
        assert!(text_of(&run(serde_json::json!({"autoReadSW": true}), "")).contains("❌ OFF"));
    }

    #[test]
    fn an_unknown_option_shows_status_and_writes_nothing() {
        for bad in ["maybe", "1", "enable", "ON!"] {
            let r = run(serde_json::json!({}), bad);
            assert!(
                text_of(&r).starts_with("👁️ *ᴀᴜᴛᴏ ʀᴇᴀᴅ sᴛᴏʀʏ*"),
                "input {bad}"
            );
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn both_switch_branches_react_with_a_check_mark() {
        for arg in ["on", "off"] {
            match run(serde_json::json!({}), arg).without_mutations() {
                Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅", "arg {arg}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn switching_off_replaces_the_object_so_no_stale_field_survives() {
        let r = run(serde_json::json!({"autoReadSW": {"enabled": true, "extra": 1}}), "off");
        assert_eq!(stored(&r), serde_json::json!({"enabled": false}));
    }
}