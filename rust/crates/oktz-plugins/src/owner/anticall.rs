//! `plugins/owner/anticall.js` — toggle global `settings.antiCall`.
//!
//! `ponytail:` status tanpa argumen membaca `config.features.antiCall`
//! (`config.js`, `false`) sebagai fallback terakhir. `Ctx` tidak membawa
//! config, jadi nilainya ikut `config.js` yang di-hardcode — persis seperti
//! `panel/cpanel.rs` melakukan untuk `config.pterodactyl.*`. Begitu
//! `config.js` diisi, baris ini ikut stale sampai router menyediakannya lewat
//! `Ctx`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy, text_keyword};

/// Kunci `db.setting` yang dibaca dan ditulis (`anticall.js:24,44,53`).
pub(crate) const KEY: &str = "antiCall";

/// `config.features.antiCall` (`config.js`) — nilai untuk `??` terakhir.
pub(crate) const CONFIG_DEFAULT: bool = false;

pub(crate) const STATUS: &str = "📞 *Anti Call*\n\n\
    > Status: *{status}*\n\n\
    *PENGGUNAAN:*\n\
    > *{prefix}anticall on* — Aktifkan\n\
    > *{prefix}anticall off* — Nonaktif\n\n\
    _Bot akan otomatis menolak panggilan masuk_";

pub(crate) const ON: &str = "📞 *Anti Call Aktif*\n\n\
    > Bot akan otomatis menolak panggilan masuk";

pub(crate) const OFF: &str = "📞 *Anti Call Nonaktif*\n\n\
    > Bot tidak akan menolak panggilan masuk";

pub(crate) const INVALID: &str = "❌ *Opsi Tidak Valid*\n\n\
    > Gunakan *{prefix}anticall on* atau *{prefix}anticall off*";

pub(crate) fn status(prefix: &str, current: bool) -> String {
    STATUS
        .replace("{status}", if current { "Aktif ✅" } else { "Nonaktif ❌" })
        .replace("{prefix}", prefix)
}

pub(crate) fn invalid(prefix: &str) -> String {
    INVALID.replace("{prefix}", prefix)
}

/// `db.setting("antiCall") ?? config.features?.antiCall ?? true`.
///
/// `??` hanya jatuh ke default pada `null`/`undefined`, jadi nilai falsehood
/// yang tersimpan (`false`, `0`, `""`) tetap dipakai dan hanya menentukan
/// teks status lewat `!m0`.
pub(crate) fn current(settings: &Value) -> bool {
    match settings.get(KEY) {
        Some(Value::Null) | None => CONFIG_DEFAULT,
        Some(v) => js_truthy(Some(v)),
    }
}

#[plugin(
    name = "anticall",
    alias = ["antitelpon", "antitelp", "rejectcall"],
    category = "owner",
    description = "Auto tolak panggilan masuk",
    usage = ".anticall on/off",
    example = ".anticall on",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false
)]
pub fn anticall(ctx: Ctx) -> Reply {
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
            command: "anticall".into(),
            text: text.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_owner: true,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("anticall").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("anticall").unwrap();
        assert_eq!(p.meta.name, "anticall");
        assert_eq!(p.meta.alias, ["antitelpon", "antitelp", "rejectcall"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Auto tolak panggilan masuk");
        assert_eq!(p.meta.usage, ".anticall on/off");
        assert_eq!(p.meta.example, ".anticall on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["antitelpon", "antitelp", "rejectcall"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "anticall");
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
        // `m.text?.toLowerCase()?.trim()` — newline di ujung ikut hilang,
        // jadi "on\n" tetap option sah, bukan input tak dikenal.
        let r = run(".", "on\n", Value::Null);
        assert_eq!(stored(&r), &json!(true));
    }

    #[test]
    fn anything_else_falls_to_the_invalid_option_branch() {
        // JS membandingkan `m.text` utuh tanpa parsing per kata, jadi
        // "on off" bukan "on" — ia jatuh ke cabang invalid dan tidak menulis.
        for text in ["on off", "enable", "1", "on.", "-", "😔"] {
            let r = run("!", text, Value::Null);
            assert_eq!(
                text_of(&r),
                "❌ *Opsi Tidak Valid*\n\n> Gunakan *!anticall on* atau *!anticall off*",
                "input {text:?}"
            );
            assert!(r.mutations().is_empty(), "{text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn a_blank_option_reports_status_and_writes_nothing() {
        for text in ["", "   ", "\n\t"] {
            let r = run("!", text, json!({KEY: true}));
            assert_eq!(
                text_of(&r),
                "📞 *Anti Call*\n\n\
                 > Status: *Aktif ✅*\n\n\
                 *PENGGUNAAN:*\n\
                 > *!anticall on* — Aktifkan\n\
                 > *!anticall off* — Nonaktif\n\n\
                 _Bot akan otomatis menolak panggilan masuk_",
                "input {text:?}"
            );
            assert!(r.mutations().is_empty(), "{text:?} tidak boleh menulis");
        }
    }

    #[test]
    fn a_missing_setting_falls_back_to_the_config_value() {
        // `config.features.antiCall` = false di config.js.
        let r = run(".", "", Value::Null);
        assert!(
            text_of(&r).contains("Status: *Nonaktif ❌*"),
            "{}",
            text_of(&r)
        );
        let r = run(".", "", json!({KEY: null}));
        assert!(text_of(&r).contains("Status: *Nonaktif ❌*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_value_is_read_back_as_the_status() {
        for truthy in [json!(true), json!(1), json!("ya"), json!({})] {
            let r = run(".", "", json!({KEY: truthy.clone()}));
            assert!(text_of(&r).contains("Status: *Aktif ✅*"), "{truthy}");
        }
        for falsy in [json!(false), json!(0), json!("")] {
            let r = run(".", "", json!({KEY: falsy.clone()}));
            assert!(text_of(&r).contains("Status: *Nonaktif ❌*"), "{falsy}");
        }
    }

    #[test]
    fn the_branch_selector_is_exactly_on_and_off() {
        // Tanpa guard `includes`, `text_keyword` sudah trim + lowercase; yang
        // tersisa adalah memastikan tidak ada nilai lain yang dianggap sah.
        // Spasi di DALAM teks tidak dihapus trim, jadi "o n" ≠ "on".
        for text in ["anti call", "aktif", "true", "o n", "on on"] {
            let r = run(".", text, Value::Null);
            assert!(r.mutations().is_empty(), "{text:?} tidak boleh menulis");
        }
    }
}
