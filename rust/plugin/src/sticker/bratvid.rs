//! `plugins/sticker/bratvid.js` — animated brat (simplified native port).

use std::process::{Command, Stdio};
use std::time::Duration;

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

fn render_raw(teks: &str, width: u32, height: u32) -> Vec<u8> {
    oktz_plugin_api::render_frame_raw(teks, width, height)
}

fn encode_mp4(raw: &[u8], width: u32, height: u32, fps: u32) -> Result<Vec<u8>, String> {
    let ffmpeg = oktz_plugin_api::ffmpeg::jalur_ffmpeg().unwrap_or_else(|| std::path::PathBuf::from("ffmpeg"));
    let mut child = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{width}x{height}"),
            "-r",
            &fps.to_string(),
            "-i",
            "pipe:0",
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "23",
            "-pix_fmt",
            "yuv420p",
            "-movflags",
            "+faststart",
            "-f",
            "mp4",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("gagal menjalankan ffmpeg: {e}"))?;

    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        stdin.write_all(raw).map_err(|e| format!("gagal menulis frame: {e}"))?;
    }
    drop(child.stdin.take());

    let mulai = std::time::Instant::now();
    loop {
        match child.try_wait().map_err(|e| format!("gagal menunggu ffmpeg: {e}"))? {
            Some(_) => break,
            None if mulai.elapsed() > Duration::from_secs(120) => {
                let _ = child.kill();
                return Err("ffmpeg timeout".to_string());
            }
            None => std::thread::sleep(Duration::from_millis(25)),
        }
    }
    let keluaran = child
        .wait_with_output()
        .map_err(|e| format!("gagal menunggu ffmpeg: {e}"))?;
    if !keluaran.status.success() {
        return Err(String::from_utf8_lossy(&keluaran.stderr).to_string());
    }
    Ok(keluaran.stdout)
}

pub fn render_frame_bratvid2(teks: &str, _frame: u32, _total: u32) -> Vec<u8> {
    render_raw(teks, 1000, 1000)
}

pub fn render_frame_bratlocal(teks: &str, _frame: u32, _total: u32) -> Vec<u8> {
    render_raw(teks, 512, 512)
}

fn run(ctx: Ctx, width: u32, height: u32, fps: u32) -> Reply {
    let text = ctx.text.trim().to_string();
    if text.is_empty() {
        return Reply::text(format!(
            "🎬 *ʙʀᴀᴛ ᴀɴɪᴍᴀᴛᴇᴅ*\n\n> Masukkan teks\n\n`Contoh: {}bratvid Hai semua`",
            ctx.prefix
        ));
    }
    let Ok(mp4) = encode_mp4(&render_raw(&text, width, height), width, height, fps) else {
        return Reply::sequence(vec![
            Reply::text(crate::common::error_template(&ctx)),
            Reply::Reaction {
                emoji: "☢".into(),
                reply: Box::new(Reply::None),
            },
        ]);
    };
    Reply::sequence(vec![
        Reply::video(MediaSource::bytes(mp4).with_mimetype("video/mp4"), String::new()),
        Reply::Reaction {
            emoji: "✅".into(),
            reply: Box::new(Reply::None),
        },
    ])
}

#[plugin(
    name = "bratvid",
    alias = ["bratgif", "bratvideo"],
    category = "sticker",
    description = "Membuat sticker brat animated",
    usage = ".bratvid <text>",
    example = ".bratvid Hai semua",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn bratvid(ctx: Ctx) -> Reply {
    run(ctx, 1000, 1000, 60)
}

#[plugin(
    name = "bratvid2",
    alias = ["bratv2"],
    category = "sticker",
    description = "Sticker brat video v2",
    usage = ".bratvid2 <text>",
    example = ".bratvid2 Hai semua",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn bratvid2(ctx: Ctx) -> Reply {
    run(ctx, 1000, 1000, 60)
}

#[plugin(
    name = "bratlocal",
    alias = ["bratgojo", "bratgojovid", "bratvermeil", "bratvermeilvid"],
    category = "sticker",
    description = "Bikin brat versi lokal",
    usage = ".bratlocal <text>",
    example = ".bratlocal Hai",
    cooldown = 15,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn bratlocal(ctx: Ctx) -> Reply {
    run(ctx, 512, 512, 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_renderers_are_nonempty() {
        assert!(!render_frame_bratvid2("halo", 0, 1).is_empty());
        assert!(!render_frame_bratlocal("halo", 0, 1).is_empty());
    }
}
