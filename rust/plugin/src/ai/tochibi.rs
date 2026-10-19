//! `plugins/ai/tochibi.js` — ubah gambar ke style Chibi lewat `oktz_scraper::seaart::live3d`.
//! Ponytail: `m.react('🕕')` hilang karena `Reply` hanya satu reaksi; hanya `✅`/`❌`/`☢` akhir.

use oktz_plugin_api::{Ctx, IncomingKind, MediaError, MediaSource, Reply, plugin};

const PROMPT: &str = "Transform into chibi style, big head and small body proportions, cute expression, big sparkling eyes, smooth shading, soft lighting, highly detailed, high quality";

const HELP: &str = "🎀 *ᴄʜɪʙɪ sᴛʏʟᴇ*\n\n> Kirim/reply gambar untuk diubah ke style Chibi\n\n`.tochibi`";
const GAGAL_DOWNLOAD: &str = "❌ Gagal mendownload gambar";

/// Unduh gambar dari quoted atau pesan pemicu. `None` = tidak ada gambar sama sekali
/// (JS menampilkan kartu pakai), `Some(Err)` = gambar ada tapi unduhan gagal.
fn ambil_gambar(ctx: &Ctx) -> Option<Result<Vec<u8>, MediaError>> {
    if let Some(q) = ctx.quoted()
        && q.kind.is_image()
    {
        return Some(ctx.download_quoted_media());
    }
    if ctx.is_media_kind(IncomingKind::Image) {
        return Some(ctx.download_media());
    }
    None
}

#[plugin(
    name = "tochibi",
    alias = ["chibi", "chibistyle"],
    category = "ai",
    description = "Ubah gambar ke style Chibi",
    usage = ".tochibi (reply gambar)",
    example = ".tochibi",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tochibi(ctx: Ctx) -> Reply {
    let Some(hasil_unduh) = ambil_gambar(&ctx) else {
        return Reply::text(HELP);
    };
    let Ok(buffer) = hasil_unduh else {
        return Reply::text(GAGAL_DOWNLOAD).with_reaction("❌");
    };
    let gambar = oktz_scraper::seaart::live3d(&*ctx.http, &buffer, PROMPT);
    match gambar {
        Ok(bytes) => Reply::image(MediaSource::bytes(bytes), String::new()).with_reaction("✅"),
        Err(_) => Reply::text(
            "☢ Kayaknya command `.tochibi` lagi ada kendala\nSilahkan coba lagi nanti, User\n\n_Jika masalah berlanjut, silahkan hubungi owner bot_"
        ).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpGet, HttpResponse, IncomingMedia, MediaFetch, MediaOrigin};
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
    }

    impl HttpGet for HttpStub {
        /// `live3d` memakai `HttpCall` (PUT byte mentah + multipart), jadi
        /// `call` diarahkan ke balasan berurutan yang sama seperti `get`.
        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.get(&req.url)
        }
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let mut h = self.hitung.lock().unwrap();
            let b = self.balasan.lock().unwrap();
            let i = (*h).min(b.len().saturating_sub(1));
            *h += 1;
            drop(h);
            drop(b);
            let (status, teks) = self.balasan.lock().unwrap()[i].clone();
            Ok(HttpResponse { status, body: HttpBody::Bytes { data: Arc::from(teks.into_bytes()), mimetype: None }, headers: Vec::new() })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> { unimplemented!() }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> { unimplemented!() }
    }

    /// Stub `MediaFetch` yang mengembalikan byte unduhan yang ditentukan.
    #[derive(Debug)]
    struct MediaStub(Result<Vec<u8>, MediaError>);
    impl MediaFetch for MediaStub {
        fn download(&self, _: MediaOrigin) -> Result<Vec<u8>, MediaError> { self.0.clone() }
    }

    use oktz_plugin_api::HttpError;
    use serde_json::Value;

    fn media_ctx(http: Arc<HttpStub>, unduh: Result<Vec<u8>, MediaError>) -> Ctx {
        Ctx {
            text: ".tochibi".into(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "tochibi".into(),
            media: Some(IncomingMedia { kind: IncomingKind::Image, mimetype: None, file_name: None, caption: String::new() }),
            media_fetch: Arc::new(MediaStub(unduh)),
            http,
            ..Ctx::default()
        }
    }

    fn run(http: Arc<HttpStub>, unduh: Result<Vec<u8>, MediaError>) -> Reply {
        tochibi(media_ctx(http, unduh))
    }

    fn image_bytes(r: &Reply) -> Vec<u8> {
        // JS `m.react('✅')` (`tochibi.js:47`) membungkus balasan jadi
        // `Reaction`; lepas dulu supaya isi gambarnya bisa diperiksa.
        match r.without_mutations().tanpa_reaksi() {
            Reply::Image { media, .. } => match media {
                Some(MediaSource::Bytes { data, .. }) => data.to_vec(),
                m => panic!("harus bytes, dapat {m:?}"),
            },
            other => panic!("harus image, dapat {other:?}"),
        }
    }

    #[test]
    fn tidak_ada_gambar_tampilkan_bantuan() {
        let http = HttpStub::urutan(vec![]);
        let ctx = Ctx { text: ".tochibi".into(), args: Vec::new(), sender: "x".into(), chat: "x".into(), prefix: ".".into(), command: "tochibi".into(), http, ..Ctx::default() };
        let r = tochibi(ctx);
        assert_eq!(r.without_mutations().text_body().unwrap(), HELP);
    }

    #[test]
    fn unduhan_gagal() {
        let http = HttpStub::urutan(vec![]);
        let r = run(http, Err(MediaError { message: "cdn mati".into() }));
        assert_eq!(r.without_mutations().text_body().unwrap(), GAGAL_DOWNLOAD);
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "❌"),
            other => panic!("harus reaksi ❌, dapat {other:?}"),
        }
    }

    #[test]
    fn berhasil_ubah_ke_chibi() {
        let http = HttpStub::urutan(vec![
            (200, r#"{"result":{"url":"https://u/x","object_name":"o/1"}}"#),
            (200, ""),
            (200, r#"{"result":{"url":"https://s/x.jpg"}}"#),
            (200, r#"{"status":true,"data":{"result_url":"https://h/y.png"}}"#),
            (200, "GAMBAR"),
        ]);
        let r = run(http, Ok(b"IMG".to_vec()));
        assert_eq!(image_bytes(&r), b"GAMBAR");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "✅"),
            other => panic!("harus reaksi ✅, dapat {other:?}"),
        }
    }

    #[test]
    fn nanobanana_gagal() {
        let http = HttpStub::urutan(vec![
            (200, r#"{"result":{"url":"https://u/x","object_name":"o/1"}}"#),
            (200, ""),
            (200, r#"{"result":{"url":"https://s/x.jpg"}}"#),
            (200, r#"{"status":false}"#),
        ]);
        let r = run(http, Ok(b"IMG".to_vec()));
        assert!(r.without_mutations().text_body().unwrap().contains("lagi ada kendala"));
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "☢"),
            other => panic!("harus reaksi ☢, dapat {other:?}"),
        }
    }
}
