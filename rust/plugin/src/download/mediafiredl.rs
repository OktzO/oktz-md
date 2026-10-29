//! `plugins/download/mediafiredl.js` — unduh file dari MediaFire.
//!
//! Scraper `mediafire` (`src/scraper/mediafire.js`) sudah ada di crate
//! `oktz-scraper` (`oktz_scraper::mediafire::mediafire`): GET halaman
//! MediaFire, ekstraksi meta + tombol unduh, dan tebakan MIME dari ekstensi.
//! Yang dipindah di sini SEMUA logika plugin: guard URL, kartu pakai, dan
//! `getFileName` beserta `decodeURIComponent`-nya.
//!
//! Dua penyimpitan yang tercatat apa adanya:
//!
//! - **`forwardingScore`/`isForwarded` tidak ikut.** JS menandai dokumen
//!   sebagai pesan terus-terusan (`mediafiredl.js:60-63`); `Reply` tidak punya
//!   field forwarding.
//! - **Reaksi `🕕` tetap menyala di jalur sukses.** JS bereaksi `🕕` sekali di
//!   awal lalu tidak bereaksi lagi kalau dokumen terkirim (`mediafiredl.js:46`),
//!   jadi reaksi terakhir yang dipakai plugin ini memang `🕕`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::now_ms;

/// `mediafiredl.js:43` — `/mediafire\.com/i`, tidak dikunci ke host.
pub(crate) fn is_mediafire_url(url: &str) -> bool {
    url.to_lowercase().contains("mediafire.com")
}

/// `mediafiredl.js:35-40` — kartu pakai, delimiter markdown-nya backtick.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}mfdl <url>`\n\n\
         > Contoh:\n\
         > `{prefix}mfdl https://www.mediafire.com/file/xxx`"
    )
}

/// `mediafiredl.js:44` — seluruh pesan dibungkus asterisk, termasuk `link`/`File`.
pub(crate) const BAD_URL: &str = "❌ *URL tidak valid. Gunakan link MediaFire.*";

/// `mediafiredl.js:52` — respons tanpa `download.link_download`.
pub(crate) const NO_LINK: &str =
    "❌ Gagal mendapatkan link download. File mungkin tidak tersedia.";

/// Bagian hasil `mediafire` (`src/scraper/mediafire.js:72-82`) yang dibaca
/// plugin: `meta.title`, `download.link_download`, dan `download.mimetype`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediafireFile {
    pub title: String,
    pub link_download: String,
    pub mimetype: String,
}

/// Baca hasil scraper. Kunci yang tidak ada berarti string kosong di JS juga,
/// karena scraper selalu mengisinya dengan `""` kalau meta tidak ada.
fn hasil_dari(data: &Value) -> MediafireFile {
    let teks = |path: &str| {
        data.pointer(path)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    MediafireFile {
        title: teks("/meta/title"),
        link_download: teks("/download/link_download"),
        mimetype: teks("/download/mimetype"),
    }
}

/// `decodeURIComponent` — hanya `%XX`, spasi tidak disentuh (beda dari
/// `decodeURIComponent` versi form). Rangkaian `%` yang tidak sah dibiarkan
/// apa adanya; di JS itu melempar `URIError`, yang tidak akan pernah terjadi
/// karena string-nya berasal dari URL MediaFire.
///
/// Argumen byte per byte lalu didekode UTF-8 sekaligus: `from_utf8_lossy`
/// satu kali menjaga urutan byte tetap sama seperti `decodeURIComponent`.
pub(crate) fn decode_uri_component(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `mediafiredl.js:20-29` — `getFileName`. Argumen `now_ms` menggantikan
/// `Date.now()` supaya cabang cadangan bisa diuji.
pub(crate) fn get_file_name(result: &MediafireFile, now_ms: i64) -> String {
    let direct_name = result.title.trim();
    let url_name = decode_uri_component(
        result
            .link_download
            .split('/')
            .next_back()
            .map(|s| s.split('?').next().unwrap_or(""))
            .unwrap_or(""),
    );
    let extension = if url_name.contains('.') {
        format!(".{}", url_name.rsplit('.').next().unwrap_or(""))
    } else {
        String::new()
    };
    if !direct_name.is_empty() && !extension.is_empty() && !direct_name.contains('.') {
        return format!("{direct_name}{extension}");
    }
    if !direct_name.is_empty() {
        return direct_name.to_string();
    }
    if !url_name.is_empty() {
        return url_name;
    }
    format!("mediafire_{now_ms}{extension}")
}

#[plugin(
    name = "mediafiredl",
    alias = ["mfdl", "mediafire", "mf"],
    category = "download",
    description = "Download file dari MediaFire",
    usage = ".mfdl <url>",
    example = ".mfdl https://www.mediafire.com/file/xxx",
    cooldown = 15,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn mediafiredl(ctx: Ctx) -> Reply {
    let url = ctx.text.trim();
    if url.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    if !is_mediafire_url(url) {
        return Reply::text(BAD_URL);
    }

    // `mediafire.js:69` melempar saat `link_download` kosong, jadi scraper itu
    // tidak pernah sampai ke pemeriksaan `mediafiredl.js:50`.
    let Ok(value) = oktz_scraper::mediafire::mediafire(ctx.http.as_ref(), url) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("🕕");
    };
    let result = hasil_dari(&value);
    if result.link_download.is_empty() {
        return Reply::text(NO_LINK).with_reaction("❌");
    }

    // `mediafiredl.js:54-66` — `sendMessage` document dengan URL, mimetype dari
    // scraper, dan nama dari `getFileName`.
    Reply::document(
        MediaSource::url(result.link_download.clone()).with_mimetype(result.mimetype.clone()),
        get_file_name(&result, now_ms()),
    )
    .with_reaction("🕕")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const SENDER: &str = "628111@s.whatsapp.net";

    /// Stub `HttpGet` yang membalas satu halaman HTML tetap — scraper dan
    /// plugin diuji tanpa jaringan.
    #[derive(Debug, Default)]
    struct StubHttp {
        dipanggil: Mutex<Vec<HttpCall>>,
        halaman: String,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.dipanggil.lock().unwrap().push(req.clone());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.halaman.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(halaman: &str) -> Arc<StubHttp> {
        Arc::new(StubHttp {
            dipanggil: Mutex::new(Vec::new()),
            halaman: halaman.to_string(),
        })
    }

    fn run(text: &str) -> Reply {
        (oktz_plugin_api::find("mediafiredl").unwrap().handler)(ctx_http(stub(""), text))
    }

    fn ctx_http(http: Arc<StubHttp>, text: &str) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "mfdl".into(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            text: text.into(),
            args: vec![text.into()],
            http,
            ..Default::default()
        }
    }

    fn ctx_dengan(text: &str, halaman: &str) -> Ctx {
        ctx_http(stub(halaman), text)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Isi balasan sebenarnya: `without_mutations` hanya membuang `Mutate`,
    /// jadi `Reaction { reply }` masih harus dibuka satu lapis.
    fn isi(r: &Reply) -> &Reply {
        match r.without_mutations() {
            Reply::Reaction { reply, .. } => reply.as_ref(),
            lain => lain,
        }
    }

    /// Halaman MediaFire yang punya meta dan tombol unduh.
    const HALAMAN: &str = r#"<html><head>
         <meta property="og:title" content="Contoh Video">
         <meta property="og:image" content="https://static.mediafire.com/t.png">
       </head><body>
         <a id="downloadButton" href="https://download.mediafire.com/a%20video.mp4?k=1">Download (12.5 MB)</a>
       </body></html>"#;

    fn file(title: &str, link: &str) -> MediafireFile {
        MediafireFile {
            title: title.to_string(),
            link_download: link.to_string(),
            mimetype: "video/mp4".to_string(),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("mediafiredl").unwrap();
        assert_eq!(p.meta.name, "mediafiredl");
        assert_eq!(p.meta.alias, ["mfdl", "mediafire", "mf"]);
        assert_eq!(p.meta.category, "download");
        assert_eq!(p.meta.description, "Download file dari MediaFire");
        assert_eq!(p.meta.usage, ".mfdl <url>");
        assert_eq!(p.meta.example, ".mfdl https://www.mediafire.com/file/xxx");
        assert_eq!(p.meta.cooldown, 15);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mfdl", "mediafire", "mf"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mediafiredl");
        }
    }

    #[test]
    fn an_empty_argument_shows_the_usage_card() {
        for text in ["", "   "] {
            assert_eq!(text_of(&run(text)), help("."), "input {text:?}");
        }
    }

    #[test]
    fn a_url_without_the_domain_is_rejected() {
        assert_eq!(text_of(&run("https://drive.google.com/x")), BAD_URL);
    }

    #[test]
    fn the_unanchored_regex_quirk_of_the_js_is_kept() {
        assert!(is_mediafire_url("https://mediafire.com.evil.example/x"));
        assert!(is_mediafire_url("https://notmediafire.com/x"));
        assert!(is_mediafire_url("https://WWW.MEDIAFIRE.COM/file/x"));
    }

    #[test]
    fn decode_uri_component_handles_utf8_and_leaves_bad_escapes_alone() {
        assert_eq!(decode_uri_component("a%20b"), "a b");
        assert_eq!(decode_uri_component("nama%20file%2Ezip"), "nama file.zip");
        assert_eq!(decode_uri_component("caf%C3%A9.txt"), "café.txt");
        assert_eq!(decode_uri_component("100%"), "100%");
        assert_eq!(decode_uri_component("%zz"), "%zz");
        assert_eq!(decode_uri_component("%2"), "%2");
        assert_eq!(decode_uri_component("+a+"), "+a+", "spasi tidak jadi plus");
    }

    #[test]
    fn the_file_name_prefers_the_page_title_plus_the_extension() {
        assert_eq!(
            get_file_name(&file("Contoh Video", "https://d.example/a%20video.mp4?k=1"), 0),
            "Contoh Video.mp4"
        );
        // `directName` yang sudah punya titik: pakai apa adanya, ekstensi URL diabaikan.
        assert_eq!(
            get_file_name(&file("arsip.tar.gz", "https://d.example/a.zip"), 0),
            "arsip.tar.gz"
        );
        // Tanpa titik pada title tapi URL juga tanpa titik: tidak ada ekstensi.
        assert_eq!(
            get_file_name(&file("Contoh Video", "https://d.example/download"), 0),
            "Contoh Video"
        );
    }

    #[test]
    fn the_file_name_falls_back_to_the_url_then_to_a_timestamp() {
        assert_eq!(
            get_file_name(&file("", "https://d.example/VIDEO.mp4?k=1"), 0),
            "VIDEO.mp4"
        );
        assert_eq!(
            get_file_name(&file("   ", "https://d.example/"), 1_700_000_000_000),
            "mediafire_1700000000000",
            "tanpa nama dan tanpa ekstensi: pakai stempel waktu"
        );
    }

    #[test]
    fn the_scraper_failure_branch_is_the_error_template() {
        // Halaman tanpa tombol unduh → `mediafire.js:69` melempar, persis yang
        // terjadi kalau parsing HTML gagal di Node.
        let r = (oktz_plugin_api::find("mediafiredl").unwrap().handler)(ctx_dengan(
            "https://www.mediafire.com/file/xxx",
            "<html><body>kosong</body></html>",
        ));
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "mfdl"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "reaksi jam harus tetap ada di cabang catch"
        );
    }

    #[test]
    fn a_parsed_page_is_sent_as_a_document_named_by_get_file_name() {
        let http = stub(HALAMAN);
        let r = (oktz_plugin_api::find("mediafiredl").unwrap().handler)(ctx_http(http.clone(), "https://www.mediafire.com/file/xxx"));
        assert!(
            matches!(r.without_mutations(), Reply::Reaction { emoji, .. } if emoji == "🕕"),
            "reaksi jam tetap jadi reaksi terakhir di jalur sukses"
        );
        match isi(&r) {
            Reply::Document {
                file_name,
                caption,
                media,
                ..
            } => {
                assert_eq!(file_name, "Contoh Video.mp4");
                assert_eq!(caption, "", "sendMessage tidak memakai caption");
                let src = media.as_ref().expect("sumber media harus terisi");
                assert_eq!(
                    src,
                    &oktz_plugin_api::MediaSource::Url {
                        url: "https://download.mediafire.com/a%20video.mp4?k=1".to_string(),
                        mimetype: Some("video/mp4".to_string()),
                    }
                );
            }
            lain => panic!("harus dokumen, dapat {lain:?}"),
        }
        // Scraper benar-benar dipanggil dengan URL yang diketik user.
        let req = http.dipanggil.lock().unwrap();
        assert_eq!(req.len(), 1);
        assert_eq!(req[0].url, "https://www.mediafire.com/file/xxx");
    }
}