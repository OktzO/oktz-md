//! `plugins/tools/wink.js` — tingkatkan video lewat `oktz_scraper::wink::wink_enhance`.
//! Ponytail: balasan proses intermediate dan nama berkas `wink-${Date.now()}` tidak
//! direplikasi (Ctx tidak punya epoch); reaksi 🕕 hilang, hanya `✅`/`❌` akhir.

use oktz_plugin_api::{Ctx, IncomingKind, MediaError, MediaSource, Reply, plugin};
use oktz_scraper::wink::{plan_poll_budget, wink_enhance, Rencana};

const MAX_BYTES: u64 = 50 * 1024 * 1024;

const HELP: &str = "✨ *ᴡɪɴᴋ ᴠɪᴅᴇᴏ ᴇɴʜᴀɴᴄᴇʀ*\n\n> Bikin video buram jadi *Ultra HD* pakai AI Wink!\n\n*Cara pakai:*\n> Kirim/reply video lalu caption `.wink`\n\n⚠️ _Fitur Premium, proses estimasi 1-4 menit._";
const GAGAL_UNDUH: &str = "❌ *GAGAL*\n\nVideonya gagal diunduh, coba kirim ulang ya!";
const TERLALU_BESAR: &str = "❌ *FILE TERLALU BESAR*\n\nMaksimal *50MB*";

fn ambil_video(ctx: &Ctx) -> Option<Result<Vec<u8>, MediaError>> {
    if let Some(q) = ctx.quoted()
        && q.kind.is_video()
    {
        return Some(ctx.download_quoted_media());
    }
    if ctx.is_media_kind(IncomingKind::Video) {
        return Some(ctx.download_media());
    }
    None
}

#[plugin(
    name = "wink",
    alias = ["winkenhance", "winkhd", "wenhance"],
    category = "tools",
    description = "Meningkatkan kualitas video menjadi Ultra HD dengan Wink AI",
    usage = ".wink (reply video)",
    example = ".wink",
    cooldown = 120,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn wink(ctx: Ctx) -> Reply {
    wink_dengan_rencana(ctx, &plan_poll_budget())
}

/// Sama dengan `wink` tetapi mengizinkan pengujian menentukan `Rencana`
/// (tanpa jeda polling agar uji tidak menunggu menit-menit).
pub(crate) fn wink_dengan_rencana(ctx: Ctx, rencana: &Rencana) -> Reply {
    let Some(hasil_unduh) = ambil_video(&ctx) else {
        return Reply::text(HELP);
    };
    let Ok(buffer) = hasil_unduh else {
        return Reply::text(GAGAL_UNDUH).with_reaction("❌");
    };
    if buffer.len() as u64 > MAX_BYTES {
        let mb = (buffer.len() as f64) / 1024.0 / 1024.0;
        return Reply::text(format!("{TERLALU_BESAR}\n\nvideo kamu *{mb:.1}MB*.")).with_reaction("❌");
    }

    let hasil = wink_enhance(&*ctx.http, &buffer, Some("wink-video.mp4"), rencana);
    match hasil {
        Ok(url) => Reply::video(MediaSource::url(url),
            "✨ *ᴡɪɴᴋ ᴇɴʜᴀɴᴄᴇ sᴇʟᴇsᴀɪ!*\n\n> Ini dia hasilnya, udah jadi *Ultra HD* kan? 😍").with_reaction("✅"),
        Err(e) => {
            let pesan = e.message;
            if pesan.contains("tidak selesai") {
                Reply::text("⏱️ *ᴘʀᴏsᴇs ᴋᴇʜᴀʟᴜ ᴛɪᴍᴇ ɴᴏᴜᴛ*\n\n> Wink tidak selesai dalam batas waktu, jadi tidak ada hasil yang bisa dikirim.\n> Energi kakak tidak terbuang percuma. Coba lagi dengan video yang lebih pendek ya.").with_reaction("❌")
            } else {
                let potong: String = pesan.chars().take(160).collect();
                Reply::text(format!("❌ *ᴡɪɴᴋ ᴇɴʜᴀɴᴄᴇ ɢᴀɢᴀʟ*\n\n> `{potong}`\n> Coba lagi nanti ya.")).with_reaction("❌")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall, HttpGet, HttpResponse, IncomingMedia, MediaFetch, MediaOrigin};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct HttpStub {
        balasan: Mutex<Vec<(u16, String)>>,
        seen: Mutex<Vec<String>>,
        hitung: Mutex<usize>,
    }

    impl HttpStub {
        fn urutan(daftar: Vec<(u16, &str)>) -> Arc<HttpStub> {
            Arc::new(HttpStub {
                balasan: Mutex::new(daftar.into_iter().map(|(s, t)| (s, t.to_string())).collect()),
                seen: Mutex::new(Vec::new()),
                hitung: Mutex::new(0),
            })
        }

        /// Balasan berikutnya dalam urutan, sekaligus mencatat URL.
        fn jawab(&self, url: &str) -> HttpResponse {
            self.seen.lock().unwrap().push(url.to_string());
            let mut h = self.hitung.lock().unwrap();
            let b = self.balasan.lock().unwrap();
            let i = (*h).min(b.len().saturating_sub(1));
            *h += 1;
            let (status, teks) = b[i].clone();
            HttpResponse { status, body: HttpBody::Bytes { data: Arc::from(teks.into_bytes()), mimetype: None }, headers: Vec::new() }
        }
    }

    impl HttpGet for HttpStub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            Ok(self.jawab(url))
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> { unimplemented!() }

        /// Scraper wink selalu lewat `HttpCall::call` (termasuk unggah
        /// multipart ke Qiniu, yang ditolak default trait) — produksi dilayani
        /// `UreqHttp::call`, jadi stub ini harus ikut menimpanya.
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            Ok(self.jawab(&req.url))
        }
    }

    // Tanpa `Default`: `Result<Vec<u8>, MediaError>` tidak punya `Default`.
    #[derive(Debug)]
    struct MediaStub(Result<Vec<u8>, MediaError>);
    impl MediaFetch for MediaStub {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> { self.0.clone() }
    }

    use oktz_plugin_api::HttpError;
    use serde_json::Value;

    fn tanpa_jeda() -> Rencana {
        Rencana { transcode_tries: 1, transcode_delay_ms: 0, result_tries: 1, result_delay_ms: 0 }
    }

    fn video_ctx(http: Arc<HttpStub>, unduh: Result<Vec<u8>, MediaError>) -> Ctx {
        Ctx {
            text: ".wink".into(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "wink".into(),
            media: Some(IncomingMedia { kind: IncomingKind::Video, mimetype: None, file_name: None, caption: String::new() }),
            media_fetch: Arc::new(MediaStub(unduh)),
            http,
            ..Ctx::default()
        }
    }

    fn run(http: Arc<HttpStub>, unduh: Result<Vec<u8>, MediaError>) -> Reply {
        wink_dengan_rencana(video_ctx(http, unduh), &tanpa_jeda())
    }

    fn video_url(r: &Reply) -> String {
        // `wink.js` membalas lewat `m.react("✅")` + `m.reply(video)`, jadi
        // balasan datang terbungkus Reaction.
        match r.without_mutations().tanpa_reaksi() {
            Reply::Video { media, .. } => match media {
                Some(MediaSource::Url { url, .. }) => url.clone(),
                m => panic!("harus URL video, dapat {m:?}"),
            },
            other => panic!("harus video, dapat {other:?}"),
        }
    }

    #[test]
    fn tidak_ada_video_tampilkan_bantuan() {
        let http = HttpStub::urutan(vec![]);
        let ctx = Ctx { text: ".wink".into(), args: Vec::new(), sender: "x".into(), chat: "x".into(), prefix: ".".into(), command: "wink".into(), http, ..Ctx::default() };
        let r = wink(ctx);
        assert!(r.without_mutations().text_body().unwrap().contains("ᴡɪɴᴋ ᴠɪᴅᴇᴏ ᴇɴʜᴀɴᴄᴇʀ"));
    }

    #[test]
    fn unduhan_gagal() {
        let http = HttpStub::urutan(vec![]);
        let r = run(http, Err(MediaError { message: "cdn mati".into() }));
        assert_eq!(r.without_mutations().text_body().unwrap(), GAGAL_UNDUH);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "❌"),
            other => panic!("harus reaksi ❌, dapat {other:?}"),
        }
    }

    #[test]
    fn berhasil_enhance_video() {
        let http = HttpStub::urutan(vec![
            (200, r#"{"code":0,"data":{"app":"A","count":3,"sig":"S","sig_time":"T","sig_version":"V","suffix":".mp4","type":"temp"}}"#),
            (200, r#"[{"qiniu":{"url":"https://upload.qiniu.com/","key":"K","token":"TOK"}}]"#),
            (200, r#"{"url":"https://cdn/asli.mp4"}"#),
            (200, r#"{"code":0,"data":{}}"#),
            (200, r#"{"code":0,"data":{"id":"T1"}}"#),
            (200, r#"{"code":0,"data":{"video":"https://cdn/asli.mp4","video_transcoded":"https://cdn/ultra.mp4"}}"#),
            (200, r#"{"code":0,"data":{"msg_id":"wpr_1"}}"#),
            (200, r#"{"code":0,"data":{"item_list":[{"result":{"error_code":0,"media_info_list":[{"media_data":"https://cdn/hasil.mp4"}]}}]}}"#),
        ]);
        let r = run(http.clone(), Ok(b"VIDEO".to_vec()));
        assert_eq!(video_url(&r), "https://cdn/hasil.mp4");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus reaksi ✅, dapat {other:?}"),
        }
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 8);
    }

    #[test]
    fn transcode_tidak_selesai() {
        let http = HttpStub::urutan(vec![
            (200, r#"{"code":0,"data":{"app":"A","count":3,"sig":"S","sig_time":"T","sig_version":"V","suffix":".mp4","type":"temp"}}"#),
            (200, r#"[{"qiniu":{"url":"https://upload.qiniu.com/","key":"K","token":"TOK"}}]"#),
            (200, r#"{"url":"https://cdn/asli.mp4"}"#),
            (200, r#"{"code":0,"data":{}}"#),
            (200, r#"{"code":0,"data":{"id":"T1"}}"#),
            (200, r#"{"code":0,"data":{"video":"https://cdn/asli.mp4","video_transcoded":"https://cdn/asli.mp4"}}"#),
        ]);
        let r = run(http, Ok(b"VIDEO".to_vec()));
        let t = r.without_mutations().text_body().unwrap();
        assert!(t.contains("kehulu") || t.contains("waktu"), "{t}");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "❌"),
            other => panic!("harus reaksi ❌, dapat {other:?}"),
        }
    }
}
