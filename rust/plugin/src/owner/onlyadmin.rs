//! `plugins/owner/onlyadmin.js` — flag `onlyAdmin` per-grup atau global.
//!
//! Tiga command berbagi satu handler: `onlyadmin` (on/off/status),
//! `selfadmin`, dan `publicadmin`. Alias/selfAdmin/publicAdmin hanya melayani
//! pem nutritious cek router — `mode_settings_from` di `pipeline.rs:250`
//! membaca ketiganya dari `settings`, dan `group_overrides_from` dari
//! `groups[chat]`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

const OFF_TEXT: &str = "❌ *ᴏɴʟʏᴀᴅᴍɪɴ ɴᴏɴᴀᴋᴛɪꜰ*\n\n> Bot bisa diakses semua orang";
const WARN_OFF: &str = "⚠️ OnlyAdmin sudah nonaktif.";
const WARN_ON: &str = "⚠️ OnlyAdmin sudah aktif.";
const INVALID: &str = "❌ Argumen tidak valid. Gunakan: `on` atau `off`";

/// `readAdminFlag` — di grup, override grup menang HANYA kalau key-nya ada;
/// key yang hilang jatuh ke setting global, bukan ke default.
pub(crate) fn read_admin_flag(ctx: &Ctx, key: &str) -> bool {
    if ctx.is_group
        && let Some(v) = ctx.group_field(key)
    {
        return crate::common::js_truthy_value(v);
    }
    ctx.setting_bool(key, false)
}

/// `writeAdminFlags` — grup dapat satu `setGroup` (Object.assign), private
///-chat dapat satu `db.setting` per key.
pub(crate) fn flag_mutations(ctx: &Ctx, flags: &[(&str, bool)]) -> Vec<Mutation> {
    if ctx.is_group {
        let data = flags
            .iter()
            .map(|(k, v)| ((*k).to_string(), serde_json::Value::Bool(*v)))
            .collect::<serde_json::Map<_, _>>();
        return vec![Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::Value::Object(data),
        }];
    }
    flags
        .iter()
        .map(|(k, v)| Mutation::SetSetting {
            key: (*k).to_string(),
            value: serde_json::Value::Bool(*v),
        })
        .collect()
}

/// Tiga flag yang selalu ditulis bersama saat menyalakan — `selfAdmin` dan
/// `publicAdmin` harus ikut mati kalau tidak, kalau tidak router masih
/// melihat mode lama.
pub(crate) const ON_FLAGS: [(&str, bool); 3] =
    [("onlyAdmin", true), ("selfAdmin", false), ("publicAdmin", false)];
pub(crate) const OFF_FLAGS: [(&str, bool); 1] = [("onlyAdmin", false)];

pub(crate) fn access_box(public_private: bool) -> String {
    format!(
        "╭┈┈⬡「 🔒 *ᴀᴋsᴇs* 」\n\
         ┃ ✅ Admin grup\n\
         ┃ ✅ Owner bot\n\
         {private_line}\
         ┃ ❌ Member biasa\n\
         ╰┈┈┈┈┈┈┈┈⬡",
        private_line = if public_private {
            "┃ ✅ Private chat (semua)\n"
        } else {
            ""
        }
    )
}

pub(crate) fn status_text(current: bool) -> String {
    format!(
        "🔒 *ᴏɴʟʏᴀᴅᴍɪɴ*\n\n\
         > Status: {status}\n\n\
         *Penggunaan:*\n\
         > `.onlyadmin on` — Aktifkan\n\
         > `.onlyadmin off` — Nonaktifkan\n\n\
         _Hanya admin grup, owner, dan private chat yang bisa akses bot_",
        status = if current { "✅ Aktif" } else { "❌ Nonaktif" }
    )
}

#[plugin(
    name = "onlyadmin",
    alias = ["selfadmin", "publicadmin", "adminonly"],
    category = "owner",
    description = "Hanya admin grup yang bisa akses command bot",
    usage = ".onlyadmin on/off",
    example = ".onlyadmin on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn onlyadmin(ctx: Ctx) -> Reply {
    let current = read_admin_flag(&ctx, "onlyAdmin");
    let args = crate::common::first_keyword(&ctx);

    // `cmd === 'selfadmin' | 'publicadmin'` — alias yang sama-sama memaksa
    // hanyaAdmin aktif, beda hanya teks akses private-nya.
    if ctx.command == "selfadmin" || ctx.command == "publicadmin" {
        if current {
            return Reply::text(OFF_TEXT)
                .with_reaction("❌")
                .mutating(flag_mutations(&ctx, &OFF_FLAGS));
        }
        let public_private = ctx.command == "publicadmin";
        return Reply::text(format!(
            "✅ *ᴏɴʟʏᴀᴅᴍɪɴ ᴀᴋᴛɪꜰ*\n\n{}\n\n> Gunakan `.onlyadmin off` untuk menonaktifkan",
            access_box(public_private)
        ))
        .with_reaction("✅")
        .mutating(flag_mutations(&ctx, &ON_FLAGS));
    }

    match args.as_deref() {
        None | Some("status") => Reply::text(status_text(current)),
        Some("on") if current => Reply::text(WARN_ON),
        Some("on") => Reply::text(format!(
            "✅ *ᴏɴʟʏᴀᴅᴍɪɴ ᴀᴋᴛɪꜰ*\n\n{}",
            access_box(true)
        ))
        .with_reaction("✅")
        .mutating(flag_mutations(&ctx, &ON_FLAGS)),
        Some("off") if !current => Reply::text(WARN_OFF),
        Some("off") => Reply::text(OFF_TEXT)
            .with_reaction("❌")
            .mutating(flag_mutations(&ctx, &OFF_FLAGS)),
        Some(_) => Reply::text(INVALID),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(command: &str, args: &str, is_group: bool, group: Value, settings: Value) -> Reply {
        let args_vec: Vec<String> = args.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: if is_group {
                "120363@g.us".into()
            } else {
                "628111@s.whatsapp.net".into()
            },
            is_group,
            text: args.into(),
            args: args_vec,
            group_data: group,
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find(command).unwrap().handler)(ctx)
    }

    fn private(command: &str, args: &str, settings: Value) -> Reply {
        run(command, args, false, Value::Null, settings)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn written(r: &Reply, is_group: bool) -> Vec<(String, Value)> {
        r.mutations()
            .iter()
            .map(|m| match (m, is_group) {
                (Mutation::SetSetting { key, value }, false) => (key.clone(), value.clone()),
                (Mutation::SetGroup { jid, data }, true) => (jid.clone(), data.clone()),
                (other, _) => panic!("mutasi tidak sesuai mode: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("onlyadmin").unwrap();
        assert_eq!(p.meta.name, "onlyadmin");
        assert_eq!(
            p.meta.alias,
            ["selfadmin", "publicadmin", "adminonly"]
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(
            p.meta.description,
            "Hanya admin grup yang bisa akses command bot"
        );
        assert_eq!(p.meta.usage, ".onlyadmin on/off");
        assert_eq!(p.meta.example, ".onlyadmin on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["selfadmin", "publicadmin", "adminonly"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "onlyadmin");
        }
    }

    #[test]
    fn no_argument_reports_status_without_writing() {
        for (settings, expected) in [
            (serde_json::json!({}), "❌ Nonaktif"),
            (serde_json::json!({"onlyAdmin": true}), "✅ Aktif"),
        ] {
            let r = private("onlyadmin", "", settings.clone());
            assert!(text_of(&r).contains(expected), "settings {settings}");
            assert!(r.mutations().is_empty());
        }
        assert!(text_of(&private("onlyadmin", "status", serde_json::json!({}))).contains("❌ Nonaktif"));
    }

    #[test]
    fn on_writes_all_three_flags_so_the_router_cannot_keep_the_old_mode() {
        let r = private("onlyadmin", "on", serde_json::json!({"onlyAdmin": false, "selfAdmin": true}));
        let w = written(&r, false);
        assert_eq!(
            w,
            vec![
                ("onlyAdmin".to_string(), serde_json::json!(true)),
                ("selfAdmin".to_string(), serde_json::json!(false)),
                ("publicAdmin".to_string(), serde_json::json!(false)),
            ]
        );
        assert!(text_of(&r).contains("✅ *ᴏɴʟʏᴀᴅᴍɪɴ ᴀᴋᴛɪꜰ*"));
    }

    #[test]
    fn on_when_already_on_only_warns() {
        let r = private("onlyadmin", "on", serde_json::json!({"onlyAdmin": true}));
        assert_eq!(text_of(&r), WARN_ON);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn off_writes_only_onlyadmin_and_warns_when_it_was_already_off() {
        let r = private("onlyadmin", "off", serde_json::json!({"onlyAdmin": true}));
        assert_eq!(written(&r, false), vec![("onlyAdmin".to_string(), serde_json::json!(false))]);
        assert_eq!(text_of(&r), OFF_TEXT);

        let r = private("onlyadmin", "off", serde_json::json!({}));
        assert_eq!(text_of(&r), WARN_OFF);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_group_override_wins_and_is_written_to_the_group_entry() {
        let r = run(
            "onlyadmin",
            "on",
            true,
            serde_json::json!({"onlyAdmin": false}),
            serde_json::json!({"onlyAdmin": true}),
        );
        // key ada di grup → nilai grup (false) yang dibaca, bukan global.
        assert!(text_of(&r).contains("✅ *ᴏɴʟʏᴀᴅᴍɪɴ ᴀᴋᴛɪꜰ*"));
        let w = written(&r, true);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].0, "120363@g.us");
        assert_eq!(w[0].1, serde_json::json!({"onlyAdmin": true, "selfAdmin": false, "publicAdmin": false}));
    }

    #[test]
    fn a_group_without_the_key_falls_back_to_the_global_setting() {
        let r = run("onlyadmin", "", true, serde_json::json!({"mute": true}), serde_json::json!({"onlyAdmin": true}));
        assert!(text_of(&r).contains("✅ Aktif"), "harus baca setting global");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn selfadmin_and_publicadmin_differ_only_in_the_private_chat_line() {
        let self_admin = private("selfadmin", "", serde_json::json!({}));
        let public_admin = private("publicadmin", "", serde_json::json!({}));
        assert!(
            !text_of(&self_admin).contains("Private chat (semua)"),
            "selfadmin menutup private chat: {}",
            text_of(&self_admin)
        );
        assert!(text_of(&public_admin).contains("┃ ✅ Private chat (semua)"));
        assert_eq!(written(&public_admin, false), written(&self_admin, false));
    }

    #[test]
    fn selfadmin_turns_the_flag_off_again() {
        let r = private("selfadmin", "", serde_json::json!({"onlyAdmin": true}));
        assert_eq!(text_of(&r), OFF_TEXT);
        assert_eq!(written(&r, false), vec![("onlyAdmin".to_string(), serde_json::json!(false))]);
    }

    #[test]
    fn an_unknown_argument_is_rejected() {
        for bad in ["ON!", "enable", "1", "yes"] {
            let r = private("onlyadmin", bad, serde_json::json!({}));
            assert_eq!(text_of(&r), INVALID, "arg {bad}");
            assert!(r.mutations().is_empty(), "arg {bad}");
        }
    }
}