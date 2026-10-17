//! `src/scraper/feeb.js` — FeelBetterBot (`feelbetterbot.com`), balasan SSE.
//!
//! Yang ikut disalin dari JS:
//! - `fetch` TIDAK melempar untuk status 4xx/5xx; `response.ok` dicek sendiri,
//!   dan kalau tidak ok hasilnya `{ status: false, code, memoryId, error }`.
//! - `parseChunk` mencoba `JSON.parse`; kalau gagal, baris itu dipakai apa
//!   adanya sebagai teks.
//! - JS membaca stream potong demi potong; di sini body dibaca utuh lalu
//!   dipecah per baris. Rakitan akhirnya sama: di JS sisa buffer terakhir juga
//!   diproses (`if (buffer.trim())`), jadi semua potongan ikut diproses.
//! - `options.history` yang bukan array: JS melempar `TypeError`; port ini
//!   memperlakukannya seperti daftar kosong (beda yang diketahui).

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

/// Instruksi sistem (`feeb.js:5-6`), huruf per huruf.
const SISTEM: &str = "Kamu adalah asisten AI yang dibuat oleh Ditzzx. Ikuti bahasa yang digunakan user dalam percakapan. Jika user memakai bahasa Indonesia, jawab dalam bahasa Indonesia yang natural, santai, jelas, dan mudah dipahami. Jangan tiba-tiba pindah bahasa kecuali user memintanya. Jika user bertanya siapa pembuatmu, penciptamu, developermu, atau siapa yang membuatmu, jawab bahwa pembuatmu adalah Ditzzx.";

/// Balasan asisten pembuka (`feeb.js:8-9`), huruf per huruf.
const PEMBUKA: &str = "Hi, I'm FeelBetterBot — I'm here to listen and help you carry whatever feels heavy, without judgment. I draw on gentle, proven ways of working through hard things, but mostly I just want to understand what you're going through. So, how are you doing right now?";

const API: &str = "https://feelbetterbot.com/";

/// `makeMemoryId` (`feeb.js:11-18`) — `kata-hewan-<1000..9999>`.
fn id_memori_acak() -> String {
    const HEWAN: [&str; 8] = ["owl", "fox", "cat", "wolf", "bear", "lion", "deer", "bird"];
    const KATA: [&str; 7] = ["safe", "calm", "soft", "kind", "warm", "bright", "gentle"];
    let kata = KATA[rand::random_range(0..KATA.len())];
    let hewan = HEWAN[rand::random_range(0..HEWAN.len())];
    // `Math.floor(1000 + Math.random() * 9000)`.
    let angka = rand::random_range(1000..10_000);
    format!("{kata}-{hewan}-{angka}")
}

/// `parseChunk` (`feeb.js:20-41`). Balikan kosong = falsy di JS.
fn parse_chunk(baris: &str) -> String {
    let mut data = baris.trim();
    if data.is_empty() || data == "[DONE]" {
        return String::new();
    }
    if let Some(sisa) = data.strip_prefix("data:") {
        data = sisa.trim();
    }
    if data.is_empty() || data == "[DONE]" {
        return String::new();
    }

    match serde_json::from_str::<Value>(data) {
        Ok(json) => {
            if let Some(s) = json.as_str() {
                return s.to_string();
            }
            // Urutan pengecekan kunci harus sama dengan JS.
            for kunci in ["content", "text", "delta", "message", "response", "answer"] {
                if let Some(s) = json.get(kunci).and_then(Value::as_str) {
                    return s.to_string();
                }
            }
            if let Some(s) = json
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(Value::as_str)
            {
                return s.to_string();
            }
            String::new()
        }
        // `catch { return data }`.
        Err(_) => data.to_string(),
    }
}

/// `options.<kunci> || ...` untuk nilai primitif: `""`, `0`, `false`, `null`
/// itu falsy. Nilai truthy di-string-kan seperti template literal JS.
fn opsi_teks(options: &Value, kunci: &str) -> Option<String> {
    let v = options.get(kunci)?;
    if !crate::common::truthy(v) {
        return None;
    }
    Some(teks_js(v))
}

/// `String(v)` untuk nilai JSON. Dipakai nilai yang sudah dipastikan truthy,
/// jadi `null` tidak sampai ke sini.
fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // `[1,2].toString()` = "1,2"; `({}).toString()` = "[object Object]".
        Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
        Value::Null => "null".to_string(),
    }
}

/// `FeelBetter(prompt, options = {})` (`feeb.js:43-109`).
pub fn feel_better(
    http: &dyn HttpGet,
    prompt: &str,
    options: &Value,
) -> Result<Value, HttpError> {
    let memory_id = opsi_teks(options, "memoryId").unwrap_or_else(id_memori_acak);
    let history = options.get("history").and_then(Value::as_array);

    // `messages`: sistem, pembuka, riwayat, lalu pertanyaan user.
    let mut messages: Vec<Value> = vec![
        json!({ "role": "system", "content": SISTEM }),
        json!({ "role": "assistant", "content": PEMBUKA }),
    ];
    if let Some(riwayat) = history {
        for item in riwayat {
            if !item.is_object() {
                continue;
            }
            // `{role: item.role, content: item.content}` — kunci `undefined`
            // tidak ikut `JSON.stringify`, jadi cuma disisipkan kalau ada.
            let mut pesan = Map::new();
            if let Some(v) = item.get("role") {
                pesan.insert("role".to_string(), v.clone());
            }
            if let Some(v) = item.get("content") {
                pesan.insert("content".to_string(), v.clone());
            }
            messages.push(Value::Object(pesan));
        }
    }
    messages.push(json!({ "role": "user", "content": prompt }));

    let body = json!({ "messages": messages });

    let res = http.call(
        &HttpCall::post(API)
            .header("sec-ch-ua-platform", "\"Android\"")
            .header(
                "user-agent",
                "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36",
            )
            .header(
                "sec-ch-ua",
                "\"Google Chrome\";v=\"147\", \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"147\"",
            )
            .header("content-type", "application/json")
            .header("sec-ch-ua-mobile", "?1")
            .header("accept", "*/*")
            .header("origin", "https://feelbetterbot.com")
            .header("referer", "https://feelbetterbot.com/")
            .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("cookie", format!("feelbet-memory={memory_id}"))
            .header("priority", "u=1, i")
            .json(&body)
            // `fetch` tidak melempar untuk 4xx/5xx — status dibaca sendiri.
            .terima_semua_status(),
    )?;

    if !(200..300).contains(&res.status) {
        return Ok(json!({
            "status": false,
            "code": res.status,
            "memoryId": memory_id,
            "error": res.text(),
        }));
    }

    let mut jawaban = String::new();
    for potongan in res.text().split('\n') {
        let chunk = parse_chunk(potongan);
        if !chunk.is_empty() {
            jawaban.push_str(&chunk);
        }
    }

    Ok(json!({
        "status": true,
        "code": res.status,
        "memoryId": memory_id,
        "question": prompt,
        "answer": jawaban,
    }))
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
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
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

    #[test]
    fn merakit_messages_dan_menggabungkan_chunk_sse() {
        let http = StubHttp::new(vec![(
            200,
            "data: {\"content\":\"Halo\"}\n\
             \n\
             data: {\"choices\":[{\"delta\":{\"content\":\" dunia\"}}]}\n\
             data: [DONE]\n"
                .to_string(),
        )]);
        let hasil = feel_better(
            &http,
            "hai",
            &json!({
                "memoryId": "aman-owl-1234",
                "history": [
                    {"role": "user", "content": "halo"},
                    {"role": "assistant", "content": "hai juga"}
                ]
            }),
        )
        .unwrap();

        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["memoryId"], "aman-owl-1234");
        assert_eq!(hasil["question"], "hai");
        assert_eq!(hasil["answer"], "Halo dunia");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://feelbetterbot.com/");
        assert_eq!(
            cari_header(&p[0], "cookie"),
            Some("feelbet-memory=aman-owl-1234".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "content-type"),
            Some("application/json".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "origin"),
            Some("https://feelbetterbot.com".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "referer"),
            Some("https://feelbetterbot.com/".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "sec-ch-ua-platform"),
            Some("\"Android\"".to_string())
        );
        assert_eq!(cari_header(&p[0], "priority"), Some("u=1, i".to_string()));

        let body = body_json(&p[0]);
        let pesan = body["messages"].as_array().unwrap();
        assert_eq!(pesan.len(), 5);
        assert_eq!(pesan[0]["role"], "system");
        assert_eq!(pesan[0]["content"], SISTEM);
        assert_eq!(pesan[1]["role"], "assistant");
        assert_eq!(pesan[1]["content"], PEMBUKA);
        assert_eq!(pesan[2], json!({"role": "user", "content": "halo"}));
        assert_eq!(pesan[3], json!({"role": "assistant", "content": "hai juga"}));
        assert_eq!(pesan[4], json!({"role": "user", "content": "hai"}));
    }

    #[test]
    fn tanpa_memory_id_dipakai_id_acak_berpola_kata_hewan_angka() {
        let http = StubHttp::new(vec![(200, "data: [DONE]\n".to_string())]);
        let hasil = feel_better(&http, "hai", &json!({})).unwrap();
        let id = hasil["memoryId"].as_str().unwrap();
        let bagian: Vec<&str> = id.split('-').collect();
        assert_eq!(bagian.len(), 3, "id tak berpola: {id}");
        assert!(!bagian[0].is_empty() && !bagian[1].is_empty());
        let angka: u32 = bagian[2].parse().unwrap();
        assert!((1000..=9999).contains(&angka), "angka di luar rentang: {id}");
        // Cookie memakai id yang sama dengan hasil.
        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            cari_header(&p[0], "cookie"),
            Some(format!("feelbet-memory={id}"))
        );
    }

    #[test]
    fn status_4xx_dikembalikan_sebagai_objek_error_seperti_fetch() {
        let http = StubHttp::new(vec![(503, "server tumbang".to_string())]);
        let hasil = feel_better(&http, "hai", &json!({"memoryId": "x-1"})).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["code"], 503);
        assert_eq!(hasil["memoryId"], "x-1");
        assert_eq!(hasil["error"], "server tumbang");
        assert!(hasil.get("answer").is_none(), "tidak ada kunci answer");
    }

    #[test]
    fn baris_yang_gagal_di_parse_dipakai_apa_adanya() {
        let http = StubHttp::new(vec![(200, "data: halo mentah\n".to_string())]);
        let hasil = feel_better(&http, "hai", &json!({})).unwrap();
        assert_eq!(hasil["answer"], "halo mentah");
    }

    #[test]
    fn parse_chunk_mengikuti_urutan_kunci_js() {
        assert_eq!(parse_chunk(""), "");
        assert_eq!(parse_chunk("[DONE]"), "");
        assert_eq!(parse_chunk("  [DONE]  "), "");
        assert_eq!(parse_chunk("data:"), "");
        assert_eq!(parse_chunk("data: [DONE]"), "");
        assert_eq!(parse_chunk("data: \"teks\""), "teks");
        assert_eq!(parse_chunk("data: {\"text\":\"a\",\"content\":\"b\"}"), "b");
        assert_eq!(parse_chunk("data: {\"delta\":\"d\"}"), "d");
        assert_eq!(
            parse_chunk("data: {\"choices\":[{\"delta\":{\"content\":\"c\"}}]}"),
            "c"
        );
        assert_eq!(parse_chunk("data: {\"lain\":1}"), "");
        assert_eq!(parse_chunk("data: {\"content\":5}"), "");
    }
}
