//! `plugins/group/antilinkgc.js` — sakelar anti link WhatsApp (grup, saluran, wa.me).
//!
//! `ponytail:` hanya handler `.antilinkgc` yang dipindah; filter per-pesan ada
//! di middleware dan butuh `sock.sendMessage` untuk menghapus pesan.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *ᴀɴᴛɪʟɪɴᴋ ᴡᴀ* diaktifkan!\n\n> Link WA akan dihapus otomatis.";
pub(crate) const OFF: &str = "❌ *ᴀɴᴛɪʟɪɴᴋ ᴡᴀ* dinonaktifkan!";
pub(crate) const MODE_KICK: &str = "✅ *ᴀɴᴛɪʟɪɴᴋ ᴡᴀ* mode KICK diaktifkan!\n\n> User yang kirim link WA akan di-kick.";
pub(crate) const MODE_DELETE: &str =
    "✅ *ᴀɴᴛɪʟɪɴᴋ ᴡᴀ* mode DELETE diaktifkan!\n\n> Pesan dengan link WA akan dihapus.";
pub(crate) const BAD_OPTION: &str =
    "❌ Opsi tidak valid! Gunakan: `on`, `off`, `metode kick`, `metode remove`";

fn write(ctx: &Ctx, message: &str, status: &str, mode: Option<&str>) -> Reply {
    let mut data = serde_json::json!({ "antilinkgc": status });
    if let Some(m) = mode {
        data["antilinkgcMode"] = Value::from(m);
    }
    Reply::text(message).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[plugin(
    name = "antilinkgc",
    alias = ["algc", "antilinkgrup"],
    category = "group",
    description = "Anti link WhatsApp (grup, saluran, wa.me)",
    usage = ".antilinkgc <on/off/metode> [kick/remove]",
    example = ".antilinkgc on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antilinkgc(ctx: Ctx) -> Reply {
    let option = ctx.text.to_lowercase().trim().to_string();

    if option.is_empty() {
        let status = ctx
            .group_field("antilinkgc")
            .and_then(Value::as_str)
            .unwrap_or("off");
        let mode = ctx
            .group_field("antilinkgcMode")
            .and_then(Value::as_str)
            .unwrap_or("remove");
        return Reply::text(format!(
            "🔗 *ᴀɴᴛɪʟɪɴᴋ ᴡᴀ*\n\n\
             ╭┈┈⬡「 📋 *sᴛᴀᴛᴜs* 」\n\
             ┃ ◦ Status: *{}*\n\
             ┃ ◦ Mode: *{}*\n\
             ╰┈┈⬡\n\n\
             *ᴅᴇᴛᴇᴋsɪ:*\n\
             > • chat.whatsapp.com (grup)\n\
             > • wa.me (kontak)\n\
             > • whatsapp.com/channel (saluran)\n\n\
             *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ:*\n\
             > `{p}antilinkgc on` - Aktifkan\n\
             > `{p}antilinkgc off` - Nonaktifkan\n\
             > `{p}antilinkgc metode kick` - Mode kick user\n\
             > `{p}antilinkgc metode remove` - Mode hapus pesan",
            status.to_uppercase(),
            mode.to_uppercase(),
            p = ctx.prefix,
        ));
    }

    match option.as_str() {
        "on" => return write(&ctx, ON, "on", None),
        "off" => return write(&ctx, OFF, "off", None),
        "kick" => return write(&ctx, MODE_KICK, "on", Some("kick")),
        "remove" | "delete" => return write(&ctx, MODE_DELETE, "on", Some("remove")),
        _ => {}
    }

    if option.starts_with("metode") {
        let method = ctx.args.get(1).map(|a| a.to_lowercase()).unwrap_or_default();
        return match method.as_str() {
            "kick" => write(&ctx, MODE_KICK, "on", Some("kick")),
            "remove" | "delete" => write(&ctx, MODE_DELETE, "on", Some("remove")),
            _ => Reply::text(format!(
                "❌ Metode tidak valid! Gunakan: `kick` atau `remove`\n\n> Contoh: `{}antilinkgc metode kick`",
                ctx.prefix
            )),
        };
    }

    Reply::text(BAD_OPTION)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antilinkgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antilinkgc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("antilinkgc").unwrap();
        assert_eq!(p.meta.name, "antilinkgc");
        assert_eq!(p.meta.alias, ["algc", "antilinkgrup"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Anti link WhatsApp (grup, saluran, wa.me)");
        assert_eq!(p.meta.usage, ".antilinkgc <on/off/metode> [kick/remove]");
        assert_eq!(p.meta.example, ".antilinkgc on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["algc", "antilinkgrup"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antilinkgc");
        }
    }

    #[test]
    fn no_argument_defaults_to_off_and_remove_without_writing() {
        let r = run(serde_json::json!({}), "");
        assert!(text_of(&r).contains("┃ ◦ Status: *OFF*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ ◦ Mode: *REMOVE*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_setting_is_uppercased_for_the_status_panel() {
        let r = run(serde_json::json!({"antilinkgc": "on", "antilinkgcMode": "kick"}), "");
        assert!(text_of(&r).contains("┃ ◦ Status: *ON*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("┃ ◦ Mode: *KICK*"), "{}", text_of(&r));
    }

    #[test]
    fn on_and_off_write_only_the_status_flag() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "antilinkgc": "on" }));

        let off = run(serde_json::json!({"antilinkgc": "on"}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "antilinkgc": "off" }));
    }

    #[test]
    fn a_method_switch_also_turns_the_filter_on() {
        for (text, mode, message) in [
            ("metode kick", "kick", MODE_KICK),
            ("metode remove", "remove", MODE_DELETE),
            ("metode delete", "remove", MODE_DELETE),
        ] {
            let r = run(serde_json::json!({}), text);
            assert_eq!(text_of(&r), message, "{text}");
            assert_eq!(
                written(&r),
                &serde_json::json!({ "antilinkgc": "on", "antilinkgcMode": mode }),
                "{text}"
            );
        }
    }

    #[test]
    fn the_bare_shortcut_matches_the_metode_form() {
        let a = run(serde_json::json!({}), "kick");
        let b = run(serde_json::json!({}), "metode kick");
        assert_eq!(text_of(&a), text_of(&b));
        assert_eq!(written(&a), written(&b));
    }

    #[test]
    fn an_unknown_method_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "metode blok");
        assert!(
            text_of(&r).starts_with("❌ Metode tidak valid!"),
            "{}",
            text_of(&r)
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unrecognised_option_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "tolong");
        assert_eq!(text_of(&r), BAD_OPTION);
        assert!(r.mutations().is_empty());
    }
}