//! `plugins/group/antiphising.js` — sakelar deteksi konten phising.
//!
//! `ponytail:` hanya handler `.antiphising` yang dipindah; deteksi per-pesan
//! ada di middleware dan butuh `sock.sendMessage` untuk menghapus pesan.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *AntiPhising diaktifkan*";
pub(crate) const OFF: &str = "❌ *AntiPhising dinonaktifkan*";
pub(crate) const MODE_KICK: &str = "✅ *AntiPhising mode KICK diaktifkan*";
pub(crate) const MODE_DELETE: &str = "✅ *AntiPhising mode DELETE diaktifkan*";
pub(crate) const BAD_METHOD: &str = "❌ Metode tidak valid! Gunakan: `kick` atau `remove`";
pub(crate) const BAD_OPTION: &str =
    "❌ Opsi tidak valid! Gunakan: `on`, `off`, `metode kick`, `metode remove`";

fn write(ctx: &Ctx, message: &str, status: &str, mode: Option<&str>) -> Reply {
    let mut data = serde_json::json!({ "antiphising": status });
    if let Some(m) = mode {
        data["antiphisingMode"] = Value::from(m);
    }
    Reply::text(message).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[plugin(
    name = "antiphising",
    alias = ["antiphishing", "antiscamlink", "nophising"],
    category = "group",
    description = "Deteksi konten phising di grup",
    usage = ".antiphising <on/off/metode> [kick/remove]",
    example = ".antiphising on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antiphising(ctx: Ctx) -> Reply {
    let option = ctx.text.to_lowercase().trim().to_string();

    if option.is_empty() {
        let status = ctx
            .group_field("antiphising")
            .and_then(Value::as_str)
            .unwrap_or("off");
        let mode = ctx
            .group_field("antiphisingMode")
            .and_then(Value::as_str)
            .unwrap_or("remove");
        return Reply::text(format!(
            "🎣 *ᴀɴᴛɪᴘʜɪsɪɴɢ*\n\n\
             > Status: *{}*\n\
             > Mode: *{}*\n\n\
             > Deteksi pesan phising seperti klik link, verifikasi akun, login palsu, shortener mencurigakan, URL IP, punycode, dan pola sejenis.\n\n\
             > `{p}antiphising on`\n\
             > `{p}antiphising off`\n\
             > `{p}antiphising metode kick`\n\
             > `{p}antiphising metode remove`",
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
            _ => Reply::text(BAD_METHOD),
        };
    }

    Reply::text(BAD_OPTION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "antiphising".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antiphising").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antiphising").unwrap();
        assert_eq!(p.meta.name, "antiphising");
        assert_eq!(
            p.meta.alias,
            ["antiphishing", "antiscamlink", "nophising"]
        );
        assert_eq!(p.meta.description, "Deteksi konten phising di grup");
        assert_eq!(p.meta.usage, ".antiphising <on/off/metode> [kick/remove]");
        assert_eq!(p.meta.example, ".antiphising on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["antiphishing", "antiscamlink", "nophising"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antiphising");
        }
    }

    #[test]
    fn no_argument_reports_the_stored_state_without_writing() {
        let r = run(json!({}), "");
        assert!(text_of(&r).contains("> Status: *OFF*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Mode: *REMOVE*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_stored_setting_is_uppercased() {
        let r = run(json!({"antiphising": "on", "antiphisingMode": "kick"}), "");
        assert!(text_of(&r).contains("> Status: *ON*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Mode: *KICK*"), "{}", text_of(&r));
    }

    #[test]
    fn on_and_off_write_only_the_status_flag() {
        let on = run(json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &json!({ "antiphising": "on" }));

        let off = run(json!({"antiphising": "on"}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &json!({ "antiphising": "off" }));
    }

    #[test]
    fn a_method_switch_also_turns_detection_on() {
        for (text, mode, message) in [
            ("metode kick", "kick", MODE_KICK),
            ("metode remove", "remove", MODE_DELETE),
            ("metode delete", "remove", MODE_DELETE),
        ] {
            let r = run(json!({}), text);
            assert_eq!(text_of(&r), message, "{text}");
            assert_eq!(
                written(&r),
                &json!({ "antiphising": "on", "antiphisingMode": mode }),
                "{text}"
            );
        }
    }

    #[test]
    fn the_bare_shortcut_matches_the_metode_form() {
        let a = run(json!({}), "kick");
        let b = run(json!({}), "metode kick");
        assert_eq!(text_of(&a), text_of(&b));
        assert_eq!(written(&a), written(&b));
    }

    #[test]
    fn an_unknown_method_is_refused_and_nothing_is_written() {
        let r = run(json!({}), "metode blok");
        assert_eq!(text_of(&r), BAD_METHOD);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unrecognised_option_is_refused_and_nothing_is_written() {
        let r = run(json!({}), "tolong");
        assert_eq!(text_of(&r), BAD_OPTION);
        assert!(r.mutations().is_empty());
    }
}