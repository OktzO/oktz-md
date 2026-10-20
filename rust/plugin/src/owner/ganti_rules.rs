//! `plugins/owner/ganti-rules.js` — ganti gambar rules.jpeg via `updateAssetUrl`.
//!
//! `ponytail:` `updateAssetUrl` dari `src/lib/uploader.js` (unggah ke asset /
//! config realtime) tidak diekspos ke plugin di registry Rust. Unduhan media
//! tersedia, tetapi unggahan hasilnya tidak dapat dieksekusi.

use oktz_plugin_api::{Ctx, IncomingKind, QuotedKind, Reply, plugin};

/// Kartu pakai saat bukan gambar.
pub(crate) fn help() -> String {
    "🖼️ *ɢᴀɴᴛɪ ʀᴜʟᴇs.ᴊᴘɢ*\n\n> Kirim/reply gambar untuk mengganti\n> File: assets/image/rules.jpeg"
        .to_string()
}

/// Cek apakah pemicu atau quoted adalah image, sama urutan dengan JS.
fn is_image(ctx: &Ctx) -> bool {
    ctx.is_media_kind(IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| matches!(q.kind, QuotedKind::Image))
}

/// Unduh buffer media: quoted dulu, baru trigger.
fn ambil_buffer(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.quoted().is_some_and(|q| q.is_media()) {
        return ctx.download_quoted_media().ok();
    }
    if ctx.is_media() {
        return ctx.download_media().ok();
    }
    None
}

#[plugin(
    name = "ganti-rules.jpg",
    alias = ["gantirules", "setfotorules"],
    category = "owner",
    description = "Ganti gambar rules.jpg (thumbnail rules)",
    usage = ".ganti-rules.jpg (reply/kirim gambar)",
    example = ".ganti-rules.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_rules(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(help());
    }

    let Some(buffer) = ambil_buffer(&ctx) else {
        return Reply::text("❌ Gagal mendownload gambar");
    };

    let _ = buffer; // buffer didapat, tapi `updateAssetUrl` tidak tersedia.

    // `ponytail:` `updateAssetUrl('rules', buffer, 'rules.jpeg')` dan
    // pembaruan config realtime tidak tersedia di registry Rust.
    Reply::text(crate::common::error_template(&ctx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn bukan_gambar_menghasilkan_kartu_pakai() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ganti-rules.jpg".into(),
            ..Ctx::default()
        };
        let r = ganti_rules(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), help());
    }

    #[test]
    fn unduhan_gagal_menghasilkan_pesan_gagal() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ganti-rules.jpg".into(),
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(oktz_plugin_api::NoMediaFetch),
            ..Ctx::default()
        };
        let r = ganti_rules(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), "❌ Gagal mendownload gambar");
    }

    #[test]
    fn quoted_image_dilayani_sama() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ganti-rules.jpg".into(),
            quoted: Some(oktz_plugin_api::QuotedMessage {
                id: "1".into(),
                sender: ME.into(),
                chat: ME.into(),
                text: String::new(),
                kind: QuotedKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            media_fetch: Arc::new(oktz_plugin_api::NoMediaFetch),
            ..Ctx::default()
        };
        let r = ganti_rules(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), "❌ Gagal mendownload gambar");
    }
}
