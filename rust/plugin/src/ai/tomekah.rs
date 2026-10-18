//! `plugins/ai/tomekah.js` — ubah background gambar ke Mekah.
//!
//! `ponytail:` `uploadImage()` dan `applyImageEffect('tomekah', ...)` dari
//! `src/lib/uploader.js` dan `src/lib/effect-api.js` tidak ada di `Ctx` API,
//! dan `sock.sendMedia` juga tidak tersedia. Unduh, upload, efek, dan kirim
//! tidak dapat dijalankan. Hanya deteksi gambar dan teks balasan yang dipindah.
//!
//! Catatan: baris `pub mod tomekah;` perlu ditambah ke `ai/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "tomekah",
    alias = ["mekah", "mecca", "tomecca"],
    category = "ai",
    description = "Ubah background gambar ke Mekah",
    usage = ".tomekah (reply gambar)",
    example = ".tomekah",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tomekah(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image());

    if !is_image {
        return Reply::text(format!(
            "🕋 *ᴍᴇᴋᴀʜ sᴛʏʟᴇ*\n\n\
             > Kirim/reply gambar\n\n\
             `.tomekah`\n\
             Prefix saat ini: `.`"
        ));
    }

    Reply::text(
        "⚠️ *Efek background Mekah tidak tersedia di port Rust*\n\n\
         > `uploadImage` dan `applyImageEffect` belum tersedia di `Ctx` API,\n\
         > jadi unduh, upload, dan kirim gambar hasil efek tidak dapat dilakukan.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{IncomingMedia, QuotedKind, QuotedMessage};

    fn run(media: Option<IncomingMedia>, quoted: Option<QuotedKind>) -> Reply {
        let ctx = Ctx {
            command: "tomekah".into(),
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
        tomekah(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("tomekah").unwrap();
        assert_eq!(p.meta.name, "tomekah");
        assert_eq!(p.meta.alias, ["mekah", "mecca", "tomecca"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 2);
    }

    #[test]
    fn tanpa_gambar_tampilkan_petunjuk() {
        let r = run(None, None);
        assert!(body(&r).contains("ᴍᴇᴋᴀʜ sᴛʏʟᴇ"));
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
