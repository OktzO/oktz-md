//! `plugins/group/listantilink.js` — `groups[chat].antilinkList` + tabel default.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

/// `DEFAULT_BLOCKED_LINKS` (`listantilink.js:9-17`), urutan persis.
pub(crate) const DEFAULT_BLOCKED_LINKS: [&str; 7] = [
    "chat.whatsapp.com",
    "wa.me",
    "bit.ly",
    "t.me",
    "telegram.me",
    "discord.gg",
    "discord.com/invite",
];

pub(crate) fn render(ctx: &Ctx) -> String {
    let custom = joined_strings(ctx.group_list("antilinkList"));

    let mut txt = String::from("🔗 *ᴅᴀꜰᴛᴀʀ ᴀɴᴛɪʟɪɴᴋ*\n\n");

    txt.push_str("╭┈┈⬡「 📌 *ᴅᴇꜰᴀᴜʟᴛ* 」\n");
    for (i, l) in DEFAULT_BLOCKED_LINKS.iter().enumerate() {
        txt.push_str(&format!("┃ {}. `{l}`\n", i + 1));
    }
    txt.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");

    if !custom.is_empty() {
        txt.push_str("╭┈┈⬡「 ➕ *ᴄᴜsᴛᴏᴍ* 」\n");
        for (i, l) in custom.iter().enumerate() {
            txt.push_str(&format!("┃ {}. `{l}`\n", i + 1));
        }
        txt.push_str("╰┈┈┈┈┈┈┈┈⬡\n\n");
    }

    txt.push_str(&format!(
        "> Default: *{}* link\n> Custom: *{}* link\n\n",
        DEFAULT_BLOCKED_LINKS.len(),
        custom.len()
    ));

    let p = ctx.prefix.as_str();
    txt.push_str(&format!("`{p}addantilink <link>` untuk tambah\n"));
    txt.push_str(&format!("`{p}delantilink <link>` untuk hapus"));
    txt
}

#[plugin(
    name = "listantilink",
    alias = ["antilinklist", "cekantilink"],
    category = "group",
    description = "Melihat daftar link yang diblokir",
    usage = ".listantilink",
    example = ".listantilink",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn listantilink(ctx: Ctx) -> Reply {
    Reply::text(render(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx_with(group: serde_json::Value) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "listantilink".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            group_data: group,
            ..Default::default()
        }
    }

    fn run(group: serde_json::Value) -> String {
        let ctx = ctx_with(group);
        (oktz_plugin_api::find("listantilink").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listantilink").unwrap();
        assert_eq!(p.meta.name, "listantilink");
        assert_eq!(p.meta.alias, ["antilinklist", "cekantilink"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Melihat daftar link yang diblokir");
        assert_eq!(p.meta.usage, ".listantilink");
        assert_eq!(p.meta.example, ".listantilink");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin);
        assert!(!p.meta.is_private && !p.meta.is_bot_admin);
    }

    #[test]
    fn aliases_resolve() {
        for a in ["antilinklist", "cekantilink"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "listantilink");
        }
    }

    #[test]
    fn absent_group_data_shows_only_the_seven_defaults() {
        let out = run(json!(null));
        assert!(out.starts_with("🔗 *ᴅᴀꜰᴛᴀʀ ᴀɴᴛɪʟɪɴᴋ*\n\n"));
        assert!(!out.contains("➕ *ᴄᴜsᴛᴏᴍ*"));
        assert!(out.contains("┃ 1. `chat.whatsapp.com`\n"));
        assert!(out.contains("┃ 7. `discord.com/invite`\n"));
        assert!(out.contains("\n> Default: *7* link\n> Custom: *0* link\n\n"));
        assert!(out.ends_with("`.delantilink <link>` untuk hapus"));
    }

    #[test]
    fn empty_custom_list_omits_the_custom_block() {
        let out = run(json!({"antilinkList": []}));
        assert!(!out.contains("➕ *ᴄᴜsᴛᴏᴍ*"));
        assert!(out.contains("> Custom: *0* link"));
    }

#[test]
    fn a_wrong_typed_antilink_list_falls_back_to_empty() {
        // `groupData.antilinkList || []`: string bukan array masih punya
        // `.length > 0`, jadi JS masuk cabang custom lalu `.forEach` meledak.
        // `group_list` mengembalikan slice kosong — tidak panic.
        let out = run(json!({"antilinkList": "bukan-array"}));
        assert!(!out.contains("➕ *ᴄᴜsᴛᴏᴍ*"));
        assert!(out.contains("> Custom: *0* link"));
    }

    #[test]
    fn custom_links_are_numbered_from_one_after_the_defaults() {
        let out = run(json!({"antilinkList": ["spam.example", "iklan.example"]}));
        assert!(out.contains("╭┈┈⬡「 ➕ *ᴄᴜsᴛᴏᴍ* 」\n┃ 1. `spam.example`\n┃ 2. `iklan.example`\n╰┈┈┈┈┈┈┈┈⬡"));
        assert!(out.contains("> Default: *7* link\n> Custom: *2* link"));
        // Blok default tetap lengkap dan tidak ikut ter-nomor ulang.
        assert!(out.contains("┃ 1. `chat.whatsapp.com`"));
        assert!(out.find("┃ 1. `chat.whatsapp.com`").unwrap() < out.find("┃ 1. `spam.example`").unwrap());
    }

    #[test]
    fn the_two_blocks_close_with_the_javascript_box_chars() {
        let out = run(json!({"antilinkList": ["x.example"]}));
        assert_eq!(out.matches("╰┈┈┈┈┈┈┈┈⬡").count(), 2);
        assert_eq!(out.matches("╭┈┈⬡").count(), 2);
    }

    #[test]
    fn prefix_is_interpolated_from_the_context() {
        let mut ctx = ctx_with(json!({"antilinkList": []}));
        ctx.prefix = "!".into();
        let out = render(&ctx);
        assert!(out.ends_with("`!delantilink <link>` untuk hapus"));
        assert!(out.contains("`!addantilink <link>` untuk tambah"));
    }

    #[test]
    fn default_table_is_the_js_list_verbatim() {
        assert_eq!(DEFAULT_BLOCKED_LINKS.len(), 7);
        assert_eq!(DEFAULT_BLOCKED_LINKS[0], "chat.whatsapp.com");
        assert_eq!(DEFAULT_BLOCKED_LINKS[6], "discord.com/invite");
    }
}