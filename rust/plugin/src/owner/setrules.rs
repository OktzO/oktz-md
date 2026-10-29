//! `plugins/owner/setrules.js` — simpan `botRules` dari teks atau reply.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.text?.trim() || m.quoted?.body || m.quoted?.text || ''` — `QuotedMessage`
/// tidak punya `body` (itu sisa key Baileys lama), jadi yang dipakai `text`.
pub(crate) fn rules_text(ctx: &Ctx) -> String {
    let typed = ctx.text.trim();
    if !typed.is_empty() {
        return typed.to_string();
    }
    ctx.quoted().map_or_else(String::new, |q| q.text.clone())
}

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "📝 *sᴇᴛ ʙᴏᴛ ʀᴜʟᴇs*\n\n\
         > Masukkan teks rules yang baru\n\n\
         `Contoh:`\n\
         `{prefix}setrules 1. Jangan spam\\n2. Hormati sesama`"
    )
}

pub(crate) fn confirm(prefix: &str) -> String {
    format!(
        "✅ *ʙᴏᴛ ʀᴜʟᴇs ᴅɪʜᴜᴘᴅᴀᴛᴇ*\n\n\
         > Rules bot berhasil diubah!\n\
         > Ketik `{prefix}rules` untuk melihat."
    )
}

#[plugin(
    name = "setrules",
    alias = ["setbotrules", "setaturanbot"],
    category = "owner",
    description = "Set rules/aturan bot custom",
    usage = ".setrules <text>",
    example = ".setrules 1. Jangan spam\n2. Hormati sesama",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn setrules(ctx: Ctx) -> Reply {
    let text = rules_text(&ctx);
    if text.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }
    Reply::text(confirm(&ctx.prefix)).with_mutation(Mutation::SetSetting {
        key: "botRules".into(),
        value: serde_json::Value::String(text),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedKind;
    use oktz_plugin_api::QuotedMessage;
    use serde_json::Value;

    fn quoted_with(text: &str) -> QuotedMessage {
        QuotedMessage {
            id: "MSG1".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
            timestamp: 0,
        }
    }

    fn run(text: &str, quoted: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setrules".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            quoted: quoted.map(quoted_with),
            ..Default::default()
        };
        (oktz_plugin_api::find("setrules").unwrap().handler)(ctx)
    }

    fn written(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == "botRules" => Some(value.clone()),
            _ => None,
        })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setrules").unwrap();
        assert_eq!(p.meta.name, "setrules");
        assert_eq!(p.meta.alias, ["setbotrules", "setaturanbot"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Set rules/aturan bot custom");
        assert_eq!(p.meta.usage, ".setrules <text>");
        assert_eq!(
            p.meta.example,
            ".setrules 1. Jangan spam\n2. Hormati sesama"
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setbotrules", "setaturanbot"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "setrules");
        }
    }

    #[test]
    fn typed_text_is_trimmed_and_stored_verbatim() {
        let r = run("  1. Jangan spam  ", None);
        assert_eq!(written(&r), Some(Value::String("1. Jangan spam".into())));
        assert!(r.text_body().unwrap().contains("✅ *ʙᴏᴛ ʀᴜʟᴇs ᴅɪʜᴜᴘᴅᴀᴛᴇ*"));
    }

    #[test]
    fn the_quoted_text_is_used_when_nothing_is_typed() {
        assert_eq!(
            written(&run("   ", Some("aturan lama"))),
            Some(Value::String("aturan lama".into()))
        );
    }

    #[test]
    fn neither_text_nor_quote_shows_usage_and_writes_nothing() {
        let r = run("", None);
        assert!(r.text_body().unwrap().starts_with("📝 *sᴇᴛ ʙᴏᴛ ʀᴜʟᴇs*"));
        assert!(r.mutations().is_empty());

        let r = run("  ", Some(""));
        assert!(r.text_body().unwrap().starts_with("📝 *sᴇᴛ ʙᴏᴛ ʀᴜʟᴇs*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn typed_text_wins_over_the_quoted_text() {
        assert_eq!(
            written(&run("baru", Some("lama"))),
            Some(Value::String("baru".into()))
        );
    }
}