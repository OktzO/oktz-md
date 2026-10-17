//! `src/scraper/soundcloud.js` — pencarian lagu lewat API seluler SoundCloud.
//!
//! Semantik yang disalin dari JS:
//! - `res.data?.collection || []` — `collection` yang hilang, `null`, `0`, atau
//!   `""` jadi array kosong; nilai truthy bukan array membuat `.map` melempar
//!   TypeError dan masuk `catch`.
//! - `track.user?.username || null`, `track.genre || null`,
//!   `track.playback_count || 0` — jatuhan `||` (angka `0` dan `""` falsy).
//! - Kunci `id`, `title`, `duration_ms`, `url`, `artwork` hilang kalau
//!   `undefined`, seperti `JSON.stringify`.
//! - `formatDuration(ms)` memakai `Math.floor` dan `%`, jadi `undefined`
//!   menghasilkan `"NaN:NaN"` — bukan string kosong dan bukan error.
//! - `catch` mengembalikan objek, bukan melempar: kegagalan HTTP jadi
//!   `{ success: false, ... }` dengan `status` dari respons kalau ada.
//!
//! Beda yang diketahui:
//! - `params` axios memakai encoder `buildURL`: `encodeURIComponent` dengan
//!   `%20` → `+`, `%3A` → `:`, `%24` → `$`, `%2C` → `,` (jadi `!'()*-._~` dan
//!   tanda baca itu tetap apa adanya). Urutan kunci mengikuti urutan objek
//!   `params`; `stage: ""` tetap jadi `stage=` di URL.
//! - `axios` melempar untuk di luar 2xx; agar `err.response.status` masih
//!   terbaca (dipakai di objek error), permintaan memakai
//!   `.terima_semua_status()` dan status diperiksa sendiri.
//! - `err.message` untuk kegagalan jaringan milik mesin Rust (`HttpError`),
//!   bukan teks axios `Network Error`.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, field, truthy};

const BASE_URL: &str = "https://api-mobi.soundcloud.com/search";
const CLIENT_ID: &str = "KKzJxmw11tYpCs6T24P4uUYhqmjalG6M";

/// `buildURL` axios untuk `params`: `encodeURIComponent` lalu `%20` → `+`,
/// `%3A` → `:`, `%24` → `$`, `%2C` → `,` (`lib/helpers/buildURL.js`).
///
/// Jadi beda dari `URLSearchParams`: `!'()*-._~` tetap apa adanya dan `:`,
/// `$`, `,` tidak jadi persen.
fn encode_query(s: &str) -> String {
    encode_uri_component(s)
        .replace("%20", "+")
        .replace("%3A", ":")
        .replace("%24", "$")
        .replace("%2C", ",")
}

/// `axios` `buildURL` untuk `params`.
fn url_pencarian(query: &str) -> String {
    format!(
        "{BASE_URL}?q={}&client_id={}&stage=",
        encode_query(query),
        encode_query(CLIENT_ID)
    )
}

/// `ToNumber` JS untuk nilai opsional: kunci yang tidak ada berarti
/// `undefined` (NaN), bukan `null` (0).
fn ke_angka_ada(v: Option<&Value>) -> f64 {
    match v {
        Some(x) => ke_angka(x),
        None => f64::NAN,
    }
}

/// `ToNumber` JS untuk nilai yang diproses aritmetika (`ms / 1000`).
/// `undefined` (kunci hilang) ditangani `ke_angka_ada`.
fn ke_angka(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => f64::from(u8::from(*b)),
        // `null` jadi 0, string kosong jadi 0, `"12"` jadi 12, lainnya NaN.
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

/// `String(n)` JS untuk angka dan `NaN`.
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

/// `formatDuration(ms)` (`soundcloud.js:5-10`) — `m:ss`, detik dua digit.
/// `track.duration` yang hilang = `undefined`, jadi `"NaN:NaN"`.
fn format_durasi(ms: Option<&Value>) -> String {
    let total = (ke_angka_ada(ms) / 1000.0).floor();
    let menit = (total / 60.0).floor();
    let detik = total % 60.0;
    let teks = angka_js(detik);
    let detik = if teks.len() < 2 {
        format!("0{teks}")
    } else {
        teks
    };
    format!("{}:{detik}", angka_js(menit))
}

/// Objek `catch`: `{ success: false, query, status, message }`.
fn gagal(query: &str, status: u16, message: &str) -> Value {
    json!({
        "success": false,
        "query": query,
        "status": status,
        "message": message,
    })
}

/// `searchSoundCloud(query)` (`soundcloud.js:12-56`).
pub fn search_sound_cloud(http: &dyn HttpGet, query: &str) -> Result<Value, HttpError> {
    let res = match http.call(
        &HttpCall::get(url_pencarian(query))
            .header("Accept", "application/json, text/javascript, */*; q=0.1")
            .header("Content-Type", "application/json")
            .header("User-Agent", "Mozilla/5.0")
            // `err.response.status` dipakai di objek `catch`.
            .terima_semua_status(),
    ) {
        Ok(r) => r,
        // Jaringan gagal: axios tidak punya `err.response`, jadi `status` 500.
        Err(e) => return Ok(gagal(query, 500, &e.message)),
    };
    if !(200..300).contains(&res.status) {
        return Ok(gagal(
            query,
            res.status,
            &format!("Request failed with status code {}", res.status),
        ));
    }

    let data: Value = res.json().unwrap_or(Value::Null);
    // `res.data?.collection || []` — nilai truthy non-array membuat `.map`
    // melempar TypeError di JS, yang sama efeknya dengan objek `catch`.
    let items: Vec<Value> = match data.get("collection") {
        Some(v) if truthy(v) => match v.as_array() {
            Some(a) => a.clone(),
            None => {
                return Ok(gagal(query, 500, "items.map is not a function"));
            }
        },
        _ => Vec::new(),
    };

    let results: Vec<Value> = items.iter().map(petakan_lagu).collect();
    Ok(json!({
        "success": true,
        "query": query,
        "total": results.len(),
        "results": results,
    }))
}

/// Satu objek hasil untuk satu `track` (`soundcloud.js:29-40`).
///
/// `id`, `title`, `duration_ms`, `url`, `artwork` hanya masuk kalau ada
/// nilainya; sisanya selalu ada karena `|| null` / `|| 0`.
fn petakan_lagu(track: &Value) -> Value {
    let mut o = Map::new();
    for kunci in ["id", "title"] {
        if let Some(v) = track.get(kunci) {
            o.insert(kunci.to_string(), v.clone());
        }
    }
    o.insert(
        "artist".to_string(),
        field(track, "user.username")
            .filter(|v| truthy(v))
            .cloned()
            .unwrap_or(Value::Null),
    );
    if let Some(v) = track.get("duration") {
        o.insert("duration_ms".to_string(), v.clone());
    }
    o.insert(
        "duration".to_string(),
        Value::String(format_durasi(track.get("duration"))),
    );
    for kunci in ["permalink_url", "artwork_url"] {
        if let Some(v) = track.get(kunci) {
            o.insert(
                if kunci == "permalink_url" {
                    "url"
                } else {
                    "artwork"
                }
                .to_string(),
                v.clone(),
            );
        }
    }
    o.insert(
        "genre".to_string(),
        track
            .get("genre")
            .filter(|v| truthy(v))
            .cloned()
            .unwrap_or(Value::Null),
    );
    for kunci in ["playback_count", "likes_count"] {
        o.insert(
            if kunci == "playback_count" {
                "plays"
            } else {
                "likes"
            }
            .to_string(),
            track
                .get(kunci)
                .filter(|v| truthy(v))
                .cloned()
                .unwrap_or_else(|| json!(0)),
        );
    }
    Value::Object(o)
}

/// Bantu test: balas `HttpResponse` tetap tanpa jaringan.
#[cfg(test)]
fn dummy_res(status: u16, body: &str) -> oktz_plugin_api::HttpResponse {
    oktz_plugin_api::HttpResponse {
        status,
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
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas status
    /// + body tetap.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        status: u16,
        balasan: String,
        gagal: bool,
    }

    impl StubHttp {
        fn baru(status: u16, balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                status,
                balasan: balasan.to_string(),
                gagal: false,
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            if self.gagal {
                return Err(HttpError {
                    message: "koneksi gagal".to_string(),
                });
            }
            Ok(dummy_res(self.status, &self.balasan))
        }
    }

    const KOLEKSI: &str = r#"{"collection":[
        {"id":123,"title":"Lagu Satu","user":{"username":"Artis"},
         "duration":215000,"permalink_url":"https://s/1","artwork_url":"https://a/1.jpg",
         "genre":"Jazz","playback_count":4321,"likes_count":10},
        {"id":124,"title":"Tanpa Data","user":{},"duration":null,
         "permalink_url":"https://s/2","genre":"","playback_count":0,"likes_count":null}
    ]}"#;

    #[test]
    fn memetakan_hasil_dan_menaati_format_durasi() {
        let http = StubHttp::baru(200, KOLEKSI);
        let hasil = search_sound_cloud(&http, "jazz").unwrap();

        assert_eq!(hasil["success"], true);
        assert_eq!(hasil["query"], "jazz");
        assert_eq!(hasil["total"], 2);
        let results = hasil["results"].as_array().unwrap();

        assert_eq!(results[0]["id"], 123);
        assert_eq!(results[0]["artist"], "Artis");
        assert_eq!(results[0]["duration_ms"], 215000);
        assert_eq!(results[0]["duration"], "3:35");
        assert_eq!(results[0]["url"], "https://s/1");
        assert_eq!(results[0]["artwork"], "https://a/1.jpg");
        assert_eq!(results[0]["genre"], "Jazz");
        assert_eq!(results[0]["plays"], 4321);
        assert_eq!(results[0]["likes"], 10);

        // `user: {}` → `artist: null`; `duration: null` → aritmetika JS
        // memperlakukan `null` sebagai 0, jadi `"0:00"` bukan `"NaN:NaN"`;
        // `genre: ""` falsy → null; `playback_count: 0` falsy → 0;
        // `artwork_url` hilang → kuncinya tidak ikut.
        assert_eq!(results[1]["artist"], Value::Null);
        assert_eq!(results[1]["duration_ms"], Value::Null);
        assert_eq!(results[1]["duration"], "0:00");
        assert_eq!(results[1]["genre"], Value::Null);
        assert_eq!(results[1]["plays"], 0);
        assert_eq!(results[1]["likes"], 0);
        assert!(results[1].get("artwork").is_none());
        assert_eq!(
            results[0]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "id",
                "title",
                "artist",
                "duration_ms",
                "duration",
                "url",
                "artwork",
                "genre",
                "plays",
                "likes"
            ]
        );

        // `params` axios: spasi `+`, `stage: ""` tetap ada sebagai `stage=`.
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(
            p[0].url,
            "https://api-mobi.soundcloud.com/search?q=jazz&client_id=KKzJxmw11tYpCs6T24P4uUYhqmjalG6M&stage="
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("User-Agent"), Some("Mozilla/5.0".to_string()));
        assert_eq!(cari("Content-Type"), Some("application/json".to_string()));
        assert_eq!(
            cari("Accept"),
            Some("application/json, text/javascript, */*; q=0.1".to_string())
        );
    }

    #[test]
    fn query_di_encode_gaya_buildurl_axios() {
        let http = StubHttp::baru(200, KOLEKSI);
        search_sound_cloud(&http, "jazz hits & more").unwrap();
        // Spasi `+`, `&` jadi `%26`.
        {
            let p = http.permintaan.lock().unwrap();
            assert!(p[0].url.contains("q=jazz+hits+%26+more"), "{}", p[0].url);
        }

        // `encodeURIComponent` (dipakai axios `buildURL`) tidak meng-encode
        // `!'()*~`; axios lalu mengembalikan `: $ ,` ke bentuk semula dan
        // mengganti spasi dengan `+`. Jadi `!` tetap `!` — bukan `%21`.
        search_sound_cloud(&http, "a:b$c,d!e").unwrap();
        let p = http.permintaan.lock().unwrap();
        assert!(p[1].url.contains("q=a:b$c,d!e"), "{}", p[1].url);
    }

    #[test]
    fn collection_kosong_hanya_menghasilkan_lagu_kosong() {
        for balasan in [r#"{"collection":[]}"#, r#"{}"#, "bukan json"] {
            let http = StubHttp::baru(200, balasan);
            let hasil = search_sound_cloud(&http, "x").unwrap();
            assert_eq!(hasil["success"], true, "balasan {balasan}");
            assert_eq!(hasil["total"], 0);
            assert_eq!(hasil["results"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    fn collection_non_array_membungkus_typeerror_like_js() {
        let http = StubHttp::baru(200, r#"{"collection":{"a":1}}"#);
        let hasil = search_sound_cloud(&http, "x").unwrap();
        assert_eq!(hasil["success"], false);
        assert_eq!(hasil["status"], 500);
        assert_eq!(hasil["message"], "items.map is not a function");
    }

    #[test]
    fn status_bukan_2xx_memakai_pesan_axios() {
        let http = StubHttp::baru(404, "{}");
        let hasil = search_sound_cloud(&http, "x").unwrap();
        assert_eq!(hasil["success"], false);
        assert_eq!(hasil["query"], "x");
        assert_eq!(hasil["status"], 404);
        assert_eq!(hasil["message"], "Request failed with status code 404");
    }

    #[test]
    fn kegagalan_jaringan_tidak_punya_response_sehingga_status_500() {
        let mut http = StubHttp::baru(200, "{}");
        http.gagal = true;
        let hasil = search_sound_cloud(&http, "x").unwrap();
        assert_eq!(hasil["success"], false);
        assert_eq!(hasil["status"], 500);
        assert_eq!(hasil["message"], "koneksi gagal");
    }

    #[test]
    fn durasi_meniru_pembulatan_dan_pad_start() {
        assert_eq!(format_durasi(Some(&json!(0))), "0:00");
        assert_eq!(format_durasi(Some(&json!(1000))), "0:01");
        assert_eq!(format_durasi(Some(&json!(60000))), "1:00");
        assert_eq!(format_durasi(Some(&json!(3599000))), "59:59");
        // `Math.floor` JS membulatkan ke bawah, bukan terdekat.
        assert_eq!(format_durasi(Some(&json!(1999))), "0:01");
        // `duration` berupa string di JS tetap dihitung (`"90000" / 60`).
        assert_eq!(format_durasi(Some(&json!("90000"))), "1:30");
        // Tidak bisa dikonversi ke angka → `NaN` di kedua ruas, `padStart`
        // tidak menambah apa-apa karena panjangnya sudah 3.
        assert_eq!(format_durasi(Some(&json!("abc"))), "NaN:NaN");
        // `duration` hilang berarti `undefined`, bukan `null`.
        assert_eq!(format_durasi(None), "NaN:NaN");
    }
}