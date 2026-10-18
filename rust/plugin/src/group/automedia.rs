//! `plugins/group/automedia.js` — sakelar `groups[chat].automedia`.
//!
//! `ponytail:` `autoMediaHandler` yang diekspor ke `handler.js` tidak ikut.
//! Fungsi itu berjalan per-pesan dan butuh `m.download()` plus
//! `sock.sendMedia` — keduanya tidak ada di `Ctx`. Yang dipindah hanya
//! perintahnya.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_truthy;

/// `groupData.automedia ?? false` — `??` hanya menutup `null`/`undefined`, jadi
/// `"off"` tetap truthy dan `if (current)` di JS ikut terbaca `true`.
pub(crate) fn enabled(ctx: &Ctx) -> bool {
    js_truthy(ctx.group_field("automedia"))
}

pub(crate) fn status_text(current: bool) -> &'static str {
    if current { "✅ Aktif" } else { "❌ Nonaktif" }
}

pub(crate) fn status(ctx: &Ctx) -> String {
    format!(
        "🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n\
         > Status: {}\n\n\
         > Gunakan:\n\
         > `{}automedia on` - aktifkan\n\
         > `{}automedia off` - nonaktifkan\n\n\
         > _Otomatis jadikan sticker jadi gambar_\n\
         > Video gak jadi bang",
        status_text(enabled(ctx)),
        ctx.prefix,
        ctx.prefix
    )
}

#[plugin(
    name = "automedia",
    alias = ["automedi"],
    category = "group",
    description = "Toggle auto media - otomatis jadikan sticker jadi gambar/video",
    usage = ".automedia on/off",
    example = ".automedia on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn automedia(ctx: Ctx) -> Reply {
    let arg = ctx.args.first().map(|a| a.to_lowercase());
    let current = enabled(&ctx);

    match arg.as_deref() {
        None => Reply::text(status(&ctx)),
        Some("on" | "1" | "aktif") if current => {
            Reply::text("🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> Sudah aktif!")
        }
        Some("on" | "1" | "aktif") => {
            Reply::text(
                "🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> ✅ Berhasil diaktifkan!\n\
                 > Sticker akan otomatis jadi gambar/video",
            )
            .with_mutation(Mutation::SetGroup {
                jid: ctx.chat.clone(),
                data: serde_json::json!({ "automedia": true }),
            })
        }
        Some("off" | "0" | "nonaktif") if !current => {
            Reply::text("🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> Sudah nonaktif!")
        }
        Some("off" | "0" | "nonaktif") => {
            Reply::text("🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> ❌ Berhasil dinonaktifkan!").with_mutation(
                Mutation::SetGroup {
                    jid: ctx.chat.clone(),
                    data: serde_json::json!({ "automedia": false }),
                },
            )
        }
        _ => Reply::text(format!("❌ Gunakan: `{}automedia on/off`", ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(args: &[&str], group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "automedia".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: args.join(" "),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("automedia").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
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
        let p = oktz_plugin_api::find("automedia").unwrap();
        assert_eq!(p.meta.name, "automedia");
        assert_eq!(p.meta.alias, ["automedi"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Toggle auto media - otomatis jadikan sticker jadi gambar/video"
        );
        assert_eq!(p.meta.usage, ".automedia on/off");
        assert_eq!(p.meta.example, ".automedia on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin, "isBotAdmin tidak ada di JS");
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("automedi").unwrap().meta.name, "automedia");
    }

    #[test]
    fn no_argument_reports_the_stored_flag() {
        for (stored, label) in [
            (serde_json::json!(true), "✅ Aktif"),
            (serde_json::json!(false), "❌ Nonaktif"),
            // `{}` truthy di JS — objek kosong bukan "tidak ada".
            (serde_json::json!({}), "✅ Aktif"),
        ] {
            let r = run(&[], serde_json::json!({ "automedia": stored }));
            assert!(body(&r).contains(&format!("> Status: {label}\n")), "{stored}: {}", body(&r));
            assert!(body(&r).contains("> `.automedia on` - aktifkan"), "{}", body(&r));
            assert!(r.mutations().is_empty(), "{stored} tidak boleh ditulis ulang");
        }
    }

    #[test]
    fn a_group_never_seen_in_the_store_reports_nonaktif() {
        let r = run(&[], Value::Null);
        assert!(body(&r).contains("> Status: ❌ Nonaktif"), "{}", body(&r));
    }

    #[test]
    fn every_on_spelling_writes_true_once() {
        for arg in ["on", "1", "aktif", "ON", "Aktif"] {
            let r = run(&[arg], serde_json::json!({}));
            assert_eq!(
                body(&r),
                "🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> ✅ Berhasil diaktifkan!\n> Sticker akan otomatis jadi gambar/video",
                "arg {arg}"
            );
            assert_eq!(group_write(&r), &serde_json::json!({"automedia": true}), "arg {arg}");
        }
    }

    #[test]
    fn every_off_spelling_writes_false_once() {
        for arg in ["off", "0", "nonaktif", "OFF"] {
            let r = run(&[arg], serde_json::json!({ "automedia": true }));
            assert_eq!(
                body(&r),
                "🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> ❌ Berhasil dinonaktifkan!",
                "arg {arg}"
            );
            assert_eq!(group_write(&r), &serde_json::json!({"automedia": false}), "arg {arg}");
        }
    }

    #[test]
    fn repeating_the_current_state_reports_it_and_writes_nothing() {
        for (arg, on) in [("on", true), ("off", false)] {
            let r = run(&[arg], serde_json::json!({ "automedia": on }));
            let expected = if on { "Sudah aktif!" } else { "Sudah nonaktif!" };
            assert_eq!(body(&r), format!("🎬 *ᴀᴜᴛᴏᴍᴇᴅɪᴀ*\n\n> {expected}"), "arg {arg}");
            assert!(r.mutations().is_empty(), "arg {arg} tidak boleh menulis");
        }
    }

    #[test]
    fn an_unknown_keyword_falls_through_to_the_usage_line() {
        for arg in ["males", "nyala", "yes"] {
            let r = run(&[arg], serde_json::json!({}));
            assert_eq!(body(&r), "❌ Gunakan: `.automedia on/off`", "arg {arg}");
            assert!(r.mutations().is_empty(), "arg {arg}");
        }
    }

    #[test]
    fn a_non_boolean_stored_value_follows_js_truthiness() {
        // `?? false` hanya menutup null/undefined; `if (current)` tetap pakai
        // kebenaran JS, jadi `"off"` dianggap aktif.
        assert!(!enabled(&Ctx {
            group_data: serde_json::json!({"automedia": false}),
            ..Default::default()
        }));
        assert!(!enabled(&Ctx {
            group_data: serde_json::json!({"automedia": 0}),
            ..Default::default()
        }));
        assert!(enabled(&Ctx {
            group_data: serde_json::json!({"automedia": "off"}),
            ..Default::default()
        }));
        assert!(enabled(&Ctx {
            group_data: serde_json::json!({"automedia": {}}),
            ..Default::default()
        }));
    }
}