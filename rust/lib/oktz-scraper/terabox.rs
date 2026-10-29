//! `src/scraper/terabox.js` — GET halaman bot `flowvideoplayer.com` untuk
//! mengambil cookie + token CSRF, lalu POST pencarian video.
//!
//! Semantik JS yang sengaja disalin:
//! - `session.headers["set-cookie"] || []` — tiap nilai cookie dipotong di `;`
//!   pertama lalu disambung `"; "`.
//! - Token dicari dengan regex `/name=["']csrf-token["']\s+content=["']([^"']+)["']/i`.
//! - `result.error === false` itu perbandingan KETAT; `error` yang hilang atau
//!   bernilai lain (mis. `0`) bukan sukses.
//! - `stream_final_url || stream_url` — jatuhan `||` JS; kalau dua-duanya
//!   falsy, kuncinya `undefined` dan tidak ikut diserialisasi.
//! - Seluruh badan ada di dalam `try`; error apa pun kembali sebagai
//!   `{ status: false, error: e.message }`.

use std::sync::LazyLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const BASE: &str = "https://flowvideoplayer.com";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

/// `name=["']csrf-token["']\s+content=["']([^"']+)["']` (`i`).
static RE_TOKEN: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?i)name=["']csrf-token["']\s+content=["']([^"']+)["']"#).unwrap()
});

/// `result.data && result.data.length > 0` untuk nilai JSON: hanya array dan
/// string yang punya `length` bermakna di sini.
fn panjang_positif(v: &Value) -> bool {
    match v {
        Value::Array(a) => !a.is_empty(),
        Value::String(s) => !s.is_empty(),
        _ => false,
    }
}

/// `TeraBoxDL(url)` (`terabox.js:8-61`) — hasil selalu `Ok`, termasuk error.
pub fn tera_box_dl(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    match alur(http, url) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": false, "error": e.message })),
    }
}

fn alur(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let sesi = http.call(&HttpCall::get(BASE).header("user-agent", UA))?;

    // `rawCookies.map((c) => c.split(";")[0]).join("; ")`.
    let cookie = sesi
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|(_, v)| v.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");

    let html = sesi.text();
    let Some(token) = RE_TOKEN
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
    else {
        // `if (!token)` — token kosong juga masuk sini karena `[^"']+`.
        return Ok(json!({ "status": false, "error": "CSRF Token not found" }));
    };

    let res = http.call(
        &HttpCall::post(format!("{BASE}/telegram/bot/search/video"))
            .header("user-agent", UA)
            .header("content-type", "application/json")
            .header("x-csrf-token", &token)
            .header("x-requested-with", "XMLHttpRequest")
            .header("cookie", &cookie)
            .header("origin", BASE)
            .header("referer", format!("{BASE}/"))
            .json(&json!({ "url": url })),
    )?;

    let result = res.json().unwrap_or(Value::Null);
    let sukses = result
        .get("error")
        .map(|v| v == &Value::Bool(false))
        .unwrap_or(false);
    let data = result.get("data");
    let ada_data = data.map(panjang_positif).unwrap_or(false);

    if sukses && ada_data {
        let item = data.and_then(Value::as_array).and_then(|a| a.first());

        let mut out = Map::new();
        out.insert("status".to_string(), Value::Bool(true));
        if let Some(item) = item {
            for kunci in ["file_name", "thumbnail", "download_url"] {
                if let Some(v) = item.get(kunci) {
                    out.insert(kunci.to_string(), v.clone());
                }
            }
            // `item.stream_final_url || item.stream_url` — `undefined` berarti
            // kunci tidak ikut keluar.
            if let Some(v) = item
                .get("stream_final_url")
                .filter(|v| truthy(v))
                .or_else(|| item.get("stream_url").filter(|v| truthy(v)))
            {
                out.insert("stream_url".to_string(), v.clone());
            }
            for kunci in [
                "file_size",
                "file_size_bytes",
                "duration",
                "extension",
                "share_url",
            ] {
                if let Some(v) = item.get(kunci) {
                    out.insert(kunci.to_string(), v.clone());
                }
            }
        }
        return Ok(Value::Object(out));
    }

    Ok(json!({ "status": false, "error": "Tidak ada data ditemukan" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet`: mencatat permintaan dan membalas antrean body JSON.
    /// Entri terakhir dipakai ulang kalau permintaan lebih banyak dari balasan.
    /// Satu balasan stub: (body, header).
    type BalasanStub = (String, Vec<(String, String)>);

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<VecDeque<BalasanStub>>,
        gagal: Option<String>,
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
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
            unreachable!("semua lewat HttpCall::call")
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
            if let Some(pesan) = &self.gagal {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
            let mut q = self.balasan.lock().unwrap();
            let (body, headers) = if q.len() > 1 {
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
                headers,
            })
        }
    }

    fn stub(balasan: Vec<(String, Vec<(String, String)>)>) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(balasan.into()),
            gagal: None,
        }
    }

    const HALAMAN: &str = r#"<html><head><meta name="csrf-token" content="tok123"></head><body></body></html>"#;
    const HASIL: &str = r#"{"error":false,"data":[{"file_name":"film.mp4","thumbnail":"https://t/x.jpg",
        "download_url":"https://d/x","stream_url":"https://s/x","file_size":"1.2 GB",
        "file_size_bytes":1288490188,"duration":"10:00","extension":"mp4","share_url":"https://terabox.com/s/abc"}]}"#;

    #[test]
    fn sukses_mengambil_csrf_dan_data_pertama() {
        let http = stub(vec![
            (
                HALAMAN.to_string(),
                vec![
                    ("set-cookie".to_string(), "a=1; Path=/; HttpOnly".to_string()),
                    ("set-cookie".to_string(), "b=2; Path=/".to_string()),
                ],
            ),
            (HASIL.to_string(), Vec::new()),
        ]);
        let hasil = tera_box_dl(&http, "https://terabox.com/s/1").unwrap();

        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["file_name"], "film.mp4");
        assert_eq!(hasil["stream_url"], "https://s/x");
        assert_eq!(hasil["share_url"], "https://terabox.com/s/abc");
        // Urutan kunci sama dengan urutan literal JS (`JSON.stringify`).
        let kunci = hasil
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            kunci,
            [
                "status",
                "file_name",
                "thumbnail",
                "download_url",
                "stream_url",
                "file_size",
                "file_size_bytes",
                "duration",
                "extension",
                "share_url"
            ]
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        // GET halaman bot.
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(p[0].url, "https://flowvideoplayer.com");
        assert_eq!(
            cari_header(&p[0], "user-agent"),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        );
        // POST pencarian: token, cookie, dan JSON body.
        assert_eq!(p[1].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[1].url, "https://flowvideoplayer.com/telegram/bot/search/video");
        assert_eq!(cari_header(&p[1], "x-csrf-token"), "tok123");
        assert_eq!(cari_header(&p[1], "cookie"), "a=1; b=2");
        assert_eq!(cari_header(&p[1], "origin"), "https://flowvideoplayer.com");
        assert_eq!(cari_header(&p[1], "referer"), "https://flowvideoplayer.com/");
        assert_eq!(
            p[1].body.as_ref().unwrap().bytes(),
            br#"{"url":"https://terabox.com/s/1"}"#
        );
    }

    fn cari_header(req: &oktz_plugin_api::HttpCall, nama: &str) -> String {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    }

    #[test]
    fn tanpa_token_csrf_tidak_ada_post() {
        let http = stub(vec![(
            "<html><body>tanpa token</body></html>".to_string(),
            Vec::new(),
        )]);
        let hasil = tera_box_dl(&http, "https://terabox.com/s/1").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "CSRF Token not found");
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }

    #[test]
    fn data_kosong_mengembalikan_pesan_js() {
        let http = stub(vec![
            (HALAMAN.to_string(), Vec::new()),
            (r#"{"error":false,"data":[]}"#.to_string(), Vec::new()),
        ]);
        let hasil = tera_box_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Tidak ada data ditemukan");
    }

    #[test]
    fn error_strict_false_bukan_sukses() {
        // `result.error === false` ketat: `0` tidak lolos.
        let http = stub(vec![
            (HALAMAN.to_string(), Vec::new()),
            (
                r#"{"error":0,"data":[{"file_name":"x"}]}"#.to_string(),
                Vec::new(),
            ),
        ]);
        let hasil = tera_box_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Tidak ada data ditemukan");
    }

    #[test]
    fn error_http_dibungkus_oleh_catch_js() {
        let http = StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(VecDeque::new()),
            gagal: Some("jaringan putus".to_string()),
        };
        let hasil = tera_box_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "jaringan putus");
    }

    #[test]
    fn stream_final_url_falsy_jatuh_ke_stream_url() {
        let http = stub(vec![
            (HALAMAN.to_string(), Vec::new()),
            (
                r#"{"error":false,"data":[{"stream_final_url":"","stream_url":"https://s/2"}]}"#
                    .to_string(),
                Vec::new(),
            ),
        ]);
        let hasil = tera_box_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["stream_url"], "https://s/2");
    }
}
