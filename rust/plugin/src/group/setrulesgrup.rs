//! `plugins/group/setrulesgrup.js` — `groups[chat].groupRules`.
//!
//! `ponytail:` sumber teks di JS adalah `m.text?.trim() || (m.quoted?.body ||
//! m.quoted?.text || '')`. `Ctx` tidak membawa pesan yang di-reply, jadi
//! cabang kedua tidak bisa dipindah; plugin hanya menerima teks yang ikut
//! diketik. Kalau quotes di Instrumentasikan nanti, tambahkan `quoted:
//! Option<Quoted>` ke `Ctx` dan pulihkan cabang itu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) fn rules_text(ctx: &Ctx) -> String {
    ctx.text.trim().to_string()
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📝 *sᴇᴛ ɢʀᴜᴘ ʀᴜʟᴇs*\n\n\
         > Masukkan teks rules yang baru\n\n\
         `Contoh:`\n\
         `{}setrulesgrup 1. Jangan spam\n2. Hormati sesama`",
        prefix
    )
}

pub(crate) fn confirm(prefix: &str) -> String {
    format!(
        "✅ *ɢʀᴜᴘ ʀᴜʟᴇs ᴅɪʜᴍᴘᴀᴛᴇ*\n\n\
         Rules grup berhasil diubah!\n\
         Ketik `{prefix}rulesgrup` untuk melihat."
    )
}

#[plugin(
    name = "setrulesgrup",
    alias = ["setgrouprules", "setaturangrup"],
    category = "group",
    description = "Set rules/aturan grup custom (admin only)",
    usage = ".setrulesgrup <text>",
    example = ".setrulesgrup 1. Jangan spam\n2. Hormati sesama",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn setrulesgrup(ctx: Ctx) -> Reply {
    let text = rules_text(&ctx);
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    Reply::text(confirm(&ctx.prefix)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "groupRules": text }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setrulesgrup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("setrulesgrup").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("setrulesgrup").unwrap();
        assert_eq!(p.meta.name, "setrulesgrup");
        assert_eq!(p.meta.alias, ["setgrouprules", "setaturangrup"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Set rules/aturan grup custom (admin only)"
        );
        assert_eq!(p.meta.usage, ".setrulesgrup <text>");
        assert_eq!(
            p.meta.example,
            ".setrulesgrup 1. Jangan spam\n2. Hormati sesama"
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setgrouprules", "setaturangrup"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "setrulesgrup");
        }
    }

    #[test]
    fn a_blank_rules_text_shows_help_and_writes_nothing() {
        for msg in ["", "   ", "\n\t"] {
            let r = run(serde_json::json!({}), msg);
            assert!(
                text_of(&r).starts_with("📝 *sᴇᴛ ɢʀᴜᴘ ʀᴜʟᴇs*"),
                "input {msg:?}"
            );
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn the_rules_are_trimmed_at_the_edges_but_keep_inner_newlines() {
        let r = run(serde_json::json!({}), "  1. Jangan spam\n2. Hormati sesama  ");
        assert_eq!(
            group_write(&r)["groupRules"],
            serde_json::json!("1. Jangan spam\n2. Hormati sesama")
        );
    }

    #[test]
    fn a_multi_line_rules_block_round_trips_exactly() {
        let rules = "1. Jangan spam\n2. Hormati sesama\n3. Jangan share link";
        let r = run(serde_json::json!({"groupRules": "lama"}), rules);
        assert_eq!(group_write(&r)["groupRules"], serde_json::json!(rules));
    }

    #[test]
    fn the_example_in_usage_carries_a_real_newline() {
        let p = oktz_plugin_api::find("setrulesgrup").unwrap();
        assert!(
            p.meta.example.contains('\n'),
            "contoh di JS itu multibaris: {:?}",
            p.meta.example
        );
    }

    #[test]
    fn the_confirm_reply_names_the_live_prefix() {
        assert!(confirm("~").contains("`~rulesgrup`"), "{}", confirm("~"));
    }

    #[test]
    fn the_success_reply_is_paired_with_the_write() {
        let r = run(serde_json::json!({}), "1. Jangan spam");
        assert!(text_of(&r).contains("berhasil diubah"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn the_help_branch_never_touches_the_stored_rules() {
        for g in [serde_json::json!({"groupRules": "lama"}), serde_json::json!({}), serde_json::json!(null)] {
            let r = run(g.clone(), "");
            assert!(r.mutations().is_empty(), "input {g}");
        }
    }
}