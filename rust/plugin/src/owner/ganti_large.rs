//! `plugins/owner/ganti-large.js` — ganti tiga aset gambar besar sekaligus.
//!
//! Satu-satunya dari keluarga `ganti-*.jpg` yang TIDAK memakai
//! `updateAssetUrl`: JS menulis byte ke `assets/images/` langsung, jadi port
//! ini utuh lewat `ctx.berkas`.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Folder tujuan (`ganti-large.js:49`).
pub(crate) const FOLDER: &str = "assets/images";

/// Nama berkas yang ditimpa (`ganti-large.js:43-47`).
pub(crate) const TARGET: [&str; 3] = ["foto.jpeg", "v8.jpeg", "v10.jpeg"];

/// Kartu pakai / penolakan bukan gambar (`ganti-large.js:22-24`).
pub(crate) fn pakai() -> String {
    "🖼️ *ɢᴀɴᴛɪ ʟᴀʀɢᴇ.ᴊᴘɢ*\n\n\
     > Kirim/reply gambar untuk mengganti kumpulan foto besar (foto.jpeg, v8.jpeg, v10.jpeg) sekaligus.\n\
     > Pastikan rasio gambar sesuai dengan yang diinginkan."
        .to_string()
}

/// `isImage` (`ganti-large.js:20`).
fn is_image(ctx: &Ctx) -> bool {
    ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image())
}

/// Unduh buffer: quoted dulu, baru trigger (`ganti-large.js:28-32`).
fn unduh_media(ctx: &Ctx) -> Option<Vec<u8>> {
    if ctx.quoted().is_some_and(|q| q.is_media()) {
        return ctx.download_quoted_media().ok();
    }
    if ctx.is_media() {
        return ctx.download_media().ok();
    }
    None
}

#[plugin(
    name = "ganti-large.jpg",
    alias = ["gantilarge", "setlarge"],
    category = "owner",
    description = "Preset: Ganti gambar foto.jpeg, serta v7 hingga v11.jpeg sekaligus",
    usage = ".ganti-large.jpg (reply/kirim gambar)",
    example = ".ganti-large.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_large(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(pakai());
    }

    let jam = || Reply::None.with_reaction("🕕");

    let Some(buffer) = unduh_media(&ctx) else {
        return Reply::sequence(vec![
            jam(),
            Reply::None.with_reaction("❌"),
            Reply::text("❌ Gagal mendownload gambar"),
        ]);
    };

    for nama in TARGET {
        if ctx.berkas().tulis(&format!("{FOLDER}/{nama}"), &buffer).is_err() {
            return Reply::sequence(vec![
                jam(),
                Reply::None.with_reaction("☢"),
                Reply::text(crate::common::error_template(&ctx)),
            ]);
        }
    }

    Reply::sequence(vec![
        jam(),
        Reply::None.with_reaction("✅"),
        Reply::text(format!(
            "✅ *ʙᴇʀʜᴀsɪʟ*\n\n\
             > Gambar bundle *ganti-large.jpg* berhasil diganti secara massal.\n\
             > Mencakup: {}\n\
             > Restart bot jika gambar tidak langsung berubah.",
            TARGET.join(", ")
        )),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::uji::{MediaStub, reaksi, teks};
    use oktz_plugin_api::berkas::{Berkas, BerkasPalsu};
    use oktz_plugin_api::{IncomingKind, IncomingMedia, MediaError};
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

    fn ctx_of(berkas: Arc<BerkasPalsu>, media: Option<IncomingKind>, hasil: Result<Vec<u8>, MediaError>) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "ganti-large.jpg".into(),
            media: media.map(|kind| IncomingMedia {
                kind,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(MediaStub(hasil)),
            berkas,
            ..Ctx::default()
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("ganti-large.jpg").unwrap();
        assert_eq!(p.meta.name, "ganti-large.jpg");
        assert_eq!(p.meta.alias, ["gantilarge", "setlarge"]);
        assert_eq!(
            p.meta.description,
            "Preset: Ganti gambar foto.jpeg, serta v7 hingga v11.jpeg sekaligus"
        );
        assert!(p.meta.is_owner);
        assert_eq!(p.meta.cooldown, 5);
    }

    #[test]
    fn bukan_gambar_menghasilkan_kartu_pakai() {
        let r = ganti_large(ctx_of(
            Arc::new(BerkasPalsu::baru()),
            Some(IncomingKind::Video),
            Ok(vec![1]),
        ));
        assert_eq!(teks(&r), pakai());
        assert!(reaksi(&r).is_empty());
    }

    #[test]
    fn gagal_unduh_membawa_jam_lalu_silang() {
        let r = ganti_large(ctx_of(
            Arc::new(BerkasPalsu::baru()),
            Some(IncomingKind::Image),
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        ));
        assert_eq!(reaksi(&r), ["🕕", "❌"]);
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
    }

    #[test]
    fn tiga_aset_ditimpa_isi_yang_sama() {
        let b = Arc::new(BerkasPalsu::baru());
        let r = ganti_large(ctx_of(b.clone(), Some(IncomingKind::Image), Ok(vec![9, 8, 7])));
        assert_eq!(reaksi(&r), ["🕕", "✅"]);
        assert!(teks(&r).contains("Mencakup: foto.jpeg, v8.jpeg, v10.jpeg"));
        for nama in TARGET {
            assert_eq!(
                b.baca(&format!("{FOLDER}/{nama}")).unwrap(),
                vec![9, 8, 7],
                "{nama}"
            );
        }
    }
}
