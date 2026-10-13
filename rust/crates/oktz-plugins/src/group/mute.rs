//! `plugins/group/mute.js` — `groups[chat].mute = true`.
//!
//! Efeknya dibaca `handler.js:811` lewat `isMuted()` (function diekspor JS,
//! bukan perintah), jadi yang ditulis di sini hanya flag-nya.
//!
//! `ponytail:` `m.groupMetadata.subject` diambil dari
//! `groups[chat].name`, yang `handler.js:327` isi dari `meta.subject`.
//! Kalau store belum pernah menyimpan nama, teksnya jatuh ke nilai cadangan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::{js_truthy, jid_number};

/// Nilai cadangan ketika `groups[chat].name` belum ada.
pub(crate) const NO_GROUP_NAME: &str = "Grup";

/// `m.groupMetadata.subject` → `groups[chat].name`.
pub(crate) fn group_name(ctx: &Ctx) -> &str {
    ctx.group_field("name")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(NO_GROUP_NAME)
}

pub(crate) const ALREADY_MUTED: &str = "❌ Grup sudah dalam keadaan mute.";

pub(crate) fn muted_now(prefix: &str, group: &str, sender_num: &str) -> String {
    format!(
        "✅ Grup *{group}* berhasil di-mute oleh @{sender_num}\n\n\
         Hanya admin yang bisa mengirim pesan.\n\
         Ketik *{prefix}unmute* untuk membuka kembali."
    )
}

#[plugin(
    name = "muteanggota",
    alias = ["bisukan"],
    category = "group",
    description = "Bisukan seluruh grup (hanya admin yang bisa kirim pesan)",
    usage = ".muteanggota",
    example = ".muteanggota",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn mute(ctx: Ctx) -> Reply {
    if js_truthy(ctx.group_field("mute")) {
        return Reply::text(ALREADY_MUTED);
    }
    Reply::Text {
        text: muted_now(&ctx.prefix, group_name(&ctx), jid_number(&ctx.sender)),
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "mute": true }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(group: serde_json::Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "mute".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("muteanggota").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &serde_json::Value {
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
        let p = oktz_plugin_api::find("muteanggota").unwrap();
        assert_eq!(p.meta.name, "muteanggota");
        assert_eq!(p.meta.alias, ["bisukan"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Bisukan seluruh grup (hanya admin yang bisa kirim pesan)"
        );
        assert_eq!(p.meta.usage, ".muteanggota");
        assert_eq!(p.meta.example, ".muteanggota");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("bisukan").unwrap().meta.name, "muteanggota");
    }

    #[test]
    fn muting_writes_the_flag_and_names_the_group() {
        let r = run(json!({"name": "Grup Diskusi"}));
        assert_eq!(
            text_of(&r),
            "✅ Grup *Grup Diskusi* berhasil di-mute oleh @628111\n\n\
             Hanya admin yang bisa mengirim pesan.\n\
             Ketik *.unmute* untuk membuka kembali."
        );
        assert_eq!(group_write(&r), &json!({"mute": true}));
    }

    #[test]
    fn the_sender_is_mentioned_not_the_group() {
        let r = run(json!({"name": "G"}));
        match r.without_mutations() {
            Reply::Text { mentions, .. } => assert_eq!(mentions, &["628111@s.whatsapp.net".to_string()]),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn a_group_that_is_already_muted_is_left_alone() {
        for truthy in [json!(true), json!(1), json!("ya"), json!({})] {
            let r = run(json!({ "mute": truthy.clone() }));
            assert_eq!(text_of(&r), ALREADY_MUTED, "input mute={truthy}");
            assert!(r.mutations().is_empty(), "tidak boleh menulis ulang: {truthy}");
        }
    }

    #[test]
    fn a_falsy_stored_flag_still_lets_the_mute_through() {
        // `if (group.mute)` di JS = `!m0`, jadi `false`/`0`/`""`/`null` lolos.
        for falsy in [json!(false), json!(0), json!(""), json!(null)] {
            let r = run(json!({ "mute": falsy.clone() }));
            assert_eq!(group_write(&r), &json!({"mute": true}), "input mute={falsy}");
        }
    }

    #[test]
    fn a_group_row_without_a_name_falls_back_instead_of_printing_undefined() {
        let r = run(json!({}));
        assert!(
            text_of(&r).starts_with(&format!("✅ Grup *{NO_GROUP_NAME}* berhasil di-mute")),
            "{}",
            text_of(&r)
        );
    }
}