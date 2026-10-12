//! `plugins/rpg/leveluprpg.js` — `users[sender].settings.rpgLevelupNotif`.
//!
//! Kloning dari `user/levelup.js` dengan nama field, ikon, dan teks berbeda.
//! Keduanya tetap file terpisah: `pluginConfig` JS-nya terpisah dan KEDUA
//! plugin menulis ke objek `settings` yang sama, jadi mereka tidak boleh
//! saling menimpa field satu sama lain.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{first_keyword, js_truthy_value};

pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    match ctx.user_field("settings.rpgLevelupNotif") {
        // `!== false`: key yang HILANG berarti aktif, bukan nonaktif.
        None => true,
        Some(v) => js_truthy_value(v),
    }
}

pub(crate) fn settings_with(ctx: &Ctx, on: bool) -> serde_json::Value {
    let mut settings = ctx
        .user_field("settings")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    settings.insert(
        "rpgLevelupNotif".to_string(),
        serde_json::Value::Bool(on),
    );
    serde_json::Value::Object(settings)
}

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "🔔 *ʀᴘɢ ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*\n\n\
         > Status saat ini: *{}*\n\n\
         *📋 *ᴜsᴀɢᴇ:*\n\n\
         > > `{}leveluprpg on` - Aktifkan\n\
         > > `{}leveluprpg off` - Nonaktifkan\n",
        if current_enabled(ctx) { "ON ✅" } else { "OFF ❌" },
        ctx.prefix,
        ctx.prefix
    )
}

#[plugin(
    name = "leveluprpg",
    alias = ["lvluprpg", "rpglevelup"],
    category = "rpg",
    description = "Toggle notifikasi level up RPG",
    usage = ".leveluprpg <on/off>",
    example = ".leveluprpg on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn leveluprpg(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        Some("on") => Reply::text(
            "✅ *ʀᴘɢ ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*\n\n> Status: *ON* ✅\n> Kamu akan menerima notifikasi RPG saat naik level!",
        )
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "settings": settings_with(&ctx, true) }),
        }),
        Some("off") => Reply::text(
            "❌ *ʀᴘɢ ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*\n\n> Status: *OFF* ❌\n> Notifikasi RPG level up dinonaktifkan.",
        )
        .with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "settings": settings_with(&ctx, false) }),
        }),
        _ => Reply::text(status_text(&ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "leveluprpg".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("leveluprpg").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn settings_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetUser { jid, data } => {
                assert_eq!(jid, "628111@s.whatsapp.net");
                data["settings"].clone()
            }
            other => panic!("harus SetUser, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("leveluprpg").unwrap();
        assert_eq!(p.meta.name, "leveluprpg");
        assert_eq!(p.meta.alias, ["lvluprpg", "rpglevelup"]);
        assert_eq!(p.meta.category, "rpg");
        assert_eq!(p.meta.description, "Toggle notifikasi level up RPG");
        assert_eq!(p.meta.usage, ".leveluprpg <on/off>");
        assert_eq!(p.meta.example, ".leveluprpg on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["lvluprpg", "rpglevelup"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "leveluprpg");
        }
    }

    #[test]
    fn the_default_when_the_flag_is_absent_is_on() {
        assert!(current_enabled(&Ctx::default()));
        for user in [json!(null), json!({}), json!({"settings": {}})] {
            assert!(
                text_of(&run(user.clone(), "")).contains("*ON ✅*"),
                "input {user}"
            );
            assert!(run(user, "").mutations().is_empty());
        }
    }

    #[test]
    fn an_explicit_false_turns_it_off() {
        let user = json!({"settings": {"rpgLevelupNotif": false}});
        assert!(text_of(&run(user, "")).contains("*OFF ❌*"));
    }

    #[test]
    fn on_and_off_write_their_own_boolean() {
        assert_eq!(
            settings_of(&run(json!({}), "on"))["rpgLevelupNotif"],
            json!(true)
        );
        assert_eq!(
            settings_of(&run(
                json!({"settings": {"rpgLevelupNotif": true}}),
                "off"
            ))["rpgLevelupNotif"],
            json!(false)
        );
    }

    /// Dua plugin ini menulis objek `settings` yang sama; kalau salah satu
    /// menimpa field milik yang lain, notifikasi yang lain ikut mati.
    #[test]
    fn writing_rpg_notif_leaves_the_plain_levelup_notif_intact() {
        let user = json!({"settings": {"levelupNotif": false}});
        let s = settings_of(&run(user, "on"));
        assert_eq!(s["levelupNotif"], json!(false), "harus tetap false");
        assert_eq!(s["rpgLevelupNotif"], json!(true));
    }

    #[test]
    fn writing_rpg_notif_preserves_other_settings_fields() {
        let user = json!({"settings": {"lain": 1, "levelupNotif": true}});
        let s = settings_of(&run(user, "off"));
        assert_eq!(s["lain"], json!(1));
        assert_eq!(s["levelupNotif"], json!(true));
    }

    #[test]
    fn no_argument_and_unknown_options_both_show_status_without_writing() {
        for bad in ["", "maybe", "1", "ON!"] {
            let r = run(json!({}), bad);
            assert!(
                text_of(&r).starts_with("🔔 *ʀᴘɢ ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"),
                "input {bad:?}"
            );
            assert!(r.mutations().is_empty(), "input {bad:?}");
        }
    }

    #[test]
    fn the_switch_replies_name_the_direction_that_was_written() {
        assert!(text_of(&run(json!({}), "on")).starts_with("✅ *ʀᴘɢ ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"));
        assert!(text_of(&run(json!({}), "off")).starts_with("❌ *ʀᴘɢ ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"));
    }

    #[test]
    fn the_switch_reply_is_paired_with_the_write() {
        for arg in ["on", "off"] {
            assert_eq!(run(json!({}), arg).mutations().len(), 1, "arg {arg}");
        }
    }

    #[test]
    fn the_status_reply_uses_the_live_prefix() {
        let t = status_text(&Ctx {
            prefix: "~".into(),
            ..Default::default()
        });
        assert!(t.contains("`~leveluprpg on`"), "{t}");
        assert!(t.contains("`~leveluprpg off`"));
    }

    #[test]
    fn the_two_levelup_toggles_keep_separate_keys() {
        for (name, keys) in [
            ("levelup", &["levelup", "lvlup", "levelnotif"][..]),
            ("leveluprpg", &["leveluprpg", "lvluprpg", "rpglevelup"][..]),
        ] {
            for k in keys {
                let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                    .iter()
                    .filter(|e| e.meta.matches(k))
                    .map(|e| e.meta.name.as_str())
                    .collect();
                assert_eq!(owners, [name], "key {k} diklaim oleh {owners:?}");
            }
        }
    }
}