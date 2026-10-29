//! `src/scraper/soundclouddl.js` — dua POST ke `convertico.com` (info lagu,
//! lalu link unduhan).
//!
//! Semantik yang disalin dari JS:
//! - Body adalah `new URLSearchParams({...})`, jadi form-urlencoded: spasi `+`,
//!   hanya alfanumerik dan `* - . _` dibiarkan, urutan kunci mengikuti urutan
//!   objek literal. `content-type` tidak ditulis di JS — axios yang menambah
//!   `application/x-www-form-urlencoded;charset=utf-8`, jadi di sini dipasang
//!   eksplisit.
//! - `if (!info.status) throw new Error("Gagal mengambil info lagu")` dan
//!   `if (!dl.file_url) throw new Error("Gagal generate link download")` — cek
//!   falsy, teks error persis.
//! - `catch (err) { throw new Error(err.message) }` jadi `Err` dengan pesan
//!   yang sama; fungsi tidak pernah mengembalikan objek error.
//! - `info.view_count.toLocaleString()` pada `null`/`undefined` melempar
//!   TypeError dengan teks JS, jadi hasilnya `Err`, bukan nilai kosong.
//! - `duration` tidak melempar walau `undefined`: `"NaN:NaN"`.
//! - `dl.file_url.split('/').map(encodeURIComponent).join('/')` memakai
//!   `encodeURIComponent` (bukan gaya form), jadi `!'()*-._~` tetap apa adanya.
//!
//! Beda yang diketahui:
//! - `toLocaleString()` tanpa locale relies ke locale bawaan Node; di sini
//!   pengelompokan thousands memakai koma gaya `en-US`.
//! - `toFixed(2)` JS memakai pembulatan ke atas pada kasus seri tepat; Rust
//!   membulatkan half-to-even. Selisihnya cuma pada angka yang persis
//!   `.xx5`.
//! - `err.message` untuk kegagalan jaringan milik mesin Rust (`HttpError`),
//!   bukan teks axios.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value};

use crate::common::{encode_uri_component, truthy};

const BASE: &str = "https://convertico.com/";
const ENDPOINT: &str = "https://convertico.com/soundcloud-downloader/soundcloud-downloader.php";

/// `encodeURIComponent` gaya `URLSearchParams`: spasi `+`, alfanumerik dan
/// `* - . _` dibiarkan. Beda dari `encode_uri_component` (yang mengikuti
/// `encodeURIComponent` dan dipakai untuk `file_url`).
fn form_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b' ' => out.push('+'),
            b'*' | b'-' | b'.' | b'_' => out.push(b as char),
            _ if b.is_ascii_alphanumeric() => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// `URLSearchParams(...).toString()`.
fn form_body(bagian: &[(&str, String)]) -> String {
    bagian
        .iter()
        .map(|(k, v)| format!("{}={}", form_escape(k), form_escape(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Header tetap dua POST (`soundclouddl.js:6-11`).
fn header_dasar(req: HttpCall) -> HttpCall {
    req.header("accept", "*/*")
        .header("origin", BASE)
        .header("referer", format!("{BASE}soundcloud-downloader/"))
        .header(
            "user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        )
        // Ditambahkan axios untuk `URLSearchParams`, tidak ada di objek JS.
        .header("content-type", "application/x-www-form-urlencoded;charset=utf-8")
}

/// `ToNumber` JS untuk nilai opsional: kunci yang tidak ada berarti
/// `undefined` (NaN), bukan `null` (0).
fn ke_angka_ada(v: Option<&Value>) -> f64 {
    match v {
        Some(x) => ke_angka(x),
        None => f64::NAN,
    }
}

/// `ToNumber` JS untuk nilai yang masuk aritmetika.
fn ke_angka(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Null => 0.0,
        Value::String(s) => {
            let s = s.trim();
            if s.is_empty() {
                0.0
            } else {
                s.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) | Value::Object(_) => f64::NAN,
    }
}

/// `String(n)` JS.
fn angka_js(n: f64) -> String {
    if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        let teks = if n > 0.0 { "Infinity" } else { "-Infinity" };
        teks.to_string()
    } else {
        format!("{n}")
    }
}

/// `v.toLocaleString()` — `null`/`undefined` melempar TypeError di JS, jadi
/// hasilnya `Err` dengan teks yang sama.
fn locale_number(v: &Value) -> Result<String, HttpError> {
    match v {
        Value::Null => Err(HttpError {
            message: "Cannot read properties of null (reading 'toLocaleString')".to_string(),
        }),
        // `String.prototype.toLocaleString` mengembalikan stringnya apa adanya.
        Value::String(s) => Ok(s.clone()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => Ok(kelompok(n.as_f64().unwrap_or(f64::NAN))),
        // `Object.prototype.toLocaleString`.
        Value::Object(_) => Ok("[object Object]".to_string()),
        // `Array.prototype.toLocaleString` = join dengan koma.
        Value::Array(a) => {
            let isi = a
                .iter()
                .map(|x| match x {
                    Value::Null => Ok(String::new()),
                    lain => locale_number(lain),
                })
                .collect::<Result<Vec<_>, HttpError>>()?;
            Ok(isi.join(","))
        }
    }
}

/// Pemisah ribuan gaya `en-US`: koma tiap tiga digit, bagian desimal tidak
/// diubah.
fn kelompok(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        let teks = if n > 0.0 { "Infinity" } else { "-Infinity" };
        return teks.to_string();
    }
    let teks = angka_js(n);
    let (bulat, desimal) = match teks.split_once('.') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (teks.clone(), None),
    };
    let negatif = bulat.starts_with('-');
    let digit = bulat.trim_start_matches('-');
    let mut keluar = String::new();
    for (i, c) in digit.chars().enumerate() {
        if i > 0 && (digit.len() - i) % 3 == 0 {
            keluar.push(',');
        }
        keluar.push(c);
    }
    if negatif {
        keluar.insert(0, '-');
    }
    match desimal {
        Some(d) => format!("{keluar}.{d}"),
        None => keluar,
    }
}

/// `(n / 1024 / 1024).toFixed(2)` — `NaN` jadi `"NaN"` seperti JS.
fn fixed2(n: f64) -> String {
    if n.is_nan() {
        "NaN".to_string()
    } else if n.is_infinite() {
        let teks = if n > 0.0 { "Infinity" } else { "-Infinity" };
        teks.to_string()
    } else {
        format!("{n:.2}")
    }
}

/// `scdl(url)` (`soundclouddl.js:2-48`).
pub fn scdl(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let res_info = http.call(&header_dasar(HttpCall::post(ENDPOINT)).form(form_body(&[
        ("action", "fetch".to_string()),
        ("url", url.to_string()),
    ])))?;
    // Body yang bukan JSON: axios memberi string, `info.status` jadi falsy.
    let info = res_info.json().unwrap_or(Value::Null);
    if !info.get("status").map(truthy).unwrap_or(false) {
        return Err(HttpError {
            message: "Gagal mengambil info lagu".to_string(),
        });
    }

    let res_dl = http.call(&header_dasar(HttpCall::post(ENDPOINT)).form(form_body(&[
        ("action", "download".to_string()),
        ("url", url.to_string()),
        ("quality", "192".to_string()),
        ("is_playlist", "0".to_string()),
    ])))?;
    let dl = res_dl.json().unwrap_or(Value::Null);
    let Some(file_url) = dl.get("file_url").filter(|v| truthy(v)) else {
        return Err(HttpError {
            message: "Gagal generate link download".to_string(),
        });
    };
    // `dl.file_url.split(...)` melempar kalau nilainya bukan string truthy.
    let Some(file_url) = file_url.as_str() else {
        return Err(HttpError {
            message: "dl.file_url.split is not a function".to_string(),
        });
    };
    let download_url = format!(
        "{BASE}soundcloud-downloader/{}",
        file_url
            .split('/')
            .map(encode_uri_component)
            .collect::<Vec<_>>()
            .join("/")
    );

    // `info.view_count.toLocaleString()` dan `info.like_count.toLocaleString()`.
    let Some(view_count) = info.get("view_count") else {
        return Err(HttpError {
            message: "Cannot read properties of undefined (reading 'toLocaleString')"
                .to_string(),
        });
    };
    let Some(like_count) = info.get("like_count") else {
        return Err(HttpError {
            message: "Cannot read properties of undefined (reading 'toLocaleString')"
                .to_string(),
        });
    };
    let views = locale_number(view_count)?;
    let likes = locale_number(like_count)?;

    let durasi = ke_angka_ada(info.get("duration"));
    let detik = angka_js(durasi % 60.0);
    let detik = if detik.len() < 2 {
        format!("0{detik}")
    } else {
        detik
    };
    let duration = format!("{}:{detik}", angka_js((durasi / 60.0).floor()));

    let mut out = Map::new();
    // `title`, `uploader`, `thumbnail`, `format` hilang kalau `undefined`;
    // urutan sisip mengikuti urutan kunci literal JS.
    for kunci in ["title", "uploader"] {
        let sumber = if kunci == "title" { "title" } else { "author" };
        if let Some(v) = info.get(sumber) {
            out.insert(kunci.to_string(), v.clone());
        }
    }
    out.insert("duration".to_string(), Value::String(duration));
    out.insert("views".to_string(), Value::String(views));
    out.insert("likes".to_string(), Value::String(likes));
    if let Some(v) = info.get("thumbnail") {
        out.insert("thumbnail".to_string(), v.clone());
    }
    out.insert(
        "size".to_string(),
        Value::String(format!(
            "{} MB",
            fixed2(ke_angka_ada(dl.get("size")) / 1024.0 / 1024.0)
        )),
    );
    if let Some(v) = dl.get("format") {
        out.insert("format".to_string(), v.clone());
    }
    out.insert("download_url".to_string(), Value::String(download_url));
    Ok(Value::Object(out))
}

/// Bantu test: balas `HttpResponse` tetap tanpa jaringan.
#[cfg(test)]
fn dummy_res(body: &str) -> oktz_plugin_api::HttpResponse {
    oktz_plugin_api::HttpResponse {
        status: 200,
        body: oktz_plugin_api::HttpBody::Bytes {
            data: std::sync::Arc::from(body.as_bytes()),
            mimetype: None,
        },
        headers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall as Call};
    use serde_json::json;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas body
    /// berurutan; setelah habis, balasan terakhir dipakai lagi.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<Call>>,
        balasan: Mutex<Vec<String>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: &[&str]) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan.iter().map(|s| s.to_string()).collect()),
                hitung: Mutex::new(0),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(&self, req: &Call) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            Ok(dummy_res(&balasan[i]))
        }
    }

    const INFO: &str = r#"{"status":true,"title":"Judul Lagu","author":"Artis",
        "duration":215,"view_count":1234567,"like_count":890,
        "thumbnail":"https://t/1.jpg"}"#;
    const DL: &str = r#"{"file_url":"/mp3/lagu satu (remix).mp3","size":5242880,
        "format":"mp3"}"#;

    #[test]
    fn dua_post_dengan_body_form_dan_url_encode_uri_component() {
        let http = StubHttp::baru(&[INFO, DL]);
        let hasil = scdl(&http, "https://soundcloud.com/a/lagu").unwrap();

        assert_eq!(hasil["title"], "Judul Lagu");
        assert_eq!(hasil["uploader"], "Artis");
        assert_eq!(hasil["duration"], "3:35");
        assert_eq!(hasil["views"], "1,234,567");
        assert_eq!(hasil["likes"], "890");
        assert_eq!(hasil["thumbnail"], "https://t/1.jpg");
        assert_eq!(hasil["size"], "5.00 MB");
        assert_eq!(hasil["format"], "mp3");
        // `encodeURIComponent`: spasi `%20`, tanda `!'()*-._~` dibiarkan. `file_url`
        // diawali `/`, jadi hasil join memang punya dua garis miring.
        assert_eq!(
            hasil["download_url"],
            "https://convertico.com/soundcloud-downloader//mp3/lagu%20satu%20(remix).mp3"
        );
        assert_eq!(
            hasil
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "title",
                "uploader",
                "duration",
                "views",
                "likes",
                "thumbnail",
                "size",
                "format",
                "download_url"
            ]
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        for req in p.iter() {
            assert_eq!(req.url, ENDPOINT);
            let cari = |nama: &str| {
                req.headers
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                    .map(|(_, v)| v.clone())
            };
            assert_eq!(cari("accept"), Some("*/*".to_string()));
            assert_eq!(cari("origin"), Some(BASE.to_string()));
            assert_eq!(
                cari("referer"),
                Some("https://convertico.com/soundcloud-downloader/".to_string())
            );
            assert!(cari("user-agent").unwrap().contains("Chrome/120.0.0.0"));
        }
        // `URLSearchParams`: spasi `+`, urutan kunci seperti objek JS.
        assert_eq!(
            p[0].body,
            Some(HttpBody::Form(
                "action=fetch&url=https%3A%2F%2Fsoundcloud.com%2Fa%2Flagu".to_string()
            ))
        );
        assert_eq!(
            p[1].body,
            Some(HttpBody::Form(
                "action=download&url=https%3A%2F%2Fsoundcloud.com%2Fa%2Flagu&quality=192&is_playlist=0"
                    .to_string()
            ))
        );
    }

    #[test]
    fn status_info_falsy_berhenti_di_permintaan_pertama() {
        for balasan in [r#"{"status":false}"#, "bukan json"] {
            let http = StubHttp::baru(&[balasan]);
            let galat = scdl(&http, "https://s/1").unwrap_err();
            assert_eq!(galat.message, "Gagal mengambil info lagu");
            assert_eq!(http.permintaan.lock().unwrap().len(), 1);
        }
    }

    #[test]
    fn file_url_kosong_berhenti_di_permintaan_kedua() {
        let http = StubHttp::baru(&[INFO, r#"{"status":true}"#]);
        let galat = scdl(&http, "https://s/1").unwrap_err();
        assert_eq!(galat.message, "Gagal generate link download");
        assert_eq!(http.permintaan.lock().unwrap().len(), 2);
    }

    #[test]
    fn view_count_hilang_ikut_teks_typeerror_js() {
        let tanpa_like = r#"{"status":true,"duration":10,"view_count":5}"#;
        let http = StubHttp::baru(&[tanpa_like, DL]);
        let galat = scdl(&http, "https://s/1").unwrap_err();
        assert_eq!(
            galat.message,
            "Cannot read properties of undefined (reading 'toLocaleString')"
        );

        let tanpa_view = r#"{"status":true,"duration":10,"like_count":5}"#;
        let http = StubHttp::baru(&[tanpa_view, DL]);
        let galat = scdl(&http, "https://s/1").unwrap_err();
        assert_eq!(
            galat.message,
            "Cannot read properties of undefined (reading 'toLocaleString')"
        );
    }

    #[test]
    fn field_hilang_tetap_menghasilkan_durasi_dan_size_nan() {
        // `duration`, `size`, `title`, `thumbnail`, `format` tidak ada.
        let http = StubHttp::baru(&[
            r#"{"status":1,"view_count":0,"like_count":0}"#,
            r#"{"file_url":"/a.mp3"}"#,
        ]);
        let hasil = scdl(&http, "https://s/1").unwrap();
        assert_eq!(hasil["duration"], "NaN:NaN");
        assert_eq!(hasil["size"], "NaN MB");
        assert_eq!(hasil["views"], "0");
        assert_eq!(hasil["download_url"], "https://convertico.com/soundcloud-downloader//a.mp3");
        assert!(hasil.get("title").is_none());
        assert!(hasil.get("thumbnail").is_none());
        assert!(hasil.get("format").is_none());
    }

    #[test]
    fn kelompok_ribuan_dan_bulat_dua_digit() {
        assert_eq!(locale_number(&json!(1234)).unwrap(), "1,234");
        assert_eq!(locale_number(&json!(-1234567)).unwrap(), "-1,234,567");
        assert_eq!(locale_number(&json!(999)).unwrap(), "999");
        assert_eq!(locale_number(&json!("1234")).unwrap(), "1234");
        assert_eq!(locale_number(&json!(true)).unwrap(), "true");
        assert_eq!(
            locale_number(&Value::Null).unwrap_err().message,
            "Cannot read properties of null (reading 'toLocaleString')"
        );
        assert_eq!(fixed2(f64::NAN), "NaN");
        assert_eq!(fixed2(2.5), "2.50");
        assert_eq!(fixed2(1.0 / 3.0), "0.33");
    }
}