//! `plugins/ai/tocermin.js` — efek cermin lewat scraper
//! `seaart::live3d` (`oktz_scraper::seaart::live3d`).
//!
//! `m.react('🕕')` jalan SEBELUM unduh (`tocermin.js:37`), jadi jalur
//! gagal-unduh membawa reaksi `🕕` lalu `❌`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const PROMPT: &str = "Create a mirror reflection effect of this image. \n\
Add a realistic reflection as if the subject is in front of a mirror or reflective surface. \n\
Ensure symmetry, smooth reflection blending, realistic lighting and shadows. \n\
Keep the original identity and details, high quality, photorealistic.";

/// Kartu pakai saat tidak ada gambar (`tocermin.js:26-33`).
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🪞 *ᴛᴏ ᴄᴇʀᴍɪɴ*\n\n> Kirim/reply gambar untuk efek cermin\n\n`{prefix}tocermin`"
    )
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
    name = "tocermin",
    alias = ["mirror", "tomirror"],
    category = "ai",
    description = "Ubah foto menjadi efek cermin (mirror reflection)",
    usage = ".tocermin (reply/kirim gambar)",
    example = ".tocermin",
    cooldown = 60,
    energi = 3,
    is_owner = false,
    is_premium = true,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tocermin(ctx: Ctx) -> Reply {
    let is_image = ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| matches!(q.kind, oktz_plugin_api::QuotedKind::Image));

    if !is_image {
        return Reply::text(help(&ctx.prefix));
    }

    let jam = || Reply::None.with_reaction("🕕");

    let Some(buffer) = ambil_buffer(&ctx) else {
        return Reply::sequence(vec![
            jam(),
            Reply::None.with_reaction("❌"),
            Reply::text("❌ Gagal mendownload gambar"),
        ]);
    };

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
    use oktz_plugin_api::{IncomingKind, IncomingMedia, MediaError, QuotedKind, QuotedMessage};
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

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
            command: "tocermin".into(),
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
            timestamp: 0,
        }),
            media_fetch: Arc::new(MediaStub(unduh)),
            http,
            ..Ctx::default()
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("tocermin").unwrap();
        assert_eq!(p.meta.name, "tocermin");
        assert_eq!(p.meta.alias, ["mirror", "tomirror"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Ubah foto menjadi efek cermin (mirror reflection)"
        );
        assert_eq!(p.meta.usage, ".tocermin (reply/kirim gambar)");
        assert_eq!(p.meta.example, ".tocermin");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 3);
        assert!(p.meta.is_premium);
        assert!(!p.meta.is_owner && !p.meta.is_group);
    }

    #[test]
    fn bukan_gambar_menghasilkan_kartu_pakai() {
        let r = tocermin(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Video),
            false,
            Ok(vec![1]),
        ));
        assert_eq!(teks(&r), help("."));
        assert!(reaksi(&r).is_empty());
    }

    #[test]
    fn gagal_unduh_membawa_jam_lalu_silang() {
        let r = tocermin(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Image),
            false,
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        ));
        assert_eq!(teks(&r), "❌ Gagal mendownload gambar");
        assert_eq!(reaksi(&r), ["🕕", "❌"]);
    }

    #[test]
    fn sukses_mengirim_gambar_hasil_provider() {
        let gambar = b" CERMIN";
        let http = HttpRangkaian::urutan(alur_live3d(gambar));
        let r = tocermin(ctx_of(http.clone(), Some(IncomingKind::Image), false, Ok(vec![1, 2])));
        assert_eq!(reaksi(&r), ["🕕", "✅"]);
        assert!(r.has_media());
        assert_eq!(http.terlihat().len(), 5);
        // Prompt ikut terkirim di URL provider.
        let url = &http.terlihat()[3];
        assert!(url.contains("mirror"), "url={url}");
    }

    #[test]
    fn provider_gagal_jadi_reaksi_sirine_dan_template_galat() {
        let r = tocermin(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Image),
            false,
            Ok(vec![1]),
        ));
        assert_eq!(reaksi(&r), ["🕕", "☢"]);
        assert_eq!(
            teks(&r),
            crate::common::error_template_bawaan(".", "tocermin")
        );
    }
}
