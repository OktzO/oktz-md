//! `plugins/download/dailymotiondl.js` — unduh video Dailymotion.
//!
//! Scraper `DailymotionDL` (`src/scraper/dailymotion.js`) sudah di-port ke
//! crate `oktz-scraper` (`oktz_scraper::dailymotion::dailymotion_dl`), jadi
//! POST `x-www-form-urlencoded` ke `vidomon.com` dengan lima header kustom
//! sekarang benar-benar terjadi. Plugin ini hanya memanggilnya — logika
//! scraper tidak diduplikasi di sini.
//!
//! Tiga penyimpitan yang tersisa, semuanya tercatat apa adanya:
//!
//! - **ffmpeg tidak dijalankan.** JS.remux video ke `storage temp` dengan
//!   `-c copy -bsf:a aac_adtstoasc` sebelum mengirimnya sebagai dokumen
//!   (`dailymotiondl.js:66-82`). Rust mengirim URL video apa adanya lewat
//!   `Reply::document(MediaSource::url(..))`. Bedanya nyata: berkas yang
//!   dikirim tidak sudah dilewatkan filter bitstream WA, jadi sebagian klien
//!   mungkin menolak memutar-nya. Butuh `queueFFmpeg` (§6 BLOCKED.md).
//! - **Thumbnail tidak dikirim.** JS mengirim dua pesan: gambar thumbnail
//!   (`dailymotiondl.js:57-61`) lalu dokumen video (`:84-94`). Satu `Reply`
//!   hanya satu pesan, dan dokumen video yang jadi tujuan plugin ini, jadi
//!   gambar thumbnail yang diabaikan. Butuh `Reply::Sequence` / multi-pesan.
//! - **Reaksi awal `🕕` dibuang** (`dailymotiondl.js:41`), mengikuti
//!   konvensi yang sudah dipakai plugin lain: `Reply::Reaction` hanya bisa
//!   satu, jadi reaksi terakhir yang ikut — `✅` setelah video terkirim, `☢`
//!   di cabang gagal.
//!
//! `Ctx` tidak punya `pushName`; tidak dipakai plugin ini.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

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

/// `dailymotiondl.js:48` — cabang `!result.status`. `error` sudah di-string-kan
/// lebih dulu supaya `undefined` ikut tercetak apa adanya seperti di JS.
pub(crate) fn gagal_pakai(error: &str) -> String {
    format!("❌ *Dailymotion Gagal*\n\n> {error}")
}

/// `dailymotiondl.js:101` — cabang `catch`.
pub(crate) const GAGAL: &str = "❌ Gagal mengambil data Dailymotion, coba lagi nanti";

/// Bentuk objek yang `DailymotionDL` kembalikan (`src/scraper/dailymotion.js:36-45`).
///
/// Field opsional `None` berarti tidak ada di respons; pemanggil yang
/// menulis `${…}` harus memperlakukannya sebagai `undefined`, bukan string
/// kosong — itu sebabnya ada test khusus untuk itu.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct HasilDailymotion {
    pub status: bool,
    pub error: Option<String>,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<String>,
    pub quality: Option<String>,
    pub video: Option<String>,
}

/// Baca hasil scraper. Kunci yang tidak ada di JSON berarti `undefined` di JS.
fn hasil_dari(data: &Value) -> HasilDailymotion {
    let teks = |k: &str| data.get(k).and_then(Value::as_str).map(str::to_string);
    HasilDailymotion {
        status: data
            .get("status")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        error: teks("error"),
        title: teks("title"),
        thumbnail: teks("thumbnail"),
        duration: teks("duration"),
        quality: teks("quality"),
        video: teks("video"),
    }
}

/// `dailymotiondl.js:51-55` — caption. `result.error` yang `undefined` ikut jadi
/// teks `"undefined"`, bukan string kosong.
pub(crate) fn caption(hasil: &HasilDailymotion) -> String {
    format!(
        "🎬 *Dailymotion*\n\n\
         > 📌 {}\n\
         > ⏱️ Durasi: {}\n\
         > 📺 Kualitas: {}",
        hasil.title.as_deref().unwrap_or("undefined"),
        hasil.duration.as_deref().unwrap_or("undefined"),
        hasil.quality.as_deref().unwrap_or("undefined")
    )
}

/// `dailymotiondl.js:90` — `(result.title || "video").replace(/[<>:"\/\\|?*]/g, "")
/// + ".mp4"`. `||` bukan `??`: title kosong ikut jadi `"video"`.
pub(crate) fn nama_berkas(title: Option<&str>) -> String {
    let dasar = match title {
        Some(t) if !t.is_empty() => t,
        _ => "video",
    };
    format!("{}.mp4", scrub_nama_berkas(dasar))
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

    let hasil = match oktz_scraper::dailymotion::dailymotion_dl(ctx.http.as_ref(), text) {
        Ok(data) => hasil_dari(&data),
        // `dailymotiondl.js:98-102`.
        Err(_) => return Reply::text(GAGAL).with_reaction("☢"),
    };

    if !hasil.status {
        // `dailymotiondl.js:46-49` — `result.error` boleh `undefined`.
        let teks = hasil.error.as_deref().unwrap_or("undefined");
        return Reply::text(gagal_pakai(teks)).with_reaction("☢");
    }

    // `dailymotiondl.js:63` — tanpa `video` JS mengirim apa saja (thumbnail
    // saja), jadi di sini tidak ada pesan sama sekali.
    let Some(video) = hasil.video.as_deref() else {
        return Reply::text(GAGAL).with_reaction("☢");
    };

    // `dailymotiondl.js:84-94` — dokumen video, mengutip pesan pemicu.
    Reply::document(MediaSource::url(video), nama_berkas(hasil.title.as_deref()))
        .with_caption(caption(&hasil))
        .with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, HttpResponse, find};
    use serde_json::json;

    const SENDER: &str = "628111@s.whatsapp.net";

    /// Stub `HttpGet` yang membalas JSON tetap — scraper dan plugin diuji
    /// tanpa jaringan.
    #[derive(Debug, Default)]
    struct StubHttp {
        dipanggil: Mutex<Vec<String>>,
        balasan: String,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini pakai form, bukan JSON")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.dipanggil.lock().unwrap().push(req.url.clone());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn ctx_dengan(text: &str, balasan: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "dailymotiondl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http: Arc::new(StubHttp {
                dipanggil: Mutex::new(Vec::new()),
                balasan: balasan.to_string(),
            }),
            ..Default::default()
        }
    }

    fn run(text: &str) -> Reply {
        (find("dailymotiondl").unwrap().handler)(ctx_dengan(
            text,
            r#"{"medias":[]}"#,
        ))
    }

    fn reaction(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    /// Isi balasan sebenarnya: `without_mutations` hanya membuang `Mutate`, jadi
/// `Reaction { reply }` masih harus dibuka satu lapis.
    fn isi(r: &Reply) -> &Reply {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            lain => lain,
        }
    }

    /// Balasan apa pun yang bukan teks — media punya `media` terisi.
    fn media(r: &Reply) -> Option<&oktz_plugin_api::MediaSource> {
        match isi(r) {
            Reply::Document { media, .. }
            | Reply::Video { media, .. }
            | Reply::Audio { media, .. } => media.as_ref(),
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
            assert_eq!(
                r.without_mutations().text_body().expect("harus ada teks"),
                help("."),
                "input {text:?}"
            );
            assert_eq!(reaction(&r), Some("❌"), "input {text:?}");
        }
    }

    #[test]
    fn the_card_follows_the_prefix() {
        let mut ctx = ctx_dengan("", r#"{"medias":[]}"#);
        ctx.prefix = "!".into();
        let r = (find("dailymotiondl").unwrap().handler)(ctx);
        let teks = r.without_mutations().text_body().expect("teks");
        assert_eq!(teks, help("!"));
        assert!(teks.contains("> *!dailymotiondl <link>*"));
    }

    #[test]
    fn status_false_menampilkan_pesan_error_scraper() {
        let ctx = ctx_dengan(
            "https://dailymotion.com/video/x",
            r#"{"status":false,"error":"Tidak ada media yang tersedia"}"#,
        );
        let r = (find("dailymotiondl").unwrap().handler)(ctx);
        assert_eq!(
            r.without_mutations().text_body().expect("teks"),
            gagal_pakai("Tidak ada media yang tersedia")
        );
        assert_eq!(reaction(&r), Some("☢"));
    }

    #[test]
    fn scraper_gagal_adalah_cabang_catch() {
        // Stub menjawab 200 tapi `medias` kosong tanpa `status` → scraper
        // mengembalikan `{status:false, error}`; untuk menguji `catch`sebenarnya
        // dipakai respons yang bukan JSON di test terpisah di bawah.
        let ctx = ctx_dengan("https://dailymotion.com/video/x", "bukan json");
        let r = (find("dailymotiondl").unwrap().handler)(ctx);
        assert_eq!(r.without_mutations().text_body().expect("teks"), GAGAL);
        assert_eq!(reaction(&r), Some("☢"));
    }

    #[test]
    fn sukses_mengirim_dokumen_video_dengan_nama_dari_title() {
        let balasan = r#"{"status":true,"title":"Kucing Lucu","duration":"02:10",
            "quality":"720","video":"https://cdn/video.mp4","thumbnail":"https://cdn/t.jpg",
            "medias":[{"videoAvailable":true,"extension":"mp4","quality":"720","url":"https://cdn/video.mp4"}]}"#;
        let ctx = ctx_dengan("https://dailymotion.com/video/x", balasan);
        let r = (find("dailymotiondl").unwrap().handler)(ctx);
        assert_eq!(reaction(&r), Some("✅"), "reaksi sukses");
        match isi(&r) {
            Reply::Document {
                file_name, caption, ..
            } => {
                assert_eq!(file_name, "Kucing Lucu.mp4");
                assert_eq!(
                    caption,
                    "🎬 *Dailymotion*\n\n> 📌 Kucing Lucu\n> ⏱️ Durasi: 02:10\n> 📺 Kualitas: 720"
                );
            }
            lain => panic!("harus dokumen, dapat {:?}", lain),
        }
        assert!(media(&r).is_some(), "sumber media harus terisi");
    }

    #[test]
    fn hasil_dari_membaca_kunci_yang_hilang_sebagai_none() {
        let hasil = hasil_dari(&json!({"status": true, "video": "https://cdn/v.mp4"}));
        assert!(hasil.status);
        assert_eq!(hasil.video.as_deref(), Some("https://cdn/v.mp4"));
        assert_eq!(hasil.title, None);
        assert_eq!(hasil.quality, None);
        // Field hilang → `undefined` di caption, bukan string kosong.
        assert!(caption(&hasil).contains("> 📌 undefined"));
    }

    #[test]
    fn the_status_false_branch_prints_undefined_verbatim() {
        // `dailymotiondl.js:48` memakai `${result.error}`; field yang tidak ada
        // tercetak literal sebagai "undefined" di JS. Bukan string kosong.
        assert_eq!(
            gagal_pakai("Tidak ada media yang tersedia"),
            "❌ *Dailymotion Gagal*\n\n> Tidak ada media yang tersedia"
        );
        assert_eq!(gagal_pakai("undefined"), "❌ *Dailymotion Gagal*\n\n> undefined");
        assert_eq!(
            HasilDailymotion::default().error.as_deref().unwrap_or("undefined"),
            "undefined"
        );
    }

    #[test]
    fn the_file_name_scrubs_reserved_characters() {
        assert_eq!(nama_berkas(None), "video.mp4");
        assert_eq!(nama_berkas(Some("")), "video.mp4");
        assert_eq!(
            nama_berkas(Some("a<b>c:d\"e/f\\g|h?i*j")),
            "abcdefghij.mp4"
        );
        assert_eq!(nama_berkas(Some("video")), "video.mp4");
        assert_eq!(nama_berkas(Some("a b-c_d.e")), "a b-c_d.e.mp4");
        assert_eq!(scrub_nama_berkas("<>:\"/\\|?*"), "");
    }

    #[test]
    fn the_plugin_only_reads_and_never_mutates() {
        assert!(run("https://x/y").mutations().is_empty());
    }
}