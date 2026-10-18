//! `plugins/owner/ganti-goodbye.js` — ganti gambar goodbye.jpg (thumbnail goodbye).
//!
//! `ponytail:` `m.download()` / `m.quoted.download()` dan
//! `updateAssetUrl('goodbye', buffer, ...)` dari `src/lib/uploader.js` tidak
//! ada di `Ctx` API, jadi unduh + upload tidak dapat dijalankan. Yang
//! dipindah: deteksi gambar (trigger atau quoted) dan teks balasan.
//!
//! Catatan: baris `pub mod ganti_goodbye;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ganti-goodbye.jpg",
    alias = ["gantigoodbye", "setfotogoodbye"],
    category = "owner",
    description = "Ganti gambar goodbye.jpg (thumbnail goodbye)",
    usage = ".ganti-goodbye.jpg (reply/kirim gambar)",
    example = ".ganti-goodbye.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_goodbye(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image());

    if !is_image {
        return Reply::text(
            "🖼️ *ɢᴀɴᴛɪ ɢᴏᴏᴅʙʏᴇ.ᴊᴘɢ*\n\n\
             > Kirim/reply gambar untuk mengganti\n\
             > File: assets/image/goodbye.jpeg",
        );
    }

    Reply::text(
        "⚠️ *Ganti gambar goodbye tidak tersedia di port Rust*\n\n\
         > `m.download()` dan `updateAssetUrl` tidak ada di `Ctx` API,\n\
         > jadi unduh dan upload gambar tidak dapat dilakukan.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{IncomingMedia, QuotedKind, QuotedMessage};

    fn run(media: Option<IncomingMedia>, quoted: Option<QuotedKind>) -> Reply {
        let ctx = Ctx {
            command: "ganti-goodbye.jpg".into(),
            media,
            quoted: quoted.map(|kind| QuotedMessage {
                id: "q1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "628111@s.whatsapp.net".into(),
                text: String::new(),
                kind,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        };
        ganti_goodbye(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("ganti-goodbye.jpg").unwrap();
        assert_eq!(p.meta.name, "ganti-goodbye.jpg");
        assert_eq!(p.meta.alias, ["gantigoodbye", "setfotogoodbye"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn tanpa_gambar_tampilkan_petunjuk() {
        let r = run(None, None);
        assert!(body(&r).contains("ɢᴀɴᴛɪ ɢᴏᴏᴅʙʏᴇ.ᴊᴘɢ"));
    }

    #[test]
    fn gambar_dari_trigger_terdeteksi() {
        let r = run(
            Some(IncomingMedia {
                kind: oktz_plugin_api::IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            None,
        );
        assert!(!body(&r).contains("Kirim/reply gambar"));
    }

    #[test]
    fn gambar_dari_quoted_terdeteksi() {
        let r = run(None, Some(QuotedKind::Image));
        assert!(!body(&r).contains("Kirim/reply gambar"));
    }

    #[test]
    fn quoted_bukan_gambar_tidak_terdeteksi() {
        let r = run(None, Some(QuotedKind::Text));
        assert!(body(&r).contains("Kirim/reply gambar"));
    }
}
