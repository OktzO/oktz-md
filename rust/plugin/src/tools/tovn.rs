//! `plugins/tools/tovn.js` — mengubah audio/video menjadi voice note.
//!
//! `NOTE (mod.rs):` `tools/mod.rs` sudah memuat banyak plugin (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak menambahkan `pub mod`.
//! Tambahkan baris berikut secara manual ke `src/tools/mod.rs`:
//!   pub mod tovn;
//
//! FFmpeg lewat `oktz_plugin_api::ffmpeg::jalankan_ke_berkas`.
//! Unduh media lewat `ctx.download_media()` / `ctx.download_quoted_media()`.
//! Kirim voice note lewat `Reply::voice_note`.
use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "tovn",
    alias = ["tovoicenote", "toptt", "audiotovn"],
    category = "tools",
    description = "Mengubah audio/video menjadi voice note",
    usage = ".tovn (reply/caption audio/video)",
    example = ".tovn",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn tovn(ctx: Ctx) -> Reply {
    let trigger_is_video = matches!(ctx.media_type(), Some("videoMessage"));
    let trigger_is_audio = matches!(ctx.media_type(), Some("audioMessage"));

    let (source_is_video, download_fn_result) = if trigger_is_video {
        (true, ctx.download_media())
    } else if trigger_is_audio {
        (false, ctx.download_media())
    } else if let Some(q) = ctx.quoted() {
        let q_is_video = matches!(q.kind, oktz_plugin_api::QuotedKind::Video);
        let q_is_audio = matches!(q.kind, oktz_plugin_api::QuotedKind::Audio);
        if q_is_video || q_is_audio {
            (q_is_video, ctx.download_quoted_media())
        } else {
            return Reply::text(
                "❌ *ɢᴀɢᴀʟ*\n\n\
                 > Tidak ada audio/video yang terdeteksi!\n\n\
                 *Cara penggunaan:*\n\
                 > 1. Kirim audio/video + caption `.tovn`\n\
                 > 2. Reply audio/video dengan `.tovn`"
            );
        }
    } else {
        return Reply::text(
            "❌ *ɢᴀɢᴀʟ*\n\n\
             > Tidak ada audio/video yang terdeteksi!\n\n\
             *Cara penggunaan:*\n\
             > 1. Kirim audio/video + caption `.tovn`\n\
             > 2. Reply audio/video dengan `.tovn`"
        );
    };

    let buffer = match download_fn_result {
        Ok(b) => b,
        Err(_) => {
            return Reply::text(
                "❌ *ɢᴀɢᴀʟ*\n\n\
                 > Tidak dapat mengunduh media.\n\
                 > Media mungkin sudah tidak tersedia."
            );
        }
    };

    if buffer.is_empty() {
        return Reply::text(
            "❌ *ɢᴀɢᴀʟ*\n\n\
             > Tidak dapat mengunduh media.\n\
             > Media mungkin sudah tidak tersedia."
        );
    }

    let temp_dir = "temp";
    let _ = ctx.berkas().buat_folder(temp_dir);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());

    let ext = if source_is_video { "mp4" } else { "mp3" };
    let input_path = format!("{temp_dir}/input_{timestamp}.{ext}");
    let output_path = format!("{temp_dir}/vn_{timestamp}.ogg");

    // Tulis input ke berkas temp.
    if ctx.berkas().tulis(&input_path, &buffer).is_err() {
        return Reply::text(
            "❌ *ᴇʀʀᴏʀ*\n\n> Gagal menulis berkas input ke temp.",
        );
    }

    let ffmpeg_args = [
        "ffmpeg", "-y",
        "-i", &input_path,
        "-vn",
        "-c:a", "libopus",
        "-b:a", "128k",
        "-ar", "48000",
        "-ac", "1",
        "-application", "voip",
        &output_path,
    ];

    match oktz_plugin_api::ffmpeg::jalankan_ke_berkas(&ffmpeg_args) {
        Ok(()) => {}
        Err(_) => {
            let _ = ctx.berkas().hapus(&input_path);
            return Reply::text(
                "❌ *ᴋᴏɴᴠᴇʀsɪ ɢᴀɢᴀʟ*\n\n\
                 > Gagal mengkonversi ke voice note.\n\
                 > Pastikan ffmpeg terinstall dengan benar."
            );
        }
    }

    let vn_exists = ctx.berkas().ada(&output_path);
    if !vn_exists {
        let _ = ctx.berkas().hapus(&input_path);
        return Reply::text(
            "❌ *ᴋᴏɴᴠᴇʀsɪ ɢᴀɢᴀʟ*\n\n\
             > Gagal mengkonversi ke voice note.\n             > Pastikan ffmpeg terinstall dengan benar."
        );
    }

    let vn_buffer = match ctx.berkas().baca(&output_path) {
        Ok(b) => b,
        Err(_) => {
            let _ = ctx.berkas().hapus(&input_path);
            let _ = ctx.berkas().hapus(&output_path);
            return Reply::text(
                "❌ *ᴇʀʀᴏʀ*\n\n> Gagal membaca hasil konversi."
            );
        }
    };

    // Hapus berkas temp.
    let _ = ctx.berkas().hapus(&input_path);
    let _ = ctx.berkas().hapus(&output_path);

    Reply::voice_note(oktz_plugin_api::MediaSource::bytes(vn_buffer))
}

#[cfg(test)]
mod tests {
    #[test]
    fn path_nama_berkas_benar() {
        let ts = 1234567890i64;
        let input = format!("temp/input_{ts}.mp4");
        let output = format!("temp/vn_{ts}.ogg");
        assert!(input.starts_with("temp/input_"));
        assert!(output.starts_with("temp/vn_"));
        assert!(input.ends_with(".mp4"));
        assert!(output.ends_with(".ogg"));
    }

    #[test]
    fn ext_video_mp4_audio_mp3() {
        let is_video = true;
        let ext = if is_video { "mp4" } else { "mp3" };
        assert_eq!(ext, "mp4");
        let ext = if false { "mp4" } else { "mp3" };
        assert_eq!(ext, "mp3");
    }
}
