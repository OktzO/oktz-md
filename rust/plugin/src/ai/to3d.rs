//! `plugins/ai/to3d.js` — ubah foto menjadi gaya 3D render.
//!
//! Catatan: `mod.rs` kategori `ai` sudah memuat plugin lain yang bukan
//! milik tugas ini, jadi baris `pub mod to3d;` perlu ditambahkan manual
//! di `rust/crates/oktz-plugins/src/ai/mod.rs`.
//!
//! `live3d` dari `oktz_scraper::seaart` menerima `&dyn HttpGet`, buffer
//! gambar, dan prompt; mengembalikan `Result<Vec<u8>, HttpError>`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const PROMPT: &str = "Transform this image into a high-quality 3D rendered style like Pixar or DreamWorks CGI. \
Apply realistic lighting, smooth textures, and that polished 3D animated movie look. \
Keep the original composition but make it look like a frame from a modern 3D animated film \
with subsurface scattering on skin, detailed hair, and cinematic lighting.";

fn is_image_input(ctx: &Ctx) -> bool {
    ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .map(|q| q.kind == oktz_plugin_api::QuotedKind::Image)
            .unwrap_or(false)
}

#[plugin(
    name = "to3d",
    alias = ["3d", "3dfy", "to3dmodel"],
    category = "ai",
    description = "Ubah foto menjadi gaya 3D render",
    usage = ".to3d (reply/kirim gambar)",
    example = ".to3d",
    cooldown = 60,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn to3d(ctx: Ctx) -> Reply {
    if !is_image_input(&ctx) {
        return Reply::text(
            "🎮 *ᴛᴏ 3ᴅ*\n\n\
             > Kirim/reply gambar untuk diubah ke gaya 3D\n\n\
             `.to3d`",
        );
    }

    let buffer = ctx
        .download_quoted_media()
        .or_else(|_| ctx.download_media());

    let Ok(buffer) = buffer else {
        return Reply::text("❌ Gagal mendownload gambar");
    };

    let result = oktz_scraper::seaart::live3d(ctx.http.as_ref(), &buffer, PROMPT);

    let Ok(image_bytes) = result else {
        return Reply::text("❌ Gagal mengubah gambar ke gaya 3D");
    };

    Reply::image(MediaSource::bytes(image_bytes), "")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Ctx {
        Ctx {
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "to3d".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("to3d").unwrap();
        assert_eq!(p.meta.name, "to3d");
        assert_eq!(p.meta.alias, ["3d", "3dfy", "to3dmodel"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 3);
        assert!(p.meta.is_premium);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn setiap_alias_menunjuk_ke_plugin_ini() {
        for a in ["3d", "3dfy", "to3dmodel"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "to3d");
        }
    }

    #[test]
    fn bukan_gambar_mengembalikan_bantuan() {
        let r = to3d(ctx());
        assert!(text_of(&r).contains("🎮 *ᴛᴏ 3ᴅ*"));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn download_gagal_mengembalikan_error() {
        // Media gambar ada, tapi `Ctx::default()` punya NoMediaFetch.
        let ctx = Ctx {
            media: Some(oktz_plugin_api::IncomingMedia {
                kind: oktz_plugin_api::IncomingKind::Image,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            ..ctx()
        };
        let r = to3d(ctx);
        assert!(text_of(&r).contains("Gagal mendownload gambar"));
    }
}
