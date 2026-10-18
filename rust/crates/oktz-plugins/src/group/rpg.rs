//! `plugins/group/rpg.js` — `groups[chat].rpg` on/off.
//!
//! Struktur identik dengan `plugins/group/game.js`; yang berbeda hanya nama
//! field, ikon, dan teks. `setGroup` mengisi default `rpg: true`
//! (`database.js:737`), jadi grup baru dianggap RPG aktif.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{js_truthy, text_keyword};

pub(crate) fn current(ctx: &Ctx) -> bool {
    match ctx.group_field("rpg") {
        Some(v) => js_truthy(Some(v)),
        None => true,
    }
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚔️ *FITUR RPG GRUP*\n\n\
         Gunakan perintah ini untuk mengatur akses member ke fitur RPG.\n\n\
         • *{p}rpg on* - Member bisa main RPG\n\
         • *{p}rpg off* - Member tidak bisa main RPG\n\n\
         *Catatan:* Admin tetap bisa mengakses RPG meskipun dimatikan.",
        p = prefix
    )
}

pub(crate) fn already(is_enable: bool) -> String {
    format!(
        "⚔️ Fitur RPG sudah *{}* di grup ini.",
        if is_enable { "AKTIF" } else { "NONAKTIF" }
    )
}

pub(crate) fn applied(is_enable: bool) -> String {
    let tail = if is_enable {
        "Member sekarang bisa menggunakan semua perintah di menu RPG."
    } else {
        "Member tidak akan bisa menggunakan perintah RPG lagi."
    };
    format!(
        "✅ Berhasil *{}* fitur RPG di grup ini!\n\n{tail}",
        if is_enable { "MENGAKTIFKAN" } else { "MENONAKTIFKAN" },
    )
}

#[plugin(
    name = "rpg",
    alias = ["togglerpg"],
    category = "group",
    description = "Mengaktifkan atau menonaktifkan fitur RPG di grup",
    usage = ".rpg <on/off>",
    example = ".rpg on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn rpg(ctx: Ctx) -> Reply {
    let is_enable = match text_keyword(&ctx).as_str() {
        "on" => true,
        "off" => false,
        _ => return Reply::text(help(&ctx.prefix)),
    };

    if current(&ctx) == is_enable {
        return Reply::text(already(is_enable));
    }

    Reply::text(applied(is_enable))
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "rpg": is_enable }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "rpg".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("rpg").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
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
        let p = oktz_plugin_api::find("rpg").unwrap();
        assert_eq!(p.meta.name, "rpg");
        assert_eq!(p.meta.alias, ["togglerpg"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Mengaktifkan atau menonaktifkan fitur RPG di grup"
        );
        assert_eq!(p.meta.usage, ".rpg <on/off>");
        assert_eq!(p.meta.example, ".rpg on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("togglerpg").unwrap().meta.name, "rpg");
    }

    #[test]
    fn an_unrecognised_option_shows_help_and_writes_nothing() {
        for bad in ["", "nyala", "1", "of", "onn"] {
            let r = run(serde_json::json!({"rpg": false}), bad);
            assert!(
                text_of(&r).starts_with("⚔️ *FITUR RPG GRUP*"),
                "input {bad:?} harus help"
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn the_help_branch_never_reads_the_stored_flag() {
        for g in [serde_json::json!({"rpg": true}), serde_json::json!({"rpg": false}), serde_json::json!({}), serde_json::json!(null)] {
            assert!(
                text_of(&run(g.clone(), "")).starts_with("⚔️ *FITUR RPG GRUP*"),
                "input {g}"
            );
        }
    }

    #[test]
    fn a_fresh_group_defaults_to_rpg_enabled() {
        // `setGroup` default `rpg: true` — `on` pada grup baru itu no-op.
        assert!(current(&Ctx::default()));
        let r = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&r), already(true));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn off_writes_false_and_on_writes_true_from_the_other_state() {
        let off = run(serde_json::json!({}), "off");
        assert_eq!(group_write(&off)["rpg"], serde_json::json!(false));
        assert!(text_of(&off).contains("MENONAKTIFKAN"));

        let on = run(serde_json::json!({"rpg": false}), "on");
        assert_eq!(group_write(&on)["rpg"], serde_json::json!(true));
        assert!(text_of(&on).contains("MENGAKTIFKAN"));
    }

    #[test]
    fn both_explicit_no_ops_are_reported_without_writing() {
        for (g, arg, label) in [
            (serde_json::json!({"rpg": true}), "on", "AKTIF"),
            (serde_json::json!({"rpg": false}), "off", "NONAKTIF"),
        ] {
            let r = run(g.clone(), arg);
            assert!(text_of(&r).contains(label), "input {g} arg {arg}");
            assert!(r.mutations().is_empty(), "input {g} arg {arg}");
        }
    }

    #[test]
    fn a_key_that_exists_but_is_falsy_is_not_treated_as_missing() {
        let g = |v: Value| Ctx {
            group_data: serde_json::json!({ "rpg": v }),
            ..Default::default()
        };
        assert!(!current(&g(serde_json::json!(0))));
        assert!(!current(&g(serde_json::json!(""))));
        assert!(!current(&g(serde_json::json!(false))));
        assert!(current(&Ctx::default()));
    }

    #[test]
    fn the_option_is_lowercased_before_comparison() {
        // `m.text?.trim()?.toLowerCase()` — jadi "ON" dikenali, bukan help.
        // Diuji dari state yang berlawanan supaya "dikenali" berarti
        // "menulis"; dari state default `true`, "on" adalah no-op.
        for arg in ["ON", "On", "on"] {
            let r = run(serde_json::json!({"rpg": false}), arg);
            assert_eq!(
                group_write(&r)["rpg"],
                serde_json::json!(true),
                "arg {arg:?} harus dikenali sebagai on"
            );
        }
        for arg in ["OFF", "Off", "off"] {
            let r = run(serde_json::json!({"rpg": true}), arg);
            assert_eq!(group_write(&r)["rpg"], serde_json::json!(false), "arg {arg:?}");
        }
    }

    #[test]
    fn a_uppercase_option_never_falls_through_to_help() {
        for arg in ["ON", "OFF"] {
            let r = run(serde_json::json!({"rpg": false}), arg);
            assert!(
                !text_of(&r).starts_with("⚔️ *FITUR RPG GRUP*"),
                "arg {arg:?} jatuh ke help"
            );
        }
    }

    #[test]
    fn the_applied_reply_is_paired_with_the_write() {
        let r = run(serde_json::json!({"rpg": false}), "on");
        assert!(text_of(&r).contains("Berhasil"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_game_and_rpg_toggles_keep_separate_keys() {
        for (name, keys) in [
            ("game", ["game", "togglegame"]),
            ("rpg", ["rpg", "togglerpg"]),
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
