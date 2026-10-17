//! `src/scraper/gemini.js` — batchexecute Gemini, cookie disimpan di dalam
//! `sessionId` base64 supaya percakapan bisa dilanjutkan.
//!
//! Yang ikut disalin dari JS:
//! - `gemini(input)` menerima string atau objek; string jadi `{message: input}`.
//! - `if (!message) throw new Error("Message is required.")` — cek falsy, bukan
//!   `== null`.
//! - `JSON.parse(Buffer.from(sessionId, "base64"))` dibungkus `try/catch` yang
//!   hanya `console.error`, jadi session rusak TIDAK melempar: `resumeArray` dan
//!   `cookie` tetap `null` dan `savedInstruction` tetap `instruction` pemanggil.
//! - Cookie diambil dari `headers["set-cookie"][0].split("; ")[0]` — hanya
//!   pasangan pertama, tanpa atribut.
//! - `data.matchAll(/^\d+\n(.+?)\n/gm)` lalu `.reverse()`, dan loop diambil
//!   dari blok TERAKHIR. Kandidat pertama yang punya
//!   `parsed[4][0][1][0]` truthy menang.
//! - `text.replace(/\*\*(.+?)\*\*/g, "*$1*")` — `**tebal**` jadi `*tebal*`.
//! - `newSessionId` adalah base64 `JSON.stringify({resumeArray, cookie,
//!   instruction})` dengan urutan kunci itu; `serde_json` di workspace memakai
//!   `preserve_order` jadi urutan tetap sama.
//!
//! Beda mesin yang perlu diketahui:
//! - axios melempar untuk status di luar 2xx dan `catch` di JS membaca
//!   `error.response.data` untuk mengganti `error.message`. `HttpGet` tidak
//!   memberi body response yang gagal, jadi permintaan memakai
//!   `.terima_semua_status()` dan penggantian pesan itu ditulis ulang sendiri
//!   (`galat_axios`) — teks akhirnya sama dengan axios.
//! - `set-coiki` yang berulang: `HttpResponse.headers` menyimpan satu entri per
//!   header, dan yang dipakai tetap entri pertama.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, HttpResponse};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const BATCHEXECUTE: &str = "https://gemini.google.com/_/BardChatUi/data/batchexecute?rpcids=maGuAc&source-path=%2F&bl=boq_assistant-bard-web-server_20250814.06_p1&f.sid=-7816331052118000090&hl=en-US&_reqid=173780&rt=c";

const STREAM_GENERATE: &str = "https://gemini.google.com/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate?bl=boq_assistant-bard-web-server_20250729.06_p0&f.sid=4206607810970164620&hl=en-US&_reqid=2813378&rt=c";

const TIPE_FORM: &str = "application/x-www-form-urlencoded;charset=UTF-8";

const JSPB: &str = "[1,null,null,null,\"9ec249fc9ad08861\",null,null,null,[4]]";

/// `String(v)` untuk nilai JSON — dipakai `new Error(...)` dan `String(...)`.
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

/// `new URLSearchParams(...).toString()` — `application/x-www-form-urlencoded`:
/// spasi jadi `+`, hanya `*-._` dan alnum yang dibiarkan.
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
            nilai &= (1u32 << bit) - 1;
        }
    }
    out
}

/// `Buffer.from(...).toString("base64")` — abjad standar dengan padding `=`.
fn base64_encode(data: &[u8]) -> String {
    const TABEL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for blok in data.chunks(3) {
        let ambil = |i: usize| u32::from(blok.get(i).copied().unwrap_or(0));
        let n = (ambil(0) << 16) | (ambil(1) << 8) | ambil(2);
        let jumlah = match blok.len() {
            3 => 4,
            2 => 3,
            _ => 2,
        };
        for i in 0..4 {
            if i < jumlah {
                out.push(TABEL[((n >> (18 - i * 6)) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Bentuk `catch` JS: axios melempar `Request failed with status code NNN`,
/// lalu `error.message` digantikan isi `error.response.data` kalau ada.
fn galat_axios(res: &HttpResponse) -> HttpError {
    let bawaan = format!("Request failed with status code {}", res.status);
    let teks = res.text();
    // axios hanya mem-parse body kalau bisa; kalau tidak, `data` tetap string.
    let pesan = match serde_json::from_str::<Value>(&teks) {
        Ok(Value::String(s)) => Some(s),
        Ok(data @ Value::Object(_)) => {
            ["message", "error"]
                .iter()
                .find_map(|k| data.get(*k).filter(|v| truthy(v)))
                .map(teks_js)
        }
        Ok(_) => None,
        Err(_) => Some(teks.clone()),
    };
    match pesan.filter(|p| !p.is_empty()) {
        Some(p) => HttpError { message: p },
        None => HttpError { message: bawaan },
    }
}

/// `resumeArray || ["", "", "", null, null, null, null, null, null, ""]`.
fn resume_bawaan() -> Value {
    json!(["", "", "", null, null, null, null, null, null, ""])
}

/// `chat(input = {})` (`gemini.js:2-169`).
///
/// `input` boleh string atau objek seperti di JS. `Value::Null`, `false`, `0`,
/// dan `""` memakai `{message: input}` — sama seperti `input || {}` yang
/// setelahnya men-|String() oleh plugin.
pub fn chat(http: &dyn HttpGet, input: &Value) -> Result<Value, HttpError> {
    let payload = match input {
        Value::String(s) => json!({ "message": s }),
        lain if !truthy(lain) => json!({ "message": lain }),
        lain => lain.clone(),
    };

    let message = payload.get("message").cloned().unwrap_or(Value::Null);
    let instruction = payload
        .get("instruction")
        .cloned()
        .unwrap_or_else(|| json!(""));
    let session_id = payload.get("sessionId").cloned().unwrap_or(Value::Null);

    if !truthy(&message) {
        return Err(HttpError {
            message: "Message is required.".to_string(),
        });
    }

    let mut resume_array = Value::Null;
    let mut cookie = Value::Null;
    let mut saved_instruction = instruction.clone();

    if truthy(&session_id) {
        // `catch` di JS hanya `console.error`, jadiparsing gagal diam-diam.
        if let Some(kode) = session_id.as_str() {
            let isi = String::from_utf8_lossy(&base64_deskep(kode)).to_string();
            if let Ok(session_data) = serde_json::from_str::<Value>(&isi) {
                resume_array = session_data.get("resumeArray").cloned().unwrap_or(Value::Null);
                cookie = session_data.get("cookie").cloned().unwrap_or(Value::Null);
                saved_instruction = if truthy(&instruction) {
                    instruction.clone()
                } else if session_data
                    .get("instruction")
                    .map(truthy)
                    .unwrap_or(false)
                {
                    session_data.get("instruction").cloned().unwrap_or(Value::Null)
                } else {
                    json!("")
                };
            }
        }
    }

    if !truthy(&cookie) {
        let res = http.call(
            &HttpCall::post(BATCHEXECUTE)
                .header("content-type", TIPE_FORM)
                .form("f.req=%5B%5B%5B%22maGuAc%22%2C%22%5B0%5D%22%2Cnull%2C%22generic%22%5D%5D%5D&")
                .terima_semua_status(),
        )?;
        if !(200..300).contains(&res.status) {
            return Err(galat_axios(&res));
        }
        cookie = json!(cookie_pertama(&res).unwrap_or_default());
    }
    let cookie = teks_js(&cookie);

    let request_body = json!([
        [message, 0, null, null, null, null, 0],
        ["en-US"],
        if truthy(&resume_array) { resume_array } else { resume_bawaan() },
        null,
        null,
        null,
        [1],
        1,
        null,
        null,
        1,
        0,
        null,
        null,
        null,
        null,
        null,
        null,
        [[0]],
        1,
        null,
        null,
        null,
        null,
        null,
        null,
        ["", "", saved_instruction, null, null, null, null, null, 0, null, 1, null, null, null, []],
        null,
        null,
        1,
        null,
        null,
        null,
        null,
        null,
        null,
        null,
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        1,
        null,
        null,
        null,
        null,
        [1]
    ]);
    let payload = json!([null, request_body.to_string()]);

    let res = http.call(
        &HttpCall::post(STREAM_GENERATE)
            .header("content-type", TIPE_FORM)
            .header("x-goog-ext-525001261-jspb", JSPB)
            .header("cookie", cookie.clone())
            .form(format!("f.req={}", encode_form(&payload.to_string())))
            .terima_semua_status(),
    )?;
    if !(200..300).contains(&res.status) {
        return Err(galat_axios(&res));
    }

    let mut parse1: Option<Value> = None;
    let re = regex::Regex::new(r"(?m)^\d+\n(.+?)\n").expect("pola tetap valid");
    // `matchAll(...)` lalu `.reverse()`: blok terakhir lebih dulu.
    let blok: Vec<String> = re
        .captures_iter(&res.text())
        .map(|c| c[1].to_string())
        .collect();
    for selected in blok.iter().rev() {
        let Ok(real_array) = serde_json::from_str::<Value>(selected) else {
            continue;
        };
        let Some(candidate) = real_array
            .get(0)
            .and_then(|v| v.get(2))
            .filter(|v| truthy(v))
        else {
            continue;
        };
        let Ok(candidate) = serde_json::from_str::<Value>(&teks_js(candidate)) else {
            continue;
        };
        let ada_teks = candidate
            .get(4)
            .and_then(|v| v.get(0))
            .and_then(|v| v.get(1))
            .and_then(|v| v.get(0))
            .map(truthy)
            .unwrap_or(false);
        if ada_teks {
            parse1 = Some(candidate);
            break;
        }
    }

    let Some(parse1) = parse1 else {
        return Err(HttpError {
            message: "Gagal mem-parsing response Gemini.".to_string(),
        });
    };

    let teks_awal = parse1
        .get(4)
        .and_then(|v| v.get(0))
        .and_then(|v| v.get(1))
        .and_then(|v| v.get(0));
    let Some(teks_awal) = teks_awal.and_then(Value::as_str) else {
        return Err(HttpError {
            message: "Gagal mem-parsing response Gemini.".to_string(),
        });
    };
    let re_tebal = regex::Regex::new(r"\*\*(.+?)\*\*").expect("pola tetap valid");
    let text = re_tebal.replace_all(teks_awal, "*$1*").to_string();

    // `[...parse1[1], parse1[4][0][0]]` — `parse1[1]` harus bisa di-spread.
    let mut new_resume: Vec<Value> = match parse1.get(1).and_then(Value::as_array) {
        Some(a) => a.clone(),
        None => {
            return Err(HttpError {
                message: "parse1[1] is not iterable".to_string(),
            });
        }
    };
    new_resume.push(
        parse1
            .get(4)
            .and_then(|v| v.get(0))
            .and_then(|v| v.get(0))
            .cloned()
            .unwrap_or(Value::Null),
    );

    let mut session = Map::new();
    session.insert("resumeArray".to_string(), Value::Array(new_resume));
    session.insert("cookie".to_string(), json!(cookie));
    session.insert("instruction".to_string(), saved_instruction);
    let new_session_id = base64_encode(Value::Object(session).to_string().as_bytes());

    Ok(json!({
        "text": text,
        "sessionId": new_session_id,
    }))
}

/// `headers["set-cookie"]?.[0]?.split("; ")[0]` — pasangan pertama saja.
fn cookie_pertama(res: &HttpResponse) -> Option<String> {
    let (_, nilai) = res
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))?;
    nilai
        .split("; ")
        .next()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// `(status, body, headers)` untuk stub HTTP — alias biar tipe
    /// balancer-nya tidak memicu `clippy::type_complexity`.
    type Balasan = (u16, String, Vec<(String, String)>);

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body, headers) berurutan; balasan terakhir dipakai lagi.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<Balasan>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: &[(u16, &str)]) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(
                    balasan
                        .iter()
                        .map(|(s, t)| (*s, t.to_string(), Vec::new()))
                        .collect(),
                ),
                hitung: Mutex::new(0),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("gemini cuma POST")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("gemini lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("gemini lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("gemini tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, body, headers) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers,
            })
        }
    }

    fn cari_header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn body_form(p: &HttpCall) -> String {
        match &p.body {
            Some(oktz_plugin_api::HttpBody::Form(s)) => s.clone(),
            lain => panic!("body bukan form: {lain:?}"),
        }
    }

    /// Balasan `batchexecute` membawa cookie lewat header response.
    fn stub_dengan_cookie(ada_cookie: bool, stream: &str) -> StubHttp {
        let http = StubHttp::baru(&[(200, stream)]);
        if ada_cookie {
            *http.balasan.lock().unwrap() = vec![
                (
                    200,
                    String::new(),
                    vec![(
                        "set-cookie".to_string(),
                        "__Secure-ENID=abc; Secure; SameSite=None".to_string(),
                    )],
                ),
                (200, stream.to_string(), Vec::new()),
            ];
        } else {
            *http.balasan.lock().unwrap() = vec![(200, stream.to_string(), Vec::new())];
        }
        http
    }

    /// Satu blok `batchexecute`: `<angka>\n<JSON>\n`, dengan
    /// `realArray[0][2]` = JSON string yang memuat `parsed[4][0][1][0]`.
    fn blok(jawaban: &str) -> String {
        let parsed = json!([
            "wrb.fr",
            ["id_lama"],
            null,
            null,
            [["id_baru", [jawaban]]]
        ]);
        format!(
            "5\n{}\n",
            json!([["wrb.fr", null, parsed.to_string()]])
        )
    }

    /// Kebalikan `encode_form` (`+` = spasi, `%XX` = byte).
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
    fn mengambil_cookie_lalu_mengirim_permintaan_lengkap() {
        let http = stub_dengan_cookie(true, &blok("Halo"));
        let hasil = chat(&http, &json!({ "message": "hai", "instruction": "Singkat." })).unwrap();
        assert_eq!(hasil["text"], "Halo");
        let session_id = hasil["sessionId"].as_str().unwrap().to_string();

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2, "batchexecute dulu, baru StreamGenerate");
        assert_eq!(p[0].url, BATCHEXECUTE);
        assert_eq!(
            body_form(&p[0]),
            "f.req=%5B%5B%5B%22maGuAc%22%2C%22%5B0%5D%22%2Cnull%2C%22generic%22%5D%5D%5D&"
        );
        assert_eq!(cari_header(&p[0], "content-type"), Some(TIPE_FORM.to_string()));

        assert_eq!(p[1].url, STREAM_GENERATE);
        assert_eq!(cari_header(&p[1], "cookie"), Some("__Secure-ENID=abc".to_string()));
        assert_eq!(cari_header(&p[1], "x-goog-ext-525001261-jspb"), Some(JSPB.to_string()));

        // Body StreamGenerate = `f.req=` + `URLSearchParams` (form-urlencoded:
        // spasi jadi `+`, `!` jadi `%21`).
        let body = body_form(&p[1]);
        let terkuti = body.strip_prefix("f.req=").expect("awalan f.req=");
        let payload: Value =
            serde_json::from_str(&decode_form(terkuti)).expect("f.req harus JSON");
        assert_eq!(payload[0], Value::Null, "payload[0] = null");
        let request_body: Value =
            serde_json::from_str(payload[1].as_str().expect("payload[1] string")).unwrap();
        assert_eq!(request_body[0][0], "hai");
        assert_eq!(request_body[1][0], "en-US");
        assert_eq!(request_body[2][0], "", "resumeArray bawaan saat session kosong");
        assert_eq!(request_body[18][0][0], 0, "[[0]] pada indeks 18");
        assert_eq!(
            request_body[26][2], "Singkat.",
            "blok instruction pada indeks 26"
        );
        assert_eq!(
            request_body[37].as_array().unwrap().len(),
            20,
            "[1..20] pada indeks 37"
        );
        assert_eq!(request_body[43], json!([1]), "ekor array adalah [1]");
        assert_eq!(request_body.as_array().unwrap().len(), 44, "44 elemen seperti JS");
        assert!(body.contains("f.req=%5Bnull%2C"), "quote di-encode: {body}");

        // `sessionId` = base64 `{resumeArray, cookie, instruction}` urut.
        let isi = String::from_utf8(base64_deskep(&session_id)).unwrap();
        let session = serde_json::from_str::<Value>(&isi).unwrap();
        assert_eq!(
            session.as_object().unwrap().keys().cloned().collect::<Vec<_>>(),
            vec![
                "resumeArray".to_string(),
                "cookie".to_string(),
                "instruction".to_string()
            ],
            "urutan kunci harus seperti JSON.stringify di JS"
        );
        assert_eq!(session["cookie"], "__Secure-ENID=abc");
        assert_eq!(session["instruction"], "Singkat.");
        assert_eq!(
            session["resumeArray"],
            json!(["id_lama", "id_baru"]),
            "resumeArray = [...parse1[1], parse1[4][0][0]]"
        );
    }

    #[test]
    fn session_id_dipakai_ulang_tanpa_permintaan_cookie() {
        // Hanya satu permintaan terjadi (cookie dari session), jadi stub cukup satu
        // balasan.
        let http = stub_dengan_cookie(false, &blok("Lanjut"));
        // Sesi dari balasan sebelumnya, dibuat manual supaya tidak bergantung
        // pada base64 dari request lain.
        let sesi = json!({
            "resumeArray": ["id_lama", "Halo"],
            "cookie": "SID=xyz",
            "instruction": "Awal",
        });
        let session_id = base64_encode(sesi.to_string().as_bytes());

        let hasil = chat(
            &http,
            &json!({ "message": "lanjut", "sessionId": session_id }),
        )
        .unwrap();
        assert_eq!(hasil["text"], "Lanjut");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1, "cookie dari session dipakai, tidak diambil lagi");
        assert_eq!(p[0].url, STREAM_GENERATE);
        assert_eq!(cari_header(&p[0], "cookie"), Some("SID=xyz".to_string()));
    }

    #[test]
    fn session_rusak_diabaikan_seperti_console_error() {
        let http = stub_dengan_cookie(false, &blok("Aman"));
        let hasil = chat(
            &http,
            &json!({ "message": "hai", "sessionId": "bukan-base64-json" }),
        )
        .unwrap();
        assert_eq!(hasil["text"], "Aman");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2, "cookie baru diambil karena session gagal di-parse");
    }

    #[test]
    fn instruction_dari_session_dipakai_saat_pemanggil_kosong() {
        let http = stub_dengan_cookie(false, &blok("Oke"));
        let sesi = json!({"cookie": "SID=1", "instruction": "Dari sesi"});
        let session_id = base64_encode(sesi.to_string().as_bytes());
        let hasil = chat(
            &http,
            &json!({"message": "hai", "instruction": "", "sessionId": session_id}),
        )
        .unwrap();
        let isi = String::from_utf8(base64_deskep(hasil["sessionId"].as_str().unwrap())).unwrap();
        let session = serde_json::from_str::<Value>(&isi).unwrap();
        assert_eq!(session["instruction"], "Dari sesi");
    }

    #[test]
    fn pesan_kosong_melempar_teks_js() {
        let http = stub_dengan_cookie(false, &blok("x"));
        let salah = chat(&http, &json!({"message": ""})).unwrap_err();
        assert_eq!(salah.message, "Message is required.");
        assert!(http.permintaan.lock().unwrap().is_empty(), "tidak ada request");

        let salah = chat(&http, &Value::Null).unwrap_err();
        assert_eq!(salah.message, "Message is required.");
    }

    #[test]
    fn input_string_dipakai_sebagai_message() {
        let http = stub_dengan_cookie(false, &blok("Halo"));
        let hasil = chat(&http, &json!("hai langsung")).unwrap();
        assert_eq!(hasil["text"], "Halo");
    }

    #[test]
    fn blok_terakhir_yang_valid_yang_dipakai() {
        // Dua blok; blok pertama isinya rusak, blok kedua jawaban.
        let rusak = format!("3\n{}\n", "{bukan json");
        let streams = format!("{rusak}{}", blok("Akhir"));
        let http = stub_dengan_cookie(false, &streams);
        let hasil = chat(&http, &json!({"message": "hai"})).unwrap();
        assert_eq!(hasil["text"], "Akhir");
    }

    #[test]
    fn respons_tanpa_blok_valid_melempar_pesan_js() {
        let http = stub_dengan_cookie(false, "5\nbukan json\n");
        let salah = chat(&http, &json!({"message": "hai"})).unwrap_err();
        assert_eq!(salah.message, "Gagal mem-parsing response Gemini.");
    }

    #[test]
    fn bintang_ganda_diubah_jadi_tunggal_satu() {
        let http = stub_dengan_cookie(false, &blok("**tebal** dan **miring**"));
        let hasil = chat(&http, &json!({"message": "hai"})).unwrap();
        assert_eq!(hasil["text"], "*tebal* dan *miring*");
    }

    #[test]
    fn galat_http_memakai_pesan_dari_body_seperti_axios() {
        let http = StubHttp::baru(&[(403, r#"{"error":"kuota habis"}"#)]);
        *http.balasan.lock().unwrap() = vec![
            (
                200,
                String::new(),
                vec![("set-cookie".to_string(), "SID=1".to_string())],
            ),
            (403, r#"{"error":"kuota habis"}"#.to_string(), Vec::new()),
        ];
        let salah = chat(&http, &json!({"message": "hai"})).unwrap_err();
        assert_eq!(salah.message, "kuota habis");

        // Body JSON tanpa `error`/`message` -> pesan bawaan axios.
        let http = StubHttp::baru(&[]);
        *http.balasan.lock().unwrap() = vec![
            (
                200,
                String::new(),
                vec![("set-cookie".to_string(), "SID=1".to_string())],
            ),
            (500, r#"{"detail":"x"}"#.to_string(), Vec::new()),
        ];
        let salah = chat(&http, &json!({"message": "hai"})).unwrap_err();
        assert_eq!(salah.message, "Request failed with status code 500");

        // Body teks biasa dipakai apa adanya.
        let http = StubHttp::baru(&[]);
        *http.balasan.lock().unwrap() = vec![
            (
                200,
                String::new(),
                vec![("set-cookie".to_string(), "SID=1".to_string())],
            ),
            (429, "terlalu cepat".to_string(), Vec::new()),
        ];
        let salah = chat(&http, &json!({"message": "hai"})).unwrap_err();
        assert_eq!(salah.message, "terlalu cepat");
    }

    #[test]
    fn cookie_tidak_ada_dipakai_sebagai_kosong() {
        let http = stub_dengan_cookie(false, &blok("Tetap jalan"));
        let hasil = chat(&http, &json!({"message": "hai"})).unwrap();
        assert_eq!(hasil["text"], "Tetap jalan");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(cari_header(&p[1], "cookie"), Some(String::new()));
    }
}