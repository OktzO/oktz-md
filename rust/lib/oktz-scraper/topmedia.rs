//! `src/scraper/topmedia.js` — text-to-speech TopMediai.
//!
//! JS memakai modul `https` bawaan Node (bukan axios), jadi tidak ada
//! `validateStatus`: status 4xx/5xx TIDAK melempar, isi body-nya tetap dibaca
//! dan baru gagal di `oss_url` yang tidak ada. Perilaku itu disalin dengan
//! `terima_semua_status()`.
//!
//! Yang dikembalikan JS adalah Promise yang resolve ke URL audio (string, bukan
//! objek), jadi port-nya `Result<String, HttpError>`.

use std::env;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Value, json};

use crate::common::truthy;

/// `topmedia.js:8` — speaker bawaan kalau argumennya falsy.
const SPEAKER_BAWAAN: &str = "001526de-3826-11ee-a861-00163e2ac61b";

/// `topmedia.js:14` — hostname + path dari opsi `https.request`.
const ENDPOINT: &str = "https://api.topmediai.com/v1/text2speech";

/// `topmedia.js:2` — `process.env.TOPMEDIA_API_KEY || ''`.
///
/// JS membacanya sekali saat modul dimuat; di sini dibaca tiap panggilan
/// supaya test tidak perlu menulis env proses (bedanya cuma kalau env berubah
/// di tengah jalan).
fn api_key() -> String {
    env::var("TOPMEDIA_API_KEY").unwrap_or_default()
}

/// `generateCustomTTS(speakerId, text)` (`topmedia.js:4-42`).
///
/// `Err("No audio URL")` = `reject(new Error("No audio URL"))`; pesan lain di
/// `Err` datang dari klien HTTP atau dari body yang bukan JSON — JS melempar
/// `SyntaxError` di situ dan teksnya milik mesin JS, tidak bisa disalin huruf
/// per huruf.
pub fn generate_custom_tts(
    http: &dyn HttpGet,
    speaker_id: &str,
    text: &str,
) -> Result<String, HttpError> {
    // `speakerId || "001526de-…"` — string kosong ikut jatuh ke bawaan.
    let speaker = if speaker_id.is_empty() {
        SPEAKER_BAWAAN
    } else {
        speaker_id
    };
    // Urutan kunci mengikuti `JSON.stringify({text, speaker, emotion})`.
    let body = json!({ "text": text, "speaker": speaker, "emotion": "Happy" });

    let res = http.call(
        &HttpCall::post(ENDPOINT)
            .header("content-type", "application/json")
            .header("x-api-key", api_key())
            // Node `https.request` tidak pernah melempar karena status.
            .terima_semua_status()
            .json(&body),
    )?;
    // Header `Content-Length` di JS tidak ikut ditulis: klien di sini
    // menghitungnya sendiri dari body, dan hasil di kabel sama.
    let data = res.json().ok_or_else(|| HttpError {
        message: "respons topmedia bukan JSON".to_string(),
    })?;

    // `const audioUrl = json?.data?.oss_url; if (!audioUrl) reject(…)`.
    match data.get("data").and_then(|d| d.get("oss_url")) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        // `resolve(audioUrl)` juga menerima nilai truthy non-string; di sini
        // dipetakan ke teks JSON-nya (API nyata selalu memberi string).
        Some(v) if truthy(v) => Ok(v.to_string()),
        _ => Err(HttpError {
            message: "No audio URL".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet`: mencatat `HttpCall`, membalas body tetap — tanpa
    /// jaringan, seperti pola `dailymotion.rs`.
    #[derive(Debug)]
    struct Stub {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: String,
        status: u16,
        gagal: bool,
    }

    impl Stub {
        fn baru(balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: balasan.to_string(),
                status: 200,
                gagal: false,
            }
        }
        fn status(balasan: &str, status: u16) -> Self {
            Self {
                status,
                ..Self::baru(balasan)
            }
        }
        fn offline() -> Self {
            Self {
                gagal: true,
                ..Self::baru("")
            }
        }
        fn jawab(&self) -> Result<HttpResponse, HttpError> {
            if self.gagal {
                return Err(HttpError {
                    message: "offline".to_string(),
                });
            }
            Ok(HttpResponse {
                status: self.status,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            self.jawab()
        }
    }

    fn cari_header<'a>(req: &'a HttpCall, nama: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn mengembalikan_oss_url_dan_mengirim_body_yang_benar() {
        let http = Stub::baru(r#"{"code":0,"data":{"oss_url":"https://cdn.example/a.mp3"}}"#);
        let hasil = generate_custom_tts(&http, "001526de-3826-11ee-a861-00163e2ac61b", "halo").unwrap();
        assert_eq!(hasil, "https://cdn.example/a.mp3");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://api.topmediai.com/v1/text2speech");
        assert_eq!(
            p[0].body,
            Some(HttpBody::Bytes {
                data: Arc::from(
                    br#"{"text":"halo","speaker":"001526de-3826-11ee-a861-00163e2ac61b","emotion":"Happy"}"#
                        .as_slice()
                ),
                mimetype: Some("application/json".to_string()),
            })
        );
        assert_eq!(cari_header(&p[0], "content-type"), Some("application/json"));
        // `x-api-key` sama dengan env proses — nilainya tidak di-hardcode.
        assert_eq!(
            cari_header(&p[0], "x-api-key"),
            Some(env::var("TOPMEDIA_API_KEY").unwrap_or_default().as_str())
        );
    }

    #[test]
    fn speaker_kosong_jatuh_ke_speaker_bawaan() {
        let http = Stub::baru(r#"{"data":{"oss_url":"https://cdn/x.mp3"}}"#);
        generate_custom_tts(&http, "", "hai").unwrap();
        let p = http.permintaan.lock().unwrap();
        let body = match &p[0].body {
            Some(HttpBody::Bytes { data, .. }) => String::from_utf8(data.to_vec()).unwrap(),
            other => panic!("harus Bytes JSON, dapat {other:?}"),
        };
        assert!(body.contains("001526de-3826-11ee-a861-00163e2ac61b"), "{body}");
    }

    #[test]
    fn oss_url_kosong_atau_hilang_jadi_no_audio_url() {
        for balasan in [
            r#"{"data":{}}"#,
            r#"{"data":{"oss_url":""}}"#,
            r#"{"data":{"oss_url":null}}"#,
            r#"{}"#,
        ] {
            let http = Stub::baru(balasan);
            let err = generate_custom_tts(&http, "s", "t").unwrap_err();
            assert_eq!(err.message, "No audio URL", "balasan {balasan}");
        }
    }

    #[test]
    fn status_4xx_tidak_melempar_dulu_seperti_https_node() {
        // Body 4xx berisi JSON tanpa `oss_url` → JS tetap sampai ke
        // `reject(new Error("No audio URL"))`, bukan error transport.
        let http = Stub::status(r#"{"error":"unauthorized"}"#, 401);
        let err = generate_custom_tts(&http, "s", "t").unwrap_err();
        assert_eq!(err.message, "No audio URL");
    }

    #[test]
    fn bukan_json_menghasilkan_error_bukan_ok() {
        let http = Stub::baru("<html>bukan json</html>");
        let err = generate_custom_tts(&http, "s", "t").unwrap_err();
        assert_eq!(err.message, "respons topmedia bukan JSON");
    }

    #[test]
    fn kegagalan_transport_diteruskan() {
        let http = Stub::offline();
        let err = generate_custom_tts(&http, "s", "t").unwrap_err();
        assert_eq!(err.message, "offline");
    }
}
