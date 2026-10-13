//! `plugins/owner/resetrules.js` — `botRules` kembali ke default router.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) fn confirm(prefix: &str) -> String {
    format!(
        "✅ *ʙᴏᴛ ʀᴜʟᴇs ᴅɪʀᴇsᴇᴛ*\n\n\
         > Rules bot berhasil direset ke default!\n\
         > Ketik `{prefix}rules` untuk melihat."
    )
}

#[plugin(
    name = "resetrules",
    alias = ["resetbotrules"],
    category = "owner",
    description = "Reset rules bot ke default",
    usage = ".resetrules",
    example = ".resetrules",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn resetrules(ctx: Ctx) -> Reply {
    Reply::text(confirm(&ctx.prefix)).with_mutation(Mutation::SetSetting {
        key: "botRules".into(),
        value: serde_json::Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(prefix: &str) -> Reply {
        (oktz_plugin_api::find("resetrules").unwrap().handler)(Ctx {
            prefix: prefix.into(),
            command: "resetrules".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("resetrules").unwrap();
        assert_eq!(p.meta.name, "resetrules");
        assert_eq!(p.meta.alias, ["resetbotrules"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Reset rules bot ke default");
        assert_eq!(p.meta.usage, ".resetrules");
        assert_eq!(p.meta.example, ".resetrules");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("resetbotrules").unwrap().meta.name,
            "resetrules"
        );
    }

    #[test]
    fn bot_rules_is_cleared_to_null() {
        let r = run(".");
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "botRules");
                assert_eq!(*value, Value::Null);
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn the_reply_points_at_the_rules_command_with_the_live_prefix() {
        assert!(run("!").text_body().unwrap().contains("> Ketik `!rules` untuk melihat."));
    }
}