//! `plugins/owner/energitoggle.js` — toggle global `settings.energi`.
//!
//! `name` di JS berupa array `['disableenergi', 'enableenergi']`: keduanya
//! terdaftar sebagai command, jadi yang kedua masuk ke `alias` (cerminan
//! `normalizePluginNames` di `plugins.js:76-85`).

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) const ON_MSG: &str = "⚡ *sɪsᴛᴇᴍ ᴇɴᴇʀɢɪ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Setiap command sekarang memerlukan energi.";
pub(crate) const OFF_MSG: &str = "🔌 *sɪsᴛᴇᴍ ᴇɴᴇʀɢɪ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*\n\n> Command tidak lagi membutuhkan energi.";

/// `['enableenergi', 'onenergi'].includes(m.command)` — daftar literal, bukan
/// `args`, jadi `enableenergi` di posisi lain tidak ikut recognized.
pub(crate) fn is_enable_command(command: &str) -> bool {
    matches!(command, "enableenergi" | "onenergi")
}

#[plugin(
    name = "disableenergi",
    alias = ["enableenergi", "offenergi", "onenergi"],
    category = "owner",
    description = "Enable/disable sistem energi",
    usage = ".disableenergi atau .enableenergi",
    example = ".disableenergi",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn energitoggle(ctx: Ctx) -> Reply {
    let enable = is_enable_command(&ctx.command);
    let msg = if enable { ON_MSG } else { OFF_MSG };
    Reply::text(msg).with_reaction(if enable { "⚡" } else { "🔌" }).with_mutation(
        Mutation::SetSetting {
            key: "energi".into(),
            value: serde_json::Value::Bool(enable),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(command: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        (oktz_plugin_api::find(command).unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn stored_energi(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, "energi");
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn reaction_of(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("disableenergi").unwrap();
        assert_eq!(p.meta.name, "disableenergi");
        assert_eq!(
            p.meta.alias,
            ["enableenergi", "offenergi", "onenergi"],
            "nama kedua pada array `name` JS ikut terdaftar"
        );
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Enable/disable sistem energi");
        assert_eq!(p.meta.usage, ".disableenergi atau .enableenergi");
        assert_eq!(p.meta.example, ".disableenergi");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_registered_name_resolves_to_this_plugin() {
        for a in ["enableenergi", "offenergi", "onenergi"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "disableenergi");
        }
    }

    #[test]
    fn the_enable_names_write_true_and_the_rest_write_false() {
        for cmd in ["enableenergi", "onenergi"] {
            let r = run(cmd);
            assert_eq!(stored_energi(&r), serde_json::json!(true), "cmd {cmd}");
            assert_eq!(text_of(&r), ON_MSG);
            assert_eq!(reaction_of(&r), "⚡");
        }
        for cmd in ["disableenergi", "offenergi"] {
            let r = run(cmd);
            assert_eq!(stored_energi(&r), serde_json::json!(false), "cmd {cmd}");
            assert_eq!(text_of(&r), OFF_MSG);
            assert_eq!(reaction_of(&r), "🔌");
        }
    }

    #[test]
    fn an_unlisted_command_defaults_to_the_disable_branch() {
        // JS tidak punya guard: `isEnable` false untuk semua command lain,
        // jadi plugin ini HANYA aktif lewat empat nama terdaftar saja.
        assert!(!is_enable_command("energi"));
        assert!(!is_enable_command("enable"));
        assert!(is_enable_command("enableenergi"));
    }

    #[test]
    fn the_setting_is_the_only_thing_written() {
        let r = run("disableenergi");
        assert_eq!(r.mutations().len(), 1);
        assert!(matches!(r.mutations()[0], Mutation::SetSetting { .. }));
    }
}