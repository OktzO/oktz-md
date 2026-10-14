//! `plugins/owner/similarity.js` — flag `similarity`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

pub(crate) const HELP: &str =
    "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `.similarity on` - Aktifkan\n> `.similarity off` - Matikan";

pub(crate) fn enabled_text(on: bool) -> String {
    format!(
        "✅ *sᴜᴋsᴇs*\n\n> Fitur similarity command *{}*",
        if on { "DIAKTIFKAN" } else { "DIMATIKAN" }
    )
}

#[plugin(
    name = "similarity",
    alias = ["setsimilarity", "sim"],
    category = "owner",
    description = "Mengaktifkan/menonaktifkan fitur similarity (saran typo)",
    usage = ".similarity <on/off>",
    example = ".similarity on",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn similarity(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        Some("on") => Reply::text(enabled_text(true))
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "similarity".into(),
                value: serde_json::Value::Bool(true),
            }),
        Some("off") => Reply::text(enabled_text(false))
            .with_reaction("✅")
            .with_mutation(Mutation::SetSetting {
                key: "similarity".into(),
                value: serde_json::Value::Bool(false),
            }),
        // Tanpa argumen DAN argumen tak dikenal memakai teks yang sama persis
        // di JS — bukan dua cabang berbeda.
        _ => Reply::text(HELP),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use serde_json::json;

    fn run(text: &str) -> Reply {
        let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: "similarity".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args,
            ..Default::default()
        };
        (oktz_plugin_api::find("similarity").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Option<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "similarity");
                Some(value.clone())
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("similarity").unwrap();
        assert_eq!(p.meta.name, "similarity");
        assert_eq!(p.meta.alias, ["setsimilarity", "sim"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Mengaktifkan/menonaktifkan fitur similarity (saran typo)"
        );
        assert_eq!(p.meta.usage, ".similarity <on/off>");
        assert_eq!(p.meta.example, ".similarity on");
        assert_eq!(p.meta.cooldown, 0, "cooldown: 0 di JS");
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setsimilarity", "sim"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "similarity");
        }
    }

    #[test]
    fn on_and_off_each_write_their_own_boolean() {
        assert_eq!(stored(&run("on")), Some(json!(true)));
        assert_eq!(stored(&run("off")), Some(json!(false)));
    }

    #[test]
    fn both_switch_branches_react_the_same_way() {
        for arg in ["on", "off"] {
            match run(arg).without_mutations() {
                Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅", "arg {arg}"),
                other => panic!("harus Reaction, dapat {other:?}"),
            }
        }
    }

    #[test]
    fn the_switch_reply_names_the_direction_that_was_written() {
        assert!(text_of(&run("on")).contains("DIAKTIFKAN"));
        assert!(text_of(&run("off")).contains("DIMATIKAN"));
        assert!(text_of(&run("on")).starts_with("✅ *sᴜᴋsᴇs*"));
    }

    #[test]
    fn a_missing_argument_shows_help_and_writes_nothing() {
        let r = run("");
        assert_eq!(text_of(&r), HELP);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unknown_argument_shows_the_same_help_and_writes_nothing() {
        for bad in ["yes", "ON!", "aktif", "1"] {
            let r = run(bad);
            assert_eq!(text_of(&r), HELP, "input {bad}");
            assert!(r.mutations().is_empty(), "input {bad} tidak boleh menulis");
        }
    }

    #[test]
    fn the_help_branch_ignores_the_stored_value() {
        // JS tidak pernah membaca `db.setting('similarity')` di plugin ini —
        // hanya menulis. Menyimpan `true` tidak boleh mengubah balasannya.
        let ctx = Ctx {
            prefix: ".".into(),
            command: "similarity".into(),
            text: String::new(),
            args: Vec::new(),
            settings: json!({"similarity": true}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("similarity").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), HELP);
    }
}