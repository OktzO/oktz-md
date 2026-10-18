//! `plugins/group/setintro.js` — `groups[chat].intro`.
//!
//! Reply konfirmasi menyebut `.intro`; perintah membacanya ada di
//! `group/intro.rs` dan memakai field yang sama.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

/// `m.fullArgs?.trim() || m.text?.trim()` — `fullArgs` adalah sisa mentah
/// termasuk spasi; `trim()` dilakukan di kedua cabang, jadi hasil selalu
/// dipangkas.
pub(crate) fn intro_text(ctx: &Ctx) -> String {
    ctx.text.trim().to_string()
}

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📝 *sᴇᴛ ɪɴᴛʀᴏ*\n\n\
         > Masukkan pesan intro!\n\n\
         *Placeholder yang tersedia:*\n\
         > @user - Nama pengguna\n\
         > @group - Nama grup\n\
         > @count - Jumlah member\n\
         > @date - Tanggal hari ini\n\
         > @time - Waktu sekarang\n\
         > @desc - Deskripsi grup\n\
         > @botname - Nama bot\n\n\
         *Contoh:*\n\
         > {p}setintro Selamat datang @user di grup @group! 👋",
        p = prefix
    )
}

pub(crate) fn confirm(prefix: &str) -> String {
    format!(
        "✅ *ɪɴᴛʀᴏ ᴅɪsᴀᴠᴇ!*\n\
         Pesan intro grup berhasil diubah.\n\
         Ketik *{prefix}intro* untuk melihat hasilnya."
    )
}

#[plugin(
    name = "setintro",
    alias = ["setperkenalan", "introset"],
    category = "group",
    description = "Set pesan intro grup (admin only)",
    usage = ".setintro <pesan>",
    example = ".setintro Selamat datang @user di @group!",
    cooldown = 10,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn setintro(ctx: Ctx) -> Reply {
    let text = intro_text(&ctx);
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    Reply::text(confirm(&ctx.prefix)).with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ "intro": text }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn run(group: Value, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setintro".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("setintro").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("setintro").unwrap();
        assert_eq!(p.meta.name, "setintro");
        assert_eq!(p.meta.alias, ["setperkenalan", "introset"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Set pesan intro grup (admin only)");
        assert_eq!(p.meta.usage, ".setintro <pesan>");
        assert_eq!(
            p.meta.example,
            ".setintro Selamat datang @user di @group!"
        );
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["setperkenalan", "introset"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "setintro");
        }
    }

    #[test]
    fn a_blank_message_shows_help_and_writes_nothing() {
        // `.trim()` membuat pesan berisi spasi saja dianggap kosong — berbeda
        // dari `setwelcome` yang TIDAK memangkas.
        for msg in ["", "   ", "\t\n"] {
            let r = run(serde_json::json!({}), msg);
            assert!(
                text_of(&r).starts_with("📝 *sᴇᴛ ɪɴᴛʀᴏ*"),
                "input {msg:?} harus help"
            );
            assert!(r.mutations().is_empty(), "input {msg:?}");
        }
    }

    #[test]
    fn the_help_branch_lists_every_placeholder_and_the_example() {
        let r = run(serde_json::json!({}), "");
        let t = text_of(&r);
        for token in ["@user", "@group", "@count", "@date", "@time", "@desc", "@botname"] {
            assert!(t.contains(token), "placeholder {token} hilang dari help");
        }
        assert!(t.contains(".setintro Selamat datang @user di grup @group! 👋"));
    }

    #[test]
    fn the_message_is_trimmed_on_both_sides_before_storage() {
        let r = run(serde_json::json!({}), "   halo dunia   ");
        assert_eq!(
            group_write(&r)["intro"],
            serde_json::json!("halo dunia"),
            "`m.text?.trim()` ada di kedua cabang JS"
        );
    }

    #[test]
    fn inner_whitespace_and_placeholders_survive_untouched() {
        let msg = "halo @user\n\n  dari @group pada @date";
        let r = run(serde_json::json!({}), msg);
        assert_eq!(group_write(&r)["intro"], serde_json::json!(msg));
    }

    #[test]
    fn the_confirm_reply_names_the_live_prefix_and_points_at_intro() {
        let r = run(serde_json::json!({}), "halo");
        let t = text_of(&r);
        assert!(t.starts_with("✅ *ɪɴᴛʀᴏ ᴅɪsᴀᴠᴇ!*"));
        assert!(t.contains("Ketik *.intro* untuk melihat hasilnya."), "{t}");

        assert!(
            confirm("!").contains("Ketik *!intro*"),
            "prefix harus ikut: {}",
            confirm("!")
        );
    }

    #[test]
    fn only_the_intro_field_is_written() {
        let r = run(serde_json::json!({"intro": "lama", "game": false}), "baru");
        let data = group_write(&r);
        assert_eq!(data.as_object().unwrap().len(), 1, "hanya `intro` disentuh");
        assert_eq!(data["intro"], serde_json::json!("baru"));
    }

    #[test]
    fn the_confirm_reply_is_paired_with_the_write() {
        let r = run(serde_json::json!({}), "halo");
        assert!(text_of(&r).contains("berhasil diubah"));
        assert_eq!(r.mutations().len(), 1);
    }

    #[test]
    fn set_and_reset_intro_keep_separate_keys() {
        for (name, keys) in [
            ("setintro", &["setintro", "setperkenalan", "introset"][..]),
            (
                "resetintro",
                &["resetintro", "introdel", "delintro", "deleteintro"][..],
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