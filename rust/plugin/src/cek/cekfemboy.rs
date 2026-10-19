//! `plugins/cek/cekfemboy.js` — cek seberapa femboy kamu.
//!
//! Scraper acak ada di crate `oktz_scraper` (`lufemboy`), bukan disalin ke
//! sini. Konversi GIF → MP4 memakai `oktz_plugin_api::ffmpeg` dengan stdin dan
//! stdout pipa; JS-nya lewat dua berkas temporer (`cekfemboy.js:23-46`).
//!
//! Semantik yang disalin dari JS:
//! - Target = `m.mentionedJid[0] || m.sender`; nama self dari `m.text?.trim()`
//!   (`cekfemboy.js:49-54`). `m.pushName` tidak ada di `Ctx`, jadi self tanpa
//!   teks jatuh ke `"Kamu"` (BLOCKED.md §18).
//! - Unduhan GIF gagal → balas teks saja (`:60-62`); konversi gagal → balas
//!   teks saja (`:64-69`) — dua `catch` kosong yang hasilnya sama.
//! - Kalimat self vs orang lain (`:71-78`) + `mentions: [mentioned]`.
//! - Kuirk: keluaran `pipe:1` tidak bisa `-movflags faststart` (butuh seek),
//!   jadi diganti fragmen `frag_keyframe+empty_moov` + `-f mp4`; sisa argumen
//!   (`-t 30`, `scale`, `libx264`, `yuv420p`, `-an`) sama dengan JS.
//! - `te(m.prefix, m.command, m.pushName)` → `common::error_template`.
//! - JS tidak memanggil `m.react` di plugin ini, jadi tidak ada reaksi.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use oktz_scraper::lufemboy::cek_femboy;

/// `convertGifToMp4` (`cekfemboy.js:33-35`) dengan keluaran dialihkan ke pipa.
pub(crate) const ARGUMEN_GIF_KE_MP4: [&str; 21] = [
    "-y",
    "-ignore_loop",
    "0",
    "-i",
    "pipe:0",
    "-t",
    "30",
    "-vf",
    "scale=trunc(iw/2)*2:trunc(ih/2)*2",
    "-c:v",
    "libx264",
    "-pix_fmt",
    "yuv420p",
    "-movflags",
    "frag_keyframe+empty_moov",
    "-preset",
    "ultrafast",
    "-an",
    "-f",
    "mp4",
    "pipe:1",
];

/// Kalimat balasan (`cekfemboy.js:71-78`).
///
/// Yang dibedakan cuma sapaan self vs orang lain; `@` + nomor tampil di
/// kedua cabang. Spasi sebelum `\n\n` di cabang orang lain ikut JS.
fn balasan(nomor: &str, is_self: bool, hasil: &str) -> String {
    if is_self {
        format!("Hai @{nomor}\n\n{hasil}")
    } else {
        format!("Kamu ingin ngecek tingkat kefemboyan @{nomor} yak? \n\n{hasil}")
    }
}

/// `fetchBuffer(result.gif)` + `convertGifToMp4` (`cekfemboy.js:60-69`).
///
/// `None` = GIF tidak bisa diunduh (axios melempar untuk 4xx/5xx, sama seperti
/// `UreqHttp`) atau ffmpeg gagal; keduanya di JS berakhir di `catch` kosong
/// lalu balas teks.
fn video_dari_gif(ctx: &Ctx, url: &str) -> Option<Vec<u8>> {
    let gif = ctx.http.get(url).ok()?;
    let gif = gif.body.bytes();
    if gif.is_empty() {
        // `Buffer.alloc(0)` truthy di JS, lalu ffmpeg menolak masukannya —
        // hasil akhirnya sama: tidak ada video.
        return None;
    }
    oktz_plugin_api::ffmpeg::jalankan_dengan_masukan(&ARGUMEN_GIF_KE_MP4, gif).ok()
}

#[plugin(
    name = "cekfemboy",
    alias = ["femboy"],
    category = "cek",
    description = "Cek seberapa femboy kamu",
    usage = ".cekfemboy <nama>",
    example = ".cekfemboy Budi",
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    cooldown = 5,
    energi = 0
)]
pub fn cekfemboy(ctx: Ctx) -> Reply {
    let target = ctx.mention_or_sender().to_string();
    let nomor = crate::common::jid_number(&target).to_string();
    let is_self = target == ctx.sender;

    let nama = if is_self {
        let input = ctx.text.trim();
        if input.is_empty() {
            // `m.pushName` tidak ada di `Ctx`; `|| "Kamu"` yang tersisa.
            "Kamu".to_string()
        } else {
            input.to_string()
        }
    } else {
        format!("@{nomor}")
    };

    // `cekfemboy(nama)` cuma melempar kalau nama kosong (`lufemboy.js:3`),
    // dan itu tidak mungkin dari cabang di atas; `Err` tetap ditangani supaya
    // tidak ada panic.
    let Ok(hasil) = cek_femboy(&*ctx.http, &nama) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command));
    };
    let teks_hasil = hasil
        .get("hasil")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    let txt = balasan(&nomor, is_self, teks_hasil);

    let video = hasil
        .get("gif")
        .and_then(serde_json::Value::as_str)
        .and_then(|url| video_dari_gif(&ctx, url));

    match video {
        Some(mp4) => Reply::Video {
            media: Some(MediaSource::bytes(mp4).with_mimetype("video/mp4")),
            caption: txt,
            mentions: vec![target],
            gif_playback: true,
        },
        None => Reply::Text {
            text: txt,
            mentions: vec![target],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{
        HttpBody, HttpError, HttpGet, HttpResponse, find,
    };
    use std::sync::Arc;

    const ME: &str = "628111@s.whatsapp.net";
    const OTHER: &str = "628999@s.whatsapp.net";

    /// Stub HTTP: balasan tetap untuk semua GET, atau error kalau `gagal`.
    #[derive(Debug)]
    struct StubHttp {
        balasan: Result<HttpBody, HttpError>,
        seen: std::sync::Mutex<Vec<String>>,
    }

    impl StubHttp {
        fn baru(data: Vec<u8>) -> Arc<StubHttp> {
            Arc::new(StubHttp {
                balasan: Ok(HttpBody::Bytes {
                    data: Arc::from(data),
                    mimetype: None,
                }),
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }

        fn offline() -> Arc<StubHttp> {
            Arc::new(StubHttp {
                balasan: Err(HttpError {
                    message: "jaringan mati".to_string(),
                }),
                seen: std::sync::Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.balasan {
                Ok(body) => Ok(HttpResponse {
                    status: 200,
                    body: body.clone(),
                    headers: Vec::new(),
                }),
                Err(e) => Err(e.clone()),
            }
        }
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
            unreachable!("plugin ini cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("plugin ini cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("plugin ini cuma GET")
        }
    }

    fn ctx_of(http: Arc<dyn HttpGet>, mentioned: Vec<String>, text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: ME.into(),
            mentioned,
            prefix: ".".into(),
            command: "cekfemboy".into(),
            http,
            ..Ctx::default()
        }
    }

    fn teks_dan_mention(r: &Reply) -> (String, Vec<String>) {
        match r {
            Reply::Text { text, mentions } => (text.clone(), mentions.clone()),
            other => panic!("harus Text, dapat {other:?}"),
        }
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = find("cekfemboy").unwrap();
        assert_eq!(p.meta.name, "cekfemboy");
        assert_eq!(p.meta.alias, ["femboy"]);
        assert_eq!(p.meta.category, "cek");
        assert_eq!(p.meta.description, "Cek seberapa femboy kamu");
        assert_eq!(p.meta.usage, ".cekfemboy <nama>");
        assert_eq!(p.meta.example, ".cekfemboy Budi");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner);
        assert!(!p.meta.is_premium);
        assert!(!p.meta.is_group);
        assert!(!p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
    }

    #[test]
    fn alias_femboy_menunjuk_ke_plugin_ini() {
        assert_eq!(find("femboy").unwrap().meta.name, "cekfemboy");
    }

    #[test]
    fn diri_sendiri_tanpa_teks_memakai_nama_kamu() {
        let r = cekfemboy(ctx_of(StubHttp::offline(), Vec::new(), ""));
        let (teks, mentions) = teks_dan_mention(&r);
        assert!(teks.starts_with("Hai @628111\n\n"), "{teks}");
        assert!(teks.contains("Kamu, kamu "), "{teks}");
        assert!(teks.contains("% femboy!"), "{teks}");
        assert_eq!(mentions, [ME.to_string()]);
    }

    #[test]
    fn diri_sendiri_pakai_teks_sebagai_nama_lalu_trim() {
        let r = cekfemboy(ctx_of(StubHttp::offline(), Vec::new(), "  Budi  "));
        let (teks, _) = teks_dan_mention(&r);
        assert!(teks.contains("Budi, kamu "), "{teks}");
        assert!(!teks.contains("Kamu, kamu "), "{teks}");
    }

    #[test]
    fn mention_orang_lain_bilang_yak_pakai_nomor_target() {
        let r = cekfemboy(ctx_of(StubHttp::offline(), vec![OTHER.to_string()], ""));
        let (teks, mentions) = teks_dan_mention(&r);
        assert!(
            teks.starts_with("Kamu ingin ngecek tingkat kefemboyan @628999 yak? \n\n"),
            "{teks}"
        );
        assert!(teks.contains("@628999, kamu "), "{teks}");
        assert_eq!(mentions, [OTHER.to_string()]);
    }

    #[test]
    fn mention_dengan_device_tidak_dibuang_dari_bagian_tampil() {
        // `cb.split("@")[0]` cuma membuang server, bukan `:3` — sama seperti JS.
        let jid_device = "628999:3@s.whatsapp.net";
        let r = cekfemboy(ctx_of(StubHttp::offline(), vec![jid_device.to_string()], ""));
        let (teks, mentions) = teks_dan_mention(&r);
        assert!(teks.contains("@628999:3 yak?"), "{teks}");
        assert_eq!(mentions, [jid_device.to_string()]);
    }

    #[test]
    fn mention_sendiri_dianggap_diri_sendiri() {
        let r = cekfemboy(ctx_of(StubHttp::offline(), vec![ME.to_string()], ""));
        let (teks, mentions) = teks_dan_mention(&r);
        assert!(teks.starts_with("Hai @628111\n\n"), "{teks}");
        assert_eq!(mentions, [ME.to_string()]);
    }

    #[test]
    fn unduhan_gif_gagal_tetap_membalas_teks() {
        let http = StubHttp::offline();
        let r = cekfemboy(ctx_of(http.clone(), Vec::new(), "Budi"));
        let (teks, _) = teks_dan_mention(&r);
        assert!(teks.contains("Budi, kamu "), "{teks}");
        // URL GIF tetap diminta sekali sebelum gagal.
        assert_eq!(
            http.seen.lock().unwrap().len(),
            1,
            "satu permintaan GIF lalu menyerah"
        );
    }

    #[test]
    fn gif_palsu_tidak_pernah_jadi_video_yang_dikirim() {
        // Byte yang bukan GIF: ffmpeg (kalau ada) menolaknya, `jalankan_dengan_masukan`
        // mengembalikan `Err`, dan JS pun jatuh ke balasan teks.
        let r = cekfemboy(ctx_of(StubHttp::baru(b"bukan gif".to_vec()), Vec::new(), "Budi"));
        let (teks, mentions) = teks_dan_mention(&r);
        assert!(teks.contains("Budi, kamu "), "{teks}");
        assert_eq!(mentions, [ME.to_string()]);
    }

    #[test]
    fn argumen_ffmpeg_pipa_mengganti_faststart() {
        let argumen = ARGUMEN_GIF_KE_MP4;
        assert!(!argumen.contains(&"faststart"), "faststart butuh keluaran seekable");
        assert_eq!(argumen[4], "pipe:0");
        assert_eq!(argumen[20], "pipe:1");
        assert_eq!(argumen[18..20], ["-f", "mp4"]);
        let ada = |a: &str, b: &str| argumen.windows(2).any(|w| w[0] == a && w[1] == b);
        assert!(ada("-t", "30"));
        assert!(ada("-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2"));
        assert!(ada("-c:v", "libx264"));
        assert!(ada("-pix_fmt", "yuv420p"));
        assert!(ada("-preset", "ultrafast"));
        assert!(ada("-ignore_loop", "0"));
        assert!(argumen.contains(&"-an"));
    }
}
