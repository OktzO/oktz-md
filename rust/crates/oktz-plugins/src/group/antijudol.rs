//! `plugins/group/antijudol.js` — status dan mode deteksi konten judol.
//!
//! `ponytail:` `checkJudol` (deteksi per-pesan) tidak ikut — butuh
//! `sock.sendMessage` untuk menghapus pesan dan melempar kicked, dan `Ctx`
//! tidak punya keduanya. Yang dipindah di sini hanya handler `.antijudol`.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *AntiJudol diaktifkan*";
pub(crate) const OFF: &str = "❌ *AntiJudol dinonaktifkan*";
pub(crate) const MODE_KICK: &str = "✅ *AntiJudol mode KICK diaktifkan*";
pub(crate) const MODE_DELETE: &str = "✅ *AntiJudol mode DELETE diaktifkan*";
pub(crate) const BAD_METHOD: &str = "❌ Metode tidak valid! Gunakan: `kick` atau `remove`";
pub(crate) const BAD_OPTION: &str =
    "❌ Opsi tidak valid! Gunakan: `on`, `off`, `metode kick`, `metode remove`";

/// `db.setGroup(m.chat, { antijudol })` atau `{ antijudol, antijudolMode }`.
///
/// Kedua field disimpan sebagai string `'on'`/`'off'` — bukan boolean seperti
/// `antimedia` — jadi `status.toUpperCase()` di JS selalu berlaku.
fn write(ctx: &Ctx, message: &str, status: &str, mode: Option<&str>) -> Reply {
    let mut data = serde_json::json!({ "antijudol": status });
    if let Some(m) = mode {
        data["antijudolMode"] = Value::from(m);
    }
    Reply::text(message).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[plugin(
    name = "antijudol",
    alias = ["antijudi", "nojudi", "antislot"],
    category = "group",
    description = "Deteksi konten judol di grup",
    usage = ".antijudol <on/off/metode> [kick/remove]",
    example = ".antijudol on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antijudol(ctx: Ctx) -> Reply {
    let option = ctx.text.to_lowercase().trim().to_string();

    if option.is_empty() {
        let status = ctx
            .group_field("antijudol")
            .and_then(Value::as_str)
            .unwrap_or("off");
        let mode = ctx
            .group_field("antijudolMode")
            .and_then(Value::as_str)
            .unwrap_or("remove");
        return Reply::text(format!(
            "🎰 *ᴀɴᴛɪᴊᴜᴅᴏʟ*\n\n\
             > Status: *{}*\n\
             > Mode: *{}*\n\n\
             > Deteksi konten judol seperti judi, slot, gacor, maxwin, togel, bonus member, link alternatif, dan pola sejenis.\n\n\
             > `{}antijudol on`\n\
             > `{}antijudol off`\n\
             > `{}antijudol metode kick`\n\
             > `{}antijudol metode remove`",
            status.to_uppercase(),
            mode.to_uppercase(),
            ctx.prefix,
            ctx.prefix,
            ctx.prefix,
            ctx.prefix
        ));
    }

    match option.as_str() {
        "on" => return write(&ctx, ON, "on", None),
        "off" => return write(&ctx, OFF, "off", None),
        // Shortcut JS: `kick` dan `remove` tanpa kata `metode` menulis pair
        // yang sama persis dengan `metode kick` / `metode remove`.
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
            command: "antijudol".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antijudol").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antijudol").unwrap();
        assert_eq!(p.meta.name, "antijudol");
        assert_eq!(p.meta.alias, ["antijudi", "nojudi", "antislot"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Deteksi konten judol di grup");
        assert_eq!(p.meta.usage, ".antijudol <on/off/metode> [kick/remove]");
        assert_eq!(p.meta.example, ".antijudol on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["antijudi", "nojudi", "antislot"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antijudol");
        }
    }

    #[test]
    fn no_argument_reports_status_and_mode_without_writing() {
        let r = run(json!({}), "");
        assert!(text_of(&r).contains("> Status: *OFF*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Mode: *REMOVE*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn stored_status_and_mode_are_read_as_strings_not_booleans() {
        let r = run(json!({"antijudol": "on", "antijudolMode": "kick"}), "");
        assert!(text_of(&r).contains("> Status: *ON*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Mode: *KICK*"), "{}", text_of(&r));
    }

    #[test]
    fn on_writes_only_the_status_flag() {
        let r = run(json!({}), "on");
        assert_eq!(text_of(&r), ON);
        assert_eq!(written(&r), &json!({ "antijudol": "on" }));
    }

    #[test]
    fn off_writes_only_the_status_flag() {
        let r = run(json!({"antijudol": "on"}), "off");
        assert_eq!(text_of(&r), OFF);
        assert_eq!(written(&r), &json!({ "antijudol": "off" }));
    }

    #[test]
    fn metode_kick_turns_detection_on_and_records_the_mode() {
        let r = run(json!({}), "metode kick");
        assert_eq!(text_of(&r), MODE_KICK);
        assert_eq!(
            written(&r),
            &json!({ "antijudol": "on", "antijudolMode": "kick" })
        );
    }

    #[test]
    fn metode_accepts_delete_as_a_synonym_of_remove() {
        let r = run(json!({}), "metode delete");
        assert_eq!(text_of(&r), MODE_DELETE);
        assert_eq!(
            written(&r),
            &json!({ "antijudol": "on", "antijudolMode": "remove" })
        );
    }

    #[test]
    fn the_bare_kick_shortcut_matches_metode_kick() {
        let bare = run(json!({}), "kick");
        let prefixed = run(json!({}), "metode kick");
        assert_eq!(text_of(&bare), text_of(&prefixed));
        assert_eq!(written(&bare), written(&prefixed));
    }

    #[test]
    fn an_unknown_method_is_refused_and_nothing_is_written() {
        let r = run(json!({}), "metode lempar");
        assert_eq!(text_of(&r), BAD_METHOD);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unrecognised_option_is_refused_and_nothing_is_written() {
        let r = run(json!({}), "tolong");
        assert_eq!(text_of(&r), BAD_OPTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn options_are_matched_after_lowercase_and_trim() {
        let r = run(json!({}), "  METODE   KICK  ");
        assert_eq!(
            written(&r),
            &json!({ "antijudol": "on", "antijudolMode": "kick" })
        );
    }
}