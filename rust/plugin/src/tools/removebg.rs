//! `plugins/tools/removebg.js` — hapus background lewat dua API pixelcut lalu
//! bgninja (`oktz_scraper::removebackground`).
//!
//! `ponytail:` `pixa()` JS menormalkan gambar ke JPEG dengan `sharp` sebelum
//! dikirim (dan menyimpannya ke berkas sementara). `sharp` tidak ada di crate
//! scraper, jadi byte dari WhatsApp dikirim apa adanya — gambar WA hampir
//! selalu JPEG. Tambahkan normalisasi kalau suatu saat butuh webp/png/heic.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

/// Kartu pakai saat bukan gambar (`removebg.js:15-18`).
pub(crate) fn pakai() -> String {
    "❌ *ɢᴀᴍʙᴀʀ ᴅɪʙᴜᴛᴜʜᴋᴀɴ*\n\n> Reply atau kirim gambar dengan caption .removebg".to_string()
}

fn is_image(ctx: &Ctx) -> bool {
    ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image)
        || ctx.quoted().is_some_and(|q| q.kind.is_image())
}

#[plugin(
    name = "removebg",
    alias = ["rmbg", "nobg", "hapusbg"],
    category = "tools",
    description = "Menghapus background gambar",
    usage = ".removebg (reply gambar)",
    example = ".removebg",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn removebg(ctx: Ctx) -> Reply {
    if !is_image(&ctx) {
        return Reply::text(pakai());
    }

    let jam = || Reply::None.with_reaction("🕕");

    // `removebg.js:22-28` — gambar pemicu diperiksa DULU, baru yang di-reply
    // (kebalikan urutan `toisland`/`tocermin`).
    let unduh = if ctx.is_media_kind(oktz_plugin_api::IncomingKind::Image) {
        ctx.download_media()
    } else if ctx.quoted().is_some_and(|q| q.kind.is_image()) {
        ctx.download_quoted_media()
    } else {
        return Reply::sequence(vec![jam(), Reply::text("❌ Gagal mengunduh gambar")]);
    };

    let Ok(buffer) = unduh else {
        // JS: unduhan yang MELEMPAR ditangkap `catch` → `te()` tanpa reaksi
        // tambahan; hanya reaksi `🕕` yang sudah terkirim.
        return Reply::sequence(vec![jam(), Reply::text(crate::common::error_template(&ctx))]);
    };

    let hasil = oktz_scraper::removebackground::pixelcut_remove(ctx.http.as_ref(), &buffer)
        .or_else(|_| oktz_scraper::removebackground::bgninja_remove(ctx.http.as_ref(), &buffer));

    match hasil {
        // JS tidak pernah `m.react('✅')` di plugin ini.
        Ok(gambar) => Reply::sequence(vec![
            jam(),
            Reply::image(
                MediaSource::bytes(gambar),
                "✅ *ʙᴀᴄᴋɢʀᴏᴜɴᴅ ᴅɪʜᴀᴘᴜs*\n\n> Background gambar berhasil dihapus",
            ),
        ]),
        Err(_) => Reply::sequence(vec![jam(), Reply::text(crate::common::error_template(&ctx))]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::uji::{HttpRangkaian, MediaStub, reaksi, teks};
    use oktz_plugin_api::{IncomingKind, IncomingMedia, MediaError};
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

    fn ctx_of(
        http: Arc<HttpRangkaian>,
        media: Option<IncomingKind>,
        unduh: Result<Vec<u8>, MediaError>,
    ) -> Ctx {
        Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "removebg".into(),
            media: media.map(|kind| IncomingMedia {
                kind,
                mimetype: Some("image/jpeg".into()),
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(MediaStub(unduh)),
            http,
            ..Ctx::default()
        }
    }

    #[test]
    fn meta_sama_dengan_js() {
        let p = oktz_plugin_api::find("removebg").unwrap();
        assert_eq!(p.meta.alias, ["rmbg", "nobg", "hapusbg"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn bukan_gambar_menghasilkan_kartu_pakai_tanpa_reaksi() {
        let r = removebg(ctx_of(HttpRangkaian::offline(), Some(IncomingKind::Video), Ok(vec![1])));
        assert_eq!(teks(&r), pakai());
        assert!(reaksi(&r).is_empty());
    }

    #[test]
    fn unduhan_melempar_jadi_template_galat_hanya_dengan_reaksi_jam() {
        let r = removebg(ctx_of(
            HttpRangkaian::offline(),
            Some(IncomingKind::Image),
            Err(MediaError {
                message: "cdn mati".into(),
            }),
        ));
        assert_eq!(reaksi(&r), ["🕕"]);
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "removebg"));
    }

    #[test]
    fn pixelcut_sukses_mengirim_gambar_tanpa_reaksi_centang() {
        let http = HttpRangkaian::urutan(vec!["PNG".to_string()]);
        let r = removebg(ctx_of(http.clone(), Some(IncomingKind::Image), Ok(vec![1, 2])));
        assert_eq!(reaksi(&r), ["🕕"]);
        assert!(r.has_media());
        assert_eq!(http.terlihat().len(), 1, "bgninja tidak dipanggil");
    }

    #[test]
    fn dua_dua_gagal_jadi_template_galat() {
        let r = removebg(ctx_of(HttpRangkaian::offline(), Some(IncomingKind::Image), Ok(vec![1])));
        assert_eq!(reaksi(&r), ["🕕"]);
        assert_eq!(teks(&r), crate::common::error_template_bawaan(".", "removebg"));
    }
}
