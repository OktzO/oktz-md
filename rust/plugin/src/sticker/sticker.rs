//! `plugins/sticker/sticker.js` — command `.s`.


use oktz_plugin_api::{
    Ctx, IncomingKind, MediaError, MediaFetch, MediaOrigin, MediaSource, OpsiS, Reply, StickerOpsi,
    plugin,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Jenis {
    Gambar,
    Video,
}

#[cfg(test)]
#[cfg(test)]
#[derive(Debug, Default)]
struct Unduhan {
    pemicu: Vec<u8>,
    kutipan: Vec<u8>,
}

impl MediaFetch for Unduhan {
    fn download(&self, origin: MediaOrigin) -> Result<Vec<u8>, MediaError> {
        match origin {
            MediaOrigin::Trigger => Ok(self.pemicu.clone()),
            MediaOrigin::Quoted => Ok(self.kutipan.clone()),
            MediaOrigin::Indexed(_) => Err(MediaError {
                message: "tidak ada media di pesan yang di-reply".to_string(),
            }),
        }
    }
}

fn is_gambar(ctx: &Ctx) -> bool {
    ctx.is_media_kind(IncomingKind::Image)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_image())
}

fn is_video(ctx: &Ctx) -> bool {
    ctx.is_media_kind(IncomingKind::Video)
        || ctx
            .quoted()
            .is_some_and(|q| q.kind.is_video())
}

fn pakai(prefix: &str) -> String {
    format!(
        "🖼️ *sᴛɪᴄᴋᴇʀ ᴍᴀᴋᴇʀ*\n\n\
Kirim/reply gambar atau video dengan caption:\n\
`{prefix}s`\n\n\
*ᴏᴘsɪ:*\n\
> `--crop` - Crop jadi kotak\n\
> `--resize WxH` - Resize ke ukuran\n\
> `--circle` - Bentuk lingkaran\n\
> `--rounded` - Sudut melengkung\n\n\
*ᴄᴏɴᴛᴏʜ:*\n\
> `{prefix}s --crop`\n\
> `{prefix}s --resize 256x256`\n\
> `{prefix}s --circle`\n\
> `{prefix}s --rounded`\n\n\
*ᴀʟʙᴜᴍ:*\n\
> Reply pesan foto/video yang banyak → semua jadi sticker dalam 1 pesan"
    )
}

fn gagal_kirim() -> String {
    "❌ Gagal mendownload media!".to_string()
}

fn terlalu_panjang(durasi: f64) -> String {
    format!(
        "❌ Video terlalu panjang!\n\n> Durasi: {:.1} detik\n> Maksimal: 10 detik",
        durasi
    )
}

fn metadata(ctx: &Ctx, packname: Option<String>, author: Option<String>) -> (String, String) {
    let cfg = ctx.config();
    let nama_pack = packname
        .or_else(|| (!cfg.sticker.packname.is_empty()).then(|| cfg.sticker.packname.clone()))
        .or_else(|| (!cfg.bot.name.is_empty()).then(|| cfg.bot.name.clone()))
        .unwrap_or_else(|| "Foto-AI".to_string());
    let nama_author = author
        .or_else(|| (!cfg.sticker.author.is_empty()).then(|| cfg.sticker.author.clone()))
        .or_else(|| (!cfg.owner.name.is_empty()).then(|| cfg.owner.name.clone()))
        .unwrap_or_else(|| "Bot".to_string());
    (nama_pack, nama_author)
}

fn unduh(ctx: &Ctx) -> Result<Vec<u8>, String> {
    if ctx
        .quoted()
        .is_some_and(|q| q.kind.is_media())
    {
        return ctx
            .download_quoted_media()
            .map_err(|e| e.message);
    }
    ctx.download_media().map_err(|e| e.message)
}

fn reaksi(emoji: &str) -> Reply {
    Reply::Reaction {
        emoji: emoji.into(),
        reply: Box::new(Reply::None),
    }
}

fn siapkan(
    jenis: Jenis,
    data: &[u8],
    opsi: &[OpsiS],
    packname: &str,
    author: &str,
) -> Result<(Vec<u8>, StickerOpsi), String> {
    let opsi_wire = match jenis {
        Jenis::Gambar => StickerOpsi::gambar(),
        Jenis::Video => StickerOpsi::video(),
    }
    .with_packname(packname)
    .with_author(author);

    let bytes = match jenis {
        Jenis::Gambar => oktz_plugin_api::gambar_ke_stiker_webp(data, opsi)?,
        Jenis::Video => oktz_plugin_api::video_ke_stiker_webp(data, opsi)?,
    };
    let bytes = oktz_plugin_api::tempel_metadata_stiker(&bytes, packname, author, &[&opsi_wire.emojis.first().cloned().unwrap_or_else(|| "🤖".to_string())])
        .unwrap_or(bytes);
    Ok((bytes, opsi_wire))
}

#[plugin(
    name = "sticker",
    alias = ["s", "stiker", "stickergif"],
    category = "sticker",
    description = "Membuat sticker dari gambar/video dengan opsi crop/resize",
    usage = ".s [--crop] [--resize WxH] [--circle] [--rounded]",
    example = ".s --crop",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn sticker(ctx: Ctx) -> Reply {
    let gambar = is_gambar(&ctx);
    let video = is_video(&ctx);

    if !gambar && !video {
        return Reply::text(pakai(&ctx.prefix));
    }

    let (opsi, packname, author) = oktz_plugin_api::opsi_s(&ctx.text);
    let jenis = if gambar { Jenis::Gambar } else { Jenis::Video };

    if !opsi.iter().any(|o| {
        matches!(
            o,
            OpsiS::Crop | OpsiS::Resize(_, _) | OpsiS::Circle | OpsiS::Rounded
        )
    }) && ctx
        .quoted()
        .is_some_and(|q| q.kind.is_image() || q.kind.is_video())
    {
        let (packname, author) = metadata(&ctx, packname.clone(), author.clone());
        let anchor_ts = ctx
            .album_media()
            .iter()
            .find(|m| ctx.quoted().map(|q| q.id.as_str()) == Some(m.id.as_str()))
            .map(|m| m.timestamp)
            .unwrap_or(0);
        let (selected, skipped) =
            oktz_plugin_api::pilih_album_media(ctx.album_media(), ctx.quoted().unwrap().id.as_str(), anchor_ts);
        if selected.len() >= 2 {
            let mut buffers = Vec::new();
            for item in selected {
                if let Ok(bytes) = ctx.download_album_media(&item.id) {
                    if let Ok(webp) = oktz_plugin_api::gambar_ke_stiker_webp(&bytes, &[]) {
                        buffers.push(webp);
                    }
                }
            }
            if buffers.len() >= 2 {
                let mut urutan = vec![
                    reaksi("🕕"),
                    Reply::sticker_pack(
                        buffers
                            .clone()
                            .into_iter()
                            .map(MediaSource::bytes)
                            .collect(),
                        packname.clone(),
                        author.clone(),
                        String::new(),
                        vec!["🎨".to_string()],
                    ),
                ];
                if skipped > 0 {
                    urutan.push(Reply::text(format!(
                        "Sticker album selesai ({}). {} media lainnya dilewati (maks 10).",
                        buffers.len(),
                        skipped
                    )));
                }
                urutan.push(reaksi("✅"));
                return Reply::sequence(urutan);
            }
        }
    }

    let Ok(data) = unduh(&ctx) else {
        return Reply::sequence(vec![Reply::text(gagal_kirim()), reaksi("❌")]);
    };

    if video
        && let Some(durasi) = oktz_plugin_api::ffmpeg::durasi_dari_masukan(&data)
        && durasi > 10.0
    {
        return Reply::sequence(vec![Reply::text(terlalu_panjang(durasi)), reaksi("☢")]);
    }

    let (packname, author) = metadata(&ctx, packname, author);
    let has_processing = opsi.iter().any(|o| {
        matches!(
            o,
            OpsiS::Crop | OpsiS::Resize(_, _) | OpsiS::Circle | OpsiS::Rounded
        )
    });

    match siapkan(jenis, &data, &opsi, &packname, &author) {
        Ok((bytes, opsi_wire)) => {
            let mut urutan = vec![
                reaksi("🕕"),
                Reply::sticker_opsi(
                    MediaSource::bytes(bytes).with_mimetype(match jenis {
                        Jenis::Gambar => "image/webp",
                        Jenis::Video => "image/webp",
                    }),
                    opsi_wire,
                ),
                reaksi("✅"),
            ];
            if has_processing {
                urutan.push(Reply::text(
                    "⚠️ *Efek tidak bisa diterapkan*\n\n> Sticker sudah dikirim, tapi efek yang kakak minta gagal diproses.\n> _Sebaiknya cek format/ukuran sumbernya ya._",
                ));
            }
            Reply::sequence(urutan)
        }
        Err(_) if has_processing => {
            match siapkan(jenis, &data, &[], &packname, &author) {
                Ok((bytes, opsi_wire)) => Reply::sequence(vec![
                    reaksi("🕕"),
                    Reply::sticker_opsi(
                        MediaSource::bytes(bytes).with_mimetype("image/webp"),
                        opsi_wire,
                    ),
                    reaksi("✅"),
                    Reply::text(
                        "⚠️ *Efek tidak bisa diterapkan*\n\n> Sticker sudah dikirim, tapi efek yang kakak minta gagal diproses.\n> _Sebaiknya cek format/ukuran sumbernya ya._",
                    ),
                ]),
                Err(_) => Reply::sequence(vec![
                    Reply::text(crate::common::error_template(&ctx)),
                    reaksi("❌"),
                ]),
            }
        }
        Err(_) => Reply::sequence(vec![
            Reply::text(crate::common::error_template(&ctx)),
            reaksi("❌"),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::config::Runtime;
    use oktz_plugin_api::IncomingMedia;
    use std::sync::Arc;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Mutex;

    static STICKER_TEST_LOCK: Mutex<()> = Mutex::new(());

    const ME: &str = "628111@s.whatsapp.net";

    fn ffprobe_palsu(durasi: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("oktz-ffprobe-{}.sh", std::process::id()));
        fs::write(
            &path,
            format!("#!/bin/sh\ncat >/dev/null\necho {durasi}\n"),
        )
        .unwrap();
        let mut izin = fs::metadata(&path).unwrap().permissions();
        izin.set_mode(0o755);
        fs::set_permissions(&path, izin).unwrap();
        path
    }

    fn png() -> Vec<u8> {
        b"RIFF0000WEBP".to_vec()
    }

    fn ctx_media(kind: IncomingKind, pemicu: Vec<u8>, kutipan: Vec<u8>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "s".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: String::new(),
            args: Vec::new(),
            media: Some(IncomingMedia {
                kind,
                mimetype: None,
                file_name: None,
                caption: String::new(),
            }),
            media_fetch: Arc::new(Unduhan { pemicu, kutipan }),
            ..Ctx::default()
        }
    }

    fn ctx_kutipan(kind: IncomingKind, pemicu: Vec<u8>, kutipan: Vec<u8>) -> Ctx {
        let kind = match kind {
            IncomingKind::Image => oktz_plugin_api::QuotedKind::Image,
            IncomingKind::Video => oktz_plugin_api::QuotedKind::Video,
            _ => oktz_plugin_api::QuotedKind::Other("media".into()),
        };
        let q = oktz_plugin_api::QuotedMessage {
            id: "Q1".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: String::new(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
            timestamp: 0,
        };
        Ctx {
            quoted: Some(q),
            ..ctx_media(IncomingKind::Image, pemicu, kutipan)
        }
    }

    #[test]
    fn plugin_registers_sticker_with_js_aliases_and_cooldown() {
        let p = oktz_plugin_api::find("sticker").unwrap();
        assert_eq!(p.meta.name, "sticker");
        assert_eq!(p.meta.alias, ["s", "stiker", "stickergif"]);
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        for key in ["s", "stiker", "stickergif"] {
            assert_eq!(oktz_plugin_api::find(key).map(|p| p.meta.name.as_str()), Some("sticker"));
        }
    }

    #[test]
    fn option_parser_matches_js_first_two_named_tokens() {
        let (opsi, packname, author) = oktz_plugin_api::opsi_s("--resize 256x256 Pack Author ketiga");
        assert_eq!(opsi, vec![OpsiS::Resize(256, 256)]);
        assert_eq!(packname.as_deref(), Some("Pack"));
        assert_eq!(author.as_deref(), Some("Author"));
    }

    #[test]
    fn image_sticker_metadata_and_context_match_js() {
        let r = sticker(ctx_media(IncomingKind::Image, png(), Vec::new()));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        let Reply::Sticker { media, opsi } = &list[1] else {
            panic!("harus sticker: {list:?}")
        };
        assert!(media.is_some());
        assert!(opsi.forwarded);
        assert_eq!(opsi.forwarding_score, Some(1));
        assert!(opsi.premium);
        assert_eq!(opsi.packname.as_deref(), Some("Foto-AI"));
    }

    #[test]
    fn video_sticker_context_is_forwarding_score_999() {
        let _guard = STICKER_TEST_LOCK.lock().unwrap();
        let r = sticker(ctx_media(IncomingKind::Video, b"RIFF0000WEBP".to_vec(), Vec::new()));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        let Reply::Sticker { opsi, .. } = &list[1] else {
            panic!("harus sticker: {list:?}")
        };
        assert!(opsi.forwarded);
        assert_eq!(opsi.forwarding_score, Some(999));
        assert!(!opsi.premium);
    }

    #[test]
    fn quoted_media_is_downloaded_before_trigger() {
        let _guard = STICKER_TEST_LOCK.lock().unwrap();
        let r = sticker(ctx_kutipan(IncomingKind::Video, b"RIFF0000WEBP".to_vec(), b"RIFF0000WEBP".to_vec()));
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert!(matches!(&list[1], Reply::Sticker { .. }));
    }

    #[test]
    fn over_ten_second_video_is_rejected_with_js_text() {
        let _guard = STICKER_TEST_LOCK.lock().unwrap();
        let path = ffprobe_palsu("10.5");
        unsafe { std::env::set_var("OTKZ_FFPROBE_PATH", &path) };
        let r = sticker(ctx_media(IncomingKind::Video, Vec::new(), Vec::new()));
        unsafe { std::env::remove_var("OTKZ_FFPROBE_PATH") };
        let _ = fs::remove_file(&path);
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert_eq!(
            list[0].without_mutations().text_body().unwrap(),
            terlalu_panjang(10.5)
        );
        assert!(matches!(&list[1], Reply::Reaction { emoji, .. } if emoji == "☢"));
    }

    #[test]
    fn processing_failure_sends_original_then_check_then_warning() {
        let r = sticker(Ctx {
            text: "--circle".into(),
            args: vec!["--circle".into()],
            ..ctx_media(IncomingKind::Image, png(), Vec::new())
        });
        let Reply::Sequence(list) = r else {
            panic!("harus sequence: {r:?}")
        };
        assert_eq!(list.len(), 4);
        assert!(matches!(&list[1], Reply::Sticker { .. }));
        assert!(matches!(&list[2], Reply::Reaction { emoji, .. } if emoji == "✅"));
        assert!(list[3].without_mutations().text_body().unwrap().contains("Sebaiknya"));
    }

    #[test]
    fn no_media_shows_js_usage_card() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "s".into(),
            ..Ctx::default()
        };
        let r = sticker(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            pakai(".")
        );
    }

    #[test]
    fn config_defaults_follow_js_truthy_chain() {
        let mut cfg = Runtime::default();
        cfg.sticker.packname.clear();
        cfg.sticker.author.clear();
        cfg.bot.name = "Foto-AI".into();
        cfg.owner.name = "Oktz".into();
        let ctx = Ctx {
            config: Arc::new(cfg),
            ..Ctx::default()
        };
        assert_eq!(metadata(&ctx, None, None), ("Foto-AI".to_string(), "Oktz".to_string()));
    }
}
