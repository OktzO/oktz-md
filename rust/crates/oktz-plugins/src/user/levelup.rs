//! `plugins/user/levelup.js` — `users[sender].settings.levelupNotif`.
//!
//! Default `true`: notifikasi aktif sampai ada yang `off` secara eksplisit.
//! Struct ini menyimpan seluruh objek `settings`, jadi field lain yang sudah
//! ada di user tidak boleh hilang saat plugin ini menulis.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_truthy_value;
use crate::common::first_keyword;

/// `user.settings.levelupNotif !== false`.
pub(crate) fn current_enabled(ctx: &Ctx) -> bool {
    match ctx.user_field("settings.levelupNotif") {
        // `!== false`: key yang HILANG berarti aktif, bukan nonaktif.
        None => true,
        Some(v) => js_truthy_value(v),
    }
}

/// Objek `settings` yang ada, dengan `levelupNotif` di-set.
pub(crate) fn settings_with(ctx: &Ctx, on: bool) -> serde_json::Value {
    let mut settings = ctx
        .user_field("settings")
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default();
    settings.insert("levelupNotif".to_string(), serde_json::Value::Bool(on));
    serde_json::Value::Object(settings)
}

pub(crate) fn enabled() -> &'static str {
    "✅ *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*\n\n> Status: *ON* ✅\n> Kamu akan menerima notifikasi saat naik level!"
}

pub(crate) fn disabled() -> &'static str {
    "❌ *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*\n\n> Status: *OFF* ❌\n> Notifikasi level up dinonaktifkan."
}

pub(crate) fn status_text(ctx: &Ctx) -> String {
    format!(
        "🔔 *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*\n\n\
         > Status saat ini: *{}*\n\n\
         ╭┈┈⬡「 📋 *ᴜsᴀɢᴇ* 」\n\
         ┃ > `{}levelup on` - Aktifkan\n\
         ┃ > `{}levelup off` - Nonaktifkan\n\
         ╰┈┈⬡",
        status_label(ctx),
        ctx.prefix,
        ctx.prefix
    )
}

pub(crate) fn status_label(ctx: &Ctx) -> &'static str {
    if current_enabled(ctx) { "ON ✅" } else { "OFF ❌" }
}

#[plugin(
    name = "levelup",
    alias = ["lvlup", "levelnotif"],
    category = "user",
    description = "Toggle notifikasi level up",
    usage = ".levelup <on/off>",
    example = ".levelup on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn levelup(ctx: Ctx) -> Reply {
    match first_keyword(&ctx).as_deref() {
        Some("on") => Reply::text(enabled()).with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "settings": settings_with(&ctx, true) }),
        }),
        Some("off") => Reply::text(disabled()).with_mutation(Mutation::SetUser {
            jid: ctx.sender.clone(),
            data: serde_json::json!({ "settings": settings_with(&ctx, false) }),
        }),
        _ => Reply::text(status_text(&ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(user: Value, arg: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "levelup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: arg.into(),
            args: if arg.is_empty() { vec![] } else { vec![arg.into()] },
            user_data: user,
            ..Default::default()
        };
        (oktz_plugin_api::find("levelup").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("levelup").unwrap();
        assert_eq!(p.meta.name, "levelup");
        assert_eq!(p.meta.alias, ["lvlup", "levelnotif"]);
        assert_eq!(p.meta.category, "user");
        assert_eq!(p.meta.description, "Toggle notifikasi level up");
        assert_eq!(p.meta.usage, ".levelup <on/off>");
        assert_eq!(p.meta.example, ".levelup on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["lvlup", "levelnotif"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "levelup");
        }
    }

    #[test]
    fn the_default_when_the_flag_is_absent_is_on() {
        // `user.settings.levelupNotif !== false` — undefined !== false.
        assert!(current_enabled(&Ctx::default()));
        for user in [serde_json::json!(null), serde_json::json!({}), serde_json::json!({"settings": {}})] {
            assert!(text_of(&run(user.clone(), "")).contains("ON ✅"), "input {user}");
            assert!(run(user.clone(), "").mutations().is_empty());
        }
    }

    #[test]
    fn an_explicit_false_turns_it_off() {
        let user = serde_json::json!({"settings": {"levelupNotif": false}});
        assert!(!current_enabled(&Ctx {
            user_data: user.clone(),
            ..Default::default()
        }));
        assert!(text_of(&run(user.clone(), "")).contains("OFF ❌"));
    }

    #[test]
    fn on_and_off_write_their_own_boolean() {
        assert_eq!(settings_of(&run(serde_json::json!({}), "on"))["levelupNotif"], serde_json::json!(true));
        assert_eq!(
            settings_of(&run(serde_json::json!({"settings": {"levelupNotif": false}}), "off"))["levelupNotif"],
            serde_json::json!(false)
        );
    }

    #[test]
    fn the_other_settings_fields_are_preserved() {
        let user = serde_json::json!({"settings": {"rpgLevelupNotif": false, "lain": 7}});
        let __balasan = run(user.clone(), "on");
        let s = settings_of(&__balasan);
        assert_eq!(s["rpgLevelupNotif"], serde_json::json!(false), "field lain harus utuh");
        assert_eq!(s["lain"], serde_json::json!(7));
        assert_eq!(s["levelupNotif"], serde_json::json!(true));
    }

    #[test]
    fn a_settings_field_that_is_not_an_object_is_replaced_not_merged() {
        // `!user.settings` hanya berlaku untuk null/undefined; kalau `settings`
        // sudah berisi string, JS akan menambah properti ke String (diam).
        // Yang aman dan konsisten: perlakukan sebagai objek kosong.
        let __balasan = run(serde_json::json!({"settings": "bukan-objek"}), "on");
        let s = settings_of(&__balasan);
        assert!(s.is_object(), "{s}");
        assert_eq!(s.as_object().unwrap().len(), 1);
    }

    #[test]
    fn no_argument_shows_status_and_writes_nothing() {
        for user in [serde_json::json!({}), serde_json::json!({"settings": {"levelupNotif": true}})] {
            let r = run(user.clone(), "");
            assert!(text_of(&r).starts_with("🔔 *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"), "input {user}");
            assert!(r.mutations().is_empty(), "input {user}");
        }
    }

    #[test]
    fn an_unknown_option_shows_status_and_writes_nothing() {
        for bad in ["maybe", "1", "ON!", "yes"] {
            let r = run(serde_json::json!({}), bad);
            assert!(text_of(&r).starts_with("🔔 *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"), "input {bad}");
            assert!(r.mutations().is_empty(), "input {bad}");
        }
    }

    #[test]
    fn the_switch_replies_name_the_direction_that_was_written() {
        assert!(text_of(&run(serde_json::json!({}), "on")).contains("*ON* ✅"));
        assert!(text_of(&run(serde_json::json!({}), "off")).contains("*OFF* ❌"));
        assert!(text_of(&run(serde_json::json!({}), "on")).starts_with("✅ *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"));
        assert!(text_of(&run(serde_json::json!({}), "off")).starts_with("❌ *ʟᴇᴠᴇʟ ᴜᴘ ɴᴏᴛɪꜰ*"));
    }

    #[test]
    fn the_switch_reply_is_paired_with_the_write() {
        for arg in ["on", "off"] {
            assert_eq!(
                run(serde_json::json!({}), arg).mutations().len(),
                1,
                "arg {arg}: reply tanpa mutasi = status palsu"
            );
        }
    }

    #[test]
    fn the_status_reply_uses_the_live_prefix() {
        let t = status_text(&Ctx {
            prefix: "~".into(),
            ..Default::default()
        });
        assert!(t.contains("~levelup on"), "{t}");
        assert!(t.contains("~levelup off"));
    }
}