//! `plugins/group/addtoxic.js` — `groups[chat].toxicWords` tambah satu.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.args.join(' ').trim().toLowerCase()` — digabung, dipangkas, lowercase.
pub(crate) fn word(ctx: &Ctx) -> String {
    ctx.args.join(" ").trim().to_lowercase()
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📝 *ᴀᴅᴅ ᴛᴏxɪᴄ*\n\n\
         > Gunakan: `.{p}addtoxic <kata>`\n\n\
         `Contoh: {p}addtoxic katakasar`",
        p = prefix
    )
}

pub(crate) const TOO_SHORT: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Kata terlalu pendek (min 2 huruf)";
pub(crate) const TOO_LONG: &str = "❌ *ɢᴀɢᴀʟ*\n\n> Kata terlalu panjang (max 30 huruf)";

pub(crate) fn already(word: &str) -> String {
    format!("❌ *ɢᴀɢᴀʟ*\n\n> Kata `{word}` sudah ada di daftar")
}

pub(crate) fn added(word: &str, total: usize) -> String {
    format!(
        "✅ *ᴋᴀᴛᴀ ᴛᴏxɪᴄ ᴅɪᴛᴀᴍʘᴀʜ*\n\n\
         ╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n\
         ┃ 📝 ᴋᴀᴛᴀ: `{word}`\n\
         ┃ 📊 ᴛᴏᴛᴀʟ: `{total}` kata\n\
         ╰┈┈⬡"
    )
}

#[plugin(
    name = "addtoxic",
    alias = ["tambahtoxic", "addkata"],
    category = "group",
    description = "Tambah kata toxic ke daftar",
    usage = ".addtoxic <kata>",
    example = ".addtoxic kata_kasar",
    cooldown = 3,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn addtoxic(ctx: Ctx) -> Reply {
    let word = word(&ctx);
    if word.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    // `word.length` di JS menghitung UTF-16 code unit; untuk kata Latin
    // `chars().count()` sama. Unicode astral (emoji) dihitung 2 di JS dan
    // 1 di Rust — `ponytail:` perbedaan itu tidak materiel untuk daftar
    // kata, tapi batas 30EMU bukan 30 byte.
    let len = word.chars().count();
    if len < 2 {
        return Reply::text(TOO_SHORT);
    }
    if len > 30 {
        return Reply::text(TOO_LONG);
    }

    if ctx
        .group_list("toxicWords")
        .iter()
        .any(|v| v.as_str() == Some(word.as_str()))
    {
        return Reply::text(already(&word));
    }

    let mut list = ctx.group_list("toxicWords").to_vec();
    list.push(serde_json::Value::String(word.clone()));
    let total = list.len();

    Reply::text(added(&word, total))
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "toxicWords": list }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "addtoxic".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("addtoxic").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("addtoxic").unwrap();
        assert_eq!(p.meta.name, "addtoxic");
        assert_eq!(p.meta.alias, ["tambahtoxic", "addkata"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Tambah kata toxic ke daftar");
        assert_eq!(p.meta.usage, ".addtoxic <kata>");
        assert_eq!(p.meta.example, ".addtoxic kata_kasar");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tambahtoxic", "addkata"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "addtoxic");
        }
    }

    #[test]
    fn an_empty_word_shows_help_and_writes_nothing() {
        // Input whitespace-only jadi args kosong, jadi `join(' ').trim()`
        // menghasilkan string kosong → cabang help.
        for msg in ["", "   ", "\t\n"] {
            let r = run(serde_json::json!({}), msg);
            assert!(text_of(&r).starts_with("📝 *ᴀᴅᴅ ᴛᴏxɪᴄ*"), "input {msg:?}");
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn a_word_shorter_than_two_characters_is_rejected() {
        let r = run(serde_json::json!({}), "x");
        assert_eq!(text_of(&r), TOO_SHORT);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn exactly_two_characters_is_accepted_and_thirty_is_the_ceiling() {
        let two = run(serde_json::json!({}), "ab");
        assert_eq!(list_write(&two).len(), 1);

        let thirty = "x".repeat(30);
        let r = run(serde_json::json!({}), &thirty);
        assert_eq!(list_write(&r).len(), 1, "30 huruf tepat masih diterima");

        let thirty_one = "x".repeat(31);
        let r = run(serde_json::json!({}), &thirty_one);
        assert_eq!(text_of(&r), TOO_LONG);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_length_limit_counts_characters_not_bytes() {
        // 30 emoji = 120 byte UTF-8 tapi hanya 30 char; JS `length` juga 30
        // untuk BMP. Yang diuji: tidak ada yang terpotong oleh batas byte.
        let emoji = "😀".repeat(30);
        let r = run(serde_json::json!({}), &emoji);
        assert_eq!(
            list_write(&r).len(),
            1,
            "batasnya karakter, bukan byte: {emoji}"
        );
    }

    #[test]
    fn an_existing_word_is_refused_and_nothing_is_written() {
        let r = run(serde_json::json!({"toxicWords": ["kata"]}), "kata");
        assert_eq!(text_of(&r), already("kata"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_word_is_lowercased_before_the_duplicate_check() {
        let refused = run(serde_json::json!({"toxicWords": ["kata"]}), "KATA");
        assert!(text_of(&refused).starts_with("❌ *ɢᴀɢᴀʟ*"));
        assert!(refused.mutations().is_empty());

        let stored = run(serde_json::json!({}), "KaTa");
        assert_eq!(list_write(&stored)[0], serde_json::json!("kata"));
    }

    #[test]
    fn multi_token_input_is_joined_with_spaces() {
        let r = run(serde_json::json!({}), "kata  kasar  sekali");
        assert_eq!(list_write(&r)[0], serde_json::json!("kata kasar sekali"));
        assert!(
            text_of(&r).contains("`kata kasar sekali`"),
            "reply mengutip kata gabungan: {}",
            text_of(&r)
        );
    }

    #[test]
    fn the_reported_total_matches_what_is_written() {
        let r = run(serde_json::json!({"toxicWords": ["a", "b"]}), "baru");
        assert_eq!(list_write(&r).len(), 3);
        assert!(text_of(&r).contains("┃ 📊 ᴛᴏᴛᴀʟ: `3` kata"), "{}", text_of(&r));
    }

    #[test]
    fn a_fresh_group_stores_one_word_and_reports_one() {
        let r = run(serde_json::json!({}), "baru");
        assert_eq!(list_write(&r).len(), 1);
        assert!(text_of(&r).contains("`1` kata"));
    }

    #[test]
    fn the_added_reply_reacts_and_is_paired_with_the_write() {
        let r = run(serde_json::json!({}), "baru");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
        assert!(text_of(&r).contains("ᴋᴀᴛᴀ ᴛᴏxɪᴄ"), "{}", text_of(&r));
        assert_eq!(r.mutations().len(), 1);
    }
}