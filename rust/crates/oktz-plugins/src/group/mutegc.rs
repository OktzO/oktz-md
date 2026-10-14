//! `plugins/group/mutegc.js` — `groups[chat].mutegc = true`.
//!
//! Efeknya dibaca router lewat `isMutegc` (function diekspor JS, bukan
//! perintah), jadi yang ditulis di sini hanya flag-nya.
//!
//! `ponytail:` `m.groupMetadata.subject` diambil dari `groups[chat].name`, yang
//! `handler.js:327` isi dari `meta.subject` — sama seperti `group/mute.rs`.
//! Kalau store belum pernah menyimpan nama, teksnya jatuh ke nilai cadangan JS
//! `"grup ini"`.
//!
//! `ponytail:` `saluranCtx()` menempelkan `contextInfo` ber-forwarding saluran
//! pada dua dari empat balasan. `Reply` tidak punya field contextInfo, jadi
//! empat balasan di sini teksnya sama persis dan forwarding-nya hilang.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_truthy;

/// Nilai cadangan `m.groupMetadata?.subject || 'grup ini'`.
pub(crate) const NO_GROUP_NAME: &str = "grup ini";

/// `isMutegc(groupJid, db)` (`mutegc.js:41`) — dibaca router sebelum
/// menjalankan command member. Parameter JS-nya `(groupJid, db)`; di sini
/// `Ctx` sudah membawa baris grup untuk `chat`, jadi cukup blok datanya.
pub(crate) fn is_mutegc(group_data: &serde_json::Value) -> bool {
    js_truthy(group_data.get("mutegc"))
}

/// `m.groupMetadata?.subject || 'grup ini'` → `groups[chat].name`.
fn subject(ctx: &Ctx) -> &str {
    ctx.group_field("name")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(NO_GROUP_NAME)
}

/// Cabang `if (groupData.mutegc)` (`mutegc.js:24-32`).
pub(crate) fn already_on(prefix: &str) -> String {
    format!(
        "🔇 *Mute GC Sudah Aktif*\n\n\
         > Member tidak bisa menggunakan command bot di grup ini\n\
         > Hanya admin grup dan owner bot yang bisa akses\n\n\
         _Ketik *{prefix}unmutegc* untuk membuka_"
    )
}

/// Cabang setelah `db.setGroup` (`mutegc.js:34-44`).
pub(crate) fn applied(prefix: &str, group: &str) -> String {
    format!(
        "🔇 *Mute GC Aktif*\n\n\
         > Grup: *{group}*\n\
         > Member tidak bisa menggunakan command bot\n\
         > Admin grup dan owner bot tetap bisa akses\n\n\
         _Ketik *{prefix}unmutegc* untuk membuka_"
    )
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
    is_owner = false,
    is_premium = false,
    is_admin = true,
    is_bot_admin = false
)]
pub fn mutegc(ctx: Ctx) -> Reply {
    if is_mutegc(&ctx.group_data) {
        return Reply::text(already_on(&ctx.prefix));
    }

    Reply::text(applied(&ctx.prefix, subject(&ctx))).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "mutegc": true }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

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
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mutegrup", "mutebot", "blockbot", "lockbot"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mutegc");
        }
    }

    #[test]
    fn muting_writes_the_flag_and_names_the_group() {
        let r = run(json!({ "name": "Grup Diskusi" }));
        assert_eq!(
            text_of(&r),
            "🔇 *Mute GC Aktif*\n\n\
             > Grup: *Grup Diskusi*\n\
             > Member tidak bisa menggunakan command bot\n\
             > Admin grup dan owner bot tetap bisa akses\n\n\
             _Ketik *.unmutegc* untuk membuka_"
        );
        assert_eq!(group_write(&r), &json!({ "mutegc": true }));
    }

    #[test]
    fn a_group_that_is_already_muted_is_left_alone() {
        // `if (groupData.mutegc)` di JS = `!m0`: objek kosong pun truthy.
        for truthy in [json!(true), json!(1), json!("ya"), json!({})] {
            let r = run(json!({ "mutegc": truthy.clone(), "name": "G" }));
            assert_eq!(text_of(&r), already_on("."), "input mutegc={truthy}");
            assert!(r.mutations().is_empty(), "tidak boleh menulis ulang: {truthy}");
        }
    }

    #[test]
    fn a_falsy_stored_flag_still_lets_the_mute_through() {
        for falsy in [json!(false), json!(0), json!(""), json!(null)] {
            let r = run(json!({ "mutegc": falsy.clone(), "name": "G" }));
            assert_eq!(group_write(&r), &json!({ "mutegc": true }), "input {falsy}");
        }
    }

    #[test]
    fn a_group_row_without_a_name_falls_back_instead_of_printing_undefined() {
        let r = run(json!({}));
        assert_eq!(
            text_of(&r),
            applied(".", NO_GROUP_NAME),
            "nama kosong harus jatuh ke fallback JS"
        );
        assert_eq!(text_of(&r), applied("~", NO_GROUP_NAME).replace('~', "."));
    }

    #[test]
    fn the_live_prefix_is_used_in_both_branches() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "mutegc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: json!({ "name": "G" }),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("mutegc").unwrap().handler)(ctx.clone());
        assert!(text_of(&r).contains("!unmutegc"), "{}", text_of(&r));

        let ctx = Ctx {
            group_data: json!({ "mutegc": true }),
            ..ctx
        };
        let r = (oktz_plugin_api::find("mutegc").unwrap().handler)(ctx);
        assert!(text_of(&r).contains("!unmutegc"), "{}", text_of(&r));
    }

    #[test]
    fn the_router_helper_reads_the_same_flag_the_handler_writes() {
        assert!(is_mutegc(&json!({ "mutegc": true })));
        assert!(!is_mutegc(&json!({ "mutegc": false })));
        assert!(!is_mutegc(&json!({})), "grup tanpa field = belum dimute");
        assert!(!is_mutegc(&Value::Null), "grup yang belum pernah ada");
    }
}
