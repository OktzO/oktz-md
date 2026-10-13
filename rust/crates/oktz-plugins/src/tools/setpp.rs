//! `plugins/tools/setpp.js` — mengubah foto profil bot.
//!
//! `ponytail:` dua langkah JS butuh `sock` dan tidak bisa di-port: unduh
//! quoted (`m.quoted.download()`) dan `sock.updateProfilePicture(botJid, buffer)`.
//! `Ctx` juga tidak punya penanda media pesan sendiri (`m.isImage`), jadi hanya
//! quoted yang bisa dikenali. Kartu usage — cabang `!buffer` — tetap bisa
//! dipertahankan karena itu murni teks.
//!
//! Balasan sukses JS tidak dipakai: tidak ada gambar yang terunggah.

use oktz_plugin_api::{Ctx, Reply, plugin};

fn usage(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > Reply gambar + `{prefix}setpp`\n\
         > Kirim gambar + caption `{prefix}setpp`"
    )
}

fn unavailable() -> &'static str {
    "⚠️ *Ubah foto profil bot tidak tersedia di port Rust*\n\n\
     > Ctx tidak punya `sock`, jadi `updateProfilePicture` tidak bisa dipanggil.\n\
     > Unduh media juga butuh `download()` yang tidak ada di `QuotedMessage`."
}

#[plugin(
    name = "setpp",
    alias = ["setprofilebot", "setppbot", "setfotobot"],
    category = "tools",
    description = "Mengubah foto profil bot",
    usage = ".setpp (reply gambar)",
    example = ".setpp",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn setpp(ctx: Ctx) -> Reply {
    match ctx.quoted() {
        Some(q) if q.kind.is_image() => Reply::text(unavailable()),
        // JS: `buffer` tetap null untuk quoted non-gambar dan untuk pesan tanpa
        // media — dua-duanya jatuh ke kartu usage.
        _ => Reply::text(usage(&ctx.prefix)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, QuotedKind, QuotedMessage};

    fn quoted(kind: QuotedKind) -> QuotedMessage {
        QuotedMessage {
            id: "q1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: String::new(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn run(quoted: Option<QuotedMessage>) -> String {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "setpp".into(),
            sender: "628111@s.whatsapp.net".into(),
            quoted,
            ..Default::default()
        };
        (oktz_plugin_api::find("setpp").unwrap().handler)(ctx)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("setpp").unwrap();
        assert_eq!(p.meta.name, "setpp");
        assert_eq!(
            p.meta.alias,
            ["setprofilebot", "setppbot", "setfotobot"]
        );
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.description, "Mengubah foto profil bot");
        assert_eq!(p.meta.usage, ".setpp (reply gambar)");
        assert_eq!(p.meta.example, ".setpp");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["setprofilebot", "setppbot", "setfotobot"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("setpp")
            );
        }
    }

    #[test]
    fn without_an_image_quote_it_shows_the_usage_card() {
        // JS hanya mengunduh kalau `m.quoted?.isImage`; sisanya → `!buffer`.
        assert_eq!(run(None), usage("."));
        for kind in [
            QuotedKind::Text,
            QuotedKind::Video,
            QuotedKind::Sticker,
            QuotedKind::Document,
        ] {
            assert_eq!(run(Some(quoted(kind.clone()))), usage("."), "{kind:?}");
        }
    }

    #[test]
    fn an_image_quote_reports_the_missing_capability_instead_of_claiming_success() {
        let t = run(Some(quoted(QuotedKind::Image)));
        assert!(t.contains("`updateProfilePicture`"), "{t}");
        assert!(
            !t.contains("berhasil diperbarui"),
            "teks sukses JS menyatakan upload yang tidak terjadi: {t}"
        );
    }
}
