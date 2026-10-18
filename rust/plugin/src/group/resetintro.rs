//! `plugins/group/resetintro.js` — hapus `groups[chat].intro` kembali ke default.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::js_truthy;

pub(crate) fn already_default() -> &'static str {
    "❌ Grup ini sudah menggunakan intro default!"
}

pub(crate) fn confirm(prefix: &str) -> String {
    format!(
        "✅ *ɪɴᴛʀᴏ ᴅɪʀᴇsᴇᴛ!*\n\
         Intro grup dikembalikan ke default.\n\n\
         Ketik *{prefix}intro* untuk melihat hasilnya."
    )
}

#[plugin(
    name = "resetintro",
    alias = ["introdel", "delintro", "deleteintro"],
    category = "group",
    description = "Reset intro grup ke default (admin only)",
    usage = ".resetintro",
    example = ".resetintro",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn resetintro(ctx: Ctx) -> Reply {
    // `if (!groupData.intro)` — termasuk string kosong, yang tetap saja sudah
    // "tidak ada intro" untuk pembaca di `group/intro.rs`.
    if !js_truthy(ctx.group_field("intro")) {
        return Reply::text(already_default());
    }
    Reply::text(confirm(&ctx.prefix)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        // `delete groupData.intro` menghapus KEY-nya. Bentuk store di sini
        // hanya bisa menulis nilai, jadi `null` dipakai sebagai padanan —
        // `intro.rs` memperlakukannya sama (`js_truthy`), jadi hasil
        // tampilannya identik.
        data: serde_json::json!({ "intro": serde_json::Value::Null }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "resetintro".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("resetintro").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("resetintro").unwrap();
        assert_eq!(p.meta.name, "resetintro");
        assert_eq!(p.meta.alias, ["introdel", "delintro", "deleteintro"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Reset intro grup ke default (admin only)"
        );
        assert_eq!(p.meta.usage, ".resetintro");
        assert_eq!(p.meta.example, ".resetintro");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["introdel", "delintro", "deleteintro"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "resetintro");
        }
    }

    #[test]
    fn an_absent_or_falsy_intro_is_already_at_the_default() {
        for g in [
            serde_json::json!(null),
            serde_json::json!({}),
            serde_json::json!({"intro": null}),
            serde_json::json!({"intro": ""}),
            serde_json::json!({"intro": 0}),
            serde_json::json!({"intro": false}),
        ] {
            let r = run(g.clone());
            assert_eq!(text_of(&r), already_default(), "input {g}");
            assert!(r.mutations().is_empty(), "input {g} tidak boleh menulis");
        }
    }

    #[test]
    fn a_stored_intro_is_cleared() {
        let r = run(serde_json::json!({"intro": "halo @user", "game": false}));
        let data = group_write(&r);
        assert_eq!(data["intro"], Value::Null);
        assert_eq!(
            data.as_object().unwrap().len(),
            1,
            "field lain tidak boleh ikut ditulis"
        );
        assert!(text_of(&r).starts_with("✅ *ɪɴᴛʀᴏ ᴅɪʀᴇsᴇᴛ!*"));
    }

    #[test]
    fn the_confirm_reply_names_the_live_prefix() {
        assert!(
            confirm("#").contains("Ketik *#intro*"),
            "{}",
            confirm("#")
        );
    }

    #[test]
    fn the_reset_reply_is_paired_with_the_write() {
        let r = run(serde_json::json!({"intro": "x"}));
        assert!(text_of(&r).contains("dikembalikan ke default"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn resetting_twice_is_a_no_op_the_second_time() {
        // Setelah `null`, cabang "sudah default" yang diambil — jadi reset
        // kedua tidak menulis apa pun.
        let first = run(serde_json::json!({"intro": "x"}));
        assert_eq!(first.mutations().len(), 1);
        let second = run(serde_json::json!({"intro": null}));
        assert_eq!(text_of(&second), already_default());
        assert!(second.mutations().is_empty());
    }
}