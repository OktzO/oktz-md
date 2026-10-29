//! `src/scraper/rednote.js` — unduh post Xiaohongshu lewat
//! `rednote.savevideodown.com`.
//!
//! Yang ikut disalin dari JS:
//! - `generateRandomIP()` = empat oktet `Math.floor(Math.random() * 255)`, jadi
//!   rentang 0-254 (bukan 0-255) dan dipakai sebagai header apa adanya.
//! - `if (!resData.success)` — dicek truthy, bukan `=== true`.
//! - `resData.error || "Gagal mengambil data"` — string kosong ikut jatuh ke
//!   bawaan karena `||`.
//! - `resData.directVideoUrl || resData.downloadUrl` di dalam array: kalau
//!   keduanya hilang, hasilnya `[undefined]`, yang `JSON.stringify` tulis
//!   `[null]` — jadi kunci `results` tetap ada dengan satu elemen `null`.
//! - Kunci `title`/`author`/`type` yang `undefined` tidak ikut hasil.
//! - Tidak ada `try`/`catch` di JS: kegagalan transport apa pun melempar, jadi
//!   di sini `Err`.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

/// `RedNoteDL(xhsUrl)` (`rednote.js:6-45`).
pub fn red_note_dl(http: &dyn HttpGet, xhs_url: &str) -> Result<Value, HttpError> {
    let fake_ip = ip_acak();
    let res = http.call(
        &HttpCall::post("https://rednote.savevideodown.com/api/download")
            .header(
                "Content-Type",
                "application/json",
            )
            .header(
                "User-Agent",
                "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36",
            )
            .header("Referer", "https://rednote.savevideodown.com/")
            .header("Origin", "https://rednote.savevideodown.com")
            .header("X-Forwarded-For", &fake_ip)
            .header("X-Real-IP", &fake_ip)
            .header("Client-IP", &fake_ip)
            .json(&json!({ "url": xhs_url })),
    )?;
    // Body non-JSON di axios menjadi string, jadi `resData.success` dan
    // `resData.error` sama-sama `undefined` — hasil sama seperti data kosong.
    let data = res.json().unwrap_or(Value::Null);

    if !data.get("success").map(truthy).unwrap_or(false) {
        // `resData.error || "Gagal mengambil data"` — nilai yang bukan string
        // ikut apa adanya, bukan dipaksa jadi teks.
        let pesan = match data.get("error").filter(|v| truthy(v)) {
            Some(v) => v.clone(),
            None => json!("Gagal mengambil data"),
        };
        return Ok(json!({ "status": false, "error": pesan }));
    }

    // `resData.type === "video"` — perbandingan ketat, nilai `true` di `type`
    // tidak ikut cabang video.
    let results = if data.get("type").and_then(Value::as_str) == Some("video") {
        let url = data
            .get("directVideoUrl")
            .filter(|v| truthy(v))
            .or_else(|| data.get("downloadUrl").filter(|v| truthy(v)))
            .cloned()
            .unwrap_or(Value::Null);
        Value::Array(vec![url])
    } else {
        match data.get("directImages").filter(|v| truthy(v)) {
            Some(v) => v.clone(),
            None => Value::Array(Vec::new()),
        }
    };

    let mut o = Map::new();
    o.insert("status".to_string(), Value::Bool(true));
    for kunci in ["title", "author", "type"] {
        if let Some(v) = data.get(kunci) {
            o.insert(kunci.to_string(), v.clone());
        }
    }
    o.insert("results".to_string(), results);
    Ok(Value::Object(o))
}

/// `generateRandomIP()` (`rednote.js:3-4`) — `Math.random()*255` di-floor.
fn ip_acak() -> String {
    (0..4)
        .map(|_| rand::random_range(0..255).to_string())
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: String,
        galat: Option<String>,
    }

    impl Stub {
        fn baru(balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: balasan.to_string(),
                galat: None,
            }
        }
        fn gagal(pesan: &str) -> Self {
            Self {
                galat: Some(pesan.to_string()),
                ..Self::baru("")
            }
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            if let Some(pesan) = &self.galat {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
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

    fn cari_header<'a>(req: &'a HttpCall, nama: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn video_memakai_direct_video_url_dan_header_ip_terisi() {
        let http = Stub::baru(
            r#"{"success":true,"title":"T","author":"A","type":"video","directVideoUrl":"https://cdn/v.mp4"}"#,
        );
        let hasil = red_note_dl(&http, "https://xhslink.com/a/1").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["title"], "T");
        assert_eq!(hasil["author"], "A");
        assert_eq!(hasil["type"], "video");
        assert_eq!(hasil["results"], json!(["https://cdn/v.mp4"]));

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(
            p[0].url,
            "https://rednote.savevideodown.com/api/download"
        );
        assert_eq!(cari_header(&p[0], "content-type"), Some("application/json"));
        assert_eq!(
            cari_header(&p[0], "referer"),
            Some("https://rednote.savevideodown.com/")
        );
        assert_eq!(
            cari_header(&p[0], "origin"),
            Some("https://rednote.savevideodown.com")
        );
        assert_eq!(
            p[0].body,
            Some(HttpBody::Bytes {
                data: Arc::from(br#"{"url":"https://xhslink.com/a/1"}"#.as_slice()),
                mimetype: Some("application/json".to_string()),
            })
        );
        // Tiga header IP memakai nilai yang sama dan tiap oktet 0-254.
        let ip = cari_header(&p[0], "x-forwarded-for").unwrap();
        let oktet: Vec<u32> = ip
            .split('.')
            .map(|o| o.parse::<u32>().expect("oktet bukan angka"))
            .collect();
        assert_eq!(oktet.len(), 4, "harus empat oktet: {ip}");
        assert!(oktet.iter().all(|o| *o < 255), "oktet di luar 0-254: {ip}");
        assert_eq!(cari_header(&p[0], "x-real-ip"), Some(ip));
        assert_eq!(cari_header(&p[0], "client-ip"), Some(ip));
    }

    #[test]
    fn jatuh_ke_download_url_kalu_direct_video_url_kosong() {
        let http = Stub::baru(
            r#"{"success":true,"type":"video","directVideoUrl":"","downloadUrl":"https://cdn/dl.mp4"}"#,
        );
        let hasil = red_note_dl(&http, "u").unwrap();
        assert_eq!(hasil["results"], json!(["https://cdn/dl.mp4"]));
        // `title`/`author` tidak ada -> kuncinya tidak ikut.
        assert!(hasil.get("title").is_none());
        assert!(hasil.get("author").is_none());
    }

    #[test]
    fn video_tanpa_url_memberi_array_dengan_null() {
        let http = Stub::baru(r#"{"success":true,"type":"video"}"#);
        let hasil = red_note_dl(&http, "u").unwrap();
        // `[undefined]` diserialisasi `[null]` oleh JSON.stringify.
        assert_eq!(hasil["results"], json!([null]));
    }

    #[test]
    fn gambar_mengembalikan_direct_images_apa_adanya() {
        let http = Stub::baru(
            r#"{"success":true,"type":"image","directImages":["https://cdn/1.jpg","https://cdn/2.jpg"]}"#,
        );
        let hasil = red_note_dl(&http, "u").unwrap();
        assert_eq!(
            hasil["results"],
            json!(["https://cdn/1.jpg", "https://cdn/2.jpg"])
        );

        let http = Stub::baru(r#"{"success":true,"type":"image"}"#);
        let hasil = red_note_dl(&http, "u").unwrap();
        assert_eq!(hasil["results"], json!([]));
    }

    #[test]
    fn success_false_memakai_pesan_error_masih_ada() {
        let http = Stub::baru(r#"{"success":false,"error":"Video tidak ditemukan"}"#);
        let hasil = red_note_dl(&http, "u").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Video tidak ditemukan");
    }

    #[test]
    fn error_kosong_ikut_jatuh_ke_pesan_bawaan() {
        // `resData.error || "Gagal mengambil data"` — string kosong falsy.
        for balasan in [r#"{"success":false}"#, r#"{"success":false,"error":""}"#] {
            let http = Stub::baru(balasan);
            let hasil = red_note_dl(&http, "u").unwrap();
            assert_eq!(hasil["status"], false);
            assert_eq!(hasil["error"], "Gagal mengambil data", "balasan {balasan}");
        }
    }

    #[test]
    fn galat_transport_lempar_tanpa_bungkus() {
        // Tidak ada try/catch di JS, jadi tidak jadi objek status false.
        let err = red_note_dl(&Stub::gagal("HTTP 502"), "u").unwrap_err();
        assert_eq!(err.message, "HTTP 502");
    }

    #[test]
    fn badan_bukan_json_ikut_pesan_bawaan() {
        // `resData` jadi string, `success` dan `error` keduanya `undefined`.
        let http = Stub::baru("<html>halo</html>");
        let hasil = red_note_dl(&http, "u").unwrap();
        assert_eq!(hasil, json!({ "status": false, "error": "Gagal mengambil data" }));
    }
}