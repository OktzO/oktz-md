//! `src/scraper/claudehaiku.js` — overchat.ai (`api.overchat.ai`), balasan SSE.
//!
//! Yang ikut disalin dari JS:
//! - `fetch` tidak melempar untuk 4xx/5xx; `response.ok` dicek sendiri →
//!   `{ status: false, code, error }` TANPA kunci `model` (beda dari
//!   `qwen3.js` yang menambahkannya).
//! - Sisa buffer terakhir TIDAK diproses: loop berhenti saat stream habis dan
//!   `buffer` tidak pernah diproses lagi, jadi baris `data:` terakhir yang belum
//!   diakhiri `\n` ikut terbuang.
//! - `typeof json.id === "string"` dan `typeof json.model === "string"` — nilai
//!   non-string TIDAK menimpa `responseId`/`responseModel` (beda dari
//!   `gpt5.js` yang memakai `if (json.id)`).
//! - `model: responseModel || model` → model konstanta kalau SSE tidak
//!   mengirim `model`.
//! - `options.history` yang bukan array: JS melempar `TypeError`; port ini
//!   memperlakukannya seperti daftar kosong (beda yang diketahui).

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const API: &str = "https://api.overchat.ai/v1/chat/completions";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

const MODEL: &str = "claude-haiku-4-5-20251001";

/// `crypto.randomUUID()` — UUID v4 huruf kecil (`claudehaiku.js:9-10`).
fn uuid_v4() -> String {
    let mut b: [u8; 16] = rand::random();
    b[6] = (b[6] & 0x0f) | 0x40; // versi 4
    b[8] = (b[8] & 0x3f) | 0x80; // varian RFC 4122
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `options.<kunci>` kalau truthy; `None` berarti `||` memakai nilai lain.
fn opsi(options: &Value, kunci: &str) -> Option<Value> {
    options.get(kunci).filter(|v| truthy(v)).cloned()
}

/// `ClaudeHaiku(prompt, options = {})` (`claudehaiku.js:8-116`).
pub fn claude_haiku(http: &dyn HttpGet, prompt: &str, options: &Value) -> Result<Value, HttpError> {
    let chat_id = opsi(options, "chatId").unwrap_or_else(|| json!(uuid_v4()));
    let device_id = opsi(options, "deviceId").unwrap_or_else(|| json!(uuid_v4()));

    let mut messages: Vec<Value> = Vec::new();
    if let Some(riwayat) = options.get("history").and_then(Value::as_array) {
        for item in riwayat {
            let mut pesan = Map::new();
            pesan.insert("id".to_string(), json!(uuid_v4()));
            // `{role: item.role, content: item.content}` — kunci `undefined`
            // tidak ikut `JSON.stringify`.
            for kunci in ["role", "content"] {
                if let Some(v) = item.get(kunci) {
                    pesan.insert(kunci.to_string(), v.clone());
                }
            }
            messages.push(Value::Object(pesan));
        }
    }
    messages.push(json!({ "id": uuid_v4(), "role": "user", "content": prompt }));
    messages.push(json!({
        "id": uuid_v4(),
        "role": "system",
        "content": "Ikuti bahasa user dan jawab dengan gaya natural, singkat, dan jelas.",
    }));

    let body = json!({
        "chatId": chat_id,
        "model": MODEL,
        "messages": messages,
        "personaId": "claude-haiku-4-5-landing",
        "frequency_penalty": 0,
        "max_tokens": 4000,
        "presence_penalty": 0,
        "stream": true,
        "temperature": 0.5,
        "top_p": 0.95,
    });

    let res = http.call(
        &HttpCall::post(API)
            .header("sec-ch-ua-platform", "\"Android\"")
            .header("x-device-uuid", teks_js(&device_id))
            .header(
                "sec-ch-ua",
                "\"Google Chrome\";v=\"147\", \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"147\"",
            )
            .header("sec-ch-ua-mobile", "?1")
            .header("x-device-language", "id-ID")
            .header("x-device-platform", "web")
            .header("x-device-version", "1.0.44")
            .header("user-agent", UA)
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .header("origin", "https://overchat.ai")
            .header("referer", "https://overchat.ai/")
            .header("accept-language", "id-ID,id;q=0.9")
            .header("priority", "u=1, i")
            .json(&body)
            // `fetch` tidak melempar untuk 4xx/5xx.
            .terima_semua_status(),
    )?;

    if !(200..300).contains(&res.status) {
        return Ok(json!({
            "status": false,
            "code": res.status,
            "error": res.text(),
        }));
    }

    let (jawaban, response_id, response_model) = rakit_sse(&res.text());

    Ok(json!({
        "status": true,
        "code": res.status,
        "question": prompt,
        "chatId": chat_id,
        "deviceId": device_id,
        "responseId": response_id,
        "model": response_model.unwrap_or_else(|| MODEL.to_string()),
        "answer": jawaban,
    }))
}

/// Baris `data:` satu stream → (jawaban, id terakhir, model terakhir).
///
/// Dipisah dari bagian HTTP supaya bisa diuji tanpa jaringan sama sekali.
fn rakit_sse(teks: &str) -> (String, Option<String>, Option<String>) {
    let mut baris: Vec<&str> = teks.split('\n').collect();
    // Sisa buffer terakhir dibuang, sama seperti JS yang tidak memprosesnya.
    baris.pop();

    let mut jawaban = String::new();
    let mut response_id = None;
    let mut response_model = None;
    for raw_line in baris {
        let line = raw_line.trim();
        let Some(sisa) = line.strip_prefix("data:") else {
            continue;
        };
        let data = sisa.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        // `catch {}` — baris yang gagal di-parse diam-diam dilewati.
        let Ok(j) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        // `typeof x === "string"` — angka tidak menimpa nilai sebelumnya.
        if let Some(s) = j.get("id").and_then(Value::as_str) {
            response_id = Some(s.to_string());
        }
        if let Some(s) = j.get("model").and_then(Value::as_str) {
            response_model = Some(s.to_string());
        }
        if let Some(content) = j
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("delta"))
            .and_then(|d| d.get("content"))
            .and_then(Value::as_str)
        {
            jawaban.push_str(content);
        }
    }
    (jawaban, response_id, response_model)
}

/// `String(v)` untuk nilai JSON, dipakai saat nilai masuk ke header (fetch
/// mengonversi nilai header apa pun ke teks).
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

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body) yang ditentukan test.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn new(balasan: Vec<(u16, String)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
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
            unreachable!("scraper ini kirim JSON")
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
            let (status, teks) = balasan[i].clone();
            Ok(oktz_plugin_api::HttpResponse {
                status,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(teks.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn body_json(p: &oktz_plugin_api::HttpCall) -> Value {
        match &p.body {
            Some(oktz_plugin_api::HttpBody::Bytes { data, .. }) => {
                serde_json::from_slice(data).expect("body JSON")
            }
            lain => panic!("body bukan JSON: {lain:?}"),
        }
    }

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn uuid_berpola(s: &str) -> bool {
        let re = regex::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        )
        .unwrap();
        re.is_match(s)
    }

    #[test]
    fn mengirim_body_lengkap_dan_merakit_jawaban_sse() {
        let http = StubHttp::new(vec![(
            200,
            "data: {\"id\":\"resp-1\",\"model\":\"claude-haiku-4-5-20251001\",\"choices\":[{\"delta\":{\"content\":\"Halo\"}}]}\n\
             : komentar keep-alive\n\
             data: {\"choices\":[{\"delta\":{\"content\":\" dunia\"}}]}\n\
             data: [DONE]\n"
                .to_string(),
        )]);
        let hasil = claude_haiku(
            &http,
            "hai",
            &json!({
                "chatId": "chat-1",
                "deviceId": "dev-1",
                "history": [{"role": "user", "content": "halo"}]
            }),
        )
        .unwrap();

        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["question"], "hai");
        assert_eq!(hasil["answer"], "Halo dunia");
        assert_eq!(hasil["responseId"], "resp-1");
        assert_eq!(hasil["model"], MODEL);
        assert_eq!(hasil["chatId"], "chat-1");
        assert_eq!(hasil["deviceId"], "dev-1");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].url, API);
        assert_eq!(cari_header(&p[0], "x-device-uuid"), Some("dev-1".to_string()));
        assert_eq!(cari_header(&p[0], "content-type"), Some("application/json".to_string()));
        // `accept-language` versi pendek, bukan milik qwen3.
        assert_eq!(
            cari_header(&p[0], "accept-language"),
            Some("id-ID,id;q=0.9".to_string())
        );

        let body = body_json(&p[0]);
        assert_eq!(body["chatId"], "chat-1");
        assert_eq!(body["model"], MODEL);
        assert_eq!(body["personaId"], "claude-haiku-4-5-landing");
        assert_eq!(body["frequency_penalty"], 0);
        assert_eq!(body["max_tokens"], 4000);
        assert_eq!(body["presence_penalty"], 0);
        assert_eq!(body["stream"], true);
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["top_p"], 0.95);

        let pesan = body["messages"].as_array().unwrap();
        assert_eq!(pesan.len(), 3);
        // Riwayat dulu, lalu user, lalu sistem.
        assert_eq!(pesan[0]["role"], "user");
        assert_eq!(pesan[0]["content"], "halo");
        assert_eq!(pesan[1]["content"], "hai");
        assert_eq!(pesan[2]["role"], "system");
        assert_eq!(
            pesan[2]["content"],
            "Ikuti bahasa user dan jawab dengan gaya natural, singkat, dan jelas."
        );
        for m in pesan {
            assert!(
                uuid_berpola(m["id"].as_str().unwrap()),
                "id bukan UUID v4: {}",
                m["id"]
            );
        }
    }

    #[test]
    fn id_dibuat_sendiri_kalau_chat_id_dan_device_id_kosong() {
        let http = StubHttp::new(vec![(200, "data: [DONE]\n".to_string())]);
        let hasil = claude_haiku(&http, "hai", &json!({"chatId": "", "deviceId": ""})).unwrap();
        assert_eq!(hasil["status"], true);
        // `responseId` tidak pernah diisi → `null`, bukan kunci hilang.
        assert_eq!(hasil["responseId"], Value::Null);
        let p = http.permintaan.lock().unwrap();
        let device = cari_header(&p[0], "x-device-uuid").unwrap();
        assert!(uuid_berpola(&device), "device id bukan UUID v4: {device}");
        let body = body_json(&p[0]);
        assert!(uuid_berpola(body["chatId"].as_str().unwrap()));
    }

    #[test]
    fn model_dari_sse_menimpa_model_konstanta() {
        let http = StubHttp::new(vec![(
            200,
            "data: {\"model\":\"claude-haiku-4-5-20251001-v2\"}\n".to_string(),
        )]);
        let hasil = claude_haiku(&http, "hai", &json!({})).unwrap();
        assert_eq!(hasil["model"], "claude-haiku-4-5-20251001-v2");
    }

    #[test]
    fn id_dan_model_bola_tidak_menimpa_karena_hanya_string() {
        // `typeof json.id === "string"` — angka diabaikan, `responseId`
        // tetap `null`.
        let http = StubHttp::new(vec![(
            200,
            "data: {\"id\":123,\"model\":456,\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n"
                .to_string(),
        )]);
        let hasil = claude_haiku(&http, "hai", &json!({})).unwrap();
        assert_eq!(hasil["responseId"], Value::Null);
        assert_eq!(hasil["model"], MODEL, "model angka tidak dipakai");
        assert_eq!(hasil["answer"], "ok");
    }

    #[test]
    fn baris_terakhir_tanpa_newline_dan_baris_rusak_dilewati() {
        let http = StubHttp::new(vec![(
            200,
            "data: {rusak\n\
             halo biasa\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"A\"}}]}\n\
             data: {\"choices\":[{\"delta\":{\"content\":\"B\"}}]}"
                .to_string(),
        )]);
        let hasil = claude_haiku(&http, "hai", &json!({})).unwrap();
        assert_eq!(hasil["answer"], "A", "Sisa buffer terakhir dibuang seperti JS");
    }

    #[test]
    fn status_4xx_dikembalikan_sebagai_objek_error_seperti_fetch() {
        let http = StubHttp::new(vec![(401, "kunci salah".to_string())]);
        let hasil = claude_haiku(&http, "hai", &json!({})).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["code"], 401);
        assert_eq!(hasil["error"], "kunci salah");
        assert!(hasil.get("model").is_none(), "JS tidak mengirim model saat gagal");
        assert!(hasil.get("answer").is_none());
    }

    #[test]
    fn history_bukan_array_dianggap_kosong_sesuai_port_lain() {
        // Beda yang diketahui: JS melempar TypeError di sini.
        let http = StubHttp::new(vec![(200, "data: [DONE]\n".to_string())]);
        let hasil = claude_haiku(&http, "hai", &json!({"history": "bukan array"})).unwrap();
        assert_eq!(hasil["status"], true);
        let p = http.permintaan.lock().unwrap();
        let body = body_json(&p[0]);
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    }
}