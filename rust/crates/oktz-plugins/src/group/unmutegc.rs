//! `plugins/group/unmutegc.js` — `groups[chat].mutegc = false`.
//!
//! `ponytail:` `m.groupMetadata.subject` diambil dari `groups[chat].name`
//! (pola yang sama dengan `mute.rs`). `Ctx` tidak membawa subject langsung.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::group::mutegc::is_mutegc;

/// `m.groupMetadata?.subject || "grup ini"` — nilai cadangan dari JS.
pub(crate) const NO_GROUP_NAME: &str = "grup ini";

/// `unmutegc.js:33` — memang belum aktif.
pub(crate) const ALREADY_OFF: &str = "🔊 *Mute GC Tidak Aktif*\n\n\
    > Member sudah bisa menggunakan command bot di grup ini";

pub(crate) fn now_off(prefix: &str, group_name: &str) -> String {
    format!(
        "🔊 *Mute GC Nonaktif*\n\n\
         > Grup: *{group_name}*\n\
         > Member sekarang bisa menggunakan command bot lagi\n\n\
         _Ketik *{prefix}mutegc* untuk memblokir kembali_"
    )
}

#[plugin(
    name = "unmutegc",
    alias = ["unmutegrup", "unmutebot", "unblockbot", "unlockbot"],
    category = "group",
    description = "Buka blokir command bot untuk member di grup",
    usage = ".unmutegc",
    example = ".unmutegc",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = false
)]
pub fn unmutegc(ctx: Ctx) -> Reply {
    if !is_mutegc(&ctx.group_data) {
        return Reply::text(ALREADY_OFF);
    }
    let group_name = ctx
        .group_field("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(NO_GROUP_NAME);
    Reply::text(now_off(&ctx.prefix, group_name)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "mutegc": false }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(prefix: &str, group: Value) -> Reply {
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "unmutegc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@gus".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("unmutegc").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@gus");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("unmutegc").unwrap();
        assert_eq!(p.meta.name, "unmutegc");
        assert_eq!(
            p.meta.alias,
            ["unmutegrup", "unmutebot", "unblockbot", "unlockbot"]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Buka blokir command bot untuk member di grup"
        );
        assert_eq!(p.meta.usage, ".unmutegc");
        assert_eq!(p.meta.example, ".unmutegc");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_here() {
        for a in ["unmutegrup", "unmutebot", "unblockbot", "unlockbot"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "unmutegc");
        }
    }

    #[test]
    fn unmuting_writes_false_and_names_the_group() {
        let r = run(".", json!({"mutegc": true, "name": "Grup Diskusi"}));
        assert_eq!(
            text_of(&r),
            "🔊 *Mute GC Nonaktif*\n\n\
             > Grup: *Grup Diskusi*\n\
             > Member sekarang bisa menggunakan command bot lagi\n\n\
             _Ketik *.mutegc* untuk memblokir kembali_"
        );
        assert_eq!(group_write(&r), &json!({"mutegc": false}));
    }

    #[test]
    fn a_group_that_is_not_muted_is_left_alone() {
        // `if (!groupData.mutegc)` = `m0`, jadi `0`/`""`/`null` ikut nonaktif.
        for falsy in [json!(false), json!(0), json!(""), json!(null)] {
            let r = run(".", json!({"mutegc": falsy.clone(), "name": "G"}));
            assert_eq!(text_of(&r), ALREADY_OFF, "mutegc={falsy}");
            assert!(
                r.mutations().is_empty(),
                "tidak boleh menulis ulang: {falsy}"
            );
        }
        let r = run(".", json!({"name": "G"}));
        assert_eq!(text_of(&r), ALREADY_OFF, "grup tanpa field mutegc");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_truthy_stored_flag_lets_the_write_through() {
        for truthy in [json!(true), json!(1), json!("ya"), json!({})] {
            let r = run(".", json!({"mutegc": truthy.clone()}));
            assert_eq!(
                group_write(&r),
                &json!({"mutegc": false}),
                "mutegc={truthy}"
            );
        }
    }

    #[test]
    fn a_group_row_without_a_name_falls_back_instead_of_printing_undefined() {
        let r = run(".", json!({"mutegc": true}));
        assert!(
            text_of(&r).contains(&format!("> Grup: *{NO_GROUP_NAME}*")),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn the_prefix_comes_from_the_context_not_the_default() {
        let r = run("!", json!({"mutegc": true}));
        assert!(text_of(&r).contains("!mutegc"), "{}", text_of(&r));
    }
}
