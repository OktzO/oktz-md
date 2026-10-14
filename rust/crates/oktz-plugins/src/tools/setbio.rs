//! `plugins/tools/setbio.js` — mengubah bio/status bot.
//!
//! `ponytail:` langkah terakhir JS, `sock.updateProfileStatus(bioToSet)`, tidak
//! bisa di-port: `Ctx` tidak punya `sock`. Karena itu balasan sukses JS
//! ("✅ *ʙɪᴏ ʙᴏᴛ ᴅɪᴜʙᴀʜ*") TIDAK dipakai — teks itu menyatakan bio benar-benar
//! berubah, dan tidak ada yang berubah. Yang dipindah: kedua guard (kartu
//! usage dan batas 139 karakter) plus pelaporan kemampuan yang hilang.
//!
//! Panjang dihitung dengan `encode_utf16().count()` karena `String.length` di JS
//! menghitung unit UTF-16, bukan karakter.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `bioToSet.length > 139` (setbio.js:31).
const MAX: usize = 139;

fn usage(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}setbio Bio bot baru`\n\
         > `{prefix}setbio clear` - Hapus bio"
    )
}

fn too_long() -> &'static str {
    "⚠️ *ᴠᴀʟɪᴅᴀsɪ*\n\n> Bio maksimal 139 karakter."
}

/// Pola yang sama dengan `owner/block.rs`: sebut panggilan yang hilang, jangan
/// mengarang hasil.
fn unavailable() -> &'static str {
    "⚠️ *Ubah bio bot tidak tersedia di port Rust*\n\n\
     > Ctx tidak punya `sock`, jadi `updateProfileStatus` tidak bisa dipanggil."
}

/// `newBio?.toLowerCase() === 'clear' ? '' : (newBio || '')` (setbio.js:29).
fn bio_to_set(new_bio: &str) -> &str {
    if new_bio.eq_ignore_ascii_case("clear") {
        ""
    } else {
        new_bio
    }
}

#[plugin(
    name = "setbio",
    alias = ["setbiobot", "setstatus", "setabout"],
    category = "tools",
    description = "Mengubah bio/status bot",
    usage = ".setbio <bio baru>",
    example = ".setbio Bot WhatsApp by Lucky Archz",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn setbio(ctx: Ctx) -> Reply {
    let new_bio = ctx.text.trim();
    if new_bio.is_empty() && ctx.args.is_empty() {
        return Reply::text(usage(&ctx.prefix));
    }

    let bio = bio_to_set(new_bio);
    if bio.encode_utf16().count() > MAX {
        return Reply::text(too_long());
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
            command: "setbio".into(),
            sender: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (oktz_plugin_api::find("setbio").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setbio").unwrap();
        assert_eq!(p.meta.name, "setbio");
        assert_eq!(p.meta.alias, ["setbiobot", "setstatus", "setabout"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Mengubah bio/status bot");
        assert_eq!(p.meta.usage, ".setbio <bio baru>");
        assert_eq!(p.meta.example, ".setbio Bot WhatsApp by Lucky Archz");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["setbiobot", "setstatus", "setabout"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("setbio")
            );
        }
    }

    #[test]
    fn a_bare_command_shows_the_usage_card() {
        for input in ["", "   "] {
            let t = run(input.clone());
            assert!(t.starts_with("⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*"), "input {input:?}: {t}");
            assert!(t.contains("> `.setbio Bio bot baru`"), "{t}");
            assert!(t.contains("> `.setbio clear` - Hapus bio"), "{t}");
        }
    }

    #[test]
    fn a_bio_of_exactly_the_limit_is_accepted() {
        let bio = "a".repeat(MAX);
        assert!(run(&bio).contains("tidak tersedia"), "tepat 139 karakter masih sah");
    }

    #[test]
    fn a_bio_one_character_too_long_is_refused() {
        let bio = "a".repeat(MAX + 1);
        assert_eq!(run(&bio), too_long());
    }

    // `String.length` = unit UTF-16; satu emoji dihitung 2 di JS.
    #[test]
    fn the_limit_counts_utf16_units_like_the_js_string_length() {
        // U+1F642 = 2 unit UTF-16: 69 emoji = 138 (sah), 70 = 140 (tolak).
        assert_eq!(run(&"\u{1F642}".repeat(69)), unavailable());
        assert_eq!(run(&"\u{1F642}".repeat(70)), too_long());
    }

    #[test]
    fn clear_is_case_insensitive_and_empties_the_bio() {
        for input in ["clear", "CLEAR", "Clear"] {
            assert_eq!(bio_to_set(input), "");
        }
        assert_eq!(bio_to_set("halo"), "halo");
    }

    #[test]
    fn the_action_never_claims_the_bio_was_changed() {
        for input in ["halo", "clear", "a", &"x".repeat(200)] {
            let t = run(input.clone());
            assert!(
                !t.contains("Bio bot sekarang") && !t.contains("berhasil dihapus"),
                "input {input:?}: teks sukses JS tidak boleh dipakai — {t}"
            );
        }
        let t = run("halo");
        assert!(t.contains("`updateProfileStatus`"), "{t}");
    }
}
