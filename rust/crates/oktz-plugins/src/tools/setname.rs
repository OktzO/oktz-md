//! `plugins/tools/setname.js` — mengubah nama profil bot.
//!
//! `ponytail:` `sock.updateProfileName(newName)` tidak ada padanannya di `Ctx`.
//! Balasan sukses JS tidak dipakai karena menyatakan nama benar-benar berubah;
//! yang dipindah: kartu usage, validasi 1-25 karakter, dan pelaporan kemampuan
//! yang hilang. Panjang pakai `encode_utf16().count()` = `String.length` JS.

use oktz_plugin_api::{Ctx, Reply, plugin};

const MIN: usize = 1;
const MAX: usize = 25;

fn usage(prefix: &str) -> String {
    format!("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `{prefix}setname Nama Bot Baru`")
}

fn out_of_range() -> &'static str {
    "⚠️ *ᴠᴀʟɪᴅᴀsɪ*\n\n> Nama bot harus 1-25 karakter."
}

fn unavailable() -> &'static str {
    "⚠️ *Ubah nama bot tidak tersedia di port Rust*\n\n\
     > Ctx tidak punya `sock`, jadi `updateProfileName` tidak bisa dipanggil."
}

#[plugin(
    name = "setname",
    alias = ["setnamebot", "setbotnama"],
    category = "tools",
    description = "Mengubah nama profil bot",
    usage = ".setname <nama baru>",
    example = ".setname Foto-AI",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn setname(ctx: Ctx) -> Reply {
    let new_name = ctx.text.trim();
    if new_name.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }
    let len = new_name.encode_utf16().count();
    if !(MIN..=MAX).contains(&len) {
        return Reply::text(out_of_range());
    }
    Reply::text(unavailable())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::Ctx;

    fn run(text: &str) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setname".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("setname").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setname").unwrap();
        assert_eq!(p.meta.name, "setname");
        assert_eq!(p.meta.alias, ["setnamebot", "setbotnama"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Mengubah nama profil bot");
        assert_eq!(p.meta.usage, ".setname <nama baru>");
        assert_eq!(p.meta.example, ".setname Foto-AI");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_back_to_this_plugin() {
        for a in ["setnamebot", "setbotnama"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("setname")
            );
        }
    }

    #[test]
    fn a_bare_command_shows_the_usage_card() {
        for input in ["", "   "] {
            assert_eq!(run(input.clone()), usage("."));
        }
    }

    #[test]
    fn the_length_window_is_one_to_twenty_five() {
        assert_eq!(run(&"a".repeat(MAX)), unavailable(), "tepat 25 masih sah");
        assert_eq!(run(&"a".repeat(MAX + 1)), out_of_range());
        // `""` sudah ditangkap kartu usage lebih dulu — `length < 1` di JS
        // tidak bisa terjadi karena `!newName` sudah keluar duluan.
        assert_eq!(run(&"a".repeat(0)), usage("."));
    }

    #[test]
    fn the_length_window_counts_utf16_units() {
        // U+1F642 = 2 unit: 12 emoji = 24 (sah), 13 = 26 (tolak).
        assert_eq!(run(&"\u{1F642}".repeat(12)), unavailable());
        assert_eq!(run(&"\u{1F642}".repeat(13)), out_of_range());
    }

    #[test]
    fn a_valid_name_never_reports_success() {
        let t = run("Foto-AI");
        assert!(t.contains("`updateProfileName`"), "{t}");
        assert!(
            !t.contains("Nama bot sekarang"),
            "teks sukses JS menyatakan perubahan yang tidak terjadi: {t}"
        );
    }
}
