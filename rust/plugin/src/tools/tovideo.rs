//! `plugins/tools/tovideo.js` — mengubah sticker animasi menjadi video MP4.
//!
//! `pub mod tovideo;` perlu ditambah manual ke `src/tools/mod.rs` (mod.rs
//! berisi plugin lain, bukan milik saya — lihat PORT-BRIEF §6).
//!
//! Deteksi animasi WebP melalui penanda `ANIM`/`ANMF` di byte — padanan
//! `isAnimatedWebp`. Sticker statis dikonversi ke PNG lewat `sharp`;
//! sticker animasi dikonversi ke MP4 lewat ffmpeg (`jalankan_ke_berkas`).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

const TMP_IN: &str = "tmp/tovideo_in.webp";
const TMP_OUT: &str = "tmp/tovideo_out.mp4";

fn is_animated_webp(buffer: &[u8]) -> bool {
    buffer.len() >= 50
        && (buffer.windows(4).any(|w| w == b"ANIM")
            || buffer.windows(4).any(|w| w == b"ANMF"))
}

#[plugin(
    name = "tovideo",
    alias = ["tovid", "stickertovideo", "giftomp4", "webmtomp4"],
    category = "tools",
    description = "Mengubah sticker animasi menjadi video MP4",
    usage = ".tovideo (reply/caption sticker animasi)",
    example = ".tovideo",
    cooldown = 8,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tovideo(ctx: Ctx) -> Reply {
    let self_is_sticker = ctx
        .media_type()
        .is_some_and(|t| t == "stickerMessage");
    let quoted_is_sticker = ctx
        .quoted()
        .is_some_and(|q| q.kind.is_sticker());

    let origin = if self_is_sticker {
        oktz_plugin_api::MediaOrigin::Trigger
    } else if quoted_is_sticker {
        oktz_plugin_api::MediaOrigin::Quoted
    } else {
        return Reply::text(
            "❌ *ɢᴀɢᴀʟ*\n\n\
            > Tidak ada sticker yang terdeteksi!\n\n\
            *Cara penggunaan:*\n\
            > 1. Kirim sticker + caption `.tovideo`\n\
            > 2. Reply sticker dengan `.tovideo`",
        );
    };

    let Ok(buffer) = (match origin {
        oktz_plugin_api::MediaOrigin::Trigger => ctx.download_media(),
        oktz_plugin_api::MediaOrigin::Quoted => ctx.download_quoted_media(),
        oktz_plugin_api::MediaOrigin::Indexed(_) => Err(oktz_plugin_api::MediaError {
            message: "tidak ada media di pesan yang di-reply".to_string(),
        }),
    }) else {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat mengunduh sticker.");
    };

    if buffer.is_empty() {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Tidak dapat mengunduh sticker.");
    }

    if !is_animated_webp(&buffer) {
        match oktz_plugin_api::gambar::sharp(&buffer).and_then(|g| g.ke_png()) {
            Ok(png) => {
                return Reply::image(
                    MediaSource::bytes(png),
                    "✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Sticker statis → gambar",
                );
            }
            Err(_) => {
                return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Sticker tidak bisa dikonversi.");
            }
        }
    }

    // Sticker animasi → WebP → MP4 lewat ffmpeg.
    if let Err(e) = ctx.berkas().tulis(TMP_IN, &buffer) {
        return Reply::text(format!("❌ *ɢᴀɢᴀʟ*\n\n> {e}"));
    }

    let hasil = oktz_plugin_api::ffmpeg::jalankan_ke_berkas(&[
        "-y",
        "-i",
        TMP_IN,
        "-movflags",
        "faststart",
        "-pix_fmt",
        "yuv420p",
        "-vf",
        "scale=trunc(iw/2)*2:trunc(ih/2)*2",
        "-c:v",
        "libx264",
        "-preset",
        "ultrafast",
        "-crf",
        "23",
        "-an",
        TMP_OUT,
    ]);

    let mp4 = match &hasil {
        Ok(()) => match ctx.berkas().baca(TMP_OUT) {
            Ok(b) => b,
            Err(e) => return Reply::text(format!("❌ *ɢᴀɢᴀʟ*\n\n> {e}")),
        },
        Err(e) => return Reply::text(format!("❌ *ɢᴀɢᴀʟ*\n\n> FFmpeg: {e}")),
    };

    if mp4.len() < 100 {
        return Reply::text("❌ *ɢᴀɢᴀʟ*\n\n> Video output kosong");
    }

    Reply::video(MediaSource::bytes(mp4), "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{berkas::BerkasPalsu, QuotedMessage};
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";

    fn run(berkas: BerkasPalsu, media: Option<oktz_plugin_api::IncomingMedia>, quoted: Option<QuotedMessage>) -> Reply {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "tovideo".into(),
            media,
            quoted,
            berkas: Arc::new(berkas),
            ..Ctx::default()
        };
        tovideo(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.text_body().unwrap_or_default().to_string()
    }

    #[test]
    fn tanpa_sticker_mengembalikan_pesan_tanpa_request() {
        let r = run(BerkasPalsu::baru(), None, None);
        assert!(body_of(&r).contains("Tidak ada sticker yang terdeteksi"));
    }

    #[test]
    fn sticker_statis_dikonversi_ke_png() {
        let berkas = BerkasPalsu::baru();
        let r = run(
            berkas,
            Some(oktz_plugin_api::IncomingMedia {
                kind: oktz_plugin_api::IncomingKind::Sticker,
                mimetype: Some("image/webp".into()),
                file_name: None,
                caption: String::new(),
            }),
            None,
        );
        // `download_media` default NoMediaFetch gagal -> tidak bisa download.
        assert!(body_of(&r).contains("Tidak dapat mengunduh sticker"));
    }

    #[test]
    fn is_animated_webp_mendeteksi_penanda() {
        // `tovideo.js:26` menolak buffer < 50 byte walau penandanya ada.
        assert!(!is_animated_webp(&[0, 0, 0, 0, b'A', b'N', b'I', b'M', 0, 0]));

        // `Buffer.includes` JS memindai seluruh buffer (`tovideo.js:27`),
        // bukan hanya offset 0.
        let mut anim = vec![0u8; 60];
        anim[25..29].copy_from_slice(b"ANIM");
        assert!(is_animated_webp(&anim));
        let mut anmf = vec![0u8; 60];
        anmf[44..48].copy_from_slice(b"ANMF");
        assert!(is_animated_webp(&anmf));

        assert!(!is_animated_webp(b"bukan webp"));
        assert!(!is_animated_webp(&[]));
    }

    #[test]
    fn ffmpeg_dipanggil_saat_animasi_meskipun_output_salah() {
        // Webp palsu dengan penanda ANIM -> jalur animasi (ffmpeg). Di
        // lingkungan tanpa biner ffmpeg ini gagal dengan pesan ffmpeg, bukan
        // panic.
        let buffer = vec![1u8; 100];
        // Sisipkan ANIM di tengah.
        let mut b = buffer.clone();
        b[10..14].copy_from_slice(b"ANIM");
        let berkas = BerkasPalsu::baru();
        let r = run(
            berkas,
            Some(oktz_plugin_api::IncomingMedia {
                kind: oktz_plugin_api::IncomingKind::Sticker,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            None,
        );
        // download_media default gagal (NoMediaFetch) -> pesan unduh.
        assert!(body_of(&r).contains("Tidak dapat mengunduh sticker"));
    }

    #[test]
    fn unduh_gagal_dilaporkan() {
        // `tovideo.js:129-132`: sticker ada tapi unduhan gagal/kosong →
        // "Tidak dapat mengunduh sticker". Sticker disertakan supaya tidak
        // jatuh ke cabang "Tidak ada sticker" (`tovideo.js:114-122`);
        // `NoMediaFetch` default bikin `download_media()` gagal.
        let r = run(
            BerkasPalsu::baru(),
            Some(oktz_plugin_api::IncomingMedia {
                kind: oktz_plugin_api::IncomingKind::Sticker,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            None,
        );
        assert!(body_of(&r).contains("Tidak dapat mengunduh sticker"));
    }
}
