//! `plugins/owner/resetlimitdefault.js` — `defaultLimit` kembali ke config.
//!
//! `ponytail:` `config.limits` tidak ada di `config.js`, jadi
//! `config.limits?.default || 25` selalu `25` — angka itu hasil runtime
//! JS, bukan tebakan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `config.limits?.default || 25` (`config.js` tidak punya `limits`).
pub(crate) const CONFIG_DEFAULT_LIMIT: i64 = 25;

pub(crate) fn confirm() -> String {
    format!(
        "✅ *ʙᴇʀʜᴀsɪʟ*\n\n\
         > Default limit direset ke config: `{CONFIG_DEFAULT_LIMIT}`\n\
         > User baru akan mendapat limit dari config"
    )
}

#[plugin(
    name = "resetlimitdefault",
    alias = ["defaultlimitreset"],
    category = "owner",
    description = "Reset default limit ke config asli",
    usage = ".resetlimitdefault",
    example = ".resetlimitdefault",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn resetlimitdefault(_ctx: Ctx) -> Reply {
    Reply::text(confirm()).with_mutation(Mutation::SetSetting {
        key: "defaultLimit".into(),
        value: serde_json::Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run() -> Reply {
        (oktz_plugin_api::find("resetlimitdefault").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "resetlimitdefault".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("resetlimitdefault").unwrap();
        assert_eq!(p.meta.name, "resetlimitdefault");
        assert_eq!(p.meta.alias, ["defaultlimitreset"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Reset default limit ke config asli");
        assert_eq!(p.meta.usage, ".resetlimitdefault");
        assert_eq!(p.meta.example, ".resetlimitdefault");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("defaultlimitreset").unwrap().meta.name,
            "resetlimitdefault"
        );
    }

    #[test]
    fn default_limit_is_written_as_null_so_the_config_value_applies_again() {
        let r = run();
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "defaultLimit");
                assert_eq!(*value, Value::Null, "bukan 0 dan bukan 25");
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn the_reply_names_the_config_default() {
        assert!(run().text_body().unwrap().contains("> Default limit direset ke config: `25`"));
    }
}