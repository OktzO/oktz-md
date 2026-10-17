//! `src/scraper/geminiVision.js` — pembungkus `gemini.js`: merakit riwayat
//! dan pesan jadi satu teks, lalu menambahkan `raw`/`model` pada hasil.
//!
//! `geminiVision.js` memanggil `gemini()` yang sudah ada — di sini lewat
//! `crate::gemini::chat`, bukan port ulang. Satu grup, satu implementasi.
//!
//! Yang ikut disalin dari JS:
//! - `history` dipangkas ke 10 entri TERAKHIR (`slice(-10)`).
//! - `item?.role === "assistant" ? "Assistant" : "User"` — peran lain (termasuk
//!   `"user"` dan `undefined`) jadi `User`.
//! - `String(item?.content || "").trim()` lalu entri kosong dibuang
//!   (`filter(Boolean)`), jadi baris tanpa konten tidak ikut.
//! - Blok riwayat hanya masuk kalau `historyText` tidak kosong.
//! - `imageBuffer` tidak pernah dikirim sebagai gambar; ia hanya menambah satu
//!   kalimat penanda. Ini perilaku JS dan ikut dipertahankan.
//! - `parts.filter(Boolean).join("\n\n").trim()` — bagian kosong di buang.
//!
//! Beda dari JS yang perlu diketahui: `history` dan `imageBuffer` di JS datang
//! dari objek tunggal; di sini keduanya argumen terpisah karena `Buffer` tidak
//! punya padanan di `serde_json::Value`. Argumen `instruction` tidak punya nilai
//! bawaan — pemanggil yang mengirim `""` seperti `geminiVision.js:37`.

use oktz_plugin_api::{HttpError, HttpGet};
use serde_json::{Value, json};

/// `String(item?.content || "")` — `false`/`0`/`""`/`null` jadi string kosong.
fn teks_js(v: Option<&Value>) -> String {
    let Some(v) = v.filter(|v| crate::common::truthy(v)) else {
        return String::new();
    };
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(|x| teks_js(Some(x))).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
        Value::Null => String::new(),
    }
}

/// `buildMessage({ message, history = [], imageBuffer = null })`
/// (`geminiVision.js:3-31`).
fn build_message(message: &str, image_buffer: Option<&[u8]>, history: &[Value]) -> String {
    let mut parts: Vec<String> = Vec::new();

    // `Array.isArray(history) && history.length > 0` — `history` di sini slice,
    // jadi syarat "bukan array" tidak mungkin terjadi.
    if !history.is_empty() {
        let mulai = history.len().saturating_sub(10);
        let history_text = history[mulai..]
            .iter()
            .map(|item| {
                let role = match item.get("role").and_then(Value::as_str) {
                    Some("assistant") => "Assistant",
                    _ => "User",
                };
                let content = teks_js(item.get("content")).trim().to_string();
                if content.is_empty() {
                    String::new()
                } else {
                    format!("{role}: {content}")
                }
            })
            .filter(|baris| !baris.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if !history_text.is_empty() {
            parts.push(format!("Riwayat percakapan:\n{history_text}"));
        }
    }

    // `if (imageBuffer)` — buffer kosong tetap object dan tetap truthy di JS,
    // jadi `Some(&[])` ikut menambah penanda.
    if image_buffer.is_some() {
        parts.push(
            "User mengirim gambar. Jika model tidak bisa melihat gambar secara langsung, tetap jawab sebaik mungkin dari konteks teks yang ada."
                .to_string(),
        );
    }

    parts.push(message.trim().to_string());

    parts
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim()
        .to_string()
}

/// `chat({ message, instruction = "", imageBuffer = null, history = [] })`
/// (`geminiVision.js:33-50`).
pub fn chat(
    http: &dyn HttpGet,
    message: &str,
    instruction: &str,
    image_buffer: Option<&[u8]>,
    history: &[Value],
) -> Result<Value, HttpError> {
    let payload = json!({
        "message": build_message(message, image_buffer, history),
        "instruction": instruction,
    });
    let result = crate::gemini::chat(http, &payload)?;

    // `result.raw || result.text` dan `result.model || "gemini"` — `gemini.js`
    // tidak pernah mengisinya, jadi nilainya selalu jatuh ke bawaan.
    let text = result.get("text").cloned().unwrap_or(Value::Null);
    let raw = result
        .get("raw")
        .filter(|v| crate::common::truthy(v))
        .cloned()
        .unwrap_or_else(|| text.clone());
    let model = result
        .get("model")
        .filter(|v| crate::common::truthy(v))
        .and_then(Value::as_str)
        .unwrap_or("gemini")
        .to_string();
    let session_id = result
        .get("sessionId")
        .filter(|v| crate::common::truthy(v))
        .cloned()
        .unwrap_or(Value::Null);

    Ok(json!({
        "text": text,
        "raw": raw,
        "model": model,
        "sessionId": session_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall, HttpResponse};
    use std::sync::Mutex;

    /// Stub `HttpGet` yang membalas respons `batchexecute` + `StreamGenerate`
    /// milik gemini: yang pertama hanya inti cookie.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: &[(u16, &str)]) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(
                    balasan
                        .iter()
                        .map(|(s, t)| (*s, t.to_string()))
                        .collect(),
                ),
                hitung: Mutex::new(0),
            }
        }

        /// Dua balasan: cookie lalu blok jawaban.
        fn gemini(jawaban: &str) -> Self {
            let parsed = json!([
                "wrb.fr",
                ["id_lama"],
                null,
                null,
                [["id_baru", [jawaban]]]
            ]);
            let stream = format!(
                "5\n{}\n",
                json!([["wrb.fr", null, parsed.to_string()]])
            );
            let http = Self::baru(&[(200, stream.as_str())]);
            *http.balasan.lock().unwrap() = vec![(200, String::new()), (200, stream)];
            http
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("geminiVision cuma POST lewat gemini")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, body) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: std::sync::Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn body_form(p: &HttpCall) -> String {
        match &p.body {
            Some(HttpBody::Form(s)) => s.clone(),
            lain => panic!("body bukan form: {lain:?}"),
        }
    }

    /// Isi `requestBody[0][0]` = pesan yang dikirim ke gemini.
    fn pesan_terkirim(http: &StubHttp) -> String {
        let p = http.permintaan.lock().unwrap();
        let stream = p.last().expect("ada permintaan");
        let body = body_form(stream);
        let terkuti = body.strip_prefix("f.req=").expect("awalan f.req=");
        let decoded = decode_form(terkuti);
        let payload: Value = serde_json::from_str(&decoded).expect("f.req JSON");
        let request_body: Value =
            serde_json::from_str(payload[1].as_str().expect("payload[1]")).unwrap();
        request_body[0][0].as_str().expect("pesan string").to_string()
    }

    fn decode_form(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    out.push(b' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                    match u8::from_str_radix(hex, 16) {
                        Ok(b) => {
                            out.push(b);
                            i += 3;
                        }
                        Err(_) => {
                            out.push(bytes[i]);
                            i += 1;
                        }
                    }
                }
                b => {
                    out.push(b);
                    i += 1;
                }
            }
        }
        String::from_utf8_lossy(&out).to_string()
    }

    #[test]
    fn pesan_tanpa_riwayat_tetap_dikirim_lengkap() {
        let http = StubHttp::gemini("Halo");
        let hasil = chat(&http, "apa kabar?", "Singkat.", None, &[]).unwrap();
        assert_eq!(hasil["text"], "Halo");
        assert_eq!(hasil["raw"], "Halo", "raw jatuh ke text");
        assert_eq!(hasil["model"], "gemini", "model tidak diisi gemini");
        assert!(hasil["sessionId"].as_str().is_some_and(|s| !s.is_empty()));
        assert_eq!(pesan_terkirim(&http), "apa kabar?");
    }

    #[test]
    fn riwayat_ditambah_dengan_peran_dan_batas_sepuluh_entri() {
        let http = StubHttp::gemini("Oke");
        let history: Vec<Value> = (0..12)
            .map(|i| json!({"role": if i % 2 == 0 { "assistant" } else { "user" }, "content": format!("pesan {i}")}))
            .collect();
        chat(&http, "lanjut", "", None, &history).unwrap();
        let pesan = pesan_terkirim(&http);
        // `slice(-10)`: pesan 0 dan 1 terpotong.
        assert!(!pesan.contains("pesan 0:"));
        assert!(!pesan.contains("pesan 1:"));
        assert!(pesan.contains("Assistant: pesan 2"));
        assert!(pesan.starts_with("Riwayat percakapan:\n"));
        assert!(pesan.contains("\nUser: pesan 11"));
        assert!(pesan.ends_with("lanjut"), "pesan user selalu di akhir");
    }

    #[test]
    fn entri_riwayat_tanpa_konten_dibuang() {
        let http = StubHttp::gemini("Oke");
        let history = vec![
            json!({"role": "user", "content": "   "}),
            json!({"role": "user", "content": ""}),
            json!({"role": "system", "content": "sistem"}),
        ];
        chat(&http, "hai", "", None, &history).unwrap();
        let pesan = pesan_terkirim(&http);
        // Peran selain "assistant" jadi `User`.
        assert_eq!(pesan, "Riwayat percakapan:\nUser: sistem\n\nhai");
    }

    #[test]
    fn gambar_hanya_menambah_penanda_bukan_byte() {
        let http = StubHttp::gemini("Oke");
        chat(&http, "apa ini?", "", Some(b"PNG-BYTES"), &[]).unwrap();
        let pesan = pesan_terkirim(&http);
        assert_eq!(
            pesan,
            "User mengirim gambar. Jika model tidak bisa melihat gambar secara langsung, tetap jawab sebaik mungkin dari konteks teks yang ada.\n\napa ini?"
        );
        // Byte gambar tidak pernah masuk body.
        let p = http.permintaan.lock().unwrap();
        for req in p.iter() {
            let body = body_form(req);
            assert!(!body.contains("PNG"), "byte gambar bocor ke body");
        }
    }

    #[test]
    fn buffer_kosong_tetap_truthy_seperti_js() {
        let http = StubHttp::gemini("Oke");
        chat(&http, "x", "", Some(&[]), &[]).unwrap();
        assert!(
            pesan_terkirim(&http).contains("User mengirim gambar."),
            "Buffer kosong tetap object di JS"
        );
    }

    #[test]
    fn pesan_kosong_melempar_pesan_dari_gemini() {
        let http = StubHttp::gemini("Oke");
        // Tidak ada riwayat dan pesan kosong -> `buildMessage` mengembalikan "",
        // lalu `gemini` yang melempar.
        let salah = chat(&http, "   ", "", None, &[]).unwrap_err();
        assert_eq!(salah.message, "Message is required.");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn konten_tidak_string_dijadi_string_seperti_js() {
        let http = StubHttp::gemini("Oke");
        let history = vec![
            json!({"role": "assistant", "content": 42}),
            json!({"role": "user", "content": false}),
            json!({"role": "user"}),
        ];
        chat(&http, "hai", "", None, &history).unwrap();
        // `42` jadi "42"; `false` dan `undefined` jadi kosong lalu dibuang.
        assert_eq!(pesan_terkirim(&http), "Riwayat percakapan:\nAssistant: 42\n\nhai");
    }
}