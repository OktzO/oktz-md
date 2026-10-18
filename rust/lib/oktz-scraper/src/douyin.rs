//! `src/scraper/douyin.js` — POST ke `snapvideotools.com/api/snap`, lalu ambil
//! tautan video dan audio.
//!
//! Yang ikut disalin dari JS:
//! - `response.data.data` — respons dibungkus satu tingkat, bukan `data` langsung.
//! - `if (!data || !data.mediaUrls)` → satu pesan untuk dua kondisi. Array
//!   kosong TIDAK falsy di JS (`[]` lolos), jadi hasilnya `video`/`audio` `null`,
//!   bukan objek error.
//! - `find(m => m.type === "video")` → yang PERTAMA cocok, urutan asli.
//! - `platform: data.platformName` → nama field di respons (`platformName`) dan
//!   nama kunci di hasil (`platform`) BERBEDA; yang ditulis ulang ke `platform`.
//! - `video ? video.url : null` → `null` kalau tidak ketemu; kalau ketemu tapi
//!   `url` tidak ada, kuncinya hilang dari JSON seperti `JSON.stringify`.
//!
//! Yang tidak bisa ikut: `signal` dari pemanggil (AbortController tidak ada di
//! Rust) dan batas 15 detik/25MB milik `httpAxios` — lihat laporan.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

/// `DouyinDL(douyinUrl, { signal })` (`douyin.js:11-48`).
pub fn douyin_dl(http: &dyn HttpGet, douyin_url: &str) -> Result<Value, HttpError> {
    let res = http.call(
        &HttpCall::post("https://snapvideotools.com/api/snap")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .header("X-Requested-With", "XMLHttpRequest")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Referer", "https://snapvideotools.com/")
            .header("Origin", "https://snapvideotools.com")
            .json(&json!({ "text": douyin_url })),
    )?;

    let body = res.json().unwrap_or(Value::Null);
    // `response.data.data` — `Value::get` sudah `None` untuk respons non-objek.
    let data = body.get("data").unwrap_or(&Value::Null);

    let Some(media_urls) = data.get("mediaUrls").filter(|v| truthy(v)) else {
        return Ok(json!({
            "status": false,
            "error": "Data tidak ditemukan",
        }));
    };
    let Some(media_urls) = media_urls.as_array() else {
        // `data.mediaUrls.find(...)` pada nilai non-array melempar TypeError,
        // dan di JS tidak ada `catch` di fungsi ini.
        return Err(HttpError {
            message: "data.mediaUrls.find is not a function".to_string(),
        });
    };

    let cari = |jenis: &str| {
        media_urls
            .iter()
            .find(|m| m.get("type").and_then(Value::as_str) == Some(jenis))
    };

    let mut out = Map::new();
    out.insert("status".to_string(), Value::Bool(true));
    // `title`/`platformName` yang `undefined` tidak ikut di JSON.stringify.
    // Kunci di hasil mengikuti nama properti objek yang di-`return` JS, jadi
    // `data.platformName` ditulis sebagai `platform`.
    for (kunci, sumber) in [("title", "title"), ("platform", "platformName")] {
        if let Some(v) = data.get(sumber) {
            out.insert(kunci.to_string(), v.clone());
        }
    }
    for kunci in ["video", "audio"] {
        match cari(kunci) {
            Some(m) => {
                // `m.url` undefined → kuncinya hilang, bukan `null`.
                if let Some(url) = m.get("url") {
                    out.insert(kunci.to_string(), url.clone());
                }
            }
            None => {
                out.insert(kunci.to_string(), Value::Null);
            }
        }
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan, membalas body tetap.
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: String,
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(
            &self,
            req: &oktz_plugin_api::HttpCall,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(balasan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: balasan.to_string(),
        }
    }

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn mengambil_video_dan_audio_dari_media_urls() {
        let http = stub(
            r#"{"data":{"title":"Kucing","platformName":"douyin","mediaUrls":[
                 {"type":"audio","url":"https://a/a.mp3"},
                 {"type":"video","url":"https://a/v.mp4"}
               ]}}"#,
        );
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["title"], "Kucing");
        assert_eq!(hasil["platform"], "douyin");
        // Audio ada di indeks 0, video di indeks 1 — `find` tidak mengambil
        // yang pertama tanpa syarat.
        assert_eq!(hasil["video"], "https://a/v.mp4");
        assert_eq!(hasil["audio"], "https://a/a.mp3");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://snapvideotools.com/api/snap");
        let body = match &p[0].body {
            Some(oktz_plugin_api::HttpBody::Bytes { data, mimetype }) => {
                assert_eq!(mimetype.as_deref(), Some("application/json"));
                serde_json::from_slice::<Value>(data).expect("body JSON")
            }
            lain => panic!("body lain: {lain:?}"),
        };
        assert_eq!(body, json!({"text": "https://v.douyin.com/x/"}));
        assert_eq!(
            cari_header(&p[0], "accept"),
            Some("application/json, text/javascript, */*; q=0.01".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "x-requested-with"),
            Some("XMLHttpRequest".to_string())
        );
        assert_eq!(cari_header(&p[0], "referer"), Some("https://snapvideotools.com/".to_string()));
        assert_eq!(cari_header(&p[0], "origin"), Some("https://snapvideotools.com".to_string()));
    }

    #[test]
    fn data_hilang_atau_media_urls_kosong_kosong_memberi_pesan_yang_sama() {
        // Tanpa `data`.
        let http = stub(r#"{"ok":true}"#);
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Data tidak ditemukan");

        // `mediaUrls` hilang.
        let http = stub(r#"{"data":{"title":"Kucing"}}"#);
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Data tidak ditemukan");

        // `mediaUrls` falsy (null) — `[]` TIDAK falsy, lihat test berikutnya.
        let http = stub(r#"{"data":{"mediaUrls":null}}"#);
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], false);
    }

    #[test]
    fn media_urls_kosong_tetap_sukses_dengan_video_audio_null() {
        let http = stub(r#"{"data":{"mediaUrls":[]}}"#);
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["video"], Value::Null);
        assert_eq!(hasil["audio"], Value::Null);
        // `data.title` tidak ada → kuncinya tidak dibangkitkan.
        assert!(hasil.get("title").is_none());
        assert!(hasil.get("platform").is_none());
    }

    #[test]
    fn url_yang_tidak_ada_dan_media_urls_bukan_array() {
        // Elemen video tanpa `url` → kunci hilang, bukan `null`.
        let http = stub(r#"{"data":{"mediaUrls":[{"type":"video"}]}}"#);
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], true);
        assert!(hasil.get("video").is_none());
        assert_eq!(hasil["audio"], Value::Null);

        // `mediaUrls` objek: JS memanggil `.find` dan melempar TypeError.
        let http = stub(r#"{"data":{"mediaUrls":{"a":1}}}"#);
        let salah = douyin_dl(&http, "https://v.douyin.com/x/").unwrap_err();
        assert_eq!(salah.message, "data.mediaUrls.find is not a function");
    }

    #[test]
    fn respons_bukan_json_tetap_gagal_comme_js() {
        let http = stub("<html>cloudflare</html>");
        let hasil = douyin_dl(&http, "https://v.douyin.com/x/").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Data tidak ditemukan");
    }
}