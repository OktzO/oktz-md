//! `src/scraper/likee.js` — unduh video Likee lewat `likeedownloader.com`.
//!
//! Yang ikut disalin dari JS:
//! - Validasi URL `/likee\.video|likee\.com|likee\.video/` — tanpa jangkar, jadi
//!   cukup substring itu ada di mana saja; kegagalan melempar
//!   `new Error('Invalid url')`.
//! - Body `new URLSearchParams({ id: url, locale: 'en' })` di-encode sebagai
//!   form-urlencoded: spasi jadi `+`, bukan `%20`. Ini beda dari
//!   `encode_uri_component` dan tidak boleh tertukar.
//! - Empat regex diambil dari `res.data.template` dengan `.match()`; yang tidak
//!   cocok berarti `null`, dan kuncinya tetap ada di objek hasil.
//! - `catch` melempar ulang (`throw new Error(e.message)`), jadi fungsi ini
//!   mengembalikan `Err`, bukan objek error.
//!
//! Beda yang diketahui: `timeout: 15000` dan `maxRedirects: 3` milik
//! `httpAxios` (`src/lib/http.js`) tidak ditiru; `UreqHttp` memakai timeout
//! global 20 detik dan mengikuti redirect sendiri.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Value, json};

use crate::common::truthy;

/// `likee(url)` (`likee.js:2-41`).
pub fn likee(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    match alur(http, url) {
        Ok(v) => Ok(v),
        // `throw new Error(e.message)` — pesan asli diteruskan tanpa dibungkus.
        Err(e) => Err(HttpError {
            message: e.message,
        }),
    }
}

fn alur(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    if !host_likee().is_match(url) {
        return Err(HttpError {
            message: "Invalid url".to_string(),
        });
    }

    let body = format!("id={}&locale=en", encode_form(url));
    let res = http.call(
        &HttpCall::post("https://likeedownloader.com/process")
            .header("User-Agent", "Mozilla/5.0 (Linux; Android 10)")
            .header("Accept", "application/json,text/javascript,*/*")
            .header("x-requested-with", "XMLHttpRequest")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("origin", "https://likeedownloader.com")
            .header("referer", "https://likeedownloader.com/en")
            .form(body),
    )?;
    let data = res.json().unwrap_or(Value::Null);

    // `res.data && res.data.template ? res.data.template : null`.
    let html = match data.get("template").filter(|v| truthy(v)) {
        None => {
            return Err(HttpError {
                message: "Fetch failed".to_string(),
            });
        }
        Some(Value::String(s)) => s.as_str(),
        // `html.match(re)` pada nilai truthy non-string melempar TypeError.
        Some(_) => {
            return Err(HttpError {
                message: "html.match is not a function".to_string(),
            });
        }
    };

    Ok(json!({
        "thumbnail": ambil_tangkapan(html, pola_thumbnail()),
        "with_watermark": ambil_tangkapan(html, pola_with_watermark()),
        "without_watermark": ambil_tangkapan(html, pola_without_watermark()),
        "stats": ambil_tangkapan(html, pola_stats()),
    }))
}

/// `/likee\.video|likee\.com|likee\.video/` (`likee.js:4`).
fn host_likee() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r"likee\.video|likee\.com|likee\.video").expect("pola host likee"))
}

/// `/<img[^>]+src="([^"]+)"/` (`likee.js:30`).
fn pola_thumbnail() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r#"<img[^>]+src="([^"]+)""#).expect("pola thumbnail"))
}

/// `/with_watermark[^>]+href="([^"]+)"/` (`likee.js:31`).
///
/// Pola ini tanpa jangkar kata, jadi di JS ia juga cocok di dalam teks
/// `without_watermark`. Pola di sini tetap dipakai apa adanya, sama seperti JS.
fn pola_with_watermark() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r#"with_watermark[^>]+href="([^"]+)""#).expect("pola with_watermark"))
}

/// `/without_watermark[^>]+href="([^"]+)"/` (`likee.js:32`).
fn pola_without_watermark() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| {
        Regex::new(r#"without_watermark[^>]+href="([^"]+)""#).expect("pola without_watermark")
    })
}

/// `/(\d{1,2}\.\d{1,2}K[^<]+)/` (`likee.js:33`).
fn pola_stats() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r"(\d{1,2}\.\d{1,2}K[^<]+)").expect("pola stats"))
}

/// `pick(re)` (`likee.js:24-27`) — kelompok tangkapan pertama, atau `null`.
fn ambil_tangkapan(html: &str, re: &Regex) -> Value {
    match re.captures(html).and_then(|c| c.get(1)) {
        Some(m) => json!(m.as_str()),
        None => Value::Null,
    }
}

/// Serialisasi `URLSearchParams` (`application/x-www-form-urlencoded`).
///
/// BEDA dari `encode_uri_component`: spasi jadi `+`, dan `!'()~` ikut di-encode;
/// yang dibiarkan hanya `*-._` dan alnum.
fn encode_form(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '*' | '-' | '.' | '_') {
            out.push(c);
        } else if c == ' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
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
            unreachable!("scraper ini mengirim form")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
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

    /// Template HTML bentuk yang dipakai `likeedownloader.com`.
    fn template() -> String {
        let html = r#"<div><img class="c" src="https://cdn.likee.video/cover.jpg"><a id="with_watermark" href="https://cdn.likee.video/wm.mp4">Download</a><a id="without_watermark" href="https://cdn.likee.video/nowm.mp4">HD</a><span>12.34K views, 567 likes</span></div>"#;
        serde_json::to_string(&json!({ "template": html })).unwrap()
    }

    #[test]
    fn mengambil_semua_empat_field_dan_mengirim_form_yang_benar() {
        let http = Stub::baru(&template());
        let hasil = likee(&http, "https://likee.video/v/abc123").unwrap();
        assert_eq!(hasil["thumbnail"], "https://cdn.likee.video/cover.jpg");
        assert_eq!(hasil["with_watermark"], "https://cdn.likee.video/wm.mp4");
        assert_eq!(hasil["without_watermark"], "https://cdn.likee.video/nowm.mp4");
        assert_eq!(hasil["stats"], "12.34K views, 567 likes");
        assert_eq!(
            hasil
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "thumbnail",
                "with_watermark",
                "without_watermark",
                "stats"
            ]
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://likeedownloader.com/process");
        assert_eq!(cari_header(&p[0], "user-agent"), Some("Mozilla/5.0 (Linux; Android 10)"));
        assert_eq!(
            cari_header(&p[0], "accept"),
            Some("application/json,text/javascript,*/*")
        );
        assert_eq!(cari_header(&p[0], "x-requested-with"), Some("XMLHttpRequest"));
        assert_eq!(
            cari_header(&p[0], "content-type"),
            Some("application/x-www-form-urlencoded")
        );
        assert_eq!(cari_header(&p[0], "origin"), Some("https://likeedownloader.com"));
        assert_eq!(cari_header(&p[0], "referer"), Some("https://likeedownloader.com/en"));
        assert_eq!(
            p[0].body,
            Some(HttpBody::Form(
                "id=https%3A%2F%2Flikee.video%2Fv%2Fabc123&locale=en".to_string()
            ))
        );
    }

    #[test]
    fn spasi_dalam_url_jadi_tanda_tambah_bukan_persen20() {
        let http = Stub::baru(&template());
        likee(&http, "https://likee.video/v/a b").unwrap();
        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            p[0].body,
            Some(HttpBody::Form(
                "id=https%3A%2F%2Flikee.video%2Fv%2Fa+b&locale=en".to_string()
            ))
        );
    }

    #[test]
    fn url_di_luar_host_likee_ditolak_tanpa_jaringan() {
        let http = Stub::baru(&template());
        let err = likee(&http, "https://youtube.com/watch?v=x").unwrap_err();
        assert_eq!(err.message, "Invalid url");
        assert_eq!(http.permintaan.lock().unwrap().len(), 0);
    }

    #[test]
    fn template_kosong_memberi_fetch_failed() {
        for balasan in [
            r#"{}"#,
            r#"{"template":null}"#,
            r#"{"template":""}"#,
            r#"{"template":0}"#,
            r#"{"template":false}"#,
        ] {
            let http = Stub::baru(balasan);
            let err = likee(&http, "https://likee.video/x").unwrap_err();
            assert_eq!(err.message, "Fetch failed", "balasan {balasan}");
        }
    }

    #[test]
    fn template_non_string_melempar_type_error_js() {
        // `html.match` tidak ada di angka -> TypeError yang diteruskan ulang.
        let http = Stub::baru(r#"{"template":123}"#);
        let err = likee(&http, "https://likee.video/x").unwrap_err();
        assert_eq!(err.message, "html.match is not a function");
    }

    #[test]
    fn encode_form_sama_dengan_urlsearchparams() {
        // `*-._` dibiarkan apa adanya; `!'()~` di-encode; spasi jadi `+`.
        assert_eq!(encode_form("a b!'()*~-_."), "a+b%21%27%28%29*%7E-_.");
        assert_eq!(encode_form("https://likee.video/v/a b"), "https%3A%2F%2Flikee.video%2Fv%2Fa+b");
    }

    #[test]
    fn regex_yang_gagal_masih_menghasilkan_kunci_null() {
        let http = Stub::baru(r#"{"template":"<div>kosong</div>"}"#);
        let hasil = likee(&http, "https://likee.video/x").unwrap();
        for kunci in [
            "thumbnail",
            "with_watermark",
            "without_watermark",
            "stats",
        ] {
            assert_eq!(hasil[kunci], Value::Null, "kunci {kunci} harus ada");
        }
    }

    #[test]
    fn galat_http_diteruskan_pesan_persis() {
        let err = likee(
            &Stub::gagal("Request failed with status code 500"),
            "https://likee.video/x",
        )
        .unwrap_err();
        assert_eq!(err.message, "Request failed with status code 500");
    }
}