//! `plugins/owner/ganti-store.js` — ganti gambar store.jpg (thumbnail store).
//!
//! `ponytail:` `updateAssetUrl` (`src/lib/uploader.js`) untuk unggah dan
//! mendapatkan URL baru tidak tersedia di `Ctx`/`Reply`; setelah unduh media
//! berhasil, handler diakhiri `error_template` + `☢` (pola `swm.rs`).
//! Unduh media itu sendiri di-port melalui `Ctx::download_media` /
//! `download_quoted_media`.
//!
//! Saat `updateAssetUrl` tersedia, tambahkan `pub mod ganti_store;` di
//! `src/owner/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, IncomingKind, Reply, plugin};

/// Kartu pakai / penolakan bukan gambar (`ganti-store.js:22-24`).
pub(crate) fn pakai(file: &str, _prefix: &str) -> String {
    format!(
        "🖼️ *ɢᴀɴᴛɪ sᴛᴏʀᴇ.ᴊᴘɢ*\n\n> Kirim/reply gambar untuk mengganti\n> File: assets/image/{file}"
    )
}

/// `isImage` (`ganti-store.js:20`).
pub(crate) fn is_image(ctx: &Ctx) -> bool {
    ctx.is_media_kind(IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image())
}

/// Unduh buffer: quoted dulu, baru trigger (`ganti-store.js:28-32`).
pub(crate) fn unduh_media(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.quoted().is_some() {
        ctx.download_quoted_media().ok()
    } else if ctx.is_media() {
        ctx.download_media().ok()
    } else {
        None
    }
}

#[plugin(
    name = "ganti-store.jpg",
    alias = ["gantistore", "setstore"],
    category = "owner",
    description = "Ganti gambar store.jpg (thumbnail store)",
    usage = ".ganti-store.jpg (reply/kirim gambar)",
    example = ".ganti-store.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_store(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(pakai("store.jpeg", &ctx.prefix));
    }

    let Some(_buffer) = unduh_media(&ctx) else {
        return Reply::text("❌ Gagal mendownload gambar");
    };

    // `updateAssetUrl('store', buffer, 'store.jpeg')` tidak tersedia —
    // diakhiri error_template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{IncomingMedia, QuotedKind, QuotedMessage};
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct StubFetch {
        ok: bool,
        bytes: Vec<u8>,
    }

    impl oktz_plugin_api::MediaFetch for StubFetch {
        fn download(&self, _: oktz_plugin_api::MediaOrigin) -> Result<Vec<u8>, oktz_plugin_api::MediaError> {
            if self.ok {
                Ok(self.bytes.clone())
            } else {
                Err(oktz_plugin_api::MediaError {
                    message: "gagal".into(),
                })
            }
        }
    }

    fn run(is_image: bool, quoted_image: bool, download_ok: bool) -> Reply {
        let media = if is_image {
            Some(IncomingMedia {
                kind: IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            })
        } else {
            None
        };
        let quoted = if quoted_image {
            Some(QuotedMessage {
                id: "Q1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: String::new(),
                kind: QuotedKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            })
        } else {
            None
        };
        let ctx = Ctx {
            prefix: ".".into(),
            command: "ganti-store.jpg".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            media,
            quoted,
            media_fetch: Arc::new(StubFetch {
                ok: download_ok,
                bytes: b"IMG".to_vec(),
            }),
            ..Ctx::default()
        };
        ganti_store(ctx)
    }

    fn teks(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("ganti-store.jpg").unwrap();
        assert_eq!(p.meta.name, "ganti-store.jpg");
        assert_eq!(p.meta.alias, ["gantistore", "setstore"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn bukan_gambar_tampilkan_pakai() {
        let r = run(false, false, true);
        assert_eq!(teks(&r), pakai("store.jpeg", "."));
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn quoted_gambar_dipakai_unduh() {
        let r = run(false, true, true);
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "ganti-store.jpg"));
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn trigger_gambar_dipakai_unduh() {
        let r = run(true, false, true);
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "ganti-store.jpg"));
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn gagal_unduh_teks_tanpa_reaksi() {
        let r = run(true, false, false);
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaksi(&r), None);
    }
}
