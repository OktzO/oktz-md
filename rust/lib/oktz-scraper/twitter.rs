//! `src/scraper/twitter.js` — cari video X/Twitter lewat `x2twitter.com`.
//!
//! Yang ikut disalin dari JS:
//! - `axios.post(url, new URLSearchParams({...}))` → form-urlencoded dengan
//!   serializer `URLSearchParams` (spasi `+`, `~` jadi `%7E`), BUKAN
//!   `encodeURIComponent`. Salah pilih menghasilkan body berbeda.
//! - Seluruh badan fungsi ada di dalam `try`; `catch` mengembalikan
//!   `{ error: true, message: e.message }` sebagai objek BALASAN, bukan error.
//!   Karena itu galat jaringan di sini pun jadi `Ok(...)`, bukan `Err`.
//! - `searchResponse.data.status !== 'ok'` → pesan tetap, bukan lempar.
//! - Token `dl.snapcdn.app` di-decode base64; `filename` ber ekstensi gambar
//!   dibuang (`return null` + `filter(Boolean)`), token rusak juga `null`.
//! - Request kedua (konversi audio) punya `try/catch` sendiri: gagal →
//!   `audio = null`, hasil video tetap sukses.
//! - `decoded.filename.split('_').pop().replace('.mp4', '')` — `replace`
//!   string di JS hanya mengganti kemunculan PERTAMA.

use std::collections::HashMap;
use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::truthy;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

const HOST_KONVERSI_BAWAAN: &str = "https://s1.twcdn.net/api/json/convert";

/// `x2twitterDl(url)` (`twitter.js:14-117`).
pub fn x2twitter_dl(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    // `try { ... } catch (e) { return { error: true, message: e.message } }`
    match dl(http, url) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({
            "error": true,
            "message": e.message,
        })),
    }
}

/// Badan fungsi; `Err` di sini berarti `catch` JS yang akan memformatnya.
fn dl(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let cf_token = generate_cf_token();
    let session_token = hex_acak(16);

    let search = http.call(
        &HttpCall::post("https://x2twitter.com/api/ajaxSearch")
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Origin", "https://x2twitter.com")
            .header("Referer", "https://x2twitter.com/id")
            .header("User-Agent", UA)
            .header(
                "Cookie",
                format!("_session={session_token}; lang=id"),
            )
            .form(format!(
                "q={}&lang=id&cftoken={}",
                encode_form(url),
                encode_form(&cf_token)
            )),
    )?;

    let data = search.json().unwrap_or(Value::Null);
    // `searchResponse.data.status` pada `null` melempar TypeError lebih dulu,
    // jadi tidak pernah sampai ke pemeriksaan status.
    if data.is_null() {
        return Err(HttpError {
            message: "Cannot read properties of null (reading 'status')".to_string(),
        });
    }
    if data.get("status").and_then(Value::as_str) != Some("ok") {
        return Ok(json!({
            "error": true,
            "message": "Gagal mengambil data dari x2twitter.",
        }));
    }

    // `const html = searchResponse.data.data` — kalau bukan teks, pemanggilan
    // `.match` di bawah melempar TypeError dan ditangkap catch terluar.
    let Some(html) = data.get("data").and_then(Value::as_str) else {
        return Err(HttpError {
            message: pesan_html_bukan_teks(&data),
        });
    };

    let thumbnail = ambil(html, "img").unwrap_or_else(|| "-".to_string());
    let duration = ambil(html, "durasi").unwrap_or_else(|| "-".to_string());

    let mut videos: Vec<Value> = Vec::new();
    for token in ambil_semua(html, "token") {
        // `token.split('.')[1]` undefined → `Buffer.from(undefined)` melempar,
        // jadi token tanpa titik tengah masuk `catch` dan jadi `null`.
        let Some(payload_b64) = token.split('.').nth(1) else {
            continue;
        };
        let isi = String::from_utf8_lossy(&base64_deskep(payload_b64)).to_string();
        let Ok(decoded) = serde_json::from_str::<Value>(&isi) else {
            continue;
        };
        let filename = decoded
            .get("filename")
            .filter(|v| truthy(v))
            .map(teks_js);
        // `/\.(jpg|jpeg|png)$/i.test(decoded.filename)` → `return null`.
        if let Some(f) = &filename
            && pola()
                .get("gambar")
                .is_some_and(|re| re.is_match(f))
        {
            continue;
        }
        let resolution = match &filename {
            Some(f) => f
                .split('_')
                .next_back()
                .unwrap_or("")
                .replacen(".mp4", "", 1),
            None => "unknown".to_string(),
        };
        videos.push(json!({
            "resolution": resolution,
            "url": format!("https://dl.snapcdn.app/get?token={token}"),
        }));
    }

    let audio_url = ambil(html, "audioUrl");
    let media_id = ambil(html, "mediaId");
    let host_convert = ambil(html, "url_convert")
        .unwrap_or_else(|| HOST_KONVERSI_BAWAAN.to_string());
    let exp = ambil(html, "exp");
    let token_page = ambil(html, "ktoken");

    let mut audio = Value::Null;
    if let (Some(media_id), Some(audio_url), Some(exp), Some(token_page)) = (
        &media_id,
        &audio_url,
        &exp,
        &token_page,
    ) {
        let form = format!(
            "ftype=mp3&v_id={}&audioUrl={}&audioType={}&fquality=128&fname=X2Twitter.com&exp={}&token={}",
            encode_form(media_id),
            encode_form(audio_url),
            encode_form("video/mp4"),
            encode_form(exp),
            encode_form(token_page)
        );
        // `try { ... } catch { audio = null }`
        if let Ok(res) = http.call(
            &HttpCall::post(host_convert.as_str())
                .header(
                    "Content-Type",
                    "application/x-www-form-urlencoded; charset=UTF-8",
                )
                .header("Accept", "*/*")
                .header("User-Agent", UA)
                .form(form),
        ) {
            let body = res.json().unwrap_or(Value::Null);
            if truthy(&body) && body.get("status").and_then(Value::as_str) == Some("success") {
                let mut hasil = Map::new();
                hasil.insert("title".to_string(), json!("X2Twitter.com.mp3"));
                // `result` undefined → kunci hilang, bukan `null`.
                if let Some(v) = body.get("result") {
                    hasil.insert("url".to_string(), v.clone());
                }
                audio = Value::Object(hasil);
            }
        }
    }

    Ok(json!({
        "code": 200,
        "status": "success",
        "metadata": {
            "thumbnail": thumbnail,
            "duration": duration,
        },
        "videos": videos,
        "audio": audio,
    }))
}

/// `e.message` untuk TypeError pertama di jalur `html`: `data.data` bukan teks
/// lalu `.match` dipanggil pada nilai itu. Teksnya mengikuti V8 supaya pesan
/// yang tampil ke user tidak berubah.
fn pesan_html_bukan_teks(data: &Value) -> String {
    match data.get("data") {
        None => "Cannot read properties of undefined (reading 'match')".to_string(),
        Some(Value::Null) => "Cannot read properties of null (reading 'match')".to_string(),
        Some(_) => "html.match is not a function".to_string(),
    }
}

/// `generateCfToken()` (`twitter.js:4-12`) — lima bagian dipisah titik.
fn generate_cf_token() -> String {
    [
        "0".to_string(),
        hex_acak(16),
        base64url_acak(32),
        base64url_acak(64),
        hex_acak(32),
    ]
    .join(".")
}

/// `crypto.randomBytes(n).toString('hex')`.
fn hex_acak(n: usize) -> String {
    (0..n)
        .map(|_| format!("{:02x}", rand::random::<u8>()))
        .collect()
}

/// `crypto.randomBytes(n).toString('base64url')` — tanpa padding, `-_`.
fn base64url_acak(n: usize) -> String {
    let data: Vec<u8> = (0..n).map(|_| rand::random::<u8>()).collect();
    base64_encode(&data)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

const ABJAD_B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `Buffer#toString("base64")` — padding `=`. Ditulis tangan karena crate ini
/// tidak punya dependency base64.
fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for blok in data.chunks(3) {
        let b0 = blok[0] as u32;
        let b1 = blok.get(1).copied().unwrap_or(0) as u32;
        let b2 = blok.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        let quartet = [
            ABJAD_B64[(n >> 18) as usize & 63],
            ABJAD_B64[(n >> 12) as usize & 63],
            ABJAD_B64[(n >> 6) as usize & 63],
            ABJAD_B64[n as usize & 63],
        ];
        out.push(quartet[0] as char);
        out.push(quartet[1] as char);
        out.push(if blok.len() > 1 {
            quartet[2] as char
        } else {
            '='
        });
        out.push(if blok.len() > 2 {
            quartet[3] as char
        } else {
            '='
        });
    }
    out
}

/// `Buffer.from(s, "base64")` — Node mengabaikan karakter di luar abjad dan
/// menerima `-_` (base64url) serta tanpa padding.
fn base64_deskep(s: &str) -> Vec<u8> {
    let mut nilai = 0u32;
    let mut bit = 0u32;
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => continue,
        } as u32;
        nilai = (nilai << 6) | v;
        bit += 6;
        if bit >= 8 {
            bit -= 8;
            out.push(((nilai >> bit) & 0xFF) as u8);
            // Sisakan hanya bit yang belum dipakai supaya tidak overflow.
            nilai &= (1u32 << bit) - 1;
        }
    }
    out
}

/// Serializer `URLSearchParams` (`application/x-www-form-urlencoded`): hanya
/// `*-._` dan alnum yang apa adanya, spasi jadi `+`, sisanya `%XX`.
/// Beda dari `encodeURIComponent` yang dipakai modul lain.
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

/// Seluruh regex modul ini, dikompilasi sekali per proses.
fn pola() -> &'static HashMap<&'static str, Regex> {
    static P: OnceLock<HashMap<&'static str, Regex>> = OnceLock::new();
    P.get_or_init(|| {
        [
            ("img", r#"<img\s+src="([^"]+)""#),
            ("durasi", r#"<p>(\d+:\d+)</p>"#),
            ("token", r#"href="https://dl\.snapcdn\.app/get\?token=([^"]+)""#),
            ("audioUrl", r#"data-audioUrl="([^"]+)""#),
            ("mediaId", r#"data-mediaId="([^"]+)""#),
            ("url_convert", r#"k_url_convert\s*=\s*"([^"]+)""#),
            ("exp", r#"k_exp\s*=\s*"([^"]+)""#),
            ("ktoken", r#"k_token\s*=\s*"([^"]+)""#),
            ("gambar", r#"(?i)\.(jpg|jpeg|png)$"#),
        ]
        .into_iter()
        .map(|(nama, src)| (nama, Regex::new(src).expect("pola regex")))
        .collect()
    })
}

/// `(html.match(re) || [])[1]` — grup tangkapan pertama dari kecocokan pertama.
fn ambil(html: &str, nama: &str) -> Option<String> {
    let c = pola().get(nama)?.captures(html)?;
    c.get(1).map(|m| m.as_str().to_string())
}

/// `[...html.matchAll(re)].map(m => m[1])` — semua kecocokan, urutan asli.
fn ambil_semua(html: &str, nama: &str) -> Vec<String> {
    pola()
        .get(nama)
        .map(|re| {
            re.captures_iter(html)
                .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// `String(v)` untuk nilai JSON.
fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
        Value::Null => "null".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: membalas antrean (sukses atau galat)
    /// sesuai urutan panggilan.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<Result<(u16, String), String>>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<Result<(u16, String), String>>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }

        fn isi(teks: &str) -> Result<(u16, String), String> {
            Ok((200, teks.to_string()))
        }

        fn galat(pesan: &str) -> Result<(u16, String), String> {
            Err(pesan.to_string())
        }
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
            unreachable!("scraper ini lewat HttpCall::call")
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
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            match balasan[i].clone() {
                Ok((status, teks)) => Ok(oktz_plugin_api::HttpResponse {
                    status,
                    body: oktz_plugin_api::HttpBody::Bytes {
                        data: std::sync::Arc::from(teks.into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                Err(pesan) => Err(HttpError { message: pesan }),
            }
        }
    }

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn form_dari(p: &oktz_plugin_api::HttpCall) -> String {
        match &p.body {
            Some(oktz_plugin_api::HttpBody::Form(f)) => f.clone(),
            lain => panic!("body bukan form: {lain:?}"),
        }
    }

    /// HTML yang dipakai beberapa test: dua token video, satu gambar, satu
    /// token rusak, plus blok data audio.
    fn html_contoh() -> (String, String, String, String) {
        let mp4 = format!("AAA.{}.CCC", base64_encode(br#"{"filename":"vid_720.mp4"}"#));
        let jpg = format!("BBB.{}.DDD", base64_encode(br#"{"filename":"foto.JPG"}"#));
        let rusak = format!("EEE.{}.FFF", base64_encode(b"bukan json"));
        let html = format!(
            r#"<img src="https://pbs.twimg.com/thumb.jpg"><p>1:23</p>
<a href="https://dl.snapcdn.app/get?token={mp4}">video</a>
<a href="https://dl.snapcdn.app/get?token={jpg}">gambar</a>
<a href="https://dl.snapcdn.app/get?token={rusak}">rusak</a>
<div data-audioUrl="https://video.twimg.com/a.m4a?x=1&y=2" data-mediaId="98765"></div>
<script>var k_url_convert = "https://convert.example/api";var k_exp = "exp-1";var k_token = "tok-1";</script>"#
        );
        (html, mp4, jpg, rusak)
    }

    /// Bentuk respons `ajaxSearch` yang sukses, dengan `data` berisi HTML.
    fn cari_ok(teks: &str) -> String {
        json!({"status": "ok", "data": teks}).to_string()
    }

    #[test]
    fn sukses_mengambil_video_dan_audio() {
        let (html, mp4, _jpg, _rusak) = html_contoh();
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(&html)),
            StubHttp::isi(r#"{"status":"success","result":"https://cdn.example/audio.mp3"}"#),
        ]);
        let hasil = x2twitter_dl(&http, "https://x.com/user/status/1").unwrap();

        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["status"], "success");
        assert_eq!(hasil["metadata"]["thumbnail"], "https://pbs.twimg.com/thumb.jpg");
        assert_eq!(hasil["metadata"]["duration"], "1:23");
        let videos = hasil["videos"].as_array().unwrap();
        // Hanya mp4: gambar JPG dan token base64 rusak jadi `null` lalu
        // dibuang `filter(Boolean)`.
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0]["resolution"], "720");
        assert_eq!(
            videos[0]["url"],
            format!("https://dl.snapcdn.app/get?token={mp4}")
        );
        assert_eq!(hasil["audio"]["title"], "X2Twitter.com.mp3");
        assert_eq!(hasil["audio"]["url"], "https://cdn.example/audio.mp3");
    }

    #[test]
    fn permintaan_pertama_form_dengan_cftoken_lima_bagian() {
        let (html, ..) = html_contoh();
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(&html)),
            StubHttp::isi(r#"{"status":"success"}"#),
        ]);
        x2twitter_dl(&http, "https://x.com/user/status/1").unwrap();

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://x2twitter.com/api/ajaxSearch");
        let form = form_dari(&p[0]);
        let cftoken = form
            .strip_prefix("q=https%3A%2F%2Fx.com%2Fuser%2Fstatus%2F1&lang=id&cftoken=")
            .expect("prefix form");
        let bagian: Vec<&str> = cftoken.split('.').collect();
        assert_eq!(bagian.len(), 5, "cftoken: 0 + 4 bagian acak");
        assert_eq!(bagian[0], "0");
        assert_eq!(bagian[1].len(), 32, "16 byte hex");
        assert_eq!(bagian[4].len(), 64, "32 byte hex");
        assert!(bagian[1].chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(bagian[2].len(), 43, "32 byte base64url tanpa padding");
        assert_eq!(bagian[3].len(), 86, "64 byte base64url tanpa padding");
        assert!(
            !bagian[2].contains('+') && !bagian[2].contains('/') && !bagian[2].contains('='),
            "base64url: {}",
            bagian[2]
        );

        assert_eq!(
            cari_header(&p[0], "content-type"),
            Some("application/x-www-form-urlencoded; charset=UTF-8".to_string())
        );
        let cookie = cari_header(&p[0], "cookie").unwrap();
        let sesi = cookie.strip_prefix("_session=").expect("prefix cookie");
        let (sesi, ekor) = sesi.split_once("; ").expect("pemisah cookie");
        assert_eq!(ekor, "lang=id");
        assert_eq!(sesi.len(), 32, "16 byte hex");
        assert!(sesi.chars().all(|c| c.is_ascii_hexdigit()), "{sesi}");
        assert_eq!(cari_header(&p[0], "referer"), Some("https://x2twitter.com/id".to_string()));
    }

    #[test]
    fn permintaan_kedua_menuju_host_konversi_dengan_body_terenkode() {
        let (html, ..) = html_contoh();
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(&html)),
            StubHttp::isi(r#"{"status":"success"}"#),
        ]);
        x2twitter_dl(&http, "https://x.com/user/status/1").unwrap();

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[1].url, "https://convert.example/api");
        assert_eq!(
            form_dari(&p[1]),
            "ftype=mp3&v_id=98765&audioUrl=https%3A%2F%2Fvideo.twimg.com%2Fa.m4a%3Fx%3D1%26y%3D2\
             &audioType=video%2Fmp4&fquality=128&fname=X2Twitter.com&exp=exp-1&token=tok-1"
        );
        assert_eq!(cari_header(&p[1], "accept"), Some("*/*".to_string()));
    }

    #[test]
    fn tanpa_data_audio_tidak_ada_request_kedua_dan_audio_null() {
        let html = "<img src=\"https://x/a.jpg\"><p>0:07</p>";
        let http = StubHttp::baru(vec![StubHttp::isi(&cari_ok(html))]);
        let hasil = x2twitter_dl(&http, "https://x.com/user/status/1").unwrap();
        assert_eq!(hasil["audio"], Value::Null);
        assert_eq!(hasil["videos"].as_array().unwrap().len(), 0);
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }

    #[test]
    fn host_konversi_bawaan_dipakai_bila_variabel_halaman_tidak_ada() {
        // `data-mediaId` ada tapi `k_url_convert` tidak → host bawaan.
        let html = "<div data-audioUrl=\"https://a/x.m4a\" data-mediaId=\"1\"></div>\
                    <script>var k_exp=\"e\";var k_token=\"t\";</script>";
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(html)),
            StubHttp::isi(r#"{"status":"success","result":"https://cdn/a.mp3"}"#),
        ]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["audio"]["url"], "https://cdn/a.mp3");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[1].url, HOST_KONVERSI_BAWAAN);
    }

    #[test]
    fn status_bukan_ok_memberi_pesan_khas() {
        let http = StubHttp::baru(vec![StubHttp::isi(r#"{"status":"error"}"#)]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["error"], true);
        assert_eq!(hasil["message"], "Gagal mengambil data dari x2twitter.");
        assert!(hasil.get("code").is_none());
    }

    #[test]
    fn galat_jaringan_dan_data_bukan_teks_jadi_objek_error_bukan_err() {
        let http = StubHttp::baru(vec![StubHttp::galat("jaringan mati")]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["error"], true);
        assert_eq!(hasil["message"], "jaringan mati");

        // `data` bukan teks → TypeError yang ditangkap catch terluar.
        let http = StubHttp::baru(vec![StubHttp::isi(r#"{"status":"ok","data":123}"#)]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["message"], "html.match is not a function");

        let http = StubHttp::baru(vec![StubHttp::isi(r#"{"status":"ok"}"#)]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(
            hasil["message"],
            "Cannot read properties of undefined (reading 'match')"
        );

        // Respons `null` gagal di akses `.status`, sebelum `.data`.
        let http = StubHttp::baru(vec![StubHttp::isi("null")]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(
            hasil["message"],
            "Cannot read properties of null (reading 'status')"
        );
    }

    #[test]
    fn request_konversi_gagal_tetap_menghasilkan_audio_null() {
        let html = "<div data-audioUrl=\"https://a/x.m4a\" data-mediaId=\"1\"></div>\
                    <script>var k_url_convert=\"https://convert.example/api\";\
                    var k_exp=\"e\";var k_token=\"t\";</script>";
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(html)),
            StubHttp::galat("convert ditolak"),
        ]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["status"], "success");
        assert_eq!(hasil["audio"], Value::Null);

        // Status bukan `success` juga tidak bikin audio.
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(html)),
            StubHttp::isi(r#"{"status":"fail"}"#),
        ]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["audio"], Value::Null);

        // `result` tidak ada → kunci `url` hilang, bukan `null`.
        let http = StubHttp::baru(vec![
            StubHttp::isi(&cari_ok(html)),
            StubHttp::isi(r#"{"status":"success"}"#),
        ]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["audio"]["title"], "X2Twitter.com.mp3");
        assert!(hasil["audio"].get("url").is_none());
    }

    #[test]
    fn metadata_default_strip_dan_token_tanpa_titik_atau_nama_file() {
        let kosong = base64_encode(b"{}");
        let html = format!(
            "<p>halo</p>\
             <a href=\"https://dl.snapcdn.app/get?token=tanpatitik\">x</a>\
             <a href=\"https://dl.snapcdn.app/get?token=A..B\">y</a>\
             <a href=\"https://dl.snapcdn.app/get?token=C.{kosong}.D\">z</a>"
        );
        let http = StubHttp::baru(vec![StubHttp::isi(&cari_ok(&html))]);
        let hasil = x2twitter_dl(&http, "https://x.com/u/s/1").unwrap();
        assert_eq!(hasil["metadata"]["thumbnail"], "-");
        assert_eq!(hasil["metadata"]["duration"], "-");
        let videos = hasil["videos"].as_array().unwrap();
        // `token.split('.')[1]` undefined (dua token pertama) dan payload tanpa
        // `filename` untuk yang ketiga.
        assert_eq!(videos.len(), 1, "dua token pertama jadi null");
        assert_eq!(videos[0]["resolution"], "unknown", "payload tanpa filename");
    }

    #[test]
    fn resolver_base64_dan_encode_form_meniru_node_dan_urlsearchparams() {
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(
            base64_deskep(&base64_encode(br#"{"filename":"a_b.mp4"}"#)),
            br#"{"filename":"a_b.mp4"}"#.to_vec()
        );
        // Base64url diterima oleh decoder Node.
        assert_eq!(base64_deskep("-_-"), vec![251, 255]);

        // Spasi jadi `+` dan `~` jadi `%7E` — berbeda dari encodeURIComponent.
        assert_eq!(encode_form("a b~c*d"), "a+b%7Ec*d");
        assert_eq!(encode_form("https://x.com/u?s=1&t=2"), "https%3A%2F%2Fx.com%2Fu%3Fs%3D1%26t%3D2");
    }
}