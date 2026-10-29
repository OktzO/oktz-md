//! `plugins/group/deltoxic.js` — `groups[chat].toxicWords` hapus satu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.args.join(' ').trim().toLowerCase()` — sama seperti `addtoxic`.
pub(crate) fn word(ctx: &Ctx) -> String {
    ctx.args.join(" ").trim().to_lowercase()
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🗑️ *ᴅᴇʟ ᴛᴏxɪᴄ*\n\n\
         > Gunakan: `.{p}deltoxic <kata>`\n\n\
         `Contoh: {p}deltoxic katakasar`",
        p = prefix
    )
}

pub(crate) fn not_found(word: &str) -> String {
    format!("❌ *ɢᴀɢᴀʟ*\n\n> Kata `{word}` tidak ada di daftar")
}

pub(crate) fn removed(word: &str, left: usize) -> String {
    format!(
        "✅ *ᴋᴀᴛᴀ ᴛᴏxɪᴄ ᴅɪʜᴀᴘᴜs*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 📝 ᴋᴀᴛᴀ: `{word}`\n\
         ┃ 📊 sɪsᴀ: `{left}` kata\n\
         ╰┈┈⬡"
    )
}

#[plugin(
    name = "deltoxic",
    alias = ["hapustoxic", "remtoxic", "removetoxic"],
    category = "group",
    description = "Hapus kata toxic dari daftar",
    usage = ".deltoxic <kata>",
    example = ".deltoxic kata_kasar",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn deltoxic(ctx: Ctx) -> Reply {
    let word = word(&ctx);
    if word.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let list = ctx.group_list("toxicWords");
    let Some(index) = list.iter().position(|v| v.as_str() == Some(word.as_str())) else {
        return Reply::text(not_found(&word));
    };

    let mut next = list.to_vec();
    next.remove(index);
    let left = next.len();

    Reply::text(removed(&word, left))
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "toxicWords": next }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "deltoxic".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("deltoxic").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn list_write(r: &Reply) -> &Vec<Value> {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data["toxicWords"].as_array().expect("harus array")
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("deltoxic").unwrap();
        assert_eq!(p.meta.name, "deltoxic");
        assert_eq!(p.meta.alias, ["hapustoxic", "remtoxic", "removetoxic"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Hapus kata toxic dari daftar");
        assert_eq!(p.meta.usage, ".deltoxic <kata>");
        assert_eq!(p.meta.example, ".deltoxic kata_kasar");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["hapustoxic", "remtoxic", "removetoxic"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "deltoxic");
        }
    }

    #[test]
    fn an_empty_word_shows_help_even_when_the_list_is_populated() {
        // Tidak seperti `delantilink`, plugin ini tidak pernah menampilkan
        // daftar — tanpa argumen selalu help.
        let r = run(serde_json::json!({"toxicWords": ["a", "b"]}), "");
        assert!(text_of(&r).starts_with("🗑️ *ᴅᴇʟ ᴛᴏxɪᴄ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_word_present_in_the_list_is_removed() {
        let r = run(serde_json::json!({"toxicWords": ["satu", "dua", "tiga"]}), "dua");
        assert_eq!(list_write(&r), &vec![serde_json::json!("satu"), serde_json::json!("tiga")]);
        assert!(text_of(&r).contains("`2` kata"), "{}", text_of(&r));
    }

    #[test]
    fn removing_the_only_word_leaves_an_empty_array() {
        let r = run(serde_json::json!({"toxicWords": ["saja"]}), "saja");
        assert!(list_write(&r).is_empty());
        assert!(text_of(&r).contains("`0` kata"));
    }

    #[test]
    fn a_missing_word_is_reported_and_nothing_is_written() {
        let r = run(serde_json::json!({"toxicWords": ["satu"]}), "lain");
        assert_eq!(text_of(&r), not_found("lain"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn removing_from_an_empty_list_reports_not_found() {
        let r = run(serde_json::json!({}), "apa");
        assert_eq!(text_of(&r), not_found("apa"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_word_is_lowercased_before_the_lookup() {
        let r = run(serde_json::json!({"toxicWords": ["kata"]}), "KATA");
        assert_eq!(list_write(&r).len(), 0);
    }

    #[test]
    fn multi_token_input_is_joined_with_spaces_before_lookup() {
        let r = run(serde_json::json!({"toxicWords": ["kata kasar"]}), "Kata  Kasar");
        assert!(list_write(&r).is_empty());
    }

    #[test]
    fn matching_is_whole_string_not_substring() {
        let r = run(serde_json::json!({"toxicWords": ["kata_kasar"]}), "kasar");
        assert!(text_of(&r).starts_with("❌ *ɢᴀɢᴀʟ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_removed_reply_reacts_and_is_paired_with_the_write() {
        let r = run(serde_json::json!({"toxicWords": ["x"]}), "x");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(text_of(&r).contains("ᴅɪʜᴀᴘᴜs"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn add_and_delete_toxic_keep_separate_keys() {
        for (name, keys) in [
            ("addtoxic", &["addtoxic", "tambahtoxic", "addkata"][..]),
            (
                "deltoxic",
                &["deltoxic", "hapustoxic", "remtoxic", "removetoxic"][..],
            ),
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