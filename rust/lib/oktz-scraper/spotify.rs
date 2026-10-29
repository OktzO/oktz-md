//! `src/scraper/spotify.js` — POST JSON ke `spotyloader.com`.
//!
//! Yang disalin persis:
//! - Respons non-2xx: pesan `spotyloader gagal: <detail>` dengan
//!   `detail = error.response?.data || error.message`. Body diparse JSON
//!   seperti `transformResponse` bawaan axios; body kosong/falsy jatuh ke
//!   pesan axios `Request failed with status code NNN` (dibuat di sini karena
//!   klien Rust tidak membawa pesan itu).
//! - `!data?.downloadLink` — falsy (termasuk `""`), bukan hanya hilang, tetap
//!   melempar.
//! - `data.post.name` saat `data.post` nullish melempar TypeError; teksnya
//!   disalin huruf per huruf.
//!
//! Tidak ada padanannya di `HttpGet`: `signal` (pembatalan) dan
//! `wrapped.status` — `HttpError` cuma punya `message`.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::{field, truthy};

/// `downloadSpotify(spotifyUrl, { signal })` (`spotify.js:8-53`).
pub fn download_spotify(http: &dyn HttpGet, spotify_url: &str) -> Result<Value, HttpError> {
    let res = http
        .call(
            &HttpCall::post("https://spotyloader.com/api/spotify/track")
                .header("Content-Type", "application/json")
                .header(
                    "User-Agent",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                )
                .header("Referer", "https://spotyloader.com/")
                .header("Origin", "https://spotyloader.com")
                .json(&json!({ "url": spotify_url })),
        )
        .map_err(|e| HttpError {
            // Jaringan gagal: `detail = error.message` di JS.
            message: format!("spotyloader gagal: {}", e.message),
        })?;

    if !(200..=299).contains(&res.status) {
        let detail = detail_galat(res.status, &res.text());
        return Err(HttpError {
            message: format!("spotyloader gagal: {detail}"),
        });
    }

    let data = res.json().unwrap_or_else(|| Value::String(res.text()));
    if !data.get("downloadLink").map(truthy).unwrap_or(false) {
        return Err(HttpError {
            message: format!(
                "spotyloader tidak mengembalikan downloadLink: {}",
                serde_json::to_string(&data).unwrap_or_default()
            ),
        });
    }

    if matches!(data.get("post"), None | Some(Value::Null)) {
        // `data.post.name` di JS melempar TypeError dengan teks ini.
        let jenjang = if data.get("post").is_none() {
            "undefined"
        } else {
            "null"
        };
        return Err(HttpError {
            message: format!("Cannot read properties of {jenjang} (reading 'name')"),
        });
    }

    let mut out = Map::new();
    for (kunci, jalur) in [("title", "post.name"), ("artist", "post.artist")] {
        if let Some(v) = field(&data, jalur) {
            out.insert(kunci.to_string(), v.clone());
        }
    }
    out.insert(
        "url".to_string(),
        data.get("downloadLink").cloned().unwrap_or(Value::Null),
    );
    if let Some(v) = field(&data, "post.mime") {
        out.insert("mime".to_string(), v.clone());
    }
    Ok(Value::Object(out))
}

/// `error.response?.data || error.message` untuk respons non-2xx.
fn detail_galat(status: u16, isi: &str) -> String {
    // axios mencoba JSON.parse dan membiarkan string apa adanya kalau gagal.
    let diparse = match serde_json::from_str::<Value>(isi) {
        Ok(v) => v,
        Err(_) => Value::String(isi.to_string()),
    };
    let nilai = if truthy(&diparse) {
        diparse
    } else {
        // Tanpa body: axios memakai `error.message`.
        Value::String(format!("Request failed with status code {status}"))
    };
    match nilai {
        // `typeof detail === "string" ? detail : JSON.stringify(detail)`
        Value::String(s) => s,
        lain => serde_json::to_string(&lain).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet`: mencatat permintaan, membalas status/body tetap, atau
    /// galat jaringan kalau `galat` diisi.
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        status: u16,
        balasan: String,
        galat: Option<String>,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("spotify cuma POST JSON")
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
            unreachable!("spotify tidak memakai form")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("spotify tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            if let Some(m) = &self.galat {
                return Err(HttpError { message: m.clone() });
            }
            Ok(oktz_plugin_api::HttpResponse {
                status: self.status,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(status: u16, balasan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            status,
            balasan: balasan.to_string(),
            galat: None,
        }
    }

    #[test]
    fn sukses_mengembalikan_title_artist_url_mime() {
        let http = stub(
            200,
            r#"{"downloadLink":"https://dl/x.mp3","post":{"name":"Lagu","artist":"Artis","mime":"audio/mpeg"}}"#,
        );
        let hasil = download_spotify(&http, "https://open.spotify.com/track/abc").unwrap();
        assert_eq!(hasil["title"], "Lagu");
        assert_eq!(hasil["artist"], "Artis");
        assert_eq!(hasil["url"], "https://dl/x.mp3");
        assert_eq!(hasil["mime"], "audio/mpeg");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://spotyloader.com/api/spotify/track");
        assert_eq!(
            p[0].body,
            Some(oktz_plugin_api::HttpBody::Bytes {
                data: Arc::from(br#"{"url":"https://open.spotify.com/track/abc"}"#.as_slice()),
                mimetype: Some("application/json".to_string()),
            })
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("content-type").unwrap(), "application/json");
        assert_eq!(cari("referer").unwrap(), "https://spotyloader.com/");
        assert_eq!(cari("origin").unwrap(), "https://spotyloader.com");
        assert_eq!(
            cari("user-agent").unwrap(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        );
    }

    #[test]
    fn kunci_hasil_yang_undefined_tidak_ikut() {
        let http = stub(200, r#"{"downloadLink":"https://dl","post":{}}"#);
        let hasil = download_spotify(&http, "https://x").unwrap();
        assert!(hasil.get("title").is_none());
        assert!(hasil.get("mime").is_none());
        assert_eq!(hasil["url"], "https://dl");
    }

    #[test]
    fn tanpa_download_link_melempar_pesan_js() {
        let http = stub(200, r#"{"post":{"name":"x"}}"#);
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(
            salah.message,
            r#"spotyloader tidak mengembalikan downloadLink: {"post":{"name":"x"}}"#
        );
    }

    #[test]
    fn download_link_kosong_juga_melempar() {
        let http = stub(200, r#"{"downloadLink":"","post":{"name":"x"}}"#);
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(
            salah.message,
            r#"spotyloader tidak mengembalikan downloadLink: {"downloadLink":"","post":{"name":"x"}}"#
        );
    }

    #[test]
    fn body_error_dipakai_sebagai_detail() {
        let http = stub(400, "blocked");
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(salah.message, "spotyloader gagal: blocked");

        let http = stub(500, r#"{"error":"rusak"}"#);
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(salah.message, "spotyloader gagal: {\"error\":\"rusak\"}");
    }

    #[test]
    fn tanpa_body_memakai_pesan_axios() {
        let http = stub(403, "");
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(
            salah.message,
            "spotyloader gagal: Request failed with status code 403"
        );
    }

    #[test]
    fn galat_jaringan_dibungkus() {
        let mut http = stub(200, "");
        http.galat = Some("connection refused".to_string());
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(salah.message, "spotyloader gagal: connection refused");
    }

    #[test]
    fn post_nullish_meniru_type_error() {
        let http = stub(200, r#"{"downloadLink":"https://dl"}"#);
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(
            salah.message,
            "Cannot read properties of undefined (reading 'name')"
        );

        let http = stub(200, r#"{"downloadLink":"https://dl","post":null}"#);
        let salah = download_spotify(&http, "https://x").unwrap_err();
        assert_eq!(
            salah.message,
            "Cannot read properties of null (reading 'name')"
        );
    }
}
