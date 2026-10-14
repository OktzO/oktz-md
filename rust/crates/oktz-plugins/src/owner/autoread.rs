//! `plugins/owner/autoread.js` — toggle global `settings.autoRead`.
//!
//! `ponytail:` status tanpa argumen membaca `config.features.autoRead`
//! (`config.js`, `true`) sebagai fallback terakhir. `Ctx` tidak membawa
//! config, jadi nilainya ikut `config.js` yang di-hardcode — persis seperti
//! `panel/cpanel.rs` melakukan untuk `config.pterodactyl.*`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy, text_keyword};

/// Kunci `db.setting` yang dibaca dan ditulis (`autoread.js:24,44,53`).
pub(crate) const KEY: &str = "autoRead";

/// `config.features.autoRead` (`config.js`) — nilai untuk `??` terakhir.
pub(crate) const CONFIG_DEFAULT: bool = true;

pub(crate) const STATUS: &str = "📖 *Auto Read*\n\n\
    > Status: *{status}*\n\n\
    *PENGGUNAAN:*\n\
    > *{prefix}autoread on* — Aktifkan\n\
    > *{prefix}autoread off* — Nonaktif\n\n\
    _Bot akan otomatis membaca pesan masuk_";

pub(crate) const ON: &str = "📖 *Auto Read Aktif*\n\n\
    > Bot akan otomatis membaca pesan masuk";

pub(crate) const OFF: &str = "📖 *Auto Read Nonaktif*\n\n\
    > Bot tidak akan otomatis membaca pesan";

pub(crate) const INVALID: &str = "❌ *Opsi Tidak Valid*\n\n\
    > Gunakan *{prefix}autoread on* atau *{prefix}autoread off*";

pub(crate) fn status(prefix: &str, current: bool) -> String {
    STATUS
        .replace("{status}", if current { "Aktif ✅" } else { "Nonaktif ❌" })
        .replace("{prefix}", prefix)
}

pub(crate) fn invalid(prefix: &str) -> String {
    INVALID.replace("{prefix}", prefix)
}

/// `db.setting("autoRead") ?? config.features?.autoRead ?? false`.
///
/// `??` hanya jatuh ke default pada `null`/`undefined`, jadi `false` yang
/// tersimpan tetap dipakai dan hanya menentukan teks status lewat `!m0`.
pub(crate) fn current(settings: &Value) -> bool {
    match settings.get(KEY) {
        Some(Value::Null) | None => CONFIG_DEFAULT,
        Some(v) => js_truthy(Some(v)),
    }
}

#[plugin(
    name = "autoread",
    alias = ["readchat", "autobaca"],
    category = "owner",
    description = "Auto read pesan masuk",
    usage = ".autoread on/off",
    example = ".autoread on",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false
)]
pub fn autoread(ctx: Ctx) -> Reply {
    let option = text_keyword(&ctx);
    match option.as_str() {
        "" => Reply::text(status(&ctx.prefix, current(&ctx.settings))),
        "on" => Reply::text(ON).with_mutation(Mutation::SetSetting {
            key: KEY.into(),
            value: Value::Bool(true),
        }),
        "off" => Reply::text(OFF).with_mutation(Mutation::SetSetting {
            key: KEY.into(),
            value: Value::Bool(false),
        }),
        _ => Reply::text(invalid(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(prefix: &str, text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "autoread".into(),
            text: text.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_owner: true,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("autoread").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, KEY);
                value
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("autoread").unwrap();
        assert_eq!(p.meta.name, "autoread");
        assert_eq!(p.meta.alias, ["readchat", "autobaca"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Auto read pesan masuk");
        assert_eq!(p.meta.usage, ".autoread on/off");
        assert_eq!(p.meta.example, ".autoread on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["readchat", "autobaca"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "autoread");
        }
    }

    #[test]
    fn on_writes_true_and_off_writes_false() {
        let r = run(".", "on", Value::Null);
        assert_eq!(text_of(&r), ON);
        assert_eq!(stored(&r), &json!(true));

        let r = run(".", "off", Value::Null);
        assert_eq!(text_of(&r), OFF);
        assert_eq!(stored(&r), &json!(false));
    }

    #[test]
    fn the_option_is_lowercased_and_trimmed_before_it_is_compared() {
        let r = run(".", "  ON  ", Value::Null);
        assert_eq!(stored(&r), &json!(true));
        let r = run(".", "\tOff\t", Value::Null);
        assert_eq!(stored(&r), &json!(false));
    }

    #[test]
    fn anything_else_falls_to_the_invalid_option_branch() {
        for text in ["on off", "enable", "1", "on.", "-", "😔"] {
            let r = run("!", text, Value::Null);
            assert_eq!(
                text_of(&r),
                "❌ *Opsi Tidak Valid*\n\n> Gunakan *.autoread on* atau *.autoread off*",
                "input {text:?}"
            );
            assert!(r.mutations().is_empty(), "{text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn a_blank_option_reports_status_and_writes_nothing() {
        for text in ["", "   ", "\n\t"] {
            let r = run("!", text, json!({KEY: false}));
            assert_eq!(
                text_of(&r),
                "📖 *Auto Read*\n\n\
                 > Status: *Nonaktif ❌*\n\n\
                 *PENGGUNAAN:*\n\
                 > *!autoread on* — Aktifkan\n\
                 > *!autoread off* — Nonaktif\n\n\
                 _Bot akan otomatis membaca pesan masuk_",
                "input {text:?}"
            );
            assert!(r.mutations().is_empty(), "{text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn a_missing_setting_falls_back_to_the_config_value() {
        // `config.features.autoRead` = true di config.js — kebalikan dari
        // anticall, jadi keduanya harus punya default sendiri.
        let r = run(".", "", Value::Null);
        assert!(
            text_of(&r).contains("Status: *Aktif ✅*"),
            "{}",
            text_of(&r)
        );
        let r = run(".", "", json!({KEY: null}));
        assert!(text_of(&r).contains("Status: *Aktif ✅*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_false_is_not_overwritten_by_the_config_default() {
        // `??` hanya reacts ke null/undefined — `false` yang tersimpan tetap
        // dipakai, tidak ditimpa `true` dari config.
        let r = run(".", "", json!({KEY: false}));
        assert!(
            text_of(&r).contains("Status: *Nonaktif ❌*"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_value_is_read_back_as_the_status() {
        for truthy in [json!(true), json!(1), json!("ya"), json!({})] {
            let r = run(".", "", json!({KEY: truthy.clone()}));
            assert!(text_of(&r).contains("Status: *Aktif ✅*"), "{truthy}");
        }
    }
}
