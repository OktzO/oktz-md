//! `src/scraper/hd.js` — unggah berkas ke `photoai.imglarger.com` (dua panggilan).
//!
//! Yang disalin dari JS:
//! - `fs.createReadStream(filePath)` + `form.append("file", ...)` — `form-data`
//!   memakai `path.basename(value.path)` sebagai `filename` dan
//!   `mime.lookup(value.path)` sebagai content-type part berkas.
//! - `form.append("type", 13)` dan `form.append("scaleRadio", 2)` — angka jadi
//!   string ("13", "2"), urutan part: file, type, scaleRadio.
//! - `return data.data;` — satu amplop `data` dari axios dibuka; kalau `data`
//!   tidak ada, JS mengembalikan `undefined` (di sini `Null`).
//!
//! `httpAxios` (`src/lib/http.js`) memasang timeout 15 detik & maxRedirects 3;
//! itu batas mesin, bukan tampilan, jadi tidak ditiru (klien Rust punya timeout
//! global sendiri).

use std::sync::Arc;

use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, MultipartPart};
use serde_json::Value;

const UPLOAD_URL: &str = "https://photoai.imglarger.com/api/PhoAi/Upload";
const STATUS_URL: &str = "https://photoai.imglarger.com/api/PhoAi/CheckStatus";
const UA: &str = "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Mobile Safari/537.36";

/// `upload(filePath)` (`hd.js:4-33`) — multipart ke `PhoAi/Upload`.
pub fn upload(http: &dyn HttpGet, path: &str) -> Result<Value, HttpError> {
    let data = std::fs::read(path).map_err(|e| HttpError {
        message: e.to_string(),
    })?;
    // `path.basename(value.path)` dari stream `fs.createReadStream`.
    let nama_berkas = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_string);
    let mime = nama_berkas
        .as_deref()
        .map(mime_dari_nama)
        .unwrap_or("application/octet-stream");

    let parts = vec![
        MultipartPart {
            name: "file".to_string(),
            file_name: nama_berkas,
            mimetype: Some(mime.to_string()),
            body: HttpBody::Bytes {
                data: Arc::from(data),
                mimetype: None,
            },
        },
        MultipartPart::field("type", "13"),
        MultipartPart::field("scaleRadio", "2"),
    ];

    // `...form.getHeaders()` tidak disalin: boundary dibuat klien Rust, dan
    // content-type pemanggil dibuang untuk body multipart.
    let res = http.call(
        &HttpCall::post(UPLOAD_URL)
            .header("accept", "application/json, text/plain, */*")
            .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("origin", "https://imglarger.com")
            .header("priority", "u=1, i")
            .header("referer", "https://imglarger.com/")
            .header(
                "sec-ch-ua",
                r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#,
            )
            .header("sec-ch-ua-mobile", "?1")
            .header("sec-ch-ua-platform", r#""Android""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("user-agent", UA)
            .multipart(parts),
    )?;
    Ok(amplop(res.json()))
}

/// `get(code)` (`hd.js:35-61`) — `PhoAi/CheckStatus`.
pub fn get(http: &dyn HttpGet, code: &str) -> Result<Value, HttpError> {
    let res = http.call(
        &HttpCall::post(STATUS_URL)
            .header("accept", "application/json, text/plain, */*")
            .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("content-type", "application/json")
            .header("origin", "https://imglarger.com")
            .header("priority", "u=1, i")
            .header("referer", "https://imglarger.com/")
            .header(
                "sec-ch-ua",
                r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#,
            )
            .header("sec-ch-ua-mobile", "?1")
            .header("sec-ch-ua-platform", r#""Android""#)
            .header("sec-fetch-dest", "empty")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-site", "same-site")
            .header("user-agent", UA)
            .json(&serde_json::json!({ "code": code, "type": 13 })),
    )?;
    Ok(amplop(res.json()))
}

/// `data.data` — `undefined` kalau tidak ada (di sini `Null`).
fn amplop(body: Option<Value>) -> Value {
    body.and_then(|v| v.get("data").cloned())
        .unwrap_or(Value::Null)
}

/// `mime.lookup` ala `mime-types` untuk ekstensi yang lazim diunggah fitur
/// HD; ekstensi lain jatuh ke `application/octet-stream` seperti bawaan
/// `form-data` (basis data `mime-types` lengkap tidak tersedia di crate ini).
fn mime_dari_nama(nama: &str) -> &'static str {
    let ext = nama
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" | "jpe" | "jfif" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "ico" => "image/vnd.microsoft.icon",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "avif" => "image/avif",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<VecDeque<String>>,
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
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak form")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn call(
            &self,
            req: &oktz_plugin_api::HttpCall,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut q = self.balasan.lock().unwrap();
            let body = if q.len() > 1 {
                q.pop_front().unwrap()
            } else {
                q.front().cloned().unwrap_or_default()
            };
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: Arc::from(body.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(balasan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(VecDeque::from([balasan.to_string()])),
        }
    }

    /// Berkas sementara untuk `fs.createReadStream`/`readFileSync` di JS.
    fn berkas_sementara(nama: &str, isi: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("oktz-hd-{}-{nama}", std::process::id()));
        std::fs::write(&path, isi).unwrap();
        path
    }

    #[test]
    fn upload_mengirim_multipart_dengan_nama_berkas_dan_mime() {
        let path = berkas_sementara("foto.png", &[1, 2, 3]);
        let http = stub(r#"{"data":{"code":"c9"}}"#);
        let hasil = upload(&http, path.to_str().unwrap()).unwrap();
        assert_eq!(hasil["code"], "c9");
        let _ = std::fs::remove_file(&path);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://photoai.imglarger.com/api/PhoAi/Upload");
        // Urutan part & isi persis seperti `form.append` berurutan di JS.
        assert_eq!(
            p[0].body,
            Some(oktz_plugin_api::HttpBody::Multipart(vec![
                MultipartPart {
                    name: "file".to_string(),
                    file_name: Some(format!("oktz-hd-{}-foto.png", std::process::id())),
                    mimetype: Some("image/png".to_string()),
                    body: HttpBody::Bytes {
                        data: Arc::from(vec![1u8, 2, 3]),
                        mimetype: None,
                    },
                },
                MultipartPart::field("type", "13"),
                MultipartPart::field("scaleRadio", "2"),
            ]))
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("origin"), Some("https://imglarger.com".to_string()));
        assert_eq!(cari("referer"), Some("https://imglarger.com/".to_string()));
        assert_eq!(cari("user-agent"), Some(UA.to_string()));
        assert_eq!(
            cari("sec-ch-ua").unwrap(),
            r#""Chromium";v="142", "Google Chrome";v="142", "Not_A Brand";v="99""#
        );
        assert_eq!(cari("accept-language").unwrap(), "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7");
    }

    #[test]
    fn upload_data_hilang_mengembalikan_null() {
        let path = berkas_sementara("h2.jpg", &[9]);
        let http = stub(r#"{"ok":true}"#);
        assert_eq!(upload(&http, path.to_str().unwrap()).unwrap(), Value::Null);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn upload_berkas_tidak_ada_melempar() {
        let http = stub("{}");
        assert!(upload(&http, "/tmp/oktz-tidak-ada/xx.png").is_err());
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn get_mengirim_json_code_dan_type() {
        let http = stub(r#"{"data":{"status":"success","downloadUrls":["https://x"]}}"#);
        let hasil = get(&http, "c1").unwrap();
        assert_eq!(hasil["status"], "success");
        assert_eq!(hasil["downloadUrls"][0], "https://x");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, "https://photoai.imglarger.com/api/PhoAi/CheckStatus");
        assert_eq!(p[0].body.as_ref().unwrap().bytes(), br#"{"code":"c1","type":13}"#);
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("content-type"), Some("application/json".to_string()));
        assert_eq!(cari("accept"), Some("application/json, text/plain, */*".to_string()));
    }
}
