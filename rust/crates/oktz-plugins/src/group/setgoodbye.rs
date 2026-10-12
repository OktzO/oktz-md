//! `plugins/group/setgoodbye.js` — `goodbyeMsg` + `goodbye: true` + `leave: true`.
//!
//! Tiga flag ditulis bersama, bukan hanya `goodbyeMsg`: `leave` yang membuat
//! bot kirim pesan perpisahan saat member keluar, dan `goodbye` yang
//! mengaktifkannya. Melewatkan dua yang terakhir akan membuat reply "berhasil"
//! while bot tetap diam.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "📝 *sᴇᴛ ɢᴏᴏᴅʙʏᴇ*\n\n\
         ╭┈┈⬡「 📋 *ᴘʟᴀᴄᴇʜᴏʟᴅᴇʀ* 」\n\
         ┃ ◦ `{{user}}` - Nama member\n\
         ┃ ◦ `{{number}}` - Nomor member\n\
         ┃ ◦ `{{group}}` - Nama grup\n\
         ┃ ◦ `{{desc}}` - Deskripsi grup\n\
         ┃ ◦ `{{count}}` - Sisa member\n\
         ┃ ◦ `{{owner}}` - Nama owner grup\n\
         ┃ ◦ `{{date}}` - Tanggal (DD/MM/YYYY)\n\
         ┃ ◦ `{{time}}` - Waktu (HH:mm WIB)\n\
         ┃ ◦ `{{day}}` - Hari (Senin, Selasa, dll)\n\
         ┃ ◦ `{{bot}}` - Nama bot\n\
         ┃ ◦ `{{prefix}}` - Prefix bot\n\
         ╰┈┈⬡\n\n\
         `Contoh:`\n\
         `{p}setgoodbye Bye {{user}}! 👋`\n\
         `Sampai jumpa lagi pada {{day}}, {{date}}`",
        p = prefix
    )
}

pub(crate) fn confirm(prefix: &str, text: &str) -> String {
    format!("✅ Goodbye berhasil di set menjadi *{text}*\nMau reset? ketik {prefix}resetgoodbye")
}

#[plugin(
    name = "setgoodbye",
    alias = ["customgoodbye"],
    category = "group",
    description = "Set custom goodbye message",
    usage = ".setgoodbye <pesan>",
    example = ".setgoodbye Bye {user}, sampai jumpa lagi!",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn setgoodbye(ctx: Ctx) -> Reply {
    let text = ctx.text.as_str();
    if text.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }
    Reply::text(confirm(&ctx.prefix, text))
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({
                "goodbyeMsg": text,
                "goodbye": true,
                "leave": true,
            }),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use serde_json::json;

    fn run(group: Value, text: &str) -> Reply {
        let args: Vec<String> = text.split_whitespace().map(str::to_string).collect();
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setgoodbye".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("setgoodbye").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("setgoodbye").unwrap();
        assert_eq!(p.meta.name, "setgoodbye");
        assert_eq!(p.meta.alias, ["customgoodbye"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Set custom goodbye message");
        assert_eq!(p.meta.usage, ".setgoodbye <pesan>");
        assert_eq!(
            p.meta.example,
            ".setgoodbye Bye {user}, sampai jumpa lagi!"
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("customgoodbye").unwrap().meta.name,
            "setgoodbye"
        );
    }

    #[test]
    fn an_empty_message_shows_help_and_writes_nothing() {
        for g in [json!(null), json!({}), json!({"goodbyeMsg": "lama"})] {
            let r = run(g.clone(), "");
            assert!(text_of(&r).starts_with("📝 *sᴇᴛ ɢᴏᴏᴅʙʏᴇ*"), "input {g}");
            assert!(text_of(&r).contains("{user}"));
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn all_three_flags_are_written_together() {
        let r = run(json!({"goodbye": false, "leave": false}), "Bye {user}!");
        let data = group_write(&r);
        assert_eq!(data["goodbyeMsg"], json!("Bye {user}!"));
        assert_eq!(data["goodbye"], json!(true));
        assert_eq!(
            data["leave"], json!(true),
            "`leave` yang memicu pesan perpisahan saat member keluar"
        );
        assert_eq!(data.as_object().unwrap().len(), 3);
    }

    #[test]
    fn the_reply_quotes_exactly_what_was_stored() {
        let r = run(json!({}), "Sampai jumpa");
        let stored = group_write(&r)["goodbyeMsg"].as_str().unwrap().to_string();
        assert!(text_of(&r).contains(&stored));
    }

    #[test]
    fn the_confirm_reply_names_the_live_prefix() {
        assert!(confirm("#", "hi").ends_with("ketik #resetgoodbye"));
    }

    #[test]
    fn a_whitespace_message_is_written_just_like_in_javascript() {
        let r = run(json!({}), " ");
        assert_eq!(group_write(&r)["goodbyeMsg"], json!(" "));
    }

#[test]
    fn resetgoodbyes_alias_is_not_claimed_here() {
        // Nama pasangannya ada di modul sendiri; alias plugin ini tidak boleh
        // ikut mengarah ke sini.
        assert!(!oktz_plugin_api::find("setgoodbye")
            .unwrap()
            .meta
            .matches("delgoodbye"));
    }
}