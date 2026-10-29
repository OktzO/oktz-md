//! `src/scraper/tempmail.js` — buat alamat email sementara dan baca inbox-nya
//! lewat `temp-mail.io`.
//!
//! Yang ikut disalin dari JS:
//! - `generateCORS(15)` = 15 karakter acak dari alnum; panjangnya default 15
//!   dan hanya dipakai sebagai nilai header `X-CORS-Header`, jadi tidak pernah
//!   diuji server selain sebagai token.
//! - `buildHeaders()` dibangun ulang untuk tiap permintaan (JS memanggilnya di
//!   dalam `axios`), jadi token `X-CORS-Header` ikut berbeda.
//! - `response.data || []` lalu `count: messages.length` — kalau `data` bukan
//!   array tapi objek, `.length` adalah `undefined` dan kuncinya tidak ikut,
//!   sedangkan `messages` tetap objek itu apa adanya.
//! - `catch` mengembalikan `{ status: false, error: e.message }`, bukan melempar.
//!
//! Beda yang diketahui: `X-CORS-Header` di JS benar-benar acak, jadi test hanya
//! boleh memeriksa bentuk (panjang 15, semua alnum), bukan nilainya.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

/// `TempMailCreate()` (`tempmail.js:23-34`).
pub fn temp_mail_create(http: &dyn HttpGet) -> Result<Value, HttpError> {
    match alur_buat(http) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": false, "error": e.message })),
    }
}

fn alur_buat(http: &dyn HttpGet) -> Result<Value, HttpError> {
    let res = http.call(
        &header(HttpCall::post(
            "https://api.internal.temp-mail.io/api/v3/email/new",
        ))
        .json(&json!({ "min_name_length": 10, "max_name_length": 12 })),
    )?;
    let data = res.json().unwrap_or(Value::Null);

    // `response.data.email` — `undefined` tidak ikut di `JSON.stringify`.
    let mut o = Map::new();
    o.insert("status".to_string(), Value::Bool(true));
    if let Some(email) = data.get("email") {
        o.insert("email".to_string(), email.clone());
    }
    Ok(Value::Object(o))
}

/// `TempMailInbox(email)` (`tempmail.js:36-47`).
pub fn temp_mail_inbox(http: &dyn HttpGet, email: &str) -> Result<Value, HttpError> {
    match alur_inbox(http, email) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": false, "error": e.message })),
    }
}

fn alur_inbox(http: &dyn HttpGet, email: &str) -> Result<Value, HttpError> {
    // `email` disisipkan apa adanya — JS tidak melakukan encode.
    let res = http.call(&header(HttpCall::get(format!(
        "https://api.internal.temp-mail.io/api/v3/email/{email}/messages"
    ))))?;
    // Body non-JSON jadi string di axios; `|| []` tidak berlaku karena string
    // itu truthy, jadi `count` = panjang string dan `messages` = string itu.
    let data = res.json().unwrap_or_else(|| Value::String(res.text()));

    // `const messages = response.data || []`.
    let messages = match data {
        Value::Null => Value::Array(Vec::new()),
        v if truthy(&v) => v,
        _ => Value::Array(Vec::new()),
    };

    let mut o = Map::new();
    o.insert("status".to_string(), Value::Bool(true));
    o.insert("email".to_string(), json!(email));
    // `messages.length` hanya ada untuk array/string; objek tidak punya, dan
    // `undefined` tidak ikut hasil.
    let panjang = match &messages {
        Value::Array(a) => Some(a.len()),
        Value::String(s) => Some(s.encode_utf16().count()),
        _ => None,
    };
    if let Some(n) = panjang {
        o.insert("count".to_string(), Value::from(n as u64));
    }
    o.insert("messages".to_string(), messages);
    Ok(Value::Object(o))
}

/// `buildHeaders()` (`tempmail.js:11-21`) — dipanggil per permintaan, termasuk
/// token acaknya.
fn header(req: HttpCall) -> HttpCall {
    req.header("Content-Type", "application/json")
        .header("Application-Name", "web")
        .header("Application-Version", "4.0.0")
        .header("X-CORS-Header", generate_cors(15))
        .header("User-Agent", UA)
        .header("Referer", "https://temp-mail.io/")
}

/// `generateCORS(length = 15)` (`tempmail.js:3-9`) — acak dari alnum.
fn generate_cors(panjang: usize) -> String {
    const ABJAD: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..panjang)
        .map(|_| ABJAD[rand::random_range(0..ABJAD.len())] as char)
        .collect()
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
            unreachable!("semua lewat HttpCall::call supaya header tercatat")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
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

    /// Tokennya acak, jadi hanya bentuknya yang boleh diperiksa.
    fn token_bentuk_ok(v: &str) -> bool {
        v.len() == 15 && v.bytes().all(|b| b.is_ascii_alphanumeric())
    }

    #[test]
    fn buat_email_mengirim_body_dan_header_yang_benar() {
        let http = Stub::baru(r#"{"email":"abc1234567@temp-mail.io"}"#);
        let hasil = temp_mail_create(&http).unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["email"], "abc1234567@temp-mail.io");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(
            p[0].url,
            "https://api.internal.temp-mail.io/api/v3/email/new"
        );
        assert_eq!(cari_header(&p[0], "content-type"), Some("application/json"));
        assert_eq!(cari_header(&p[0], "application-name"), Some("web"));
        assert_eq!(cari_header(&p[0], "application-version"), Some("4.0.0"));
        assert_eq!(cari_header(&p[0], "referer"), Some("https://temp-mail.io/"));
        assert!(token_bentuk_ok(cari_header(&p[0], "x-cors-header").unwrap()));
        assert_eq!(
            p[0].body,
            Some(HttpBody::Bytes {
                data: Arc::from(br#"{"min_name_length":10,"max_name_length":12}"#.as_slice()),
                mimetype: Some("application/json".to_string()),
            })
        );
    }

    #[test]
    fn token_cors_berbeda_untuk_tiap_panggilan() {
        let http = Stub::baru(r#"{"email":"a@b.c"}"#);
        temp_mail_create(&http).unwrap();
        temp_mail_create(&http).unwrap();
        let p = http.permintaan.lock().unwrap();
        assert_ne!(p[0].headers, p[1].headers, "token harus diacak ulang");
    }

    #[test]
    fn inbox_menghitung_pesan_dan_mengembalikan_email_yang_diminta() {
        // `response.data` sendiri adalah array — `messages = response.data || []`,
        // tanpa membungkus dalam objek.
        let http = Stub::baru(r#"[{"id":"1","subject":"halo"},{"id":"2","subject":"dunia"}]"#);
        let hasil = temp_mail_inbox(&http, "abc@temp-mail.io").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["email"], "abc@temp-mail.io");
        assert_eq!(hasil["count"], 2);
        assert_eq!(hasil["messages"][1]["subject"], "dunia");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://api.internal.temp-mail.io/api/v3/email/abc@temp-mail.io/messages"
        );
        assert!(p[0].body.is_none());
    }

    #[test]
    fn data_kosong_memberi_count_nol_dan_messages_kosong() {
        // `null` dan `[]` saja yang jadi array kosong. `{}` itu TRUTHY di JS,
        // jadi `|| []` tidak berlaku dan `length`-nya `undefined` — sama
        // dengan kasus objek di test sebelumnya.
        for balasan in [r#"null"#, r#"[]"#] {
            let http = Stub::baru(balasan);
            let hasil = temp_mail_inbox(&http, "a@b.c").unwrap();
            assert_eq!(hasil["count"], 0, "balasan {balasan}");
            assert_eq!(hasil["messages"], json!([]), "balasan {balasan}");
        }
    }

    #[test]
    fn objek_tanpa_panjang_menghilangkan_kunci_count() {
        // `messages.length` pada objek adalah `undefined` -> tidak ikut JSON.
        let http = Stub::baru(r#"{"hydra:member":[]}"#);
        let hasil = temp_mail_inbox(&http, "a@b.c").unwrap();
        assert!(hasil.get("count").is_none());
        assert_eq!(hasil["messages"], json!({"hydra:member": []}));
    }

    #[test]
    fn body_bukan_json_menjadi_string_truthy_bukan_array_kosong() {
        // `response.data` string adalah truthy, jadi `|| []` tidak jalan dan
        // `messages.length` = panjang string.
        let http = Stub::baru("<html>halo</html>");
        let hasil = temp_mail_inbox(&http, "a@b.c").unwrap();
        // `<html>halo</html>` = 6 + 4 + 7 = 17.
        assert_eq!(hasil["count"], 17);
        assert_eq!(hasil["messages"], json!("<html>halo</html>"));
    }

    #[test]
    fn galat_transport_kembali_sebagai_objek_status_false() {
        let hasil = temp_mail_create(&Stub::gagal("HTTP 429")).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "HTTP 429");

        let hasil = temp_mail_inbox(&Stub::gagal("HTTP 429"), "a@b.c").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "HTTP 429");
    }
}