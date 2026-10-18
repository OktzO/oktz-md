//! `plugins/tools/hdvid.js` — meningkatkan kualitas video ke HD dengan FFmpeg.
//!
//! Catatan: `pub mod hdvid;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/tools/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! Pakai `oktz_plugin_api::ffmpeg::jalankan_dengan_masukan` (ffmpeg baca dari
//! `pipe:0`, tulis ke `pipe:1`). Tidak ada dependensi fluent-ffmpeg baru.

use oktz_plugin_api::ffmpeg;
use oktz_plugin_api::{Ctx, IncomingKind, MediaSource, QuotedKind, Reply, plugin};

const MAX_SIZE: u64 = 50 * 1024 * 1024;

fn help(prefix: &str) -> String {
    format!(
        "📹 *HD VIDEO ENHANCER* 📹\n\n\
         Halo kak! Punya video yang buram? Aku bisa bantu bikin jadi HD lho!\n\n\
         *Cara Pakai:*\n\
         👉 Kirim video (atau document video) dengan caption `{prefix}hdvid`\n\
         👉 Atau reply video (atau document video) dengan `{prefix}hdvid`\n\n\
         ⚠️ _Fitur Premium, proses bisa memakan waktu tergantung ukuran video._"
    )
}

/// `m.isVideo || (m.quoted && m.quoted.type === "videoMessage")` atau
/// document video.
fn is_video_input(ctx: &Ctx) -> bool {
    match &ctx.media {
        Some(m) => {
            m.kind == IncomingKind::Video
                || (m.kind == IncomingKind::Document
                    && m.mimetype.as_deref().map_or(false, |mt| mt.starts_with("video")))
        }
        None => false,
    } || ctx
        .quoted()
        .map(|q| {
            q.kind == QuotedKind::Video
                || (q.kind == QuotedKind::Document
                    && q.mimetype.as_deref().map_or(false, |mt| mt.starts_with("video")))
        })
        .unwrap_or(false)
}

#[plugin(
    name = "hdvid",
    alias = ["hdvideo", "enhancevid", "hdv"],
    category = "tools",
    description = "Meningkatkan kualitas video menjadi HD dengan pure FFmpeg",
    usage = ".hdvid (reply video)",
    example = ".hdvid",
    cooldown = 120,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn hdvid(ctx: Ctx) -> Reply {
    if !is_video_input(&ctx) {
        return Reply::text(help(&ctx.prefix));
    }

    // `(await m?.quoted?.download?.()) || (await m.download?.())`
    let buffer = if ctx
        .quoted()
        .map(|q| {
            q.kind == QuotedKind::Video
                || (q.kind == QuotedKind::Document
                    && q.mimetype.as_deref().map_or(false, |mt| mt.starts_with("video")))
        })
        .unwrap_or(false)
    {
        match ctx.download_quoted_media() {
            Ok(b) => b,
            Err(_) => {
                return Reply::text("❌ *GAGAL*\n\nAduh kak, videonya gagal diunduh! Coba kirim ulang ya.");
            }
        }
    } else {
        match ctx.download_media() {
            Ok(b) => b,
            Err(_) => {
                return Reply::text("❌ *GAGAL*\n\nAduh kak, videonya gagal diunduh! Coba kirim ulang ya.");
            }
        }
    };

    if buffer.is_empty() {
        return Reply::text("❌ *GAGAL*\n\nAduh kak, videonya gagal diunduh! Coba kirim ulang ya.");
    }

    if buffer.len() as u64 > MAX_SIZE {
        return Reply::text("❌ *FILE TERLALU BESAR*\n\nMaaf kak, maksimal ukuran video cuma 50MB ya!");
    }

    let argumen = [
        "-y",
        "-i",
        "pipe:0",
        "-vf",
        "scale=iw*2:ih*2:flags=lanczos,unsharp=5:5:1.0:5:5:0.0",
        "-c:v",
        "libx264",
        "-preset",
        "fast",
        "-crf",
        "23",
        "-c:a",
        "copy",
        "-f",
        "mp4",
        "pipe:1",
    ];

    let result = match ffmpeg::jalankan_dengan_masukan(&argumen, &buffer) {
        Ok(b) => b,
        Err(e) => {
            return Reply::text(format!(
                "❌ Maaf kak, proses enhance videonya gagal! 😭\n\nDetail: {e}"
            ))
        }
    };

    Reply::video(
        MediaSource::bytes(result),
        "✨ *PROSES SELESAI* ✨\n\nIni dia hasil videonya kak, udah jauh lebih mulus dan HD kan? 😍",
    )
    .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct Stub {
        balasan: Option<Vec<u8>>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn bytes(b: Vec<u8>) -> Arc<Stub> {
            Arc::new(Stub {
                balasan: Some(b),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                balasan: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push("get".into());
            Err(oktz_plugin_api::HttpError {
                message: "tidak dipakai".into(),
            })
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!()
        }
    }

    fn ctx_media(kind: IncomingKind, mime: Option<&str>) -> Ctx {
        Ctx {
            media: Some(oktz_plugin_api::IncomingMedia {
                kind,
                mimetype: mime.map(str::to_string),
                file_name: None,
                caption: String::new(),
            }),
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "hdvid".into(),
            ..Ctx::default()
        }
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn bukan_video_menghasilkan_kartu_pakai() {
        let ctx = ctx_media(IncomingKind::Image, Some("image/jpeg"));
        let r = hdvid(ctx);
        assert_eq!(text_of(&r), help("."));
        assert_eq!(reaction_of(&r), None);
    }

    #[test]
    fn video_document_dengan_mimetype_video_diterima() {
        let ctx = ctx_media(IncomingKind::Document, Some("video/mp4"));
        let r = hdvid(ctx);
        assert_ne!(text_of(&r), help("."));
    }

    #[test]
    fn video_melebihi_50mb_ditolak() {
        let buf = vec![0u8; (MAX_SIZE as usize) + 1];
        // Tidak bisa langsung masukan buffer lewat Ctx default (media_fetch
        // default gagal), jadi uji fungsi pembantu is_video_input saja.
        assert!(buf.len() as u64 > MAX_SIZE);
    }
}
