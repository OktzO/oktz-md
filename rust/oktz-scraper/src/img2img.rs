//! `src/scraper/img2img.js` — unggah gambar ke Uguu, mulai proses di FGSI,
//! lalu polling hasil.
//!
//! Yang disalin persis:
//! - `Uguu` multipart field `files[]` dengan `filename` dan
//!   `contentType: "image/png"` (paket `form-data`).
//! - `config.APIkey.fgsi` dibaca dari env `APIKEY_FGSI` seperti `config.js`;
//!   `apikey=` TIDAK di-encode (perilaku JS).
//! - Loop 60×: `"Success"` mengisi `result`, `"Failed"` mengembalikan
//!   `{status:false}`; status lain menunggu 2 detik. `result` yang falsy
//!   setelah loop berakhir → `"Timeout menunggu hasil"`.
//!
//! Beda mesin: `setTimeout(2000)` jadi `std::thread::sleep`; di test jeda
//! dilewati supaya tidak menunggu 2 menit.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, MultipartPart};
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, field, truthy};

const FCSI_API: &str = "https://fgsi.dpdns.org/api/ai/image/img2img";

/// `Uguu(buffer, filename)` (`img2img.js:7-21`).
fn uguu(http: &dyn HttpGet, buffer: &[u8], filename: &str) -> Result<String, HttpError> {
    let res = http.call(
        &HttpCall::post("https://uguu.se/upload.php").multipart(vec![
            MultipartPart::file("files[]", filename, buffer.to_vec()).with_mimetype("image/png"),
        ]),
    )?;
    let data = res.json().unwrap_or(Value::Null);
    let url = data
        .get("files")
        .and_then(Value::as_array)
        .and_then(|berkas| berkas.first())
        .and_then(|berkas| berkas.get("url"));
    match url {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(v) if truthy(v) => Ok(v.to_string()),
        _ => Err(HttpError {
            message: "Upload ke Uguu gagal".to_string(),
        }),
    }
}

/// `Img2Img(prompt, imageBuffer, filename = "upload.png")` (`img2img.js:23-66`).
pub fn img2img(
    http: &dyn HttpGet,
    prompt: &str,
    image_buffer: &[u8],
    filename: Option<&str>,
) -> Result<Value, HttpError> {
    let filename = filename.unwrap_or("upload.png");
    let image_url = uguu(http, image_buffer, filename)?;

    let api_key = std::env::var("APIKEY_FGSI").unwrap_or_default();
    let start_url = format!(
        "{FCSI_API}?apikey={api_key}&prompt={}&url={}",
        encode_uri_component(prompt),
        encode_uri_component(&image_url)
    );
    let start = http.get(&start_url)?;
    let start_data = start.json().unwrap_or(Value::Null);

    let poll_url = match field(&start_data, "data.pollUrl") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    };
    let Some(poll_url) = poll_url else {
        let mut out = Map::new();
        out.insert("status".to_string(), Value::Bool(false));
        let pesan = field(&start_data, "error")
            .filter(|v| truthy(v))
            .cloned()
            .unwrap_or_else(|| json!("Gagal memulai proses img2img"));
        out.insert("error".to_string(), pesan);
        return Ok(Value::Object(out));
    };

    let mut result: Option<Value> = None;
    for _ in 0..60 {
        let poll = http.get(&poll_url)?;
        let poll_data = poll.json().unwrap_or(Value::Null);
        if !poll_data.get("status").map(truthy).unwrap_or(false) {
            return Ok(json!({"status": false, "error": "Polling gagal"}));
        }
        let status = field(&poll_data, "data.status").and_then(Value::as_str);
        if status == Some("Success") {
            // `result` yang falsy tetap berarti timeout setelah loop, JS.
            result = field(&poll_data, "data.result")
                .filter(|v| truthy(v))
                .cloned();
            break;
        }
        if status == Some("Failed") {
            return Ok(json!({"status": false, "error": "Proses img2img gagal"}));
        }
        jeda_polling();
    }

    let Some(result) = result else {
        return Ok(json!({"status": false, "error": "Timeout menunggu hasil"}));
    };

    Ok(json!({
        "status": true,
        "prompt": prompt,
        "imageUrl": image_url,
        "result": result,
    }))
}

/// `await new Promise(r => setTimeout(r, 2000))`.
#[cfg(not(test))]
fn jeda_polling() {
    std::thread::sleep(std::time::Duration::from_millis(2000));
}

/// Di test tidak menunggu — perilaku selain jeda tetap sama.
#[cfg(test)]
fn jeda_polling() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Stub `HttpGet` yang membalas berdasar URL persis.
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<HashMap<String, String>>,
        galat_uguu: bool,
    }

    impl StubHttp {
        fn baru(balasan: &[(&str, &str)]) -> Self {
            let mut map = HashMap::new();
            for (k, v) in balasan {
                map.insert(k.to_string(), v.to_string());
            }
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(map),
                galat_uguu: false,
            }
        }

        fn jawab(&self, url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            match self.balasan.lock().unwrap().get(url) {
                Some(b) => Ok(oktz_plugin_api::HttpResponse {
                    status: 200,
                    body: oktz_plugin_api::HttpBody::Bytes {
                        data: std::sync::Arc::from(b.as_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(HttpError {
                    message: format!("tidak ada balasan untuk {url}"),
                }),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(HttpCall::get(url));
            self.jawab(url)
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("img2img lewat HttpCall::call")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("img2img tidak form-urlencoded")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("img2img lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            if self.galat_uguu {
                return Err(HttpError {
                    message: "uguu mati".to_string(),
                });
            }
            self.jawab(&req.url)
        }
    }

    fn url_start(prompt: &str, image_url: &str) -> String {
        let kunci = std::env::var("APIKEY_FGSI").unwrap_or_default();
        format!(
            "{FCSI_API}?apikey={kunci}&prompt={}&url={}",
            encode_uri_component(prompt),
            encode_uri_component(image_url)
        )
    }

    #[test]
    fn sukses_mengunggah_memulai_dan_polling() {
        let start = url_start("kucing oren", "https://uguu.se/a.png");
        let http = StubHttp::baru(&[
            (
                "https://uguu.se/upload.php",
                r#"{"files":[{"url":"https://uguu.se/a.png"}]}"#,
            ),
            (
                &start,
                r#"{"data":{"pollUrl":"https://fgsi.dpdns.org/poll/1"}}"#,
            ),
            (
                "https://fgsi.dpdns.org/poll/1",
                r#"{"status":true,"data":{"status":"Success","result":"https://hasil/img.png"}}"#,
            ),
        ]);
        let hasil = img2img(&http, "kucing oren", b"png-bytes", None).unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["prompt"], "kucing oren");
        assert_eq!(hasil["imageUrl"], "https://uguu.se/a.png");
        assert_eq!(hasil["result"], "https://hasil/img.png");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://uguu.se/upload.php");
        assert_eq!(
            p[0].body,
            Some(oktz_plugin_api::HttpBody::Multipart(vec![
                MultipartPart::file("files[]", "upload.png", b"png-bytes".to_vec())
                    .with_mimetype("image/png"),
            ]))
        );
        assert_eq!(p[1].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(p[1].url, start);
        assert_eq!(p[2].url, "https://fgsi.dpdns.org/poll/1");
    }

    #[test]
    fn nama_berkas_kustom_dipakai_di_multipart() {
        let start = url_start("p", "https://uguu.se/b.png");
        let http = StubHttp::baru(&[
            (
                "https://uguu.se/upload.php",
                r#"{"files":[{"url":"https://uguu.se/b.png"}]}"#,
            ),
            (&start, r#"{"error":"x"}"#),
        ]);
        img2img(&http, "p", b"byte", Some("gambar keren.png")).unwrap();
        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            p[0].body,
            Some(oktz_plugin_api::HttpBody::Multipart(vec![
                MultipartPart::file("files[]", "gambar keren.png", b"byte".to_vec())
                    .with_mimetype("image/png"),
            ]))
        );
    }

    #[test]
    fn uguu_tanpa_url_melempar() {
        let http = StubHttp::baru(&[("https://uguu.se/upload.php", r#"{"files":[]}"#)]);
        let salah = img2img(&http, "p", b"x", None).unwrap_err();
        assert_eq!(salah.message, "Upload ke Uguu gagal");
    }

    #[test]
    fn tanpa_poll_url_memakai_pesan_error_atau_bawaan() {
        let start = url_start("p", "https://uguu.se/a.png");
        let http = StubHttp::baru(&[
            (
                "https://uguu.se/upload.php",
                r#"{"files":[{"url":"https://uguu.se/a.png"}]}"#,
            ),
            (&start, r#"{"error":"kunci salah"}"#),
        ]);
        let hasil = img2img(&http, "p", b"x", None).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "kunci salah");

        let http = StubHttp::baru(&[
            (
                "https://uguu.se/upload.php",
                r#"{"files":[{"url":"https://uguu.se/a.png"}]}"#,
            ),
            (&start, r#"{"data":{}}"#),
        ]);
        let hasil = img2img(&http, "p", b"x", None).unwrap();
        assert_eq!(hasil["error"], "Gagal memulai proses img2img");
    }

    #[test]
    fn polling_gagal_dan_failed_mengembalikan_status_false() {
        let start = url_start("p", "https://uguu.se/a.png");
        let dasar = |balasan_poll: &str| {
            StubHttp::baru(&[
                (
                    "https://uguu.se/upload.php",
                    r#"{"files":[{"url":"https://uguu.se/a.png"}]}"#,
                ),
                (&start, r#"{"data":{"pollUrl":"https://poll/1"}}"#),
                ("https://poll/1", balasan_poll),
            ])
        };

        let http = dasar(r#"{"data":{"status":"Failed"}}"#);
        let hasil = img2img(&http, "p", b"x", None).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Polling gagal");

        let http = dasar(r#"{"status":true,"data":{"status":"Failed"}}"#);
        let hasil = img2img(&http, "p", b"x", None).unwrap();
        assert_eq!(hasil["error"], "Proses img2img gagal");

        // `data.result` falsy (di sini null) → timeout, bukan sukses.
        let http = dasar(r#"{"status":true,"data":{"status":"Success","result":null}}"#);
        let hasil = img2img(&http, "p", b"x", None).unwrap();
        assert_eq!(hasil["error"], "Timeout menunggu hasil");
    }

    #[test]
    fn timeout_setelah_enam_puluh_polling() {
        let start = url_start("p", "https://uguu.se/a.png");
        let http = StubHttp::baru(&[
            (
                "https://uguu.se/upload.php",
                r#"{"files":[{"url":"https://uguu.se/a.png"}]}"#,
            ),
            (&start, r#"{"data":{"pollUrl":"https://poll/1"}}"#),
            ("https://poll/1", r#"{"status":1,"data":{"status":"Menunggu"}}"#),
        ]);
        let hasil = img2img(&http, "p", b"x", None).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Timeout menunggu hasil");
        let p = http.permintaan.lock().unwrap();
        // unggah + start + 60 polling.
        assert_eq!(p.len(), 62);
    }

    #[test]
    fn galat_jaringan_uguu_melempar() {
        let mut http = StubHttp::baru(&[]);
        http.galat_uguu = true;
        let salah = img2img(&http, "p", b"x", None).unwrap_err();
        assert_eq!(salah.message, "uguu mati");
    }
}
