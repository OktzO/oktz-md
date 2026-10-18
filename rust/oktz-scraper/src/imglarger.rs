//! `src/scraper/imglarger.js` — upscaler imgupscaler.com (`get1.imglarger.com`).
//!
//! Yang ikut disalin dari JS:
//! - `process` = unggah, ambil `data.code`, lalu polling `waitForResult`
//!   (maksimal 10 percobaan, tidur 3 detik di antara percobaan, lalu
//!   `throw new Error("Timeout after 10 attempts")`).
//! - `axios` melempar untuk 4xx/5xx; port ini memakai perilaku bawaan
//!   `HttpGet` (status ≥ 400 jadi `Err`), sama seperti axios.
//!
//! Beda yang diketahui:
//! - JS membaca berkas dari disk (`fs.createReadStream(imagePath)`); scraper
//!   Rust tidak menyentuh berkas, jadi byte gambar masuk sebagai argumen
//!   (`image`) dan `imagePath` hanya dipakai untuk `path.basename` seperti
//!   `form.append("myfile", stream, filename)`.
//! - `timeout: 120000` di axios tidak ditiru (batas mesin, bukan tampilan);
//!   `UreqHttp` punya timeout global 20 detik per permintaan. Tidur 3 detik
//!   antar percobaan tetap ditiru dengan `std::thread::sleep`, jadi fungsi ini
//!   memblokir worker selama polling.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, MultipartPart};
use serde_json::{Value, json};

use crate::common::truthy;

const BASE: &str = "https://get1.imglarger.com";

const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/145.0.0.0 Safari/537.36";

/// `this.headers` (`imglarger.js:9-16`) — dipakai kedua permintaan POST.
fn header_dasar(req: HttpCall) -> HttpCall {
    req.header("User-Agent", UA)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Origin", "https://imgupscaler.com")
        .header("Referer", "https://imgupscaler.com/")
}

/// `path.basename` (POSIX): garis miring di ujung diabaikan.
fn nama_berkas(jalur: &str) -> &str {
    let mut s = jalur;
    while s.len() > 1 && s.ends_with('/') {
        s = &s[..s.len() - 1];
    }
    if s == "/" {
        return "";
    }
    match s.rfind('/') {
        Some(i) => &s[i + 1..],
        None => s,
    }
}

/// `mime.lookup(nama)` milik `form-data` untuk tipe yang umum; di luar daftar
/// ini `form-data` memakai `FormData.DEFAULT_CONTENT_TYPE`
/// (`application/octet-stream`) selama nilainya bukan string polos.
fn mime_berkas(nama: &str) -> &'static str {
    let ext = match nama.rfind('.') {
        Some(i) => nama[i + 1..].to_ascii_lowercase(),
        None => String::new(),
    };
    match ext.as_str() {
        "jpg" | "jpeg" | "jpe" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "avif" => "image/avif",
        "heic" | "heif" => "image/heic",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
}

/// `uploadImage(imagePath)` (`imglarger.js:19-38`) → `response.data`.
pub fn upload_image(
    http: &dyn HttpGet,
    image_path: &str,
    image: &[u8],
) -> Result<Value, HttpError> {
    let nama = nama_berkas(image_path);
    let res = http.call(
        &header_dasar(HttpCall::post(format!("{BASE}/api/UpscalerNew/UploadNew"))).multipart(vec![
            MultipartPart::file("myfile", nama, image.to_vec()).with_mimetype(mime_berkas(nama)),
            MultipartPart::field("scaleRadio", "2"),
        ]),
    )?;
    res.json()
        .ok_or_else(|| HttpError {
            message: "respons UploadNew bukan JSON".to_string(),
        })
}

/// `checkStatus(code)` (`imglarger.js:40-57`) → `response.data`.
pub fn check_status(http: &dyn HttpGet, code: &Value) -> Result<Value, HttpError> {
    let res = http.call(
        &header_dasar(HttpCall::post(format!("{BASE}/api/UpscalerNew/CheckStatusNew")))
            .header("Content-Type", "application/json")
            .json(&json!({ "code": code, "scaleRadio": 2 })),
    )?;
    res.json()
        .ok_or_else(|| HttpError {
            message: "respons CheckStatusNew bukan JSON".to_string(),
        })
}

/// `waitForResult(code)` (`imglarger.js:59-80`) — 10 percobaan, jeda 3 detik.
pub fn wait_for_result(http: &dyn HttpGet, code: &Value) -> Result<Value, HttpError> {
    let max_attempts = 10;
    let mut attempts = 0;
    while attempts < max_attempts {
        let hasil = check_status(http, code)?;

        // `const data = result?.data || {}` — `data` falsy jadi objek kosong.
        let data = match hasil.get("data") {
            Some(v) if truthy(v) => v.clone(),
            _ => json!({}),
        };

        // `Array.isArray(data.downloadUrls) ? Boolean(data.downloadUrls[0])
        //  : Boolean(data.download_url || data.img_url)`
        let has_download = match data.get("downloadUrls") {
            Some(Value::Array(daftar)) => daftar.first().map(truthy).unwrap_or(false),
            _ => {
                let unduh = data.get("download_url").map(truthy).unwrap_or(false);
                let gambar = data.get("img_url").map(truthy).unwrap_or(false);
                unduh || gambar
            }
        };
        let is_success = data.get("status").and_then(Value::as_str) == Some("success") || has_download;

        // `result?.code === 200 && isSuccess`
        if hasil.get("code").and_then(Value::as_f64) == Some(200.0) && is_success {
            return Ok(hasil);
        }

        attempts += 1;
        // JS juga tidur setelah percobaan terakhir; itu ditiru apa adanya.
        std::thread::sleep(std::time::Duration::from_millis(3000));
    }
    Err(HttpError {
        message: format!("Timeout after {max_attempts} attempts"),
    })
}

/// `process(imagePath)` (`imglarger.js:82-96`).
pub fn process(http: &dyn HttpGet, image_path: &str, image: &[u8]) -> Result<Value, HttpError> {
    let upload = upload_image(http, image_path, image)?;
    let code = upload.get("data").and_then(|d| d.get("code")).cloned();
    let Some(code) = code.filter(truthy) else {
        return Err(HttpError {
            message: "ImgLarger upload failed to return code".to_string(),
        });
    };
    let result = wait_for_result(http, &code)?;
    Ok(json!({ "upload": upload, "result": result }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall, HttpResponse};
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body) berurutan; setelah habis, balasan terakhir dipakai lagi.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn new(balasan: Vec<(u16, String)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, teks) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: std::sync::Arc::from(teks.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn cari_header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn upload_mengirim_multipart_nama_berkas_dan_scale_radio() {
        let http = StubHttp::new(vec![(
            200,
            r#"{"code":123,"data":{"code":456,"status":"success"}}"#.to_string(),
        )]);
        let data = upload_image(&http, "/tmp/berkas/foto.png", &[1u8, 2, 3]).unwrap();
        assert_eq!(data["code"], 123);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].url, "https://get1.imglarger.com/api/UpscalerNew/UploadNew");
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].name, "myfile");
        assert_eq!(parts[0].file_name.as_deref(), Some("foto.png"));
        assert_eq!(parts[0].mimetype.as_deref(), Some("image/png"));
        assert_eq!(parts[0].body.bytes(), &[1u8, 2, 3][..]);
        assert_eq!(parts[1].name, "scaleRadio");
        assert_eq!(parts[1].file_name, None);
        assert_eq!(parts[1].body.bytes(), &b"2"[..]);
        // Header kelas ImgUpscaler ikut di kedua permintaan.
        assert!(cari_header(&p[0], "user-agent").unwrap().contains("Chrome/145"));
        assert_eq!(
            cari_header(&p[0], "origin"),
            Some("https://imgupscaler.com".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "referer"),
            Some("https://imgupscaler.com/".to_string())
        );
    }

    #[test]
    fn nama_berkas_mengikuti_path_basename() {
        assert_eq!(nama_berkas("/tmp/a/b.jpg"), "b.jpg");
        assert_eq!(nama_berkas("b.jpg"), "b.jpg");
        assert_eq!(nama_berkas("/tmp/a/b/"), "b");
        assert_eq!(nama_berkas("/"), "");
        // Ekstensi tak dikenal → application/octet-stream seperti form-data.
        assert_eq!(mime_berkas("x.xyz"), "application/octet-stream");
        assert_eq!(mime_berkas("X.JPEG"), "image/jpeg");
    }

    #[test]
    fn check_status_mengirim_body_json_persis() {
        let http = StubHttp::new(vec![(200, r#"{"code":200,"data":{}}"#.to_string())]);
        let data = check_status(&http, &json!(456)).unwrap();
        assert_eq!(data["code"], 200);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            p[0].url,
            "https://get1.imglarger.com/api/UpscalerNew/CheckStatusNew"
        );
        let Some(HttpBody::Bytes { data, mimetype }) = &p[0].body else {
            panic!("body bukan JSON mentah: {:?}", p[0].body);
        };
        // `JSON.stringify({code, scaleRadio: 2})` dari axios — urutan kunci
        // mengikuti objek JS.
        assert_eq!(data.as_ref(), &br#"{"code":456,"scaleRadio":2}"#[..]);
        assert_eq!(mimetype.as_deref(), Some("application/json"));
        assert_eq!(
            cari_header(&p[0], "content-type"),
            Some("application/json".to_string())
        );
    }

    #[test]
    fn wait_for_result_berhenti_di_percobaan_pertama_yang_sukses() {
        let http = StubHttp::new(vec![(
            200,
            r#"{"code":200,"data":{"status":"success"}}"#.to_string(),
        )]);
        let hasil = wait_for_result(&http, &json!(456)).unwrap();
        assert_eq!(hasil["code"], 200);
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }

    #[test]
    fn wait_for_result_menerima_download_urls_maupun_download_url() {
        let http = StubHttp::new(vec![(
            200,
            r#"{"code":200,"data":{"downloadUrls":["https://x/foto.jpg"]}}"#.to_string(),
        )]);
        let hasil = wait_for_result(&http, &json!(1)).unwrap();
        assert_eq!(hasil["data"]["downloadUrls"][0], "https://x/foto.jpg");

        let http = StubHttp::new(vec![(
            200,
            r#"{"code":200,"data":{"img_url":"https://y/foto.jpg"}}"#.to_string(),
        )]);
        assert_eq!(
            wait_for_result(&http, &json!(1)).unwrap()["data"]["img_url"],
            "https://y/foto.jpg"
        );
    }

    #[test]
    fn process_melempar_kalau_upload_tanpa_code() {
        let http = StubHttp::new(vec![(200, r#"{"data":{}}"#.to_string())]);
        let err = process(&http, "/tmp/foto.jpg", &[1u8]).unwrap_err();
        assert_eq!(err.message, "ImgLarger upload failed to return code");
    }

    #[test]
    fn process_menggabungkan_upload_dan_result() {
        let http = StubHttp::new(vec![
            (200, r#"{"data":{"code":99}}"#.to_string()),
            (
                200,
                r#"{"code":200,"data":{"status":"success","download_url":"https://x/a.png"}}"#
                    .to_string(),
            ),
        ]);
        let hasil = process(&http, "/tmp/a.png", &[7u8]).unwrap();
        assert_eq!(hasil["upload"]["data"]["code"], 99);
        assert_eq!(hasil["result"]["data"]["download_url"], "https://x/a.png");
        assert_eq!(http.permintaan.lock().unwrap().len(), 2);
    }

    /// JS tidur 3 detik setelah tiap percobaan gagal, jadi 10 percobaan ≈ 30
    /// detik. Sama persis dengan JS — dijalankan manual saja.
    #[test]
    #[ignore = "meniru JS: 10 percobaan x 3 detik tidur; jalankan manual dengan --ignored"]
    fn wait_for_result_timeout_setelah_10_percobaan() {
        let http = StubHttp::new(vec![(200, r#"{"code":200,"data":{}}"#.to_string())]);
        let err = wait_for_result(&http, &json!(1)).unwrap_err();
        assert_eq!(err.message, "Timeout after 10 attempts");
        assert_eq!(http.permintaan.lock().unwrap().len(), 10);
    }
}
