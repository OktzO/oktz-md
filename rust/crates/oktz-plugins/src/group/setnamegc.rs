//! `plugins/group/setnamegc.js` — ubah nama grup.
//!
//! Yang dipindah: seluruh gerbang (tanpa argumen, batas panjang). Penggantian
//! sebenarnya `sock.groupUpdateSubject(m.chat, newName)` (`setnamegc.js:39`)
//! tidak ada di API plugin, jadi setelah gerbang lolos plugin diam — bukan
//! mengirim "berhasil mengubah nama grup".

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `setnamegc.js:30` — `newName.length > 100` (batas bawahnya sudah lewat
/// `if (!newName)` di baris 22).
pub(crate) const MAX_NAME: usize = 100;

/// `setnamegc.js:23-26`.
fn help(prefix: &str) -> String {
    format!("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `{prefix}setnamegc Nama Grup Baru`")
}

/// `setnamegc.js:31-34`.
const TOO_LONG: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ*\n\n> Nama grup harus 1-100 karakter.";

#[plugin(
    name = "setnamegc",
    alias = ["setnamegrup", "setgcname", "setnamegroup", "setnamagrup"],
    category = "group",
    description = "Mengubah nama grup",
    usage = ".setnamegc <nama baru>",
    example = ".setnamegc Grup Keren",
    cooldown = 10,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn setnamegc(ctx: Ctx) -> Reply {
    let new_name = ctx.text.trim();

    if new_name.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `ponytail:` JS menghitung `.length` sebagai unit UTF-16, Rust menghitung
    // scalar; emoji di luar BMP terhitung 1 di sini dan 2 di JS.
    if new_name.chars().count() > MAX_NAME {
        return Reply::text(TOO_LONG);
    }

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    fn run(text: &str) -> Reply {
        run_with(".", text)
    }

    fn run_with(prefix: &str, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: prefix.into(),
            command: "setnamegc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(String::from).collect(),
            ..Default::default()
        };
        (find("setnamegc").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("setnamegc").unwrap();
        assert_eq!(e.meta.name, "setnamegc");
        assert_eq!(
            e.meta.alias,
            ["setnamegrup", "setgcname", "setnamegroup", "setnamagrup"]
        );
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Mengubah nama grup");
        assert_eq!(e.meta.usage, ".setnamegc <nama baru>");
        assert_eq!(e.meta.example, ".setnamegc Grup Keren");
        assert_eq!(e.meta.cooldown, 10);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["setnamegrup", "setgcname", "setnamegroup", "setnamagrup"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("setnamegc"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn an_empty_or_blank_name_shows_the_usage() {
        let expected = "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `.setnamegc Nama Grup Baru`";
        assert_eq!(body(&run("")), expected);
        assert_eq!(body(&run("    ")), expected, "`m.text?.trim()` kosong");
    }

    #[test]
    fn the_usage_carries_the_live_prefix() {
        assert!(body(&run_with("!", "")).contains("> `!setnamegc Nama Grup Baru`"));
    }

    #[test]
    fn a_name_over_the_limit_is_rejected() {
        let r = run(&"n".repeat(MAX_NAME + 1));
        assert_eq!(body(&r), TOO_LONG);
        assert!(r.mutations().is_empty());
        assert!(
            run(&"n".repeat(MAX_NAME)).is_none(),
            "tepat 100 belum di-tolak"
        );
    }

    #[test]
    fn a_valid_name_goes_silent_instead_of_claiming_success() {
        let r = run("Grup Keren");
        assert!(r.is_none(), "tidak ada `groupUpdateSubject`");
        assert!(r.mutations().is_empty());
    }
}
