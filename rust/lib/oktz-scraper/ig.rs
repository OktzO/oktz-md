//! `src/scraper/ig.js` — `instagramDownloader` di atas `igdl()` dari paket npm
//! `btch-downloader`.
//!
//! Yang ikut disalin dari JS:
//! - `igdl` sendiri tidak pernah melempar: kegagalan jaringan / status bukan 2xx
//!   / body bukan JSON semuanya berakhir di `catch` SDK dan jadi objek
//!   `status: false` (`node_modules/btch-downloader/dist/index.js:103-125`).
//!   Jadi `instagramDownloader` yang melempar, dengan teks yang persis sama
//!   dua kali: `Gagal mengambil media dari Instagram`.
//! - `filter(item => String(item?.url || "").trim() !== "")` terjadi SEBELUM
//!   pemetaan, dan `media.length === 0` dicek lagi setelahnya — dua-duanya
//!   melempar teks yang sama.
//! - `url: item.url` menyimpan nilai mentah respons, bukan hasil `String(...)`.
//!
//! Beda yang diketahui:
//! - Permintaan `igdl` Reproduksi `node_modules/btch-http/dist/HttpClient.js`:
//!   `GET {baseUrl}/{endpoint}?url=…` dengan `url` di-encode gaya
//!   `URLSearchParams` (bukan `encodeURIComponent`), header
//!   `User-Agent: btch/<versi>` dan `X-Client-Version`. Timeout 60 detik milik
//!   SDK tidak ditiru — `UreqHttp` punya batas global 20 detik.
//! - `Buffer.from(x, "base64")` ditulis tangan (Node menerima alfabet
//!   `base64url`, mengabaikan karakter lain, dan tidak butuh padding).

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::truthy;

/// `config.baseUrl` (`btch-downloader/dist/Defaults/site.js:15`).
const BTCH_BASE: &str = "https://backend1.tioo.eu.org";

/// `package.json` versi 6.3.6 — SDK memakai nilai ini untuk `X-Client-Version`.
const BTCH_VERSI: &str = "6.3.6";

/// `throw new Error("Gagal mengambil media dari Instagram")` — muncul dua kali
/// di JS (`ig.js:20` dan `:36`), jadi memang satu konstanta.
const GAGAL: &str = "Gagal mengambil media dari Instagram";

/// `searchParams` Node (`URLSearchParams`): alfanumerik + `*-._` dibiarkan,
/// spasi jadi `+`, sisanya persen huruf besar. Beda dari `encodeURIComponent`
/// yang dipakai `common::encode_uri_component` (spasi `%20`, `!'()~` lolos).
fn encode_query(s: &str) -> String {
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

/// `v || bawaan` untuk nilai JSON: falsy JS (`0`, `""`, `null`, `false`) jatuh
/// ke bawaan, selain itu nilai mentahnya disalin apa adanya.
fn nilai_atau(v: Option<&Value>, bawaan: Value) -> Value {
    match v {
        Some(x) if truthy(x) => x.clone(),
        _ => bawaan,
    }
}

/// `String(v || "")` — untuk penyaringan dan pencocokan `.mp4`.
fn teks_js_atau_kosong(v: Option<&Value>) -> String {
    let kosong = Value::String(String::new());
    let x = nilai_atau(v, kosong);
    teks_js(&x)
}

/// `String(v)` — hanya bentuk yang muncul di `ig.js`: string, dan objek/array
/// yang jatuh ke bentuk string bawaan JS.
fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a
            .iter()
            .map(teks_js)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `Buffer.from(s, "base64").toString()` — Node mengabaikan karakter di luar
/// abjad dan menerima `-_` (base64url) serta tanpa padding.
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

/// `new URL(u).searchParams.get(nama)` — nilai pertama yang cocok, sudah
/// di-decode (`+` jadi spasi, `%XX` jadi byte). `None` = tidak ada.
fn parameter_url(u: &str, nama: &str) -> Option<String> {
    let tanpa_hash = u.split('#').next().unwrap_or(u);
    let (_, query) = tanpa_hash.split_once('?')?;
    for bagian in query.split('&') {
        let (k, v) = bagian.split_once('=').unwrap_or((bagian, ""));
        if decode_komponen(k) == nama {
            return Some(decode_komponen(v));
        }
    }
    None
}

/// Decode satu komponen `application/x-www-form-urlencoded`.
fn decode_komponen(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 3 <= b.len() => match std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                Some(byte) => {
                    out.push(byte);
                    i += 3;
                }
                None => {
                    out.push(b'%');
                    i += 1;
                }
            },
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// `/\.([a-z0-9]+)(?:\?|$)/i` — kemunculan pertama, sama seperti `.match()`.
fn pola_ekstensi() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r"(?i)\.([a-z0-9]+)(?:\?|$)").expect("pola ekstensi"))
}

/// `extFromToken(mediaUrl)` (`ig.js:3-14`) — setiap kegagalan (URL rusak,
/// token tanpa titik, base64 bukan JSON, `payload.url` kosong) berakhir `""`.
fn ext_dari_token(media_url: &str) -> String {
    let Some(token) = parameter_url(media_url, "token") else {
        return String::new();
    };
    if token.is_empty() {
        return String::new();
    }
    let Some(bagian) = token.split('.').nth(1) else {
        return String::new();
    };
    let isi = String::from_utf8_lossy(&base64_deskep(bagian)).to_string();
    let Ok(payload) = serde_json::from_str::<Value>(&isi) else {
        return String::new();
    };
    let url = teks_js_atau_kosong(payload.get("url"));
    pola_ekstensi()
        .captures(&url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_lowercase())
        .unwrap_or_default()
}

/// `igdl(url)` (`ig.js:17`) dengan bentuk balikin SDK: objek selalu punya
/// `status`, dan `result` hanya ada kalau pemanggilan berhasil.
fn igdl(http: &dyn HttpGet, url: &str) -> Value {
    let alamat = format!("{BTCH_BASE}/igdl?url={}", encode_query(url));
    let Ok(res) = http.call(
        &HttpCall::get(&alamat)
            .header("content-type", "application/json")
            .header("user-agent", format!("btch/{BTCH_VERSI}"))
            .header("x-client-version", BTCH_VERSI),
    ) else {
        return json!({ "status": false });
    };
    // `JSON.parse` gagal / hasil bukan array / array kosong → `status: false`.
    let Some(daftar) = res.json().and_then(|v| v.as_array().cloned()) else {
        return json!({ "status": false });
    };
    if daftar.is_empty() {
        return json!({ "status": false });
    }
    let result: Vec<Value> = daftar
        .iter()
        .map(|item| {
            json!({
                "thumbnail": nilai_atau(item.get("thumbnail"), Value::String(String::new())),
                "url": nilai_atau(item.get("url"), Value::String(String::new())),
            })
        })
        .collect();
    json!({ "status": true, "result": result })
}

/// `instagramDownloader(url)` (`ig.js:16-50`).
pub fn instagram_download(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let data = igdl(http, url);
    let gagal = !data.get("status").is_some_and(truthy)
        || !data
            .get("result")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty());
    if gagal {
        return Err(HttpError {
            message: GAGAL.to_string(),
        });
    }
    let items = data
        .get("result")
        .and_then(Value::as_array)
        .expect("result sudah dicek di atas")
        .clone();

    let mut media: Vec<Value> = Vec::new();
    for item in &items {
        let url_teks = teks_js_atau_kosong(item.get("url"));
        if url_teks.trim().is_empty() {
            continue;
        }
        let ext = ext_dari_token(&url_teks);
        let video = matches!(ext.as_str(), "mp4" | "mov" | "m3u8") || url_teks.contains(".mp4");
        let mut m = Map::new();
        m.insert(
            "type".to_string(),
            Value::String((if video { "video" } else { "image" }).to_string()),
        );
        m.insert(
            "url".to_string(),
            item.get("url").cloned().unwrap_or(Value::Null),
        );
        m.insert(
            "thumbnail".to_string(),
            nilai_atau(item.get("thumbnail"), Value::String(String::new())),
        );
        media.push(Value::Object(m));
    }
    if media.is_empty() {
        return Err(HttpError {
            message: GAGAL.to_string(),
        });
    }

    let mut out = Map::new();
    out.insert("status".to_string(), Value::Bool(true));
    out.insert("username".to_string(), Value::String("-".to_string()));
    out.insert("title".to_string(), Value::String("Instagram".to_string()));
    out.insert("caption".to_string(), Value::String(String::new()));
    // `media[0]?.thumbnail || ""` — nilai `thumbnail` sudah pernah di-`|| ""`.
    out.insert(
        "thumbnail".to_string(),
        nilai_atau(media[0].get("thumbnail"), Value::String(String::new())),
    );
    out.insert("avatar".to_string(), Value::String(String::new()));
    out.insert("media".to_string(), Value::Array(media));
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` yang mencatat permintaan dan membalas JSON tetap —
    /// tidak ada jaringan di test.
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: String,
        gagal: bool,
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
            unreachable!("scraper ini hanya GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
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
            if self.gagal {
                return Err(HttpError {
                    message: "koneksi ditolak".to_string(),
                });
            }
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
            gagal: false,
        }
    }

    #[test]
    fn memetakan_media_dan_meneruskan_permintaan_seperti_sdk() {
        let http = stub(
            r#"[{"thumbnail":"https://cdn/t.jpg","url":"https://cdn/a.jpg"},
                {"thumbnail":"","url":"https://cdn/b.mp4"}]"#,
        );
        let hasil = instagram_download(&http, "https://www.instagram.com/p/abc").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["username"], "-");
        assert_eq!(hasil["title"], "Instagram");
        assert_eq!(hasil["caption"], "");
        assert_eq!(hasil["avatar"], "");
        assert_eq!(hasil["thumbnail"], "https://cdn/t.jpg");

        let media = hasil["media"].as_array().unwrap();
        assert_eq!(media.len(), 2);
        assert_eq!(media[0]["type"], "image");
        assert_eq!(media[0]["url"], "https://cdn/a.jpg");
        assert_eq!(media[1]["type"], "video", "ekstensi .mp4 di URL");
        assert_eq!(media[1]["thumbnail"], "");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://backend1.tioo.eu.org/igdl?url=https%3A%2F%2Fwww.instagram.com%2Fp%2Fabc"
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("user-agent").as_deref(), Some("btch/6.3.6"));
        assert_eq!(cari("x-client-version").as_deref(), Some("6.3.6"));
        assert_eq!(cari("content-type").as_deref(), Some("application/json"));
    }

    #[test]
    fn token_base64_membuat_ext_dan_jenis_video_terbaca() {
        // Bagian tengah = base64 `{"url":"https://cdn/x/pic.JPEG","auth":1}`;
        // regex-nya menangkap "jpeg" (huruf besar ikut, huruf kecilkan).
        let token = "aaa.eyJ1cmwiOiJodHRwczovL2Nkbi94L3BpYy5KUEVHIiwiYXV0aCI6MX0.bbb";
        let http = stub(&format!(
            r#"[{{"thumbnail":"t","url":"https://cdn/media?token={token}"}}]"#
        ));
        let hasil = instagram_download(&http, "https://www.instagram.com/reel/x").unwrap();
        assert_eq!(hasil["media"][0]["type"], "image");
        assert_eq!(
            hasil["media"][0]["url"],
            format!("https://cdn/media?token={token}")
        );
    }

    #[test]
    fn request_gagal_tetap_melempar_teks_yang_sama_seperti_js() {
        // `igdl` menangkap error-nya sendiri jadi `status: false`, sehingga
        // teks yang keluar tetap dari `instagramDownloader`.
        let http = StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: String::new(),
            gagal: true,
        };
        let err = instagram_download(&http, "https://www.instagram.com/p/abc").unwrap_err();
        assert_eq!(err.message, "Gagal mengambil media dari Instagram");
    }

    #[test]
    fn hasil_kosong_dan_url_kosong_keduanya_melempar_teks_sama() {
        let kosong = stub("[]");
        assert_eq!(
            instagram_download(&kosong, "https://x").unwrap_err().message,
            "Gagal mengambil media dari Instagram"
        );

        // `result` ada tapi semua `url`-nya kosong → habis penyaringan.
        let tanpa_url = stub(r#"[{"thumbnail":"t","url":"   "}]"#);
        assert_eq!(
            instagram_download(&tanpa_url, "https://x").unwrap_err().message,
            "Gagal mengambil media dari Instagram"
        );

        // Body bukan array → `for (const item of data)` melempar di dalam SDK.
        let bukan_array = stub(r#"{"status":true}"#);
        assert_eq!(
            instagram_download(&bukan_array, "https://x").unwrap_err().message,
            "Gagal mengambil media dari Instagram"
        );
    }

    #[test]
    fn base64_meniru_node_untuk_token_rusak() {
        // Node menerima base64 tanpa padding dan mengabaikan karakter lain.
        let byte = base64_deskep("aGVsbG8gd29ybGQ");
        assert_eq!(String::from_utf8_lossy(&byte), "hello world");
        // Token tanpa titik kedua → `split(".")[1]` undefined → `""`.
        assert_eq!(ext_dari_token("https://cdn/a.jpg?token=abc"), "");
        assert_eq!(ext_dari_token("bukan-url"), "");
    }
}