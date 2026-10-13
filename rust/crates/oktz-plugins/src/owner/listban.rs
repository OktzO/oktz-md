//! `plugins/owner/listban.js` — daftar `bannedUsers`.
//!
//! `ponytail:` JS membaca `config.bannedUsers` dulu dan baru jatuh ke
//! `db.setting('bannedUsers')` kalau cache itu kosong. `config.bannedUsers`
//! adalah cache in-memory milik `config.js` (`config.js:521`), tidak ada di
//! `Ctx`, jadi yang dipakai selalu setting.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::joined_strings;

/// `(i + 1). \`${bannedUsers[i]}\`` — `${el}` di JS, jadi elemen non-string
/// dirender sebagai teksnya, bukan `[object Object]` gaya JS yang sebenarnya.
pub(crate) fn numbered(items: &[String]) -> Vec<String> {
    items
        .iter()
        .enumerate()
        .map(|(i, v)| format!("┃ {}. `{v}`", i + 1))
        .collect()
}

pub(crate) fn caption(items: &[String]) -> String {
    let mut out = String::from("🚫 *ʟɪsᴛ ʙᴀɴɴᴇᴅ*\n\n╭┈┈⬡「 ⛔ *ᴜsᴇʀs* 」\n");
    for line in numbered(items) {
        out.push_str(&line);
        out.push('\n');
    }
    out.push_str("╰┈┈⬡\n\n");
    out.push_str(&format!(
        "> ᴛᴏᴛᴀʟ: `{}` ʙᴀɴɴᴇᴅ ᴜsᴇʀ",
        items.len()
    ));
    out
}

pub(crate) fn empty(prefix: &str) -> String {
    format!("🚫 *ʟɪsᴛ ʙᴀɴɴᴇᴅ*\n\n> Tidak ada user yang dibanned\n\n`Gunakan: {prefix}ban <nomor>`")
}

#[plugin(
    name = "listban",
    alias = ["listbanned", "banlist"],
    category = "owner",
    description = "Melihat daftar banned user",
    usage = ".listban",
    example = ".listban",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn listban(ctx: Ctx) -> Reply {
    let banned = joined_strings(ctx.setting_list("bannedUsers"));
    if banned.is_empty() {
        return Reply::text(empty(&ctx.prefix));
    }
    Reply::text(caption(&banned))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(settings: serde_json::Value, prefix: &str) -> String {
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "listban".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            settings,
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("listban").unwrap().handler)(ctx);
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("listban").unwrap();
        assert_eq!(p.meta.name, "listban");
        assert_eq!(p.meta.alias, ["listbanned", "banlist"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Melihat daftar banned user");
        assert_eq!(p.meta.usage, ".listban");
        assert_eq!(p.meta.example, ".listban");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["listbanned", "banlist"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "listban");
        }
    }

    #[test]
    fn an_empty_or_missing_list_shows_the_short_message() {
        for settings in [json!({}), json!({"bannedUsers": []}), json!({"bannedUsers": "x"})] {
            let out = run(settings.clone(), ".");
            assert!(out.starts_with("🚫 *ʟɪsᴛ ʙᴀɴɴᴇᴅ*"), "{settings}");
            assert!(out.contains("> Tidak ada user yang dibanned"), "{settings}");
            assert!(out.contains("`Gunakan: .ban <nomor>`"), "{settings}");
        }
    }

    #[test]
    fn the_empty_message_carries_the_live_prefix() {
        assert!(run(json!({}), "!").contains("`Gunakan: !ban <nomor>`"));
    }

    #[test]
    fn entries_are_numbered_from_one_and_the_total_is_reported() {
        let out = run(json!({"bannedUsers": ["6281", "6282", "6283"]}), ".");
        assert!(out.contains("╭┈┈⬡「 ⛔ *ᴜsᴇʀs* 」"));
        assert!(out.contains("┃ 1. `6281`"));
        assert!(out.contains("┃ 2. `6282`"));
        assert!(out.contains("┃ 3. `6283`"));
        assert!(out.ends_with("> ᴛᴏᴛᴀʟ: `3` ʙᴀɴɴᴇᴅ ᴜsᴇʀ"), "{out}");
    }

    #[test]
    fn nothing_is_written_by_a_read_only_command() {
        let ctx = Ctx {
            command: "listban".into(),
            settings: json!({"bannedUsers": ["6281"]}),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("listban").unwrap().handler)(ctx);
        assert!(r.mutations().is_empty());
    }
}