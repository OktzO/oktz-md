//! `src/scraper/pindl.js` — Pinterest lewat proxy `ilovepin.net/proxy.php`.
//!
//! Yang ikut disalin dari JS:
//! - Dua permintaan berantai: halaman `/id` hanya diambil untuk cookie, lalu
//!   cookie itu dikirim pada POST kedua.
//! - Tubuh POST dibangun `qs.stringify({ url: pinUrl })`. Enkode `qs` itu
//!   RFC 3986 ketat: hanya `A-Za-z0-9-_.~` dibiarkan, spasi `%20`. Beda dari
//!   `encodeURIComponent` dan dari `URLSearchParams` (yang memakai `+`).
//! - `api.mediaItems || []` disaring `i.type === "Video"` / `"Image"`; kalau
//!   tidak ada video, gambar yang dipakai. Tipe `Audio` dihitung JS tapi tidak
//!   pernah dipakai, jadi tidak ada efeknya.
//! - Urutan video memakai `parseFloat(ukuran) * (ukuran.includes("MB") ? 1024
//!   : 1)`. Satuan KB dan GB tidak dikonversi di JS, jadi perbandingan ikut
//!   campuran seperti di JS (menyalin apa adanya, bukan memperbaiki).
//! - `catch` mengembalikan `null` — termasuk saat `api.status !== "OK"`, jadi
//!   teks `Gagal mengambil data atau URL tidak valid.` tidak pernah sampai ke
//!   pemanggil; ia hanya masuk `console.error`.
//!
//! Beda yang diketahui:
//! - `{ signal }` (pembatalan) tidak punya padanan di `HttpGet`, dan timeout 15
//!   detik milik `httpAxios` tidak ditiru — `UreqHttp` memakai batas global
//!   20 detik.
//! - Nilai `set-cookie` digabung apa adanya dengan `"; "`. Kalau lapisan HTTP
//!   menggabungkan beberapa header `Set-Cookie` menjadi satu nilai yang dipisah
//!   koma, string cookie bisa berbeda dari Node.

use std::cmp::Ordering;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value};

use crate::common::truthy;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

const BAHASA: &str = "id,en;q=0.9,en-GB;q=0.8,en-US;q=0.7";

/// `qs.stringify` (`qs@6.16`, encoder RFC 3986): `A-Za-z0-9-_.~` dibiarkan,
/// spasi `%20`, sisanya persen huruf besar.
fn encode_qs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `parseFloat(s)` — baca bagian depan, sisanya diabaikan; `NaN` kalau tidak
/// ada digit sama sekali.
fn parse_float_js(s: &str) -> f64 {
    let s = s.trim_start();
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut ada_titik = false;
    while i < bytes.len() {
        match bytes[i] {
            b'0'..=b'9' => i += 1,
            b'.' if !ada_titik => {
                ada_titik = true;
                i += 1;
            }
            _ => break,
        }
    }
    s[..i].parse::<f64>().unwrap_or(f64::NAN)
}

/// `parseFloat(ukuran) * (ukuran.includes("MB") ? 1024 : 1)`. Nilai non-string
/// membuat `.includes` melempar di JS; itu ditangani pemanggil sebagai `null`.
fn ukuran_video(v: &Value) -> Result<f64, HttpError> {
    let ukuran = v
        .get("mediaFileSize")
        .and_then(Value::as_str)
        .ok_or_else(|| HttpError {
            message: "mediaFileSize bukan string".to_string(),
        })?;
    let kalikan = if ukuran.contains("MB") { 1024.0 } else { 1.0 };
    Ok(parse_float_js(ukuran) * kalikan)
}

/// Komparator `sizeB - sizeA`; `NaN` dianggap `0` oleh `Array#sort` (stabil).
fn banding_ukuran(a: &f64, b: &f64) -> Ordering {
    b.partial_cmp(a).unwrap_or(Ordering::Equal)
}

/// `String(v)` — dipakai hanya untuk `SD (${mediaRes})`.
fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `api.description?.trim() || "-"`. Nilai non-string yang bukan null membuat
/// `.trim` melempar di JS — itu berarti hasil akhirnya `null`.
fn deskripsi(v: Option<&Value>) -> Result<Value, HttpError> {
    match v {
        None | Some(Value::Null) => Ok(Value::String("-".to_string())),
        Some(Value::String(s)) => {
            let potong = s.trim();
            Ok(if potong.is_empty() {
                Value::String("-".to_string())
            } else {
                Value::String(potong.to_string())
            })
        }
        Some(_) => Err(HttpError {
            message: "description bukan string".to_string(),
        }),
    }
}

/// `extension` / `size` / `url` — nama kunci hasil berbeda dari nama field di
/// respons (`mediaExtension`, `mediaFileSize`, `mediaUrl`). Field yang tidak ada
/// di respons tidak ikut, sama seperti `JSON.stringify`.
fn sisip_tiga_field(m: &mut Map<String, Value>, v: &Value) {
    for (keluaran, sumber) in [
        ("extension", "mediaExtension"),
        ("size", "mediaFileSize"),
        ("url", "mediaUrl"),
    ] {
        if let Some(x) = v.get(sumber) {
            m.insert(keluaran.to_string(), x.clone());
        }
    }
}

/// `scrapePinterest(pinUrl)` tanpa `try/catch` — `null` dihasilkan oleh
/// pembungkus yang memanggilnya.
fn scrape_inti(http: &dyn HttpGet, pin_url: &str) -> Result<Value, HttpError> {
    let halaman = http.call(&HttpCall::get("https://ilovepin.net/id").header("user-agent", UA))?;
    let cookie = halaman
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.as_str())
        .collect::<Vec<_>>()
        .join("; ");

    let body = format!("url={}", encode_qs(pin_url));
    let res = http.call(
        &HttpCall::post("https://ilovepin.net/proxy.php")
            .header("user-agent", UA)
            .header("accept", "*/*")
            .header("accept-language", BAHASA)
            .header("x-requested-with", "XMLHttpRequest")
            .header("origin", "https://ilovepin.net")
            .header("referer", "https://ilovepin.net/id")
            .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
            .header("cookie", cookie.as_str())
            .raw(body.into_bytes(), "application/x-www-form-urlencoded; charset=UTF-8"),
    )?;
    let data = res.json().unwrap_or(Value::Null);

    // `if (!data.api || data.api.status !== "OK")`
    let gagal = match data.get("api") {
        Some(api) => api.get("status").and_then(Value::as_str) != Some("OK"),
        None => true,
    };
    if gagal {
        return Err(HttpError {
            message: "Gagal mengambil data atau URL tidak valid.".to_string(),
        });
    }
    let api = data.get("api").cloned().unwrap_or(Value::Null);

    // `const items = api.mediaItems || []` — nilai truthy non-array membuat
    // `.filter` melempar di JS, itu berarti `catch` lalu `null`.
    let items: Vec<Value> = match api.get("mediaItems") {
        Some(v) if truthy(v) => match v.as_array() {
            Some(a) => a.clone(),
            None => {
                return Err(HttpError {
                    message: "mediaItems bukan array".to_string(),
                });
            }
        },
        _ => Vec::new(),
    };
    let videos: Vec<Value> = items
        .iter()
        .filter(|i| i.get("type").and_then(Value::as_str) == Some("Video"))
        .cloned()
        .collect();
    let images: Vec<Value> = items
        .iter()
        .filter(|i| i.get("type").and_then(Value::as_str) == Some("Image"))
        .cloned()
        .collect();

    let mut result = Map::new();
    // `title: api.title` — `undefined` tidak ikut di `JSON.stringify`.
    if let Some(t) = api.get("title") {
        result.insert("title".to_string(), t.clone());
    }
    result.insert("description".to_string(), deskripsi(api.get("description"))?);

    let mut author = Map::new();
    for (keluaran, sumber) in [
        ("name", "name"),
        ("username", "username"),
        ("avatar", "userAvatar"),
    ] {
        if let Some(v) = api.get("userInfo").and_then(|u| u.get(sumber)) {
            author.insert(keluaran.to_string(), v.clone());
        }
    }
    result.insert("author".to_string(), Value::Object(author));

    let mut stats = Map::new();
    for (keluaran, sumber) in [("likes", "likesCount"), ("shares", "sharesCount")] {
        if let Some(v) = api.get("mediaStats").and_then(|s| s.get(sumber)) {
            stats.insert(keluaran.to_string(), v.clone());
        }
    }
    result.insert("stats".to_string(), Value::Object(stats));

    let media: Vec<Value> = if !videos.is_empty() {
        // `sort_by` Rust juga stabil, jadi ukuran sama tetap urut seperti JS.
        let mut berukuran: Vec<(f64, Value)> = Vec::with_capacity(videos.len());
        for v in &videos {
            berukuran.push((ukuran_video(v)?, v.clone()));
        }
        berukuran.sort_by(|a, b| banding_ukuran(&a.0, &b.0));
        berukuran
            .iter()
            .map(|(_, v)| {
                let mut m = Map::new();
                m.insert("type".to_string(), Value::String("video".to_string()));
                let hd = v.get("mediaQuality").and_then(Value::as_str) == Some("HD");
                m.insert(
                    "quality".to_string(),
                    Value::String(if hd {
                        "HD".to_string()
                    } else {
                        format!("SD ({})", v.get("mediaRes").map(teks_js).unwrap_or_default())
                    }),
                );
                sisip_tiga_field(&mut m, v);
                Value::Object(m)
            })
            .collect()
    } else if !images.is_empty() {
        images
            .iter()
            .map(|img| {
                let mut m = Map::new();
                m.insert("type".to_string(), Value::String("image".to_string()));
                m.insert("quality".to_string(), Value::String("Original".to_string()));
                sisip_tiga_field(&mut m, img);
                Value::Object(m)
            })
            .collect()
    } else {
        Vec::new()
    };
    result.insert("media".to_string(), Value::Array(media));

    Ok(Value::Object(result))
}

/// `scrapePinterest(pinUrl, { signal } = {})` (`pindl.js:17-104`) — `catch`
/// mengembalikan `null`, jadi fungsi ini tidak pernah gagal.
pub fn scrape_pinterest(http: &dyn HttpGet, pin_url: &str) -> Result<Value, HttpError> {
    match scrape_inti(http, pin_url) {
        Ok(v) => Ok(v),
        Err(_) => Ok(Value::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` yang mencatat permintaan dan membalas JSON tetap per
    /// nomor permintaan (0 = halaman cookie, 1 = proxy).
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<String>>,
        header: Vec<(String, String)>,
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini mengirim form, bukan JSON")
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
            let nomor = {
                let mut p = self.permintaan.lock().unwrap();
                p.push(req.clone());
                p.len() - 1
            };
            let isi = self
                .balasan
                .lock()
                .unwrap()
                .get(nomor)
                .cloned()
                .unwrap_or_default();
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(isi.into_bytes()),
                    mimetype: None,
                },
                headers: self.header.clone(),
            })
        }
    }

    fn stub(daftar: &[&str]) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(daftar.iter().map(|s| (*s).to_string()).collect()),
            header: Vec::new(),
        }
    }

    const VIDEO: &str = r#"{"api":{"status":"OK","title":"heyeeog's Media",
        "description":"  ","userInfo":{"name":"H","username":"heyeeog","userAvatar":"https://a/ava.jpg"},
        "mediaStats":{"likesCount":"1K","sharesCount":"40K"},
        "mediaItems":[
          {"type":"Video","mediaQuality":"SD","mediaRes":"360x640","mediaExtension":"MP4","mediaFileSize":"760.32 KB","mediaUrl":"https://s15/360p"},
          {"type":"Video","mediaQuality":"HD","mediaRes":"720p","mediaExtension":"MP4","mediaFileSize":"2.16 MB","mediaUrl":"https://s15/720p"},
          {"type":"Video","mediaQuality":"HD","mediaRes":"480p","mediaExtension":"MP4","mediaFileSize":"2.16 MB","mediaUrl":"https://s15/480p"},
          {"type":"Image","mediaExtension":"JPG","mediaFileSize":"1 MB","mediaUrl":"https://s15/gambar"}
        ]}}"#;

    #[test]
    fn video_terurut_dari_yang_terbesar_serta_kirim_cookie() {
        let mut http = stub(&["<html>", VIDEO]);
        http.header = vec![
            ("Set-Cookie".to_string(), "PHPSESSID=abc; path=/".to_string()),
            ("set-cookie".to_string(), "csrf=def".to_string()),
        ];
        let hasil =
            scrape_pinterest(&http, "https://id.pinterest.com/pin/87186942777228203/").unwrap();
        assert_eq!(hasil["title"], "heyeeog's Media");
        assert_eq!(hasil["description"], "-", "spasi kosong jadi '-'");
        assert_eq!(hasil["author"]["name"], "H");
        assert_eq!(hasil["author"]["username"], "heyeeog");
        assert_eq!(hasil["author"]["avatar"], "https://a/ava.jpg");
        assert_eq!(hasil["stats"]["likes"], "1K");
        assert_eq!(hasil["stats"]["shares"], "40K");

        let media = hasil["media"].as_array().unwrap();
        assert_eq!(media.len(), 3, "gambar diabaikan karena ada video");
        // 2.16 MB (2.16*1024) lebih besar dari 760.32 KB (760.32), dan dua
        // "HD" berukuran sama tetap urut input karena sort JS stabil.
        assert_eq!(media[0]["url"], "https://s15/720p");
        assert_eq!(media[0]["quality"], "HD");
        assert_eq!(media[1]["url"], "https://s15/480p");
        assert_eq!(media[2]["quality"], "SD (360x640)");
        assert_eq!(media[2]["extension"], "MP4");
        assert_eq!(media[2]["size"], "760.32 KB");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].url, "https://ilovepin.net/id");
        assert_eq!(p[1].url, "https://ilovepin.net/proxy.php");
        assert_eq!(
            p[1].body,
            Some(oktz_plugin_api::HttpBody::Bytes {
                data: std::sync::Arc::from(
                    "url=https%3A%2F%2Fid.pinterest.com%2Fpin%2F87186942777228203%2F"
                        .to_string()
                        .into_bytes()
                ),
                mimetype: Some("application/x-www-form-urlencoded; charset=UTF-8".to_string()),
            })
        );
        let cari = |nama: &str| {
            p[1].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(
            cari("cookie").as_deref(),
            Some("PHPSESSID=abc; path=/; csrf=def"),
            "nilai set-cookie digabung apa adanya dengan '; '"
        );
        assert_eq!(cari("origin").as_deref(), Some("https://ilovepin.net"));
        assert_eq!(cari("referer").as_deref(), Some("https://ilovepin.net/id"));
        assert_eq!(cari("x-requested-with").as_deref(), Some("XMLHttpRequest"));
    }

    #[test]
    fn gambar_dipakai_kalau_tidak_ada_video_dan_kunci_kosong_hilang() {
        let http = stub(&[
            "<html>",
            r#"{"api":{"status":"OK","title":"Gambar","description":" deskripsi ",
                "mediaItems":[{"type":"Image","mediaExtension":"JPG","mediaFileSize":"1 MB","mediaUrl":"https://s15/a.jpg"}]}}"#,
        ]);
        let hasil = scrape_pinterest(&http, "https://pin.it/abc").unwrap();
        assert_eq!(hasil["description"], "deskripsi");
        assert_eq!(hasil["media"][0]["type"], "image");
        assert_eq!(hasil["media"][0]["quality"], "Original");
        assert_eq!(hasil["media"][0]["url"], "https://s15/a.jpg");
        // `userInfo`/`mediaStats` tidak ada → objek author dan stats kosong,
        // bukan null.
        assert!(hasil["author"].as_object().unwrap().is_empty());
        assert!(hasil["stats"].as_object().unwrap().is_empty());
    }

    #[test]
    fn gagal_selalu_menghasilkan_null_seperti_catch_js() {
        // Status API bukan "OK" → teks hanya masuk `console.error`, hasilnya
        // tetap `null`.
        let bukan_ok = stub(&["<html>", r#"{"api":{"status":"ERROR"}}"#]);
        assert!(scrape_pinterest(&bukan_ok, "https://x").unwrap().is_null());

        let tanpa_api = stub(&["<html>", "{}"]);
        assert!(scrape_pinterest(&tanpa_api, "https://x").unwrap().is_null());

        // Body bukan JSON → `!data.api` → `null`.
        let bukan_json = stub(&["<html>", "<html>gaben"]);
        assert!(scrape_pinterest(&bukan_json, "https://x").unwrap().is_null());

        // `mediaItems` truthy tapi bukan array → `.filter` melempar → `null`.
        let media_items_aneh = stub(&["<html>", r#"{"api":{"status":"OK","mediaItems":{"a":1}}}"#]);
        assert!(scrape_pinterest(&media_items_aneh, "https://x")
            .unwrap()
            .is_null());

        // `mediaFileSize` bukan string pada salah satu video → `.includes`
        // melempar → `null`.
        let ukuran_aneh = stub(&[
            "<html>",
            r#"{"api":{"status":"OK","mediaItems":[{"type":"Video","mediaFileSize":216}]}}"#,
        ]);
        assert!(scrape_pinterest(&ukuran_aneh, "https://x").unwrap().is_null());
    }

    #[test]
    fn encode_qs_meniru_encoder_strict_paket_qs() {
        assert_eq!(
            encode_qs("https://x.com/a b?c=1&d=!*()~'"),
            "https%3A%2F%2Fx.com%2Fa%20b%3Fc%3D1%26d%3D%21%2A%28%29~%27"
        );
        // Tidak seperti `encodeURIComponent`, yang membiarkan `!'()*`.
        assert_eq!(encode_qs("a!*()~'-_."), "a%21%2A%28%29~%27-_.");
    }
}