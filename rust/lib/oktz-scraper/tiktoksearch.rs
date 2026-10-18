//! `src/scraper/tiktoksearch.js` — GET pencarian TikTok via `api.nexray.eu.cc`.
//!
//! Semantik yang disalin:
//! - `normalizeUrl` — kalau ada lebih dari satu `http` di dalam URL, potong
//!   dari kemunculan terakhir (`lastIndexOf("http")`).
//! - `normalizeNumber` — `"1.2M"`/`"3K"`/`"1B"` pakai `String(value)`,
//!   `parseFloat`, lalu `Math.round`; angka mentah dilewatkan apa adanya.
//! - `!data?.status || !Array.isArray(data?.result)` → throw dengan
//!   `data?.message || "TikTok search gagal"`.
//! - Item hasil tanpa `link` (setelah normalisasi) dibuang.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, field, truthy};

const TTSEARCH_API: &str = "https://api.nexray.eu.cc/search/tiktok?q=";

/// `tiktokSearchVideo(query)` (`tiktoksearch.js:52-65`).
pub fn tiktok_search_video(http: &dyn HttpGet, query: &str) -> Result<Value, HttpError> {
    let url = format!("{TTSEARCH_API}{}", encode_uri_component(query));
    let res = http.call(&HttpCall::get(&url).header("user-agent", "Mozilla/5.0"))?;
    let data = res.json().unwrap_or_else(|| Value::String(res.text()));

    if !data.get("status").map(truthy).unwrap_or(false)
        || !matches!(data.get("result"), Some(Value::Array(_)))
    {
        let pesan = match data.get("message") {
            Some(m) if truthy(m) => string_js(m),
            _ => "TikTok search gagal".to_string(),
        };
        return Err(HttpError { message: pesan });
    }

    let mut hasil = Vec::new();
    if let Some(items) = data.get("result").and_then(Value::as_array) {
        for item in items {
            let norm = normalisasi_item(item);
            if norm.get("link").map(truthy).unwrap_or(false) {
                hasil.push(norm);
            }
        }
    }
    Ok(Value::Array(hasil))
}

/// `normalizeItem(item)` (`tiktoksearch.js:26-50`).
fn normalisasi_item(item: &Value) -> Value {
    let author = item.get("author");
    let stats = item.get("stats");

    let mut penulis = Map::new();
    penulis.insert(
        "nickname".to_string(),
        atau(
            author.and_then(|a| a.get("nickname")),
            atau(author.and_then(|a| a.get("fullname")), json!("")),
        ),
    );
    penulis.insert(
        "fullname".to_string(),
        atau(author.and_then(|a| a.get("fullname")), json!("")),
    );
    penulis.insert(
        "avatar".to_string(),
        normalisasi_url(author.and_then(|a| a.get("avatar"))),
    );

    let mut statistik = Map::new();
    statistik.insert(
        "plays".to_string(),
        normalisasi_angka(stats.and_then(|s| s.get("views"))),
    );
    statistik.insert(
        "likes".to_string(),
        normalisasi_angka(stats.and_then(|s| s.get("likes"))),
    );
    statistik.insert(
        "comments".to_string(),
        normalisasi_angka(stats.and_then(|s| s.get("comment"))),
    );
    statistik.insert(
        "shares".to_string(),
        normalisasi_angka(stats.and_then(|s| s.get("share"))),
    );
    statistik.insert(
        "downloads".to_string(),
        normalisasi_angka(stats.and_then(|s| s.get("download"))),
    );

    let mut out = Map::new();
    out.insert("title".to_string(), atau(item.get("title"), json!("")));
    out.insert("cover".to_string(), normalisasi_url(item.get("cover")));
    out.insert(
        "originCover".to_string(),
        normalisasi_url(item.get("cover")),
    );
    out.insert("link".to_string(), normalisasi_url(item.get("data")));
    out.insert(
        "watermarkLink".to_string(),
        normalisasi_url(item.get("data")),
    );
    out.insert(
        "music".to_string(),
        normalisasi_url(field(item, "music_info.url")),
    );
    out.insert("duration".to_string(), atau(item.get("duration"), json!("")));
    out.insert("region".to_string(), atau(item.get("region"), json!("")));
    out.insert("id".to_string(), atau(item.get("id"), json!("")));
    out.insert("author".to_string(), Value::Object(penulis));
    out.insert("stats".to_string(), Value::Object(statistik));
    Value::Object(out)
}

/// `normalizeUrl(url)` (`tiktoksearch.js:5-11`).
fn normalisasi_url(url: Option<&Value>) -> Value {
    let Some(Value::String(s)) = url else {
        return Value::Null;
    };
    if s.is_empty() {
        return Value::Null;
    }
    let re = Regex::new(r"https?://").unwrap();
    if re.find_iter(s).count() <= 1 {
        return Value::String(s.clone());
    }
    // `url.slice(url.lastIndexOf("http"))` — potong dari kemunculan terakhir.
    match s.rfind("http") {
        Some(i) => Value::String(s[i..].to_string()),
        None => Value::String(s.clone()),
    }
}

/// `normalizeNumber(value)` (`tiktoksearch.js:13-24`).
fn normalisasi_angka(value: Option<&Value>) -> Value {
    let Some(v) = value else {
        return json!(0);
    };
    if !truthy(v) {
        return json!(0);
    }
    // `typeof value === "number"` → dikembalikan apa adanya.
    if let Value::Number(n) = v {
        return Value::Number(n.clone());
    }

    let teks = string_js(v);
    let dibersihkan: String = teks
        .chars()
        .filter(|c| {
            c.is_ascii_digit()
                || *c == '.'
                || matches!(c.to_ascii_uppercase(), 'K' | 'M' | 'B')
        })
        .collect();
    let Some(num) = parse_float_js(&dibersihkan) else {
        return json!(0);
    };
    let upper = teks.to_uppercase();
    let faktor = if upper.contains('B') {
        1_000_000_000.0
    } else if upper.contains('M') {
        1_000_000.0
    } else if upper.contains('K') {
        1_000.0
    } else {
        1.0
    };
    angka_bulat(num * faktor)
}

/// `parseFloat` JS untuk string yang cuma berisi digit dan titik.
fn parse_float_js(s: &str) -> Option<f64> {
    let mut potongan = String::new();
    for c in s.trim_start().chars() {
        // Digit selalu masuk, titik hanya yang pertama; sisanya menghentikan.
        let diterima = c.is_ascii_digit() || (c == '.' && !potongan.contains('.'));
        if !diterima {
            break;
        }
        potongan.push(c);
    }
    // `parseFloat(".")` = NaN; `parseFloat("1.")` = 1.
    let potongan = potongan.trim_end_matches('.');
    if potongan.is_empty() {
        return None;
    }
    potongan.parse::<f64>().ok()
}

/// `Math.round(...)` sebagai angka JSON; integer ditulis tanpa `.0` supaya
/// sama dengan `JSON.stringify` JS.
fn angka_bulat(f: f64) -> Value {
    if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
        Value::Number(serde_json::Number::from(f as i64))
    } else {
        serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

/// `String(value)` JS untuk nilai JSON.
fn string_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(items) => items
            .iter()
            .map(string_js)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `a || bawaan` JS.
fn atau(nilai: Option<&Value>, bawaan: Value) -> Value {
    match nilai {
        Some(v) if truthy(v) => v.clone(),
        _ => bawaan,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: String,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("pencarian ini GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("pencarian ini GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("pencarian ini GET")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
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
    fn menormalkan_url_angka_dan_membuang_item_tanpa_link() {
        let http = stub(
            r#"{"status":true,"result":[
                {"title":"Video A",
                 "cover":"https://p16.akamaized.net/a.jpg/https://p16.akamaized.net/a.jpg",
                 "data":"https://www.tiktok.com/@a/video/1",
                 "music_info":{"url":"https://music/a.mp3"},
                 "duration":15,"region":"ID","id":"1",
                 "author":{"nickname":"","fullname":"Nama Lengkap","avatar":"https://a/av.jpg"},
                 "stats":{"views":"1.2M","likes":"3400","comment":"12","share":"1B","download":"abc"}},
                {"title":"Tanpa link"}
            ]}"#,
        );
        let hasil = tiktok_search_video(&http, "kucing lucu").unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 1, "item tanpa link harus dibuang");
        let item = &arr[0];
        assert_eq!(item["title"], "Video A");
        assert_eq!(item["cover"], "https://p16.akamaized.net/a.jpg");
        assert_eq!(item["originCover"], "https://p16.akamaized.net/a.jpg");
        assert_eq!(item["link"], "https://www.tiktok.com/@a/video/1");
        assert_eq!(item["watermarkLink"], "https://www.tiktok.com/@a/video/1");
        assert_eq!(item["music"], "https://music/a.mp3");
        assert_eq!(item["duration"], 15);
        assert_eq!(item["region"], "ID");
        assert_eq!(item["id"], "1");
        assert_eq!(item["author"]["nickname"], "Nama Lengkap");
        assert_eq!(item["author"]["fullname"], "Nama Lengkap");
        assert_eq!(item["author"]["avatar"], "https://a/av.jpg");
        assert_eq!(item["stats"]["plays"], 1_200_000);
        assert_eq!(item["stats"]["likes"], 3_400);
        assert_eq!(item["stats"]["comments"], 12);
        assert_eq!(item["stats"]["shares"], 1_000_000_000);
        assert_eq!(item["stats"]["downloads"], 0);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://api.nexray.eu.cc/search/tiktok?q=kucing%20lucu"
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("user-agent").unwrap(), "Mozilla/5.0");
    }

    #[test]
    fn angka_mentah_dilewatkan_apa_adanya() {
        // `typeof value === "number"` → tidak dibulatkan/dikonversi.
        let http = stub(r#"{"status":1,"result":[{"data":"https://x/1","stats":{"views":1234.5}}]}"#);
        let hasil = tiktok_search_video(&http, "q").unwrap();
        assert_eq!(hasil[0]["stats"]["plays"], 1234.5);
        assert_eq!(hasil[0]["stats"]["likes"], 0);
    }

    #[test]
    fn status_palsu_melempar_pesan_data_atau_bawaan() {
        let http = stub(r#"{"status":false,"message":"batas tercapai"}"#);
        assert_eq!(
            tiktok_search_video(&http, "q").unwrap_err().message,
            "batas tercapai"
        );

        let http = stub(r#"{"status":true,"result":{}}"#);
        assert_eq!(
            tiktok_search_video(&http, "q").unwrap_err().message,
            "TikTok search gagal"
        );

        // body bukan JSON → data string → status undefined → teks bawaan.
        let http = stub("bukan json");
        assert_eq!(
            tiktok_search_video(&http, "q").unwrap_err().message,
            "TikTok search gagal"
        );
    }

    #[test]
    fn parse_float_dan_sufiks_mengikuti_js() {
        assert_eq!(parse_float_js("1.2"), Some(1.2));
        assert_eq!(parse_float_js("1..2"), Some(1.0));
        assert_eq!(parse_float_js(".5"), Some(0.5));
        assert_eq!(parse_float_js("."), None);
        assert_eq!(parse_float_js(""), None);
        assert_eq!(normalisasi_angka(Some(&json!("3.7K"))), json!(3700));
        assert_eq!(normalisasi_angka(Some(&json!("1MB"))), json!(1_000_000_000));
        assert_eq!(normalisasi_angka(Some(&json!("abc"))), json!(0));
        assert_eq!(normalisasi_angka(None), json!(0));
    }
}
