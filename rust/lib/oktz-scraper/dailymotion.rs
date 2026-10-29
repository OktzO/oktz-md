//! `src/scraper/dailymotion.js` — POST ke `vidomon.com`, pilih media terbaik.
//!
//! Port contoh untuk seluruh crate ini: satu fungsi JS → satu fungsi Rust,
//! `&dyn HttpGet` sebagai argumen pertama, hasil `serde_json::Value` dengan
//! bentuk objek yang SAMA seperti `return` JS (kunci yang `undefined` tidak
//! ikut, seperti `JSON.stringify`).
//!
//! Perhatian khusus yang ikut disalin dari JS:
//! - `data.medias` bisa tidak ada → `!data.medias || data.medias.length === 0`.
//! - `filter(m => m.videoAvailable && m.extension === "mp4")` — `videoAvailable`
//!   dicek truthy, bukan `=== true`.
//! - `sort((a, b) => parseInt(b.quality) - parseInt(a.quality))` — di JS
//!   komparator yang `NaN` diperlakukan sebagai `0` (urutan stabil), jadi
//!   `quality` yang tidak bisa diparse TIDAK dibuang, cuma tidak mengubah urutan.
//! - `best ? best.url : data.medias[0].url` — jatuhan ke media pertama.

use std::cmp::Ordering;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, parse_int_js, truthy};

/// `getRndIP` (`dailymotion.js:3-4`) — empat oktet `Math.floor(Math.random()*255)`.
fn ip_acak() -> String {
    (0..4)
        .map(|_| rand::random_range(0..255).to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// `parseInt(b.quality) - parseInt(a.quality)`; `NaN` → urutan tidak berubah.
fn banding_kualitas(a: &Value, b: &Value) -> Ordering {
    let ka = parse_int_js(a.get("quality").unwrap_or(&Value::Null));
    let kb = parse_int_js(b.get("quality").unwrap_or(&Value::Null));
    match (ka, kb) {
        (Some(ka), Some(kb)) => kb.partial_cmp(&ka).unwrap_or(Ordering::Equal),
        // `NaN` di JS diperlakukan `Array#sort` sebagai 0.
        _ => Ordering::Equal,
    }
}

/// `DailymotionDL(url)` (`dailymotion.js:6-38`).
pub fn dailymotion_dl(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let fake_ip = ip_acak();
    let body = format!("url={}", encode_uri_component(url));
    let res = http.call(
        &HttpCall::post("https://vidomon.com/wp-json/aio-dl/video-data/")
            .header(
                "user-agent",
                "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 Chrome/148.0.0.0 Mobile Safari/537.36",
            )
            .header("referer", format!("https://vidomon.com/dailymotion-video-downloader/#url={url}"))
            .header("x-forwarded-for", &fake_ip)
            .header("x-real-ip", &fake_ip)
            .form(body),
    )?;
    let data = res.json().ok_or_else(|| HttpError {
        message: "respons vidomon bukan JSON".to_string(),
    })?;

    let medias = match data.get("medias").and_then(Value::as_array) {
        Some(m) if !m.is_empty() => m,
        _ => {
            return Ok(json!({
                "status": false,
                "error": "Tidak ada media yang tersedia",
            }));
        }
    };

    let mut kandidat: Vec<&Value> = medias
        .iter()
        .filter(|m| {
            m.get("videoAvailable").map(truthy).unwrap_or(false)
                && m.get("extension").and_then(Value::as_str) == Some("mp4")
        })
        .collect();
    kandidat.sort_by(|a, b| banding_kualitas(a, b));
    let best = kandidat.first().copied();

    // Kunci opsional hanya masuk kalau JS punya nilainya — `undefined` tidak
    // muncul di `JSON.stringify`.
    let mut out = Map::new();
    out.insert("status".to_string(), Value::Bool(true));
    for kunci in ["title", "thumbnail", "duration", "source"] {
        if let Some(v) = data.get(kunci) {
            out.insert(kunci.to_string(), v.clone());
        }
    }
    let pilihan = best.unwrap_or(&medias[0]);
    out.insert(
        "video".to_string(),
        pilihan.get("url").cloned().unwrap_or(Value::Null),
    );
    out.insert(
        "quality".to_string(),
        pilihan.get("quality").cloned().unwrap_or(Value::Null),
    );
    out.insert("medias".to_string(), Value::Array(medias.clone()));
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` yang mencatat permintaan dan membalas JSON tetap —
    /// pola yang dipakai semua test scraper: tidak ada jaringan di test.
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
            unreachable!("scraper ini pakai form, bukan JSON")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
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

    #[test]
    fn memilih_mp4_dengan_kualitas_tertinggi() {
        let http = stub(
            r#"{"title":"Judul","thumbnail":"https://t","duration":"1:2",
                "source":"dm","medias":[
                  {"videoAvailable":true,"extension":"mp4","quality":"360","url":"https://rendah"},
                  {"videoAvailable":true,"extension":"mp4","quality":"720","url":"https://tinggi"},
                  {"videoAvailable":false,"extension":"mp4","quality":"9999","url":"https://tidak"},
                  {"videoAvailable":true,"extension":"webm","quality":"8888","url":"https://bukanmp4"}
                ]}"#,
        );
        let hasil = dailymotion_dl(&http, "https://www.dailymotion.com/video/x").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["title"], "Judul");
        assert_eq!(hasil["video"], "https://tinggi");
        assert_eq!(hasil["quality"], "720");
        assert_eq!(hasil["medias"].as_array().unwrap().len(), 4);

        // Permintaan yang dikirim: POST form dengan header khas JS.
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://vidomon.com/wp-json/aio-dl/video-data/");
        assert_eq!(
            p[0].body,
            Some(oktz_plugin_api::HttpBody::Form(
                "url=https%3A%2F%2Fwww.dailymotion.com%2Fvideo%2Fx".to_string()
            ))
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert!(cari("x-forwarded-for").unwrap().split('.').count() == 4);
        assert_eq!(cari("x-forwarded-for"), cari("x-real-ip"));
        assert_eq!(
            cari("referer").unwrap(),
            "https://vidomon.com/dailymotion-video-downloader/#url=https://www.dailymotion.com/video/x"
        );
    }

    #[test]
    fn tanpa_media_mengembalikan_status_false_seperti_js() {
        let http = stub(r#"{"medias":[]}"#);
        let hasil = dailymotion_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Tidak ada media yang tersedia");
    }

    #[test]
    fn kualitas_tidak_terparse_tidak_mengubah_urutan() {
        // `parseInt("abc")` = NaN → komparator 0 → urutan tetap seperti JS.
        let http = stub(
            r#"{"medias":[
                 {"videoAvailable":true,"extension":"mp4","url":"https://a"},
                 {"videoAvailable":true,"extension":"mp4","quality":"abc","url":"https://b"}
               ]}"#,
        );
        let hasil = dailymotion_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["video"], "https://a");
    }
}
