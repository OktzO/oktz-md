//! `plugins/group/setwelcome.js` — `groups[chat].welcomeMsg` + `welcome: true`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.text || m.args.join(" ")` — teks setelah command. `m.text` sudah
/// mencakup sisa argumen, jadi cabang `||` di JS tidak pernah fallen-back ke
/// `join` dalam kondisi normal; tetap ditulis sebagai satu nilai.
pub(crate) fn message(ctx: &Ctx) -> &str {
    ctx.text.as_str()
}

/// Cabang tanpa teks — daftar placeholder `{user}`, `{group}`, dst.
pub(crate) fn usage(prefix: &str) -> String {
    format!(
        "📝 *sᴇᴛ ᴡᴇʟᴄᴏᴍᴇ*\n\n\
         ╭┈┈⬡「 📋 *ᴘʟᴀᴄᴇʜᴏʟᴅᴇʀ* 」\n\
         ┃ ◦ `{{user}}` - Nama member\n\
         ┃ ◦ `{{number}}` - Nomor member\n\
         ┃ ◦ `{{group}}` - Nama grup\n\
         ┃ ◦ `{{desc}}` - Deskripsi grup\n\
         ┃ ◦ `{{count}}` - Jumlah member\n\
         ┃ ◦ `{{owner}}` - Nama owner grup\n\
         ┃ ◦ `{{date}}` - Tanggal (DD/MM/YYYY)\n\
         ┃ ◦ `{{time}}` - Waktu (HH:mm WIB)\n\
         ┃ ◦ `{{day}}` - Hari (Senin, Selasa, dll)\n\
         ┃ ◦ `{{bot}}` - Nama bot\n\
         ┃ ◦ `{{prefix}}` - Prefix bot\n\
         ╰┈┈⬡\n\n\
         `Contoh:`\n\
         `{p}setwelcome Halo {{user}}! 👋`\n\
         `Selamat datang di {{group}} pada {{day}}, {{date}}`",
        p = prefix
    )
}

pub(crate) fn confirm(prefix: &str, text: &str) -> String {
    format!("✅ Welcome berhasil di set menjadi *{text}*\nMau reset? ketik {prefix}resetwelcome")
}

#[plugin(
    name = "setwelcome",
    alias = ["customwelcome"],
    category = "group",
    description = "Set custom welcome message",
    usage = ".setwelcome <pesan>",
    example = ".setwelcome Halo {user}, selamat datang di {group}!",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn setwelcome(ctx: Ctx) -> Reply {
    let text = message(&ctx);
    if text.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }
    Reply::text(confirm(&ctx.prefix, text))
        .with_reaction("✅")
        .with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "welcomeMsg": text, "welcome": true }),
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
            command: "setwelcome".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("setwelcome").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus tepat satu mutasi");
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
        let p = oktz_plugin_api::find("setwelcome").unwrap();
        assert_eq!(p.meta.name, "setwelcome");
        assert_eq!(p.meta.alias, ["customwelcome"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Set custom welcome message");
        assert_eq!(p.meta.usage, ".setwelcome <pesan>");
        assert_eq!(
            p.meta.example,
            ".setwelcome Halo {user}, selamat datang di {group}!"
        );
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin, "isAdmin: true");
        assert!(!p.meta.is_bot_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn the_alias_resolves_back_to_this_plugin() {
        assert_eq!(
            oktz_plugin_api::find("customwelcome").unwrap().meta.name,
            "setwelcome"
        );
    }

    #[test]
    fn an_empty_message_shows_the_placeholder_help_and_writes_nothing() {
        for g in [json!(null), json!({}), json!({"welcomeMsg": "lama"})] {
            let r = run(g.clone(), "");
            assert!(text_of(&r).starts_with("📝 *sᴇᴛ ᴡᴇʟᴄᴏᴍᴇ*"), "input {g}");
            assert!(text_of(&r).contains("{user}"), "placeholder harus disebut");
            assert!(text_of(&r).contains("{day}"));
            assert!(r.mutations().is_empty(), "tanpa teks tidak boleh menulis");
        }
    }

    #[test]
    fn the_help_branch_does_not_depend_on_the_stored_group_row() {
        // Store yang sudah punya `welcomeMsg` tetap masuk cabang help kalau
        // argumen kosong — pemeriksaan ada di argumen, bukan di store.
        let r = run(json!({"welcomeMsg": "Halo lama"}), "");
        assert!(text_of(&r).starts_with("📝 *sᴇᴛ ᴡᴇʟᴄᴏᴍᴇ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_message_stores_welcome_msg_and_turns_welcome_on() {
        let r = run(json!({}), "Halo {user}, selamat datang!");
        let data = group_write(&r);
        assert_eq!(data["welcomeMsg"], json!("Halo {user}, selamat datang!"));
        assert_eq!(
            data["welcome"], json!(true),
            "`welcome: true` ikut ditulis, bukan hanya welcomeMsg"
        );
        assert!(text_of(&r).starts_with("✅ Welcome berhasil di set menjadi *"));
        assert!(text_of(&r).ends_with("Mau reset? ketik .resetwelcome"));
    }

    #[test]
    fn the_confirm_reply_names_the_live_prefix() {
        assert!(confirm("!", "hi").ends_with("ketik !resetwelcome"));
    }

    #[test]
    fn the_written_text_is_byte_identical_to_what_the_reply_quotes() {
        // Reply mengutip teks yang disimpan; kalau reply dan write berbeda,
        // admin melihat satu pesan sementara tersimpan yang lain.
        let msg = " Halo   {user}  ";
        let r = run(json!({}), msg);
        let stored = group_write(&r)["welcomeMsg"].as_str().unwrap().to_string();
        assert_eq!(stored, msg, "JS tidak trim teks yang tersimpan");
        assert!(
            text_of(&r).contains(&stored),
            "reply harus mengutip teks yang benar-benar ditulis"
        );
    }

    #[test]
    fn the_write_is_paired_with_the_success_reply() {
        // Reply "berhasil diset" tanpa mutasi = klaim palsu.
        let r = run(json!({}), "Halo");
        assert!(text_of(&r).contains("berhasil di set"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn a_whitespace_only_message_is_still_a_message_in_javascript() {
        // `if (!text)` di JS — `" "` itu truthy, jadi masuk cabang tulis dan
        // `welcomeMsg` berisi spasi. `is_empty()` (bukan `trim().is_empty()`)
        // yang menjaga itu; memakai trim di sini akan mengubahnya jadi help.
        let r = run(json!({}), " ");
        assert_eq!(group_write(&r)["welcomeMsg"], json!(" "));
        assert!(text_of(&r).contains("berhasil di set"));
    }
}