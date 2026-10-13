//! `plugins/user/notiflimit.js` — toggle global notifikasi potong limit.
//!
//! File-nya ada di `plugins/user/` jadi modulnya ikut ke `src/user/`, tapi
//! `pluginConfig.category` tetap `"owner"` dan itu yang ditulis ke manifest.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

pub(crate) const SETTING_KEY: &str = "notiflimit";
pub(crate) const ON_LABEL: &str = "AKTIF ✅";
pub(crate) const OFF_LABEL: &str = "MATI ❌";

/// `db.setting('notiflimit') ?? false` lalu `!current` — `??` hanya usherkan
/// `null`/`undefined`, jadi nilai `0` atau `""` ikut di-negasi apa adanya.
pub(crate) fn toggled(ctx: &Ctx) -> Value {
    let current = match ctx.setting(SETTING_KEY) {
        None | Some(Value::Null) => false,
        Some(v) => crate::common::js_truthy_value(v),
    };
    Value::Bool(!current)
}

pub(crate) fn text(value: &Value) -> String {
    let label = if crate::common::js_truthy_value(value) {
        ON_LABEL
    } else {
        OFF_LABEL
    };
    format!("*NOTIFIKASI LIMIT (GLOBAL)*\n\nStatus saat ini: *{label}*\n\n> Ketika aktif, bot akan selalu memberitahu sisa limit SEMUA PENGGUNA setiap kali ada pemotongan saat menggunakan fitur bot.")
}

#[plugin(
    name = "notiflimit",
    alias = ["notifenergi"],
    category = "owner",
    description = "Mengaktifkan atau mematikan notifikasi potong limit secara global.",
    usage = ".notiflimit",
    example = ".notiflimit",
    cooldown = 0,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn notiflimit(ctx: Ctx) -> Reply {
    let next = toggled(&ctx);
    Reply::text(text(&next)).with_mutation(Mutation::SetSetting {
        key: SETTING_KEY.into(),
        value: next.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notiflimit".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("notiflimit").unwrap().handler)(ctx)
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, SETTING_KEY);
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("notiflimit").unwrap();
        assert_eq!(p.meta.name, "notiflimit");
        assert_eq!(p.meta.alias, ["notifenergi"]);
        assert_eq!(
            p.meta.category, "owner",
            "category dari pluginConfig, bukan dari folder"
        );
        assert_eq!(
            p.meta.description,
            "Mengaktifkan atau mematikan notifikasi potong limit secara global."
        );
        assert_eq!(p.meta.usage, ".notiflimit");
        assert_eq!(p.meta.example, ".notiflimit");
        assert_eq!(p.meta.cooldown, 0);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("notifenergi").unwrap().meta.name, "notiflimit");
    }

    #[test]
    fn a_missing_setting_turns_the_flag_on() {
        let r = run(json!({}));
        assert_eq!(stored(&r), json!(true));
        assert!(r.text_body().unwrap().contains("Status saat ini: *AKTIF ✅*"));
    }

    #[test]
    fn each_call_flips_the_stored_value() {
        let r = run(json!({"notiflimit": true}));
        assert_eq!(stored(&r), json!(false));
        assert!(r.text_body().unwrap().contains("Status saat ini: *MATI ❌*"));
    }

    #[test]
    fn an_explicit_null_is_treated_like_a_missing_key() {
        // `?? false` hanya usherkan null/undefined, jadi null -> false -> !false.
        assert_eq!(stored(&run(json!({"notiflimit": null}))), json!(true));
    }

    #[test]
    fn falsy_non_null_values_are_negated_by_truthiness_not_replaced() {
        assert_eq!(stored(&run(json!({"notiflimit": 0}))), json!(true));
        assert_eq!(stored(&run(json!({"notiflimit": ""}))), json!(true));
    }
}