//! `plugins/group/setdeskgc.js` — ubah deskripsi grup.
//!
//! Yang dipindah: seluruh gerbang (tanpa argumen, kata `clear`, batas
//! panjang). Penyuntingan sebenarnya `sock.groupUpdateDescription(m.chat,
//! descToSet)` (`setdeskgc.js:40`) tidak ada di API plugin, jadi setelah
//! gerbang lolos plugin diam — bukan mengirim "deskripsi berhasil diperbarui".

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `setdeskgc.js:31` — `descToSet.length > 2048`.
pub(crate) const MAX_DESC: usize = 2048;

/// `const descToSet = newDesc.toLowerCase() === 'clear' ? '' : newDesc`
/// (`setdeskgc.js:29`).
pub(crate) fn desc_to_set(new_desc: &str) -> String {
    if new_desc.to_lowercase() == "clear" {
        String::new()
    } else {
        new_desc.to_string()
    }
}

/// `setdeskgc.js:22-26`.
fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}setdeskgc Deskripsi baru`\n\
         > `{prefix}setdeskgc clear` - Hapus deskripsi"
    )
}

/// `setdeskgc.js:32-35`.
const TOO_LONG: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ*\n\n> Deskripsi maksimal 2048 karakter.";

#[plugin(
    name = "setdeskgc",
    alias = ["setdesc", "setdescgc", "setdeskripsi", "setdesk"],
    category = "group",
    description = "Mengubah deskripsi grup",
    usage = ".setdeskgc <deskripsi baru>",
    example = ".setdeskgc Grup untuk diskusi",
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
pub fn setdeskgc(ctx: Ctx) -> Reply {
    let new_desc = ctx.text.trim();

    if ctx.text.is_empty() && ctx.args.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `ponytail:` JS menghitung `.length` sebagai unit UTF-16, Rust menghitung
    // scalar; emoji di luar BMP terhitung 1 di sini dan 2 di JS. Ambang 2048
    // jauh dari batas itu untuk deskripsi biasa.
    let desc = desc_to_set(new_desc);
    if desc.chars().count() > MAX_DESC {
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
            command: "setdeskgc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            text: text.into(),
            args: text.split_whitespace().map(String::from).collect(),
            ..Default::default()
        };
        (find("setdeskgc").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("setdeskgc").unwrap();
        assert_eq!(e.meta.name, "setdeskgc");
        assert_eq!(
            e.meta.alias,
            ["setdesc", "setdescgc", "setdeskripsi", "setdesk"]
        );
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Mengubah deskripsi grup");
        assert_eq!(e.meta.usage, ".setdeskgc <deskripsi baru>");
        assert_eq!(e.meta.example, ".setdeskgc Grup untuk diskusi");
        assert_eq!(e.meta.cooldown, 10);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["setdesc", "setdescgc", "setdeskripsi", "setdesk"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("setdeskgc"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn an_empty_command_shows_both_forms() {
        let r = run("");
        assert_eq!(
            body(&r),
            "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
             > `.setdeskgc Deskripsi baru`\n\
             > `.setdeskgc clear` - Hapus deskripsi"
        );
    }

    #[test]
    fn whitespace_only_is_not_the_empty_command() {
        // `!m.text` di JS: `"   "` itu truthy, jadi cabang cara pakai dilewati
        // walau `m.text?.trim()` kosong. Kesenjangan itu bawaan JS.
        assert!(run("   ").is_none(), "spasi saja bukan cabang cara pakai");
        assert_eq!(desc_to_set(""), "");
    }

    #[test]
    fn the_help_box_carries_the_live_prefix() {
        assert!(body(&run_with("!", "")).contains("> `!setdeskgc Deskripsi baru`"));
    }

    #[test]
    fn the_word_clear_becomes_an_empty_description() {
        for arg in ["clear", "CLEAR", "ClEaR"] {
            assert_eq!(desc_to_set(arg), "", "arg {arg:?}");
        }
        // `" Clear "` hanya jadi `""` kalau `trim()` jalan lebih dulu; itu
        // tugas `setdeskgc.js:20`, bukan fungsi ini.
        assert_eq!(desc_to_set(" Clear "), " Clear ");
        assert_eq!(
            desc_to_set("clearan"),
            "clearan",
            "tepat kata, bukan awalan"
        );
    }

    #[test]
    fn a_description_that_is_too_long_is_rejected_before_any_write() {
        let long = "a".repeat(MAX_DESC + 1);
        let r = run(&long);
        assert_eq!(body(&r), TOO_LONG);
        assert!(r.mutations().is_empty());
        // Tepat di batas masih lolos.
        let ok = "a".repeat(MAX_DESC);
        assert!(run(&ok).is_none(), "tepat 2048 belum di-tolak");
    }

    #[test]
    fn a_valid_description_goes_silent_instead_of_claiming_success() {
        let r = run("Grup untuk diskusi");
        assert!(r.is_none(), "tidak ada `groupUpdateDescription`");
        assert!(r.mutations().is_empty());
    }
}
