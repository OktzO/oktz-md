//! `plugins/group/game.js` — `groups[chat].game` on/off.
//!
//! Bentuknya identik dengan `plugins/group/rpg.js` (`rpg`), hanya nama field
//! dan teksnya berbeda. Keduanya tetap file terpisah karena `pluginConfig`
// JS-nya terpisah dan alias-nya berbeda.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::text_keyword;

/// `db.getGroup(chat) || db.setGroup(chat)` — `setGroup` mengisi default
/// `game: true` (`database.js:736`), jadi grup yang belum pernah ada mulai
/// dari AKTIF, bukan `undefined`. Itu yang membuat cek "sudah dalam keadaan"
/// langsung kena pada grup baru.
pub(crate) fn current(ctx: &Ctx) -> bool {
    match ctx.group_field("game") {
        Some(v) => crate::common::js_truthy(Some(v)),
        None => true,
    }
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎮 *FITUR GAME GRUP*\n\n\
         Gunakan perintah ini untuk mengatur akses member ke fitur game.\n\n\
         • *{p}game on* - Member bisa main game\n\
         • *{p}game off* - Member tidak bisa main game\n\n\
         *Catatan:* Admin tetap bisa mengakses game meskipun dimatikan.",
        p = prefix
    )
}

pub(crate) fn already(is_enable: bool) -> String {
    format!(
        "🎮 Fitur game sudah *{}* di grup ini.",
        if is_enable { "AKTIF" } else { "NONAKTIF" }
    )
}

pub(crate) fn applied(is_enable: bool) -> String {
    let tail = if is_enable {
        "Member sekarang bisa menggunakan semua perintah di menu game."
    } else {
        "Member tidak akan bisa menggunakan perintah game lagi."
    };
    format!(
        "✅ Berhasil *{}* fitur game di grup ini!\n\n{tail}",
        if is_enable { "MENGAKTIFKAN" } else { "MENONAKTIFKAN" },
    )
}

#[plugin(
    name = "game",
    alias = ["togglegame"],
    category = "group",
    description = "Mengaktifkan atau menonaktifkan fitur game di grup",
    usage = ".game <on/off>",
    example = ".game on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn game(ctx: Ctx) -> Reply {
    // `if (args !== 'on' && args !== 'off')` diperiksa SEBELUM store dibaca —
    // jadi command tanpa argumen tidak pernah menyentuh data grup.
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
            data: serde_json::json!({ "game": is_enable }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "game".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("game").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("game").unwrap();
        assert_eq!(p.meta.name, "game");
        assert_eq!(p.meta.alias, ["togglegame"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Mengaktifkan atau menonaktifkan fitur game di grup"
        );
        assert_eq!(p.meta.usage, ".game <on/off>");
        assert_eq!(p.meta.example, ".game on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("togglegame").unwrap().meta.name, "game");
    }

    #[test]
    fn an_unrecognised_option_shows_help_and_writes_nothing() {
        for bad in ["", "enable", "1", "on off", "onn"] {
            let r = run(serde_json::json!({"game": false}), bad);
            assert!(
                text_of(&r).starts_with("🎮 *FITUR GAME GRUP*"),
                "input {bad:?} harus menampilkan help"
            );
            assert!(r.mutations().is_empty(), "input {bad:?} tidak boleh menulis");
        }
    }

    #[test]
    fn the_help_branch_never_reads_the_stored_flag() {
        // Argumen diperiksa duluan; store dengan `game`Whatever tidak
        // boleh mengubah balasannya.
        for g in [serde_json::json!({"game": true}), serde_json::json!({"game": false}), serde_json::json!({}), serde_json::json!(null)] {
            let r = run(g.clone(), "");
            assert!(text_of(&r).starts_with("🎮 *FITUR GAME GRUP*"), "input {g}");
        }
    }

    #[test]
    fn a_fresh_group_counts_as_enabled_so_on_is_a_no_op() {
        // `setGroup` default `game: true` — grup baru sudah AKTIF, jadi
        // `.game on` harus bilang "sudah AKTIF", bukan menulis ulang.
        assert!(current(&Ctx::default()));
        let r = run(serde_json::json!({}), "on");
        assert_eq!(text_of(&r), already(true));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn turning_it_off_from_a_fresh_group_writes_false() {
        let r = run(serde_json::json!({}), "off");
        assert_eq!(group_write(&r)["game"], serde_json::json!(false));
        assert!(text_of(&r).contains("MENONAKTIFKAN"));
        assert!(text_of(&r).contains("tidak akan bisa"));
    }

    #[test]
    fn turning_it_back_on_writes_true_after_a_real_off() {
        let r = run(serde_json::json!({"game": false}), "on");
        assert_eq!(group_write(&r)["game"], serde_json::json!(true));
        assert!(text_of(&r).contains("MENGAKTIFKAN"));
    }

    #[test]
    fn an_explicit_no_op_is_reported_without_writing() {
        let already_on = run(serde_json::json!({"game": true}), "on");
        assert_eq!(text_of(&already_on), already(true));
        assert!(already_on.mutations().is_empty());

        let already_off = run(serde_json::json!({"game": false}), "off");
        assert_eq!(text_of(&already_off), already(false));
        assert!(already_off.mutations().is_empty());
    }

    #[test]
    fn the_stored_flag_is_read_with_javascript_truthiness() {
        // `group.game === isEnable` adalah perbandingan ketat, tapi
        // `getGroup() || setGroup()` mengisi default `true` lebih dulu —
        // jadi hanya key yang HILANG yang berarti "seperti default".
        // Key yang ada tapi falsy (`0`, `""`) tetap dibaca apa adanya.
        let g = |v: Value| Ctx {
            group_data: serde_json::json!({ "game": v }),
            ..Default::default()
        };
        assert!(current(&g(serde_json::json!(true))));
        assert!(!current(&g(serde_json::json!(false))));
        assert!(!current(&g(serde_json::json!(0))), "0 falsy di JS");
        assert!(!current(&g(serde_json::json!(""))), "string kosong falsy di JS");
        assert!(current(&Ctx::default()), "key hilang = default setGroup");
    }

    #[test]
    fn the_option_is_lowercased_before_comparison() {
        // `m.text?.trim()?.toLowerCase()` — jadi "ON" dikenali, bukan help.
        // Diuji dari state `false` supaya "dikenali" berarti "menulis",
        // karena dari state default `true`, "on" adalah no-op.
        for arg in ["ON", "On", "on"] {
            let r = run(serde_json::json!({"game": false}), arg);
            assert_eq!(
                group_write(&r)["game"],
                serde_json::json!(true),
                "arg {arg:?} harus dikenali sebagai on"
            );
        }
        for arg in ["OFF", "Off", "off"] {
            let r = run(serde_json::json!({"game": true}), arg);
            assert_eq!(group_write(&r)["game"], serde_json::json!(false), "arg {arg:?}");
        }
    }

    #[test]
    fn a_uppercase_option_never_falls_through_to_help() {
        for arg in ["ON", "OFF"] {
            let r = run(serde_json::json!({"game": false}), arg);
            assert!(
                !text_of(&r).starts_with("🎮 *FITUR GAME GRUP*"),
                "arg {arg:?} jatuh ke help"
            );
        }
    }

    #[test]
    fn the_applied_reply_is_paired_with_the_write() {
        let r = run(serde_json::json!({"game": false}), "on");
        assert!(text_of(&r).contains("Berhasil"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn this_plugin_does_not_claim_the_rpg_toggle_keys() {
        let keys = ["game", "togglegame"];
        for k in keys {
            let owners: Vec<&str> = oktz_plugin_api::all_plugins()
                .iter()
                .filter(|e| e.meta.matches(k))
                .map(|e| e.meta.name.as_str())
                .collect();
            assert_eq!(owners, ["game"], "key {k} diklaim oleh {owners:?}");
        }
    }
}
