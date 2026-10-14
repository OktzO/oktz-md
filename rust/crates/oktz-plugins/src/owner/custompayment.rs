//! `plugins/owner/custompayment.js` — teks custom untuk `.payment`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::json;

fn help(ctx: &Ctx, current: &str) -> String {
    format!(
        "📝 *CUSTOM PAYMENT TEXT*\n\n\
         Teks saat ini:\n{cur}\n\n\
         *PLACEHOLDER YANG TERSEDIA:*\n\
         • `{{botname}}` — Nama bot\n\
         • `{{owner}}` — Nama owner\n\
         • `{{methods}}` — Daftar e-wallet\n\
         • `{{banks}}` — Daftar bank\n\
         • `{{qris}}` — Status QRIS\n\n\
         *CONTOH:*\n\
         > `{p}custompayment Halo! Bayar ke {{methods}}`\n\n\
         > `{p}custompayment reset` — Kembalikan ke default",
        cur = if current.is_empty() {
            "_(belum diatur, pakai default)_"
        } else {
            current
        },
        p = ctx.prefix
    )
}

pub(crate) const RESET_MSG: &str = "✅ Teks custom payment direset ke default.";

#[plugin(
    name = "custompayment",
    alias = ["setpayment", "setpaytext"],
    category = "owner",
    description = "Atur teks custom untuk .payment dengan placeholder",
    usage = ".custompayment <teks> / .custompayment reset",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn custompayment(ctx: Ctx) -> Reply {
    let input = ctx.text.trim();
    let current = ctx
        .setting("customPaymentText")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if input.is_empty() {
        return Reply::text(help(&ctx, current));
    }

    // `input.toLowerCase() === 'reset'` — "RESET" juga kena.
    if input.to_lowercase() == "reset" {
        return Reply::text(RESET_MSG).with_mutation(Mutation::SetSetting {
            key: "customPaymentText".into(),
            value: json!(""),
        });
    }

    Reply::text(format!(
        "✅ Teks custom payment disimpan!\n\nPreview:\n{input}"
    ))
    .with_mutation(Mutation::SetSetting {
        key: "customPaymentText".into(),
        value: json!(input),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(text: &str, settings: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "custompayment".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("custompayment").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "customPaymentText");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("custompayment").unwrap();
        assert_eq!(p.meta.name, "custompayment");
        assert_eq!(p.meta.alias, ["setpayment", "setpaytext"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Atur teks custom untuk .payment dengan placeholder"
        );
        assert_eq!(
            p.meta.usage,
            ".custompayment <teks> / .custompayment reset"
        );
        // `pluginConfig` JS tidak punya `example`.
        assert_eq!(p.meta.example, "");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setpayment", "setpaytext"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "custompayment"
            );
        }
    }

    #[test]
    fn an_empty_text_shows_the_panel_and_writes_nothing() {
        for text in ["", "   "] {
            let r = run(text.clone(), json!({}));
            assert!(
                text_of(&r).starts_with("📝 *CUSTOM PAYMENT TEXT*"),
                "input {text:?}"
            );
            assert!(
                text_of(&r).contains("_(belum diatur, pakai default)_"),
                "input {text:?}"
            );
            assert!(r.mutations().is_empty(), "input {text:?}");
        }
    }

    #[test]
    fn the_stored_text_is_shown_verbatim() {
        let r = run("", json!({ "customPaymentText": "Halo {owner}" }));
        let t = text_of(&r);
        assert!(t.contains("Teks saat ini:\nHalo {owner}"), "{t}");
        assert!(!t.contains("belum diatur"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn reset_clears_the_text_regardless_of_case() {
        for text in ["reset", "RESET", " Reset "] {
            let r = run(text.clone(), json!({ "customPaymentText": "lama" }));
            assert_eq!(text_of(&r), RESET_MSG, "input {text:?}");
            assert_eq!(stored(&r), json!(""), "input {text:?}");
        }
    }

    #[test]
    fn a_word_containing_reset_is_kept_as_text() {
        // Guard: `=== 'reset'`, bukan `.includes('reset')`.
        let r = run("reset dulu ya", json!({}));
        assert_eq!(stored(&r), json!("reset dulu ya"));
    }

    #[test]
    fn the_preview_repeats_the_saved_text() {
        let r = run("Bayar ke {methods}", json!({}));
        assert_eq!(stored(&r), json!("Bayar ke {methods}"));
        assert!(
            text_of(&r).contains("Preview:\nBayar ke {methods}"),
            "{}",
            text_of(&r)
        );
    }
}