//! `plugins/owner/onlypc.js` — flag global `onlyPc` (+ `onlyGc` ikut dimatikan).
//!
//! Simetris dengan [`crate::owner::onlygc`]; bedanya hanya nama setting dan
//! teks. Keduanya tetap file terpisah karena `pluginConfig` JS-nya terpisah dan
//! alias-nya tidak tumpang tindih.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::first_keyword;

pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    ctx.setting("onlyPc")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "💬 *Only Private*\n\n\
         > Status: *{}*\n\n\
         *PENGGUNAAN:*\n\
         > *{p}onlypc on* — Bot hanya bisa diakses di private chat\n\
         > *{p}onlypc off* — Bot bisa diakses di mana saja\n\n\
         _Jika aktif, mode Only Group akan otomatis nonaktif_",
        if current_enabled(ctx) {
            "Aktif ✅"
        } else {
            "Nonaktif ❌"
        },
        p = ctx.prefix
    )
}

pub(crate) const ON_MSG: &str =
    "💬 *Only Private Aktif*\n\n> Bot hanya bisa diakses di private chat\n> Mode Only Group dinonaktifkan";
pub(crate) const OFF_MSG: &str = "💬 *Only Private Nonaktif*\n\n> Bot bisa diakses di mana saja";

pub(crate) fn invalid(prefix: &str) -> String {
    format!("❌ *Opsi Tidak Valid*\n\n> Gunakan *{prefix}onlypc on* atau *{prefix}onlypc off*")
}

pub(crate) fn set_only_pc(enabled: bool) -> Reply {
    let mut r = Reply::text(if enabled { ON_MSG } else { OFF_MSG })
        .with_reaction(if enabled { "✅" } else { "❌" })
        .with_mutation(Mutation::SetSetting {
            key: "onlyPc".into(),
            value: serde_json::Value::Bool(enabled),
        });
    if enabled {
        r = r.with_mutation(Mutation::SetSetting {
            key: "onlyGc".into(),
            value: serde_json::Value::Bool(false),
        });
    }
    r
}

#[plugin(
    name = "onlypc",
    alias = ["onlyprivate", "privateonly"],
    category = "owner",
    description = "Toggle mode bot hanya di private chat",
    usage = ".onlypc on/off",
    example = ".onlypc on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn onlypc(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        None => Reply::text(status_text(&ctx)),
        Some("on") => set_only_pc(true),
        Some("off") => set_only_pc(false),
        Some(_) => Reply::text(invalid(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(settings: Value, text: &str) -> Reply {
        let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: "onlypc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("onlypc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting_of(r: &Reply, key: &str) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key: k, value } if k == key => Some(value.clone()),
            _ => None,
        })
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("onlypc").unwrap();
        assert_eq!(p.meta.name, "onlypc");
        assert_eq!(p.meta.alias, ["onlyprivate", "privateonly"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Toggle mode bot hanya di private chat");
        assert_eq!(p.meta.usage, ".onlypc on/off");
        assert_eq!(p.meta.example, ".onlypc on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["onlyprivate", "privateonly"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "onlypc");
        }
    }

    #[test]
    fn no_argument_reports_status_and_writes_nothing() {
        for (settings, expected) in [
            (serde_json::json!({}), "Nonaktif ❌"),
            (serde_json::json!({"onlyPc": false}), "Nonaktif ❌"),
            (serde_json::json!({"onlyPc": true}), "Aktif ✅"),
        ] {
            let r = run(settings.clone(), "");
            assert!(text_of(&r).contains(expected), "input {settings}");
            assert!(text_of(&r).starts_with("💬 *Only Private*"));
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn a_non_boolean_setting_reads_as_the_js_default() {
        let r = run(serde_json::json!({"onlyPc": 7}), "");
        assert!(text_of(&r).contains("Nonaktif ❌"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn on_writes_onlypc_true_and_forces_onlygc_false() {
        let r = run(serde_json::json!({"onlyPc": false, "onlyGc": true}), "on");
        assert_eq!(r.mutations().len(), 2);
        assert_eq!(setting_of(&r, "onlyPc"), Some(serde_json::json!(true)));
        assert_eq!(setting_of(&r, "onlyGc"), Some(serde_json::json!(false)));
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(text_of(&r), ON_MSG);
    }

    #[test]
    fn off_writes_only_onlypc_false() {
        let r = run(serde_json::json!({"onlyPc": true, "onlyGc": true}), "off");
        assert_eq!(r.mutations().len(), 1);
        assert_eq!(setting_of(&r, "onlyPc"), Some(serde_json::json!(false)));
        assert_eq!(setting_of(&r, "onlyGc"), None);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert_eq!(text_of(&r), OFF_MSG);
    }

    #[test]
    fn an_unknown_option_is_rejected_and_writes_nothing() {
        for bad in ["onx", "private", "2"] {
            let r = run(serde_json::json!({}), bad);
            assert!(text_of(&r).starts_with("❌ *Opsi Tidak Valid*"));
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn keyword_matching_is_case_insensitive() {
        assert_eq!(setting_of(&run(serde_json::json!({}), "ON"), "onlyPc"), Some(serde_json::json!(true)));
        assert_eq!(
            setting_of(&run(serde_json::json!({}), "OFF"), "onlyPc"),
            Some(serde_json::json!(false))
        );
    }

    #[test]
    fn the_invalid_option_reply_carries_the_prefix() {
        assert_eq!(
            invalid("~"),
            "❌ *Opsi Tidak Valid*\n\n> Gunakan *~onlypc on* atau *~onlypc off*"
        );
    }

    /// Kunci `onlygc`/`onlypc` tidak boleh saling menimpa lewat alias.
    #[test]
    fn the_two_only_modes_do_not_claim_each_others_keys() {
        for (name, keys) in [
            ("onlygc", ["onlygc", "onlygroup", "grouponly"]),
            ("onlypc", ["onlypc", "onlyprivate", "privateonly"]),
        ] {
            for k in keys {
                let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(k))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(owners, [name], "key {k} diklaim oleh {owners:?}");
            }
        }
    }
}