//! `plugins/group/listtoxic.js` — `groups[chat].toxicWords` + tabel default
//! dari `antitoxic.js:20-24`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

/// `DEFAULT_TOXIC_WORDS` (`antitoxic.js:21-23`), urutan persis — 20 kata.
pub(crate) const DEFAULT_TOXIC_WORDS: [&str; 20] = [
    "anjing",
    "bangsat",
    "kontol",
    "memek",
    "ngentot",
    "babi",
    "tolol",
    "goblok",
    "idiot",
    "bodoh",
    "kampret",
    "asu",
    "jancok",
    "bajingan",
    "keparat",
    "setan",
    "iblis",
    "tai",
    "brengsek",
    "sialan",
];

pub(crate) fn render(ctx: &Ctx) -> String {
    let custom = joined_strings(ctx.group_list("toxicWords"));

    let mut text = String::from("📋 *ᴅᴀꜰᴛᴀʀ ᴋᴀᴛᴀ ᴛᴏxɪᴄ*\n\n");

    if !custom.is_empty() {
        text.push_str(&format!(
            "╭┈┈⬡「 ✏️ *ᴄᴜsᴛᴏᴍ* ({}) 」\n",
            custom.len()
        ));
        for (i, w) in custom.iter().enumerate() {
            text.push_str(&format!("┃ {}. {w}\n", i + 1));
        }
        text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    }

    text.push_str(&format!(
        "╭┈┈⬡「 📦 *ᴅᴇꜰᴀᴜʟᴛ* ({}) 」\n",
        DEFAULT_TOXIC_WORDS.len()
    ));
    for (i, w) in DEFAULT_TOXIC_WORDS.iter().enumerate() {
        text.push_str(&format!("┃ {}. {w}\n", i + 1));
    }
    text.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    text.push_str(&format!(
        "Total: *{}* kata\n",
        custom.len() + DEFAULT_TOXIC_WORDS.len()
    ));
    text.push_str("`.addtoxic <kata>` untuk tambah\n");
    text.push_str("`.deltoxic <kata>` untuk hapus");
    text
}

#[plugin(
    name = "listtoxic",
    alias = ["toxiclist", "katatoxic", "lihatkata"],
    category = "group",
    description = "Lihat daftar kata toxic",
    usage = ".listtoxic",
    example = ".listtoxic",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn listtoxic(ctx: Ctx) -> Reply {
    Reply::text(render(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(group: serde_json::Value) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "listtoxic".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("listtoxic").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listtoxic").unwrap();
        assert_eq!(p.meta.name, "listtoxic");
        assert_eq!(p.meta.alias, ["toxiclist", "katatoxic", "lihatkata"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Lihat daftar kata toxic");
        assert_eq!(p.meta.usage, ".listtoxic");
        assert_eq!(p.meta.example, ".listtoxic");
        assert_eq!(p.meta.cooldown, 5);
        assert!(p.meta.is_group && p.meta.is_admin && !p.meta.is_private);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["toxiclist", "katatoxic", "lihatkata"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "listtoxic");
        }
    }

    #[test]
    fn the_default_table_is_the_js_list_verbatim() {
        assert_eq!(DEFAULT_TOXIC_WORDS.len(), 20);
        assert_eq!(DEFAULT_TOXIC_WORDS[0], "anjing");
        assert_eq!(DEFAULT_TOXIC_WORDS[12], "jancok");
        assert_eq!(DEFAULT_TOXIC_WORDS[19], "sialan");
        assert!(DEFAULT_TOXIC_WORDS.iter().all(|w| !w.is_empty()));
    }

    #[test]
    fn absent_group_data_shows_only_defaults() {
        let out = run(serde_json::json!(null));
        assert!(out.starts_with("📋 *ᴅᴀꜰᴛᴀʀ ᴋᴀᴛᴀ ᴛᴏxɪᴄ*\n\n"));
        assert!(!out.contains("✏️ *ᴄᴜsᴛᴏᴍ*"));
        assert!(out.contains("╭┈┈⬡「 📦 *ᴅᴇꜰᴀᴜʟᴛ* (20) 」\n"));
        assert!(out.contains("┃ 1. anjing\n"));
        assert!(out.contains("┃ 20. sialan\n"));
        assert!(out.contains("Total: *20* kata\n"));
        assert!(out.ends_with("`.deltoxic <kata>` untuk hapus"));
    }

    #[test]
    fn empty_custom_list_omits_the_custom_block() {
        let out = run(serde_json::json!({"toxicWords": []}));
        assert!(!out.contains("✏️ *ᴄᴜsᴛᴏᴍ*"));
        assert!(out.contains("Total: *20* kata"));
    }

    #[test]
    fn custom_words_come_first_and_raise_the_total() {
        let out = run(serde_json::json!({"toxicWords": ["kazak", "bangsat-kecil"]}));
        assert!(out.contains("╭┈┈⬡「 ✏️ *ᴄᴜsᴛᴏᴍ* (2) 」\n┃ 1. kazak\n┃ 2. bangsat-kecil\n╰┈┈┈┈┈┈┈┈⬡"));
        assert!(out.contains("Total: *22* kata"));
        assert!(
            out.find("✏️ *ᴄᴜsᴛᴏᴍ*").unwrap() < out.find("📦 *ᴅᴇꜰᴀᴜʟᴛ*").unwrap(),
            "blok custom mendahului blok default"
        );
    }

    #[test]
    fn a_wrong_typed_toxic_list_does_not_panic() {
        let out = run(serde_json::json!({"toxicWords": {"a": 1}}));
        assert!(!out.contains("✏️ *ᴄᴜsᴛᴏᴍ*"));
        assert!(out.contains("Total: *20* kata"));
    }

    #[test]
    fn both_blocks_close_and_count_is_two_when_custom_exists() {
        let out = run(serde_json::json!({"toxicWords": ["x"]}));
        assert_eq!(out.matches("╰┈┈┈┈┈┈┈┈⬡").count(), 2);
        assert_eq!(out.matches("╭┈┈⬡").count(), 2);
    }
}