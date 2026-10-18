//! `plugins/ai/tojapanese.js` — ubah gambar ke style Japanese.
//!
//! `ponytail:` `uploadImage` (`src/lib/uploader.js`) dan
//! `applyImageEffect` (`src/lib/effect-api.js`) tidak tersedia di
//! `Ctx`/`Reply`; handler untuk gambar diakhiri `error_template` + `☢`
//! (pola `swm.rs`). Pemeriksaan gambar dan teks pakai di-port agar bisa diuji.
//! Saat unggah + efek tersedia, tambahkan `pub mod tojapanese;` di
//! `src/ai/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, IncomingKind, Reply, plugin};

/// Kartu pakai / penolakan bukan gambar (`tojapanese.js:24`).
pub(crate) fn pakai(prefix: &str) -> String {
    format!(
        "🎌 *ᴊᴀᴘᴀɴᴇsᴇ sᴛʏʟᴇ*\n\n> Kirim/reply gambar untuk diubah ke style Japanese\n\n`{prefix}tojapanese`"
    )
}

/// `isImage` (`tojapanese.js:21`).
pub(crate) fn is_image(ctx: &Ctx) -> bool {
    ctx.is_media_kind(IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image())
}

/// Unduh gambar: quoted dulu, baru trigger (`tojapanese.js:31-35`).
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
    name = "tojapanese",
    alias = ["japanese", "japanesestyle"],
    category = "ai",
    description = "Ubah gambar ke style Japanese",
    usage = ".tojapanese (reply gambar)",
    example = ".tojapanese",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tojapanese(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(pakai(&ctx.prefix));
    }

    let Some(_buffer) = unduh_media(&ctx) else {
        return Reply::text("❌ Gagal mendownload gambar");
    };

    // `uploadImage` + `applyImageEffect('tojapanese', ...)` tidak tersedia —
    // diakhiri error_template seperti swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
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
            command: "tojapanese".into(),
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
        tojapanese(ctx)
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
        let p = oktz_plugin_api::find("tojapanese").unwrap();
        assert_eq!(p.meta.name, "tojapanese");
        assert_eq!(p.meta.alias, ["japanese", "japanesestyle"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn bukan_gambar_tampilkan_pakai() {
        let r = run(false, false, true);
        assert_eq!(teks(&r), pakai("."));
        assert_eq!(reaksi(&r), None);
    }

    #[test]
    fn quoted_gambar_diprioritaskan() {
        let r = run(false, true, true);
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "tojapanese")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn trigger_gambar_dipakai_unduh() {
        let r = run(true, false, true);
        assert_eq!(
            teks(&r),
            crate::common::error_template(".", "tojapanese")
        );
        assert_eq!(reaksi(&r), Some("☢"));
    }

    #[test]
    fn gagal_unduh_teks_tanpa_reaksi() {
        let r = run(true, false, false);
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaksi(&r), None);
    }
}
