//! `plugins/main/owner.js` — teks owner + kartu kontak/vCard.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

fn digits(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).collect()
}

fn first_nonempty(value: Option<&str>, fallback: &str) -> String {
    value.filter(|s| !s.is_empty()).unwrap_or(fallback).to_string()
}

fn bot_name(ctx: &Ctx) -> String {
    first_nonempty(Some(ctx.config().bot.name.as_str()), "Bot")
}

fn owner_name(ctx: &Ctx, number: &str) -> String {
    if number.is_empty() {
        return first_nonempty(Some(ctx.config().owner.name.as_str()), "Owner");
    }
    let clean = digits(number);
    let mapped = ctx
        .setting("ownerNames")
        .and_then(Value::as_object)
        .and_then(|m| m.get(&clean))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    if let Some(nama) = mapped {
        return nama.to_string();
    }
    let configured = ctx
        .config()
        .owner
        .number
        .iter()
        .any(|n| digits(n) == clean);
    if configured {
        first_nonempty(Some(ctx.config().owner.name.as_str()), "Owner")
    } else {
        "Owner".to_string()
    }
}

fn owner_numbers(ctx: &Ctx) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for raw in ctx
        .config()
        .owner
        .number
        .iter()
        .chain(ctx.db_owners())
    {
        if !seen.iter().any(|s| s == raw) {
            seen.push(raw.clone());
        }
    }
    seen
}

fn vcard(ctx: &Ctx, number: &str, normal: bool) -> String {
    let nama = owner_name(ctx, number);
    let clean = digits(number);
    let fn_nama = if normal {
        format!("{nama} (Owner {})", bot_name(ctx))
    } else {
        nama
    };
    format!(
        "BEGIN:VCARD\nVERSION:3.0\nFN:{fn_nama}\nTEL;type=CELL;type=VOICE;waid={clean}:+{clean}\nEND:VCARD"
    )
}

fn owner_text(ctx: &Ctx, numbers: &[String]) -> String {
    let names = numbers
        .iter()
        .map(|n| owner_name(ctx, n))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "👑 *ᴏᴡɴᴇʀ ɪɴꜰᴏʀᴍᴀᴛɪᴏɴ*\n\n\
╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
┃ ㊗ ɴᴀᴍᴀ: *{names}*\n\
┃ ㊗ ʙᴏᴛ: *{bot}*\n\
┃ ㊗ sᴛᴀᴛᴜs: *🟢 Online*\n\
╰┈┈⬡\n\n\
> _Jika ada pertanyaan atau kendala,_\n\
> _silakan hubungi owner di atas!_\n\
> _📞 Contact card di bawah._",
        bot = bot_name(ctx)
    )
}

fn owner_type_two(ctx: &Ctx) -> bool {
    ctx.setting("ownerType")
        .and_then(Value::as_f64)
        .is_some_and(|n| n == 2.0)
}

#[plugin(
    name = "owner",
    alias = ["creator", "dev", "developer"],
    category = "main",
    description = "Menampilkan kontak owner bot",
    usage = ".owner",
    example = ".owner",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn owner(ctx: Ctx) -> Reply {
    let numbers = owner_numbers(&ctx);

    if owner_type_two(&ctx) {
        if numbers.is_empty() {
            return Reply::Contacts {
                display_name: "Ini adalah owner kami".into(),
                contacts: Vec::new(),
            };
        }
        let contacts = numbers.iter().map(|n| vcard(&ctx, n, false)).collect();
        return Reply::sequence(vec![
            Reply::Contacts {
                display_name: "Ini adalah owner kami".into(),
                contacts,
            },
            Reply::TextQuotedPrevious {
                text: "💬 Jika kamu memiliki pertanyaan, jangan ragu untuk bertanya, owner ramah kok"
                    .into(),
            },
        ]);
    }

    let mut urutan = vec![Reply::text(owner_text(&ctx, &numbers))];
    for n in &numbers {
        urutan.push(Reply::Contacts {
            display_name: owner_name(&ctx, n),
            contacts: vec![vcard(&ctx, n, true)],
        });
    }
    Reply::sequence(urutan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_owner() -> oktz_plugin_api::config::Runtime {
        let mut cfg = oktz_plugin_api::config::Runtime::default();
        cfg.owner.name = "Oktz".into();
        cfg.owner.number = vec!["628999".into()];
        cfg.bot.name = "Oktz Bot".into();
        cfg
    }

    fn ctx(settings: Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "owner".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            settings,
            config: std::sync::Arc::new(cfg_with_owner()),
            ..Default::default()
        }
    }

    fn ctx_with_owners(settings: Value, db_owners: Vec<String>) -> Ctx {
        Ctx {
            db_owners,
            ..ctx(settings)
        }
    }

    fn contacts(reply: &Reply) -> &[String] {
        match reply {
            Reply::Contacts { contacts, .. } => contacts,
            _ => panic!("bukan Reply::Contacts: {reply:?}"),
        }
    }

    #[test]
    fn owner_union_config_then_db_preserves_raw_strings() {
        let c = ctx_with_owners(
            serde_json::json!({"owner": ["628111", "628111"]}),
            vec!["628111".into(), "+62 811-1".into(), "628999".into()],
        );
        assert_eq!(
            owner_numbers(&c),
            vec![
                "628999".to_string(),
                "628111".to_string(),
                "+62 811-1".to_string()
            ]
        );
    }

    #[test]
    fn settings_owner_and_settings_owner_numbers_are_ignored() {
        let c = ctx_with_owners(
            serde_json::json!({
                "owner": ["628111"],
                "ownerNumbers": ["628111", "628222"],
                "settings": {"owner": ["628333"]}
            }),
            Vec::new(),
        );
        assert_eq!(owner_numbers(&c), ["628999".to_string()]);
    }

    #[test]
    fn owner_names_map_then_config_membership_then_default() {
        let c = ctx(serde_json::json!({
            "ownerNames": {"628111": "Zann", "": "Kosong"}
        }));
        assert_eq!(owner_name(&c, "628111"), "Zann");
        assert_eq!(owner_name(&c, "+62 899-9"), "Oktz");
        assert_eq!(owner_name(&c, "628000"), "Owner");
        assert_eq!(owner_name(&c, ""), "Oktz");
        assert_eq!(owner_name(&c, "  "), "Kosong");
    }

    #[test]
    fn owner_type_two_only_for_numeric_two() {
        assert!(owner_type_two(&ctx(serde_json::json!({"ownerType": 2}))));
        assert!(owner_type_two(&ctx(serde_json::json!({"ownerType": 2.0}))));
        assert!(!owner_type_two(&ctx(serde_json::json!({"ownerType": "2"}))));
        assert!(!owner_type_two(&ctx(serde_json::json!({"ownerType": 0}))));
        assert!(!owner_type_two(&ctx(serde_json::json!({}))));
    }

    #[test]
    fn normal_owner_sends_text_then_contact_per_owner() {
        let r = owner(ctx(serde_json::json!({})));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert!(matches!(list[0], Reply::Text { .. }));
        assert_eq!(list.len(), 2);
        assert!(matches!(list[1], Reply::Contacts { .. }));
    }

    #[test]
    fn type_two_sends_combined_contacts_then_quoted_text() {
        let r = owner(ctx(serde_json::json!({"ownerType": 2})));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert!(matches!(list[0], Reply::Contacts { .. }));
        assert!(matches!(list[1], Reply::TextQuotedPrevious { .. }));
        assert_eq!(contacts(&list[0]).len(), 1);
    }

    #[test]
    fn vcard_has_no_email_and_exact_display_name() {
        let c = ctx(serde_json::json!({}));
        assert_eq!(
            vcard(&c, "628999", true),
            "BEGIN:VCARD\nVERSION:3.0\nFN:Oktz (Owner Oktz Bot)\nTEL;type=CELL;type=VOICE;waid=628999:+628999\nEND:VCARD"
        );
        assert_eq!(
            vcard(&c, "628999", false),
            "BEGIN:VCARD\nVERSION:3.0\nFN:Oktz\nTEL;type=CELL;type=VOICE;waid=628999:+628999\nEND:VCARD"
        );
    }

    #[test]
    fn type_two_previous_text_uses_extended_text_not_variant_nine() {
        let r = owner(ctx(serde_json::json!({"ownerType": 2})));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert!(matches!(list[1], Reply::TextQuotedPrevious { .. }));
    }

    #[test]
    fn zero_contacts_stops_trailing_text() {
        let mut config = cfg_with_owner();
        config.owner.number = Vec::new();
        let c = Ctx {
            config: std::sync::Arc::new(config),
            settings: serde_json::json!({"ownerType": 2}),
            ..Default::default()
        };
        let r = owner(c);
        let Reply::Contacts { contacts, .. } = r else {
            panic!("harus gagal eksplisit: {r:?}")
        };
        assert!(contacts.is_empty());
    }

    #[test]
    fn failed_contact_stops_trailing_text() {
        let r = owner(ctx(serde_json::json!({"ownerType": 2})));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert_eq!(list.len(), 2);
        assert!(matches!(list[1], Reply::TextQuotedPrevious { .. }));
    }

    #[test]
    fn db_owners_not_added_to_identity_owners() {
        let c = ctx_with_owners(serde_json::json!({}), vec!["628111".into()]);
        assert_eq!(c.db_owners(), ["628111"]);
        assert!(!c.is_owner);
    }
}
