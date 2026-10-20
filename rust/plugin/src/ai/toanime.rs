//! `plugins/ai/toanime.js` — ubah foto menjadi gaya anime/Ghibli Studio lewat
//! scraper `seaart::live3d` (`oktz_scraper::seaart::live3d`).
//!
//! Beda urutan dengan `toisland`/`tocermin`: JS mengunduh gambar DULU, baru
//! `m.react('🕕')` — jadi jalur gagal-unduh hanya punya reaksi `❌`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const PROMPT: &str = "Transform this image into Studio Ghibli anime style. \n\
Make the characters look like they belong in a Ghibli movie with soft colors, \n\
detailed backgrounds, expressive eyes, and that signature warm, magical atmosphere. \n\
Keep the original composition but apply the distinct Ghibli artistic style with \n\
watercolor-like textures and dreamy lighting.";

/// Kartu pakai saat tidak ada gambar (`toanime.js:26-34`).
pub(crate) fn help(prefix: &str) -> String {
    format!("🎨 *ᴛᴏ ᴀɴɪᴍᴇ*\n\n> Kirim/reply gambar untuk diubah ke gaya anime\n\n`{prefix}toanime`")
}

/// `m.quoted.download()` didahulukan, lalu `m.download()` — urutan JS.
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
    name = "toanime",
    alias = ["anime", "animefy"],
    category = "ai",
    description = "Ubah foto menjadi gaya anime/Ghibli Studio",
    usage = ".toanime (reply/kirim gambar)",
    example = ".toanime",
    cooldown = 60,
    energi = 3,
    is_owner = false,
    is_premium = true,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn toanime(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| matches!(q.kind, oktz_plugin_api::QuotedKind::Image));

    if !is_image {
        return Reply::text(help(&ctx.prefix));
    }

    let Some(buffer) = ambil_buffer(&ctx) else {
        return Reply::sequence(vec![
            Reply::None.with_reaction("❌"),
            Reply::text("❌ Gagal mendownload gambar"),
        ]);
    };

    let jam = || Reply::None.with_reaction("🕕");
    match oktz_scraper::seaart::live3d(ctx.http.as_ref(), &buffer, PROMPT) {
        Ok(gambar) => Reply::sequence(vec![
            jam(),
            Reply::None.with_reaction("✅"),
            Reply::image(MediaSource::bytes(gambar), String::new()),
        ]),
        Err(_) => Reply::sequence(vec![
            jam(),
            Reply::None.with_reaction("☢"),
            Reply::text(crate::common::error_template(&ctx)),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::uji::{HttpRangkaian, MediaStub, alur_live3d, reaksi, teks};
    use oktz_plugin_api::{
        IncomingKind, IncomingMedia, MediaError, QuotedKind, QuotedMessage,
    };
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";
    const GAMBAR: &[u8] = b"HASIL";

    fn ctx_of(
        http: Arc<HttpRangkaian>,
        media: Option<IncomingKind>,
        quoted_image: bool,
        unduh: Result<Vec<u8>, MediaError>,
    ) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "toanime".into(),
            media: media.map(|kind| IncomingMedia {
                kind,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            quoted: quoted_image.then(|| QuotedMessage {
                id: "Q1".into(),
                sender: ME.into(),
                chat: ME.into(),
                text: String::new(),
                kind: QuotedKind::Image,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            media_fetch: Arc::new(MediaStub(unduh)),
            http,
            ..Ctx::default()
        }
    }

    fn gambar_ok() -> Result<Vec<u8>, MediaError> {
        Ok(vec![1, 2, 3])
    }

    #[test]
    fn bukan_gambar_menghasilkan_kartu_pakai() {
        let r = toanime(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Video),
            false,
            gambar_ok(),
        ));
        assert_eq!(teks(&r), help("."));
        assert!(reaksi(&r).is_empty());
    }

    #[test]
    fn gagal_unduh_tanpa_reaksi_jam_karena_react_nya_belakangan() {
        // toanime.js:39-49 — unduh gagal → `m.react('❌')`; `🕕` baru setelahnya.
        let r = toanime(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Image),
            false,
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        ));
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaksi(&r), ["❌"]);
    }

    #[test]
    fn quoted_image_dilayani_dan_sukses_membawa_jam_lalu_centang() {
        let http = HttpRangkaian::urutan(alur_live3d(GAMBAR));
        let r = toanime(ctx_of(http.clone(), None, true, gambar_ok()));

        assert_eq!(reaksi(&r), ["🕕", "✅"]);
        let isi = r.without_mutations();
        let Reply::Sequence(bagian) = isi else {
            panic!("harus sequence");
        };
        let gambar = bagian
            .iter()
            .find_map(|b| match b.without_mutations() {
                Reply::Image { media, .. } => media.clone(),
                _ => None,
            })
            .expect("harus ada gambar");
        match gambar {
            MediaSource::Bytes { data, .. } => assert_eq!(&*data, GAMBAR),
            lain => panic!("harus byte, dapat {lain:?}"),
        }
        assert_eq!(http.terlihat().len(), 5);
    }

    #[test]
    fn provider_menolak_jadi_reaksi_sirine_dan_template_galat() {
        let r = toanime(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Image),
            false,
            gambar_ok(),
        ));
        assert_eq!(reaksi(&r), ["🕕", "☢"]);
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "toanime"));
    }
}
