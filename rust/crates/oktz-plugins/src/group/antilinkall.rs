//! `plugins/group/antilinkall.js` — sakelar anti semua jenis link.
//!
//! `ponytail:` hanya handler `.antilinkall` yang dipindah. Deteksi per-pesan
//! ada di middleware dan butuh `sock.sendMessage` untuk menghapus pesan —
//! `Ctx` tidak punya itu.

use serde_json::Value;
use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON: &str = "✅ *Antilink All Aktif*\n\n> Semua link akan dideteksi otomatis\n> Mendeteksi domain extension, bukan hanya http/https";
pub(crate) const OFF: &str = "❌ *Antilink All Nonaktif*\n\n> Link tidak akan difilter lagi";
pub(crate) const MODE_KICK: &str = "✅ *Antilink All — Mode Kick*\n\n> User yang kirim link akan di-kick";
pub(crate) const MODE_DELETE: &str = "✅ *Antilink All — Mode Delete*\n\n> Pesan dengan link akan dihapus";
pub(crate) const BAD_METHOD: &str = "❌ *Metode Tidak Valid*\n\n> Gunakan *kick* atau *remove*\n> Contoh: *.antilinkall metode kick*";
pub(crate) const BAD_OPTION: &str =
    "❌ *Opsi Tidak Valid*\n\n> Gunakan *on*, *off*, *metode kick*, atau *metode remove*";

fn write(ctx: &Ctx, message: &str, status: &str, mode: Option<&str>) -> Reply {
    let mut data = serde_json::json!({ "antilinkall": status });
    if let Some(m) = mode {
        data["antilinkallMode"] = Value::from(m);
    }
    Reply::text(message).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data,
    })
}

#[plugin(
    name = "antilinkall",
    alias = ["alall", "antialllink"],
    category = "group",
    description = "Anti semua jenis link (deteksi domain extension)",
    usage = ".antilinkall <on/off/metode> [kick/remove]",
    example = ".antilinkall on",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn antilinkall(ctx: Ctx) -> Reply {
    let option = ctx.text.to_lowercase().trim().to_string();

    if option.is_empty() {
        let status = ctx
            .group_field("antilinkall")
            .and_then(Value::as_str)
            .unwrap_or("off");
        let mode = ctx
            .group_field("antilinkallMode")
            .and_then(Value::as_str)
            .unwrap_or("remove");
        return Reply::text(format!(
            "🔗 *Antilink All*\n\n\
             > Status: *{}*\n\
             > Mode: *{}*\n\n\
             *DETEKSI:*\n\
             > • https:// / http:// (dengan protokol)\n\
             > • www. (subdomain)\n\
             > • Domain extension (.com, .id, .io, .net, dll)\n\
             > • Shortlink (bit.ly, t.me, tinyurl, dll)\n\n\
             *PENGGUNAAN:*\n\
             > *{p}antilinkall on* — Aktifkan\n\
             > *{p}antilinkall off* — Nonaktifkan\n\
             > *{p}antilinkall metode kick* — Mode kick user\n\
             > *{p}antilinkall metode remove* — Mode hapus pesan",
            if status == "on" { "Aktif ✅" } else { "Nonaktif ❌" },
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
                "{BAD_METHOD}\n> Contoh: *{}antilinkall metode kick*",
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
            command: "antilinkall".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("antilinkall").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("antilinkall").unwrap();
        assert_eq!(p.meta.name, "antilinkall");
        assert_eq!(p.meta.alias, ["alall", "antialllink"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Anti semua jenis link (deteksi domain extension)"
        );
        assert_eq!(p.meta.usage, ".antilinkall <on/off/metode> [kick/remove]");
        assert_eq!(p.meta.example, ".antilinkall on");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["alall", "antialllink"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "antilinkall");
        }
    }

    #[test]
    fn no_argument_reports_the_stored_state_without_writing() {
        let r = run(serde_json::json!({}), "");
        assert!(text_of(&r).contains("> Status: *Nonaktif ❌*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Mode: *REMOVE*"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_active_setting_is_reported_as_active() {
        let r = run(serde_json::json!({"antilinkall": "on", "antilinkallMode": "kick"}), "");
        assert!(text_of(&r).contains("> Status: *Aktif ✅*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Mode: *KICK*"), "{}", text_of(&r));
    }

    #[test]
    fn on_and_off_write_only_the_status_flag() {
        let on = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&on), ON);
        assert_eq!(written(&on), &serde_json::json!({ "antilinkall": "on" }));

        let off = run(serde_json::json!({"antilinkall": "on"}), "off");
        assert_eq!(text_of(&off), OFF);
        assert_eq!(written(&off), &serde_json::json!({ "antilinkall": "off" }));
    }

    #[test]
    fn metode_kick_also_turns_the_filter_on() {
        let r = run(serde_json::json!({}), "metode kick");
        assert_eq!(text_of(&r), MODE_KICK);
        assert_eq!(
            written(&r),
            &serde_json::json!({ "antilinkall": "on", "antilinkallMode": "kick" })
        );
    }

    #[test]
    fn delete_is_a_synonym_of_remove_on_both_spellings() {
        for text in ["delete", "metode delete"] {
            let r = run(serde_json::json!({}), text);
            assert_eq!(text_of(&r), MODE_DELETE, "{text}");
            assert_eq!(
                written(&r),
                &serde_json::json!({ "antilinkall": "on", "antilinkallMode": "remove" }),
                "{text}"
            );
        }
    }

    #[test]
    fn the_bare_shortcut_writes_the_same_pair_as_metode() {
        for (short, long) in [("kick", "metode kick"), ("remove", "metode remove")] {
            let a = run(serde_json::json!({}), short);
            let b = run(serde_json::json!({}), long);
            assert_eq!(text_of(&a), text_of(&b), "{short}");
            assert_eq!(written(&a), written(&b), "{short}");
        }
    }

    #[test]
    fn an_unknown_method_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "metode blok");
        assert!(text_of(&r).starts_with("❌ *Metode Tidak Valid*"), "{}", text_of(&r));
        assert!(text_of(&r).contains(".antilinkall metode kick"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn an_unrecognised_option_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({}), "tolong");
        assert_eq!(text_of(&r), BAD_OPTION);
        assert!(r.mutations().is_empty());
    }
}