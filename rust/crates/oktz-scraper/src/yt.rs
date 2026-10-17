//! `src/scraper/yt.js` — ssyoutube.com, POST konversi link YouTube.
//!
//! Yang ikut disalin dari JS:
//! - NAMAUNYA tetap `ssyoutube` (`yt.js:45`), bukan `ytdl.js` — endpoint-nya
//!   `https://ssyoutube.com/api/convert`.
//! - `qs.stringify(payload)` memakai encoder RFC 3986 miliknya sendiri: hanya
//!   `A-Z a-z 0-9 - . _ ~` yang dibiarkan, spasi jadi `%20` (bukan `+`), dan
//!   `!'()*` ikut di-escape. `encodeURIComponent` yang dipakai crate ini
//!   membiarkan `!'()*`, jadi memanggilnya di sini akan menghasilkan body
//!   yang berbeda dari JS.
//! - `qs` tidak mengurutkan kunci: urutan mengikuti objek `payload`, yaitu
//!   `sf_url`, `ts`, `_ts`, `_tsc`, `_s`.
//! - `!data || !data.url` — `url` berupa array KOSONG tetap lolos (`[]` truthy),
//!   lalu `Array.isArray` benar dan `downloads` jadi array kosong.
//! - `formatSize` memakai `Math.log`, jadi byte negatif/`NaN`/di luar
//!   `['Bytes','KB','MB','GB']` menghasilkan teks seperti `"NaN undefined"`.
//!   `toFixed(2)` JS memakai pembulatan ke atas pada hasil tepat `.xx5`
//!   (`3200` byte → `3.13 KB`), jadi tidak boleh pakai `format!("{:.2}")`
//!   yang memakai pembulatan ke-even.
//!
//! Beda yang diketahui:
//! - `crypto.createHash('sha256')` tidak punya crate di `Cargo.toml`
//!   (`md-5` dan `sha1` saja), jadi SHA-256 ditulis tangan di modul ini.
//!   `test` membandingkannya dengan vektor standar FIPS 180-4.
//! - Cabang `catch` JS yang mengembalikan `{ error: 'Gagal membuat signature
//!   keamanan.' }` tidak bisa terjadi di sini: `createHash` JS tidak pernah
//!   melempar untuk input string, dan port ini juga tidak gagal.
//! - `catch` terakhir JS mengembalikan `{ error: error.message }`; teks
//!   transportednya dari `UreqHttp` (`"HTTP 403"`, pesan ureq), bukan
//!   `"Request failed with status code 403"` milik axios.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const BASE_URL: &str = "https://ssyoutube.com";

/// `CONFIG.API.CONVERT` (`yt.js:7`).
const API: &str = "/api/convert";

/// `CONFIG.SECRETS.SALT` (`yt.js:10`).
const SALT: &str = "384d5028ee4a399f6cae0175025a1708aa924fc0ccb08be1aa359cd856dd1639";

/// `CONFIG.SECRETS.FIXED_TS` (`yt.js:11`).
const FIXED_TS: &str = "1765962059039";

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// `['Bytes', 'KB', 'MB', 'GB']` (`yt.js:40`) — dilampaui indeks di luar
/// rentang, hasilnya `undefined` seperti akses array JS.
const SIFAT: [&str; 4] = ["Bytes", "KB", "MB", "GB"];

/// `Date.now().toString()` — milidetik sejak epoch.
fn milidetik_sekarang() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// `qs` encoder (`node_modules/qs/lib/utils.js:218`) — RFC 3986, bukan
/// `encodeURIComponent` dan bukan form-urlencoded.
///
/// Perbedaan yang paling sering menipu: `encodeURIComponent` membiarkan
/// `!'()*~`, sedangkan `qs` meng-escape `!'()*` dan hanya membiarkan `~`.
/// Spasi tetap `%20`, jadi bukan `+` seperti `URLSearchParams`.
fn encode_qs(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~') {
            out.push(c);
        } else {
            // Chunk 1024 karakter di JS tidak mengubah hasil: setiap karakter
            // di-encode sendiri.
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).as_bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

/// `Number(v)` — hanya bentuk yang muncul di `filesize`. Beda dari
/// `parseInt`: `"12abc"` itu `NaN`, bukan `12`.
fn number_js(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
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

/// `(x).toFixed(2)` — pembulatan ke atas pada `.xx5` tepat, bukan ke-even
/// seperti `format!("{:.2}")`. Contoh yang berbeda: `3200 / 1024 = 3.125`
/// → JS `"3.13"`, `format!("{:.2}")` menghasilkan `"3.12"`.
fn to_fixed_2(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if !x.is_finite() {
        return if x.is_sign_positive() {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    let y = x * 100.0;
    let dasar = if y >= 0.0 { y.floor() } else { y.ceil() };
    // `n/10^f - x` harus se dekat mungkin dengan nol; hasil imbang mengambil
    // `n` yang lebih besar (spesifikasi `Number.prototype.toFixed`).
    let n = if (y - dasar).abs() >= 0.5 {
        if y >= 0.0 { dasar + 1.0 } else { dasar - 1.0 }
    } else {
        dasar
    };
    let negatif = n < 0.0;
    let a = n.abs() as u64;
    format!(
        "{}{}.{:02}",
        if negatif { "-" } else { "" },
        a / 100,
        a % 100
    )
}

/// `formatSize(bytes)` (`yt.js:38-43`). `bytes` yang falsy → `"Unknown"`;
/// indeks `Math.floor(Math.log(bytes)/Math.log(1024))` di luar 0..3 memberi
/// satuan `undefined` (teks itu benar-benar muncul di keluaran JS).
fn format_size(bytes: Option<&Value>) -> String {
    let Some(v) = bytes.filter(|v| truthy(v)) else {
        return "Unknown".to_string();
    };
    let n = number_js(v);
    let i = (n.ln() / 1024f64.ln()).floor();
    let satuan = if i.is_finite() && (0.0..4.0).contains(&i) {
        SIFAT[i as usize]
    } else {
        "undefined"
    };
    // `parseFloat((bytes / Math.pow(1024, i)).toFixed(2))` lalu disambung
    // satuan dengan `+ ' '`, jadi angkanya jadi teks lagi oleh `ToString`.
    let nilai = n / 1024f64.powf(i);
    format!("{} {}", to_fixed_2(nilai).parse::<f64>().unwrap_or(f64::NAN), satuan)
}

/// `crypto.createHash('sha256').update(x).digest('hex')` (`yt.js:27`).
///
/// `sha2` tidak ada di `Cargo.toml`, jadi algoritmanya ditulis di sini.
/// Bentuknya persis FIPS 180-4; test membandingkan dengan vektor standar.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let mut pesan = data.to_vec();
    pesan.push(0x80);
    while pesan.len() % 64 != 56 {
        pesan.push(0);
    }
    pesan.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());

    for blok in pesan.chunks(64) {
        let mut w = [0u32; 64];
        for (t, isi) in w.iter_mut().enumerate().take(16) {
            let o = t * 4;
            *isi = u32::from_be_bytes([blok[o], blok[o + 1], blok[o + 2], blok[o + 3]]);
        }
        for t in 16..64 {
            let s0 = w[t - 15].rotate_right(7) ^ w[t - 15].rotate_right(18) ^ (w[t - 15] >> 3);
            let s1 = w[t - 2].rotate_right(17) ^ w[t - 2].rotate_right(19) ^ (w[t - 2] >> 10);
            w[t] = w[t - 16]
                .wrapping_add(s0)
                .wrapping_add(w[t - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for t in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[t])
                .wrapping_add(w[t]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v[7] = v[6];
            v[6] = v[5];
            v[5] = v[4];
            v[4] = v[3].wrapping_add(t1);
            v[3] = v[2];
            v[2] = v[1];
            v[1] = v[0];
            v[0] = t1.wrapping_add(t2);
        }
        for (i, x) in v.iter().enumerate() {
            h[i] = h[i].wrapping_add(*x);
        }
    }

    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// `ssyoutube.download(videoUrl)` (`yt.js:46-105`).
pub fn ssyoutube_download(http: &dyn HttpGet, video_url: &str) -> Result<Value, HttpError> {
    // `!videoUrl || (!includes('youtube.com') && !includes('youtu.be'))` —
    // string kosong ikut falsy.
    if video_url.is_empty()
        || (!video_url.contains("youtube.com") && !video_url.contains("youtu.be"))
    {
        return Ok(json!({
            "error": "URL tidak valid. Harap gunakan URL YouTube.",
        }));
    }

    let ts = milidetik_sekarang().to_string();
    let signature = sha256_hex(format!("{video_url}{ts}{SALT}").as_bytes());

    // `qs.stringify(payload)` — urutan kunci tetap urutan objek di JS.
    let body = [
        format!("sf_url={}", encode_qs(video_url)),
        format!("ts={}", encode_qs(&ts)),
        format!("_ts={}", encode_qs(FIXED_TS)),
        format!("_tsc={}", encode_qs("0")),
        format!("_s={}", encode_qs(&signature)),
    ]
    .join("&");

    let res = http.call(
        &HttpCall::post(format!("{BASE_URL}{API}"))
            .header("user-agent", UA)
            .header("accept", "application/json, text/plain, */*")
            .header("content-type", "application/x-www-form-urlencoded;charset=UTF-8")
            .header("origin", BASE_URL)
            .header("referer", format!("{BASE_URL}/"))
            .form(body),
    )?;

    // `data` yang bukan objek (mis. teks) gagal di `data.url`, sama seperti
    // `undefined` — jadi `Option` yang `None` ikut ke cabang error.
    let data = res.json().unwrap_or(Value::Null);
    let url_data = data.get("url");
    if !truthy(&data) || url_data.is_none_or(|u| !truthy(u)) {
        return Ok(json!({
            "error": "Gagal mengambil data. Server mungkin memblokir request.",
        }));
    }

    // `meta` hanya memuat kunci yang punya nilai; `undefined` hilang dari
    // `JSON.stringify`.
    let mut meta = Map::new();
    for (sumber, kunci) in [
        (data.get("id"), "id"),
        (data.get("meta").and_then(|m| m.get("title")), "title"),
        (
            data.get("meta").and_then(|m| m.get("duration")),
            "duration",
        ),
        (data.get("thumb"), "thumbnail"),
    ] {
        if let Some(v) = sumber {
            meta.insert(kunci.to_string(), v.clone());
        }
    }

    let mut downloads: Vec<Value> = Vec::new();
    if let Some(items) = url_data.and_then(Value::as_array) {
        for item in items {
            // `!item.no_audio` — kunci yang hilang juga falsy, jadi item
            // tanpa `no_audio` ikut terbawa.
            if truthy(item.get("no_audio").unwrap_or(&Value::Null)) {
                continue;
            }
            let mut out = Map::new();
            // `item.quality || item.subname`.
            let quality = match item.get("quality") {
                Some(q) if truthy(q) => Some(q.clone()),
                _ => item.get("subname").cloned(),
            };
            if let Some(q) = quality {
                out.insert("quality".to_string(), q);
            }
            if let Some(v) = item.get("ext") {
                out.insert("format".to_string(), v.clone());
            }
            out.insert(
                "size".to_string(),
                Value::String(format_size(item.get("filesize"))),
            );
            if let Some(v) = item.get("url") {
                out.insert("url".to_string(), v.clone());
            }
            if let Some(v) = item.get("audio") {
                out.insert("audio".to_string(), v.clone());
            }
            downloads.push(Value::Object(out));
        }
    }

    Ok(json!({
        "meta": Value::Object(meta),
        "downloads": Value::Array(downloads),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat `HttpCall` dan membalas JSON
    /// tetap.
    #[derive(Debug)]
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

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn form(p: &oktz_plugin_api::HttpCall) -> String {
        match &p.body {
            Some(oktz_plugin_api::HttpBody::Form(f)) => f.clone(),
            lain => panic!("body bukan form: {lain:?}"),
        }
    }

    #[test]
    fn sha256_sama_dengan_vektor_standar() {
        // FIPS 180-4 / `crypto.createHash('sha256')` Node.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Panjang melewati satu blok (64 byte) dan batas padding 55/56/64.
        assert_eq!(
            sha256_hex(&vec![b'a'; 1_000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 55]),
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 64]),
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb"
        );
    }

    #[test]
    fn encode_qs_mengikuti_encoder_qs_bukan_encodeuricomponent() {
        assert_eq!(encode_qs("https://x/y?a=1&b=2"), "https%3A%2F%2Fx%2Fy%3Fa%3D1%26b%3D2");
        assert_eq!(encode_qs("a b"), "a%20b", "qs tidak memakai +");
        // `!'()*` dibiarkan encodeURIComponent tapi di-escape `qs`.
        assert_eq!(encode_qs("!'()*~"), "%21%27%28%29%2A~");
        assert_eq!(encode_qs("-._azAZ09"), "-._azAZ09");
        assert_eq!(encode_qs("Kobo"), "Kobo");
    }

    #[test]
    fn format_size_mengikuti_log_dan_tofixed_js() {
        assert_eq!(format_size(Some(&json!(0))), "Unknown", "0 itu falsy");
        assert_eq!(format_size(None), "Unknown");
        assert_eq!(format_size(Some(&json!("0"))), "NaN undefined", "\"0\" truthy tapi log 0 = -Inf");
        assert_eq!(format_size(Some(&json!(3449447))), "3.29 MB");
        assert_eq!(format_size(Some(&json!(1231355))), "1.17 MB");
        assert_eq!(format_size(Some(&json!(1024))), "1 KB");
        assert_eq!(format_size(Some(&json!(500))), "500 Bytes");
        // 3.125 tepat: `toFixed(2)` JS ke atas → 3.13 (bukan 3.12 ke-even).
        assert_eq!(format_size(Some(&json!(3200))), "3.13 KB");
        // Di luar 4 satuan → indeks array JS `undefined`, meski nilainya 1.
        assert_eq!(format_size(Some(&json!(1024u64.pow(5)))), "1 undefined");
        assert_eq!(format_size(Some(&json!([]))), "NaN undefined");
        assert_eq!(format_size(Some(&json!("abc"))), "NaN undefined");
    }

    #[test]
    fn url_bukan_youtube_tidak_pernah_minta_jaringan() {
        let http = stub("{}");
        // Catatan: `"https://ssyoutube.com"` justru LOLOS, karena `includes` di JS
        // hanya cari substring `youtube.com` yang ada di dalamnya.
        for url in [
            "https://vimeo.com/1",
            "",
            "https://google.com/search?q=youtube",
            "not a url",
        ] {
            let hasil = ssyoutube_download(&http, url).unwrap();
            assert_eq!(hasil["error"], "URL tidak valid. Harap gunakan URL YouTube.");
        }
        assert!(http.permintaan.lock().unwrap().is_empty(), "tidak ada request");
    }

    #[test]
    fn alur_sukses_menampilkan_meta_dan_daftar_unduhan() {
        let http = stub(
            r#"{
              "id": "dQw4w9WgXcQ",
              "meta": {"title": "Rick Astley", "duration": "3:33"},
              "thumb": "https://i.ytimg.com/hq.jpg",
              "url": [
                {"quality": "720", "ext": "mp4", "filesize": 0, "url": "https://a", "audio": false},
                {"quality": "", "subname": "audio", "ext": "m4a", "filesize": 3449447,
                 "url": "https://b", "audio": true, "no_audio": false},
                {"ext": "opus", "url": "https://c", "audio": true, "no_audio": 1},
                {"quality": "49"}
              ]
            }"#,
        );
        let hasil = ssyoutube_download(&http, "https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(hasil["meta"]["id"], "dQw4w9WgXcQ");
        assert_eq!(hasil["meta"]["title"], "Rick Astley");
        assert_eq!(hasil["meta"]["duration"], "3:33");
        assert_eq!(hasil["meta"]["thumbnail"], "https://i.ytimg.com/hq.jpg");

        let dl = hasil["downloads"].as_array().unwrap();
        // `no_audio` truthy (1) dibuang; sisanya ikut walau `no_audio` hilang.
        assert_eq!(dl.len(), 3);
        assert_eq!(dl[0]["quality"], "720");
        assert_eq!(dl[0]["format"], "mp4");
        assert_eq!(dl[0]["size"], "Unknown");
        assert_eq!(dl[0]["audio"], false);
        // `quality` kosong jatuh ke `subname`.
        assert_eq!(dl[1]["quality"], "audio");
        assert_eq!(dl[1]["size"], "3.29 MB");
        // Kunci `url` yang tidak ada tidak ikut di `JSON.stringify`.
        assert!(dl[2].get("url").is_none(), "tidak ada url");
        assert_eq!(dl[2]["quality"], "49");
        assert_eq!(dl[2]["size"], "Unknown");
        assert!(dl[2].get("format").is_none(), "ext tidak ada");
        assert!(dl[2].get("audio").is_none(), "audio tidak ada");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://ssyoutube.com/api/convert");
        assert_eq!(cari_header(&p[0], "origin").as_deref(), Some("https://ssyoutube.com"));
        assert_eq!(cari_header(&p[0], "referer").as_deref(), Some("https://ssyoutube.com/"));
        assert_eq!(cari_header(&p[0], "user-agent").as_deref(), Some(UA));
        assert_eq!(
            cari_header(&p[0], "content-type").as_deref(),
            Some("application/x-www-form-urlencoded;charset=UTF-8")
        );
        assert_eq!(
            cari_header(&p[0], "accept").as_deref(),
            Some("application/json, text/plain, */*")
        );

        // Body `qs.stringify`: urutan kunci tetap, signature = sha256(url+ts+salt).
        let body = form(&p[0]);
        let bagian: Vec<&str> = body.split('&').collect();
        assert_eq!(bagian.len(), 5, "body: {body}");
        assert_eq!(bagian[0], "sf_url=https%3A%2F%2Fyoutu.be%2FdQw4w9WgXcQ");
        assert!(bagian[1].starts_with("ts="), "bagian[1] = {}", bagian[1]);
        let ts = bagian[1].trim_start_matches("ts=");
        assert!(ts.chars().all(|c| c.is_ascii_digit()) && ts.len() == 13, "ts = {ts}");
        assert_eq!(bagian[2], "_ts=1765962059039");
        assert_eq!(bagian[3], "_tsc=0");
        assert_eq!(
            bagian[4],
            format!("_s={}", sha256_hex(
                format!("https://youtu.be/dQw4w9WgXcQ{ts}{SALT}").as_bytes()
            ))
        );
    }

    #[test]
    fn url_kosong_punya_nilai_tetap_lolos_ke_downloads_kosong() {
        // `![]` = false di JS, jadi array kosong tidak masuk cabang error.
        let http = stub(r#"{"id":"x","url":[]}"#);
        let hasil = ssyoutube_download(&http, "https://youtube.com/watch?v=x").unwrap();
        assert!(hasil.get("error").is_none());
        assert_eq!(hasil["downloads"].as_array().unwrap().len(), 0);
        assert_eq!(hasil["meta"]["id"], "x");
        assert!(hasil["meta"].get("title").is_none(), "meta tidak ada");
        assert!(hasil["meta"].get("thumbnail").is_none(), "thumb tidak ada");
    }

    #[test]
    fn url_hilang_atau_kosong_memberi_teks_error_yang_pasti() {
        for balasan in [
            r#"{"id":"x"}"#,
            r#"{"url":null}"#,
            r#"{"url":""}"#,
            r#"{"url":0}"#,
            "bukan json",
        ] {
            let http = stub(balasan);
            let hasil = ssyoutube_download(&http, "https://youtu.be/x").unwrap();
            assert_eq!(
                hasil["error"],
                "Gagal mengambil data. Server mungkin memblokir request.",
                "balasan: {balasan}"
            );
        }
    }

    #[test]
    fn kegagalan_transport_dilewatkan_seperti_catch_js() {
        #[derive(Debug)]
        struct Mati;
        impl oktz_plugin_api::HttpGet for Mati {
            fn get(&self, _: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                Err(HttpError { message: "HTTP 403".into() })
            }
            fn post_json(&self, _: &str, _: &Value) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_form(&self, _: &str, _: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_multipart(
                &self,
                _: &str,
                _: &[(&str, oktz_plugin_api::HttpBody)],
            ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                unreachable!()
            }
            fn call(&self, _: &oktz_plugin_api::HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                Err(HttpError { message: "HTTP 403".into() })
            }
        }
        // `catch` JS membungkus kegagalan jadi objek, bukan melempar.
        let err = ssyoutube_download(&Mati, "https://youtu.be/x").unwrap_err();
        assert_eq!(err.message, "HTTP 403");
    }
}