//! `plugins/group/mutegc.js` — `groups[chat].mutegc = true`.
//!
//! Efeknya dibaca `handler.js:829` lewat `isMutegc`, jadi yang ditulis di sini
//! hanya flag-nya.
//!
//! `ponytail:` `m.groupMetadata.subject` diambil dari `groups[chat].name`
//! (pola yang sama dengan `mute.rs`). `Ctx` tidak membawa subject langsung.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy_value;

/// `m.groupMetadata?.subject || "grup ini"` — nilai cadangan dari JS.
pub(crate) const NO_GROUP_NAME: &str = "grup ini";

/// `mutegc.js:33` — sudah aktif.
pub(crate) const ALREADY_ON: &str = "🔇 *Mute GC Sudah Aktif*\n\n\
    > Member tidak bisa menggunakan command bot di grup ini\n\
    > Hanya admin grup dan owner bot yang bisa akses\n\n\
    _Ketik *{prefix}unmutegc* untuk membuka_";

pub(crate) fn already_on(prefix: &str) -> String {
    ALREADY_ON.replace("{prefix}", prefix)
}

pub(crate) fn now_on(prefix: &str, group_name: &str) -> String {
    format!(
        "🔇 *Mute GC Aktif*\n\n\
         > Grup: *{group_name}*\n\
         > Member tidak bisa menggunakan command bot\n\
         > Admin grup dan owner bot tetap bisa akses\n\n\
         _Ketik *{prefix}unmutegc* untuk membuka_"
    )
}

/// `isMutegc(groupJid, db)` (`mutegc.js:52`) — middleware cuma butuh
/// kebenaran flag grup, jadi cukup menerima objek grup dari store.
pub(crate) fn is_mutegc(group_data: &Value) -> bool {
    group_data.get("mutegc").is_some_and(js_truthy_value)
}

#[plugin(
    name = "mutegc",
    alias = ["mutegrup", "mutebot", "blockbot", "lockbot"],
    category = "group",
    description = "Blokir command bot untuk member, hanya admin/owner yang bisa pakai",
    usage = ".mutegc",
    example = ".mutegc",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = false
)]
pub fn mutegc(ctx: Ctx) -> Reply {
    if is_mutegc(&ctx.group_data) {
        return Reply::text(already_on(&ctx.prefix));
    }
    let group_name = ctx
        .group_field("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(NO_GROUP_NAME);
    Reply::text(now_on(&ctx.prefix, group_name)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "mutegc": true }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;


    fn run(group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mutegc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("mutegc").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("mutegc").unwrap();
        assert_eq!(p.meta.name, "mutegc");
        assert_eq!(p.meta.alias, ["mutegrup", "mutebot", "blockbot", "lockbot"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Blokir command bot untuk member, hanya admin/owner yang bisa pakai"
        );
        assert_eq!(p.meta.usage, ".mutegc");
        assert_eq!(p.meta.example, ".mutegc");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["mutegrup", "mutebot", "blockbot", "lockbot"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mutegc");
        }
    }

    #[test]
    fn muting_writes_the_flag_and_names_the_group() {
        let r = run(serde_json::json!({"name": "Grup Diskusi"}));
        assert_eq!(
            text_of(&r),
            "🔇 *Mute GC Aktif*\n\n\
             > Grup: *Grup Diskusi*\n\
             > Member tidak bisa menggunakan command bot\n\
             > Admin grup dan owner bot tetap bisa akses\n\n\
             _Ketik *.unmutegc* untuk membuka_"
        );
        assert_eq!(group_write(&r), &serde_json::json!({"mutegc": true}));
    }

    #[test]
    fn a_group_already_muted_is_left_alone() {
        // `if (groupData.mutegc)` = `!m0`, jadi `1`/`"ya"`/`{}` ikut truthy.
        for truthy in [serde_json::json!(true), serde_json::json!(1), serde_json::json!("ya"), serde_json::json!({})] {
            let r = run(serde_json::json!({"mutegc": truthy.clone(), "name": "G"}));
            assert_eq!(text_of(&r), already_on("."), "input mutegc={truthy}");
            assert!(
                r.mutations().is_empty(),
                "tidak boleh menulis ulang: {truthy}"
            );
        }
    }

    #[test]
    fn a_falsy_stored_flag_still_lets_the_write_through() {
        for falsy in [serde_json::json!(false), serde_json::json!(0), serde_json::json!(""), serde_json::json!(null)] {
            let r = run(serde_json::json!({"mutegc": falsy.clone()}));
            assert_eq!(group_write(&r), &serde_json::json!({"mutegc": true}), "mutegc={falsy}");
        }
    }

    #[test]
    fn a_group_row_without_a_name_falls_back_instead_of_printing_undefined() {
        for row in [serde_json::json!({}), serde_json::json!({"name": ""}), serde_json::json!({"name": 5})] {
            let r = run(row.clone());
            assert!(
                text_of(&r).contains(&format!("> Grup: *{NO_GROUP_NAME}*")),
                "{}",
                text_of(&r)
            );
        }
    }

    #[test]
    fn the_prefix_comes_from_the_context_not_the_default() {
        let ctx = Ctx {
            prefix: "!".into(),
            chat: "120363@g.us".into(),
            group_data: serde_json::json!({}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("mutegc").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("!unmutegc"), "{}", text_of(&r));
    }

    #[test]
    fn the_exported_helper_matches_the_truthiness_the_handler_uses() {
        for truthy in [serde_json::json!(true), serde_json::json!(1), serde_json::json!("ya"), serde_json::json!({})] {
            assert!(is_mutegc(&serde_json::json!({ "mutegc": truthy.clone() })), "{truthy}");
        }
        for falsy in [serde_json::json!(false), serde_json::json!(0), serde_json::json!(""), serde_json::json!(null)] {
            assert!(!is_mutegc(&serde_json::json!({ "mutegc": falsy.clone() })), "{falsy}");
        }
        assert!(!is_mutegc(&serde_json::json!({})), "grup tanpa field = tidak aktif");
        assert!(
            !is_mutegc(&Value::Null),
            "grup belum pernah ada = tidak aktif"
        );
    }
}
