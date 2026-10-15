//! `plugins/download/terabox.js` — unduh video/file dari TeraBox.
//!
//! `ponytail:` scraper `TeraBoxDL` (`src/scraper/terabox.js`) melakukan GET ke
//! `flowvideoplayer.com` untuk memetik token CSRF dari HTML lalu POST JSON ke
//! endpoint yang sama sambil mengirim `X-CSRF-TOKEN`, `X-Requested-With`,
//! `Cookie`, dan `Referer`. `HttpGet` tidak punya kontrol header, jadi scraper
//! itu tidak bisa di-port. Yang dipindah di sini SEMUA logika plugin: guard teks
//! kosong, kartu pakai, penyusun caption, dan peng Scrub nama berkas.
//! `terabox_dl` selalu gagal, persis seperti scraper yang melempar.
//!
//! Blok media juga butuh `child_process` (`exec("ffmpeg", …)` untuk stream
//! `.m3u8`), `fs`, dan `sock.sendMessage` untuk dokumen — tidak ada padanannya di
//! API plugin.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::scrub_nama_berkas;

/// `terabox.js:32-38` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📦 *TeraBox Downloader*\n\n\
         Download video atau file dari TeraBox.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}terabox <link>*\n\n\
         *CONTOH:*\n\
         > *{prefix}terabox https://terabox.com/s/xxx*\n\n\
         _File dikirim sebagai dokumen, mungkin agak lama_"
    )
}

/// `terabox.js:49` — `result.status` falsy; `result.error` ikut tercetak
/// `undefined` kalau field-nya tidak ada.
#[allow(dead_code)] // cabang `!result.status` tidak terjangkau di port ini
pub(crate) fn gagal_pakai(error: &str) -> String {
    format!("❌ *TeraBox Gagal*\n\n> {error}")
}

/// `terabox.js:123` — cabang `catch`.
pub(crate) const GAGAL: &str = "❌ Gagal mengambil data TeraBox, coba lagi nanti";

/// Bentuk objek yang `TeraBoxDL` kembalikan (`src/scraper/terabox.js:40-52`).
#[allow(dead_code)] // dipakai `caption`/`nama_berkas`, keduanya hanya diuji
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HasilTerabox {
    pub status: bool,
    /// `null` berarti field-nya tidak ada; JS tetap mencetak `undefined`.
    pub error: Option<String>,
    pub file_name: String,
    pub thumbnail: Option<String>,
    pub download_url: Option<String>,
    pub stream_url: Option<String>,
    pub file_size: String,
    pub duration: String,
    pub extension: Option<String>,
}

/// `terabox.js:52-56` — caption.
#[allow(dead_code)] // tidak terjangkau di port ini; teksnya dijaga lewat test
pub(crate) fn caption(hasil: &HasilTerabox) -> String {
    format!(
        "📦 *TeraBox*\n\n\
         > 📌 {}\n\
         > 📏 Size: {}\n\
         > ⏱️ Durasi: {}",
        hasil.file_name, hasil.file_size, hasil.duration
    )
}

/// `terabox.js:64` — `result.stream_url && result.stream_url.endsWith(".m3u8")`.
///
/// `endsWith` tidak membedakan huruf besar/kecil: `MOV.M3U8` **tidak** ikut,
/// karena `"MOV.M3U8".endsWith(".m3u8")` bernilai `false` di JS.
#[allow(dead_code)] // tidak terjangkau di port ini
pub(crate) fn stream_hls(hasil: &HasilTerabox) -> bool {
    hasil
        .stream_url
        .as_deref()
        .is_some_and(|u| !u.is_empty() && u.ends_with(".m3u8"))
}

/// `terabox.js:91` — nama berkas video hasil ffmpeg.
#[allow(dead_code)] // tidak terjangkau di port ini
pub(crate) fn nama_berkas_video(file_name: &str) -> String {
    let dasar = if file_name.is_empty() { "video" } else { file_name };
    format!("{}.mp4", scrub_nama_berkas(dasar))
}

/// `terabox.js:103-105` — nama berkas unduhan biasa; `|| ".mp4"` hanya dipakai
/// kalau `result.extension` kosong atau `null`.
#[allow(dead_code)] // tidak terjangkau di port ini
pub(crate) fn nama_berkas_unduhan(file_name: &str, extension: Option<&str>) -> String {
    let dasar = if file_name.is_empty() { "file" } else { file_name };
    let ext = match extension {
        Some(e) if !e.is_empty() => e,
        _ => ".mp4",
    };
    format!("{}{}", scrub_nama_berkas(dasar), ext)
}


/// `TeraBoxDL` — satu-satunya bagian yang tidak bisa di-port. `Err` = melempar,
/// jadi cabang `catch` yang menyala.
#[allow(dead_code)] // scraper tidak ada di port ini
fn terabox_dl(_url: &str) -> Result<HasilTerabox, ()> {
    Err(())
}

#[plugin(
    name = "terabox",
    alias = ["tb", "tera", "teraboxdl", "tbdl"],
    category = "download",
    description = "Download video/file dari TeraBox",
    usage = ".terabox <url>",
    example = ".terabox https://terabox.com/s/xxx",
    cooldown = 15,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn terabox(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    let hasil = match terabox_dl(text) {
        Ok(hasil) => hasil,
        // `terabox.js:120-124`.
        Err(()) => return Reply::text(GAGAL).with_reaction("☢"),
    };

    if !hasil.status {
        let teks = hasil.error.as_deref().unwrap_or("undefined");
        return Reply::text(gagal_pakai(teks)).with_reaction("☢");
    }

    // `terabox.js:52-119` — caption lalu thumbnail; cabang `.m3u8` memproses
    // stream dengan ffmpeg, cabang lain menarik `download_url` ke memori lalu
    // mengirim dokumen. Semuanya butuh `sock` + `fs` (+ `child_process`), jadi
    // tidak ada padanannya dan jalur ini berakhir di `catch`.
    Reply::text(GAGAL).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn run(text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "terabox".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("terabox").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    fn hasil_lengkap() -> HasilTerabox {
        HasilTerabox {
            status: true,
            error: None,
            file_name: "Video HD.mp4".into(),
            thumbnail: Some("https://img/t.jpg".into()),
            download_url: Some("https://cdn/d.mp4".into()),
            stream_url: Some("https://cdn/s.m3u8".into()),
            file_size: "120 MB".into(),
            duration: "00:45".into(),
            extension: Some(".mp4".into()),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("terabox").unwrap();
        assert_eq!(p.meta.name, "terabox");
        assert_eq!(p.meta.alias, ["tb", "tera", "teraboxdl", "tbdl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video/file dari TeraBox");
        assert_eq!(p.meta.usage, ".terabox <url>");
        assert_eq!(p.meta.example, ".terabox https://terabox.com/s/xxx");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["tb", "tera", "teraboxdl", "tbdl"] {
            assert_eq!(find(a).unwrap().meta.name, "terabox", "alias {a}");
        }
    }

    #[test]
    fn an_empty_argument_reacts_then_shows_the_card() {
        for text in ["", "   ", "\t"] {
            let r = run(text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaksi(&r), Some("❌"), "input {text:?}");
        }
    }

    #[test]
    fn the_card_follows_the_prefix() {
        let ctx = Ctx {
            prefix: "!!".into(),
            command: "terabox".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        };
        let r = (find("terabox").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), help("!!"));
    }

    #[test]
    fn the_scraper_failure_is_the_catch_branch() {
        // Terabox tidak punya guard host: teks apa pun diteruskan ke scraper.
        for text in [
            "https://terabox.com/s/xxx",
            "https://example.com/x",
            "bukan url",
            "😀 emoji",
        ] {
            let r = run(text);
            assert_eq!(text_of(&r), GAGAL, "input {text}");
            assert_eq!(reaksi(&r), Some("☢"), "input {text}");
        }
    }

    #[test]
    fn the_status_false_branch_prints_undefined_verbatim() {
        assert_eq!(
            gagal_pakai("Tidak ada data ditemukan"),
            "❌ *TeraBox Gagal*\n\n> Tidak ada data ditemukan"
        );
        assert_eq!(gagal_pakai("undefined"), "❌ *TeraBox Gagal*\n\n> undefined");
    }

    #[test]
    fn the_caption_uses_the_three_js_labels() {
        assert_eq!(
            caption(&hasil_lengkap()),
            "📦 *TeraBox*\n\n> 📌 Video HD.mp4\n> 📏 Size: 120 MB\n> ⏱️ Durasi: 00:45"
        );
        // Field kosong tetap menyisakan barisnya, bukan dihapus.
        let kosong = HasilTerabox::default();
        assert_eq!(
            caption(&kosong),
            "📦 *TeraBox*\n\n> 📌 \n> 📏 Size: \n> ⏱️ Durasi: "
        );
    }

    #[test]
    fn only_a_lowercase_m3u8_suffix_counts_as_hls() {
        assert!(stream_hls(&hasil_lengkap()));
        let mut lain = hasil_lengkap();
        lain.stream_url = Some("https://cdn/s.M3U8".into());
        assert!(!stream_hls(&lain), "endsWith di JS tidak case-insensitive");
        lain.stream_url = Some("https://cdn/s.mp4".into());
        assert!(!stream_hls(&lain));
        lain.stream_url = None;
        assert!(!stream_hls(&lain));
        lain.stream_url = Some(String::new());
        assert!(!stream_hls(&lain), "string kosong tidak lewat &&");
    }

    #[test]
    fn the_two_file_names_scrub_reserved_characters() {
        assert_eq!(nama_berkas_video(""), "video.mp4");
        // `@` bukan anggota kelas karakter JS `[<>:"/\\|?*]`, jadi ikut tinggal.
        assert_eq!(nama_berkas_video("x<>:@\"|?*y"), "x@y.mp4");
        assert_eq!(nama_berkas_video("a/b"), "ab.mp4");
    }

    #[test]
    fn the_download_file_name_uses_the_reported_extension() {
        assert_eq!(nama_berkas_unduhan("Arsip", Some(".zip")), "Arsip.zip");
        assert_eq!(nama_berkas_unduhan("", Some(".rar")), "file.rar");
        // `result.extension || ".mp4"` — `null` dan string kosong sama-sama
        // jatuh ke `.mp4`.
        assert_eq!(nama_berkas_unduhan("A", None), "A.mp4");
        assert_eq!(nama_berkas_unduhan("A", Some("")), "A.mp4");
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        assert!(run("https://terabox.com/s/x").mutations().is_empty());
    }
}