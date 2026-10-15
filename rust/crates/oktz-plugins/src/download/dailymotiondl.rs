//! `plugins/download/dailymotiondl.js` — unduh video Dailymotion.
//!
//! `ponytail:` scraper `DailymotionDL` (`src/scraper/dailymotion.js`) mengirim
//! **POST** `x-www-form-urlencoded` ke `vidomon.com` dengan lima header kustom
//! (`User-Agent`, `Referer`, `X-Forwarded-For`, `X-Real-IP` dan IP acak yang
//! dibuat sendiri). `HttpGet` tidak punya kontrol header maupun POST ber-header,
//! jadi scraper itu tidak bisa di-port. Yang dipindah di sini SEMUA logika
//! plugin: guard teks kosong, kartu pakai, penyusun caption, dan peng Scrub nama
//! berkas. `dailymotion_dl` selalu gagal, persis seperti scraper yang melempar.
//!
//! Blok media juga butuh `child_process` (`exec("ffmpeg", …)`), `fs`, dan
//! `sock.sendMessage` untuk dokumen — tidak ada padanannya di API plugin, jadi
//! bahkan setelah scraper berhasil jalurnya berakhir di `catch`, sama seperti
//! di Node saat `ffmpeg` tidak ada di `PATH`.
//!
//! `ponytail:` `Ctx` tidak punya `pushName`; tidak dipakai plugin ini.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::scrub_nama_berkas;

/// `dailymotiondl.js:31-37` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎬 *Dailymotion Downloader*\n\n\
         Download video dari Dailymotion, otomatis dikonversi ke MP4.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}dailymotiondl <link>*\n\n\
         *CONTOH:*\n\
         > *{prefix}dailymotiondl https://www.dailymotion.com/video/xxx*\n\n\
         _Proses konversi mungkin agak lama_"
    )
}

/// `dailymotiondl.js:48` — `result.status` falsy. `error` sudah di-string-kan
/// lebih dulu supaya `undefined` ikut tercetak apa adanya seperti di JS.
#[allow(dead_code)] // cabang `!result.status` tidak terjangkau di port ini
pub(crate) fn gagal_pakai(error: &str) -> String {
    format!("❌ *Dailymotion Gagal*\n\n> {error}")
}

/// `dailymotiondl.js:101` — cabang `catch`.
pub(crate) const GAGAL: &str = "❌ Gagal mengambil data Dailymotion, coba lagi nanti";

/// Bentuk objek yang `DailymotionDL` kembalikan (`src/scraper/dailymotion.js:26-35`).
#[allow(dead_code)] // dipakai `caption`/`nama_berkas`, keduanya hanya diuji
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HasilDailymotion {
    pub status: bool,
    /// `null` berarti field-nya tidak ada; JS tetap mencetak `undefined`.
    pub error: Option<String>,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: String,
    pub quality: String,
    pub video: Option<String>,
}

/// `dailymotiondl.js:51-55` — caption. `result.error` yang `undefined` ikut jadi
/// teks `"undefined"`, bukan string kosong.
#[allow(dead_code)] // tidak terjangkau di port ini; teksnya dijaga lewat test
pub(crate) fn caption(hasil: &HasilDailymotion) -> String {
    format!(
        "🎬 *Dailymotion*\n\n\
         > 📌 {}\n\
         > ⏱️ Durasi: {}\n\
         > 📺 Kualitas: {}",
        hasil.title, hasil.duration, hasil.quality
    )
}

/// `dailymotiondl.js:90` — `(result.title || "video").replace(/[<>:"\/\\|?*]/g, "")
/// + ".mp4"`. `||` bukan `??`: title kosong ikut jadi `"video"`.
#[allow(dead_code)] // tidak terjangkau di port ini; teksnya dijaga lewat test
pub(crate) fn nama_berkas(title: &str) -> String {
    let dasar = if title.is_empty() { "video" } else { title };
    format!("{}.mp4", scrub_nama_berkas(dasar))
}


/// `DailymotionDL` — satu-satunya bagian yang tidak bisa di-port. `Err` =
/// melempar, jadi cabang `catch` yang menyala.
#[allow(dead_code)] // scraper tidak ada di port ini
fn dailymotion_dl(_url: &str) -> Result<HasilDailymotion, ()> {
    Err(())
}

#[plugin(
    name = "dailymotiondl",
    alias = ["dailymotion", "dmdl"],
    category = "download",
    description = "Download video dari Dailymotion",
    usage = ".dailymotiondl <url>",
    example = ".dailymotiondl https://www.dailymotion.com/video/xxx",
    cooldown = 15,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dailymotiondl(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    let hasil = match dailymotion_dl(text) {
        Ok(hasil) => hasil,
        // `dailymotiondl.js:98-102`.
        Err(()) => return Reply::text(GAGAL).with_reaction("☢"),
    };

    if !hasil.status {
        // `dailymotiondl.js:46-49` — `result.error` boleh `undefined`.
        let teks = hasil.error.as_deref().unwrap_or("undefined");
        return Reply::text(gagal_pakai(teks)).with_reaction("☢");
    }

    // `dailymotiondl.js:51-95` — caption sudah dirakit, lalu thumbnail dikirim
    // sebagai gambar dan `result.video` (selalu terisi dari scraper yang hidup)
    // diproses ffmpeg. Tanpa `child_process`/`fs`/`sock` langkah itu melempar,
    // jadi `catch` yang menyala — persis yang terjadi di Node tanpa ffmpeg.
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
            command: "dailymotiondl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            ..Default::default()
        };
        (find("dailymotiondl").unwrap().handler)(ctx)
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

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("dailymotiondl").unwrap();
        assert_eq!(p.meta.name, "dailymotiondl");
        assert_eq!(p.meta.alias, ["dailymotion", "dmdl"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download video dari Dailymotion");
        assert_eq!(p.meta.usage, ".dailymotiondl <url>");
        assert_eq!(
            p.meta.example,
            ".dailymotiondl https://www.dailymotion.com/video/xxx"
        );
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["dailymotion", "dmdl"] {
            assert_eq!(find(a).unwrap().meta.name, "dailymotiondl", "alias {a}");
        }
    }

    #[test]
    fn an_empty_argument_reacts_then_shows_the_card() {
        for text in ["", "   ", "\t\n"] {
            let r = run(text);
            assert_eq!(text_of(&r), help("."), "input {text:?}");
            assert_eq!(reaksi(&r), Some("❌"), "input {text:?}");
        }
    }

    #[test]
    fn the_card_follows_the_prefix() {
        // `m.prefix` disisipkan di dua baris; kartu tidak ditulis ulang.
        let ctx = Ctx {
            prefix: "!".into(),
            command: "dailymotiondl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            ..Default::default()
        };
        let r = (find("dailymotiondl").unwrap().handler)(ctx);
        assert_eq!(text_of(&r), help("!"));
        assert!(text_of(&r).contains("> *!dailymotiondl <link>*"));
    }

    #[test]
    fn the_scraper_failure_is_the_catch_branch() {
        for text in [
            "https://www.dailymotion.com/video/xxx",
            "https://dailymotion.com/video/x",
            "bukan url sama sekali",
        ] {
            let r = run(text);
            assert_eq!(text_of(&r), GAGAL, "input {text}");
            assert_eq!(reaksi(&r), Some("☢"), "input {text}");
        }
    }

    #[test]
    fn the_status_false_branch_prints_undefined_verbatim() {
        // `dailymotiondl.js:48` memakai `${result.error}`; field yang tidak ada
        // tercetak literal sebagai "undefined" di JS. Bukan string kosong.
        assert_eq!(gagal_pakai("Tidak ada media yang tersedia"), "❌ *Dailymotion Gagal*\n\n> Tidak ada media yang tersedia");
        assert_eq!(gagal_pakai("undefined"), "❌ *Dailymotion Gagal*\n\n> undefined");
        assert_eq!(
            HasilDailymotion {
                status: false,
                error: None,
                ..Default::default()
            }
            .error
            .as_deref()
            .unwrap_or("undefined"),
            "undefined"
        );
    }

    #[test]
    fn the_caption_uses_the_three_js_labels() {
        let hasil = HasilDailymotion {
            status: true,
            error: None,
            title: "Judul Video".into(),
            thumbnail: Some("https://img/t.jpg".into()),
            duration: "02:10".into(),
            quality: "720".into(),
            video: Some("https://cdn/v.mp4".into()),
        };
        assert_eq!(
            caption(&hasil),
            "🎬 *Dailymotion*\n\n> 📌 Judul Video\n> ⏱️ Durasi: 02:10\n> 📺 Kualitas: 720"
        );
    }

    #[test]
    fn the_file_name_scrubs_reserved_characters() {
        assert_eq!(nama_berkas(""), "video.mp4");
        assert_eq!(nama_berkas("a<b>c:d\"e/f\\g|h?i*j"), "abcdefghij.mp4");
        assert_eq!(nama_berkas("video"), "video.mp4");
        assert_eq!(nama_berkas("a b-c_d.e"), "a b-c_d.e.mp4");
        assert_eq!(scrub_nama_berkas("<>:\"/\\|?*"), "");
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        assert!(run("https://x/y").mutations().is_empty());
    }
}