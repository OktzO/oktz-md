//! `src/scraper/deepseek.js` — notegpt.io, balasan SSE (`/api/v2/chat/stream`).
//!
//! Yang ikut disalin dari JS:
//! - `validateStatus: () => true` → `.terima_semua_status()`: status 4xx/5xx
//!   TIDAK jadi error, body-nya tetap dibaca dan `status` ikut dikembalikan di
//!   hasil. Di JS `success` biasanya `false` untuk respons seperti itu.
//! - `parseSSE` memecah SELURUH body per baris (`rawBody.split(/\r?\n/)`) —
//!   tidak ada `lines.pop()` seperti `qwen3.js`/`feeb.js`, jadi potongan
//!   terakhir yang tidak diakhiri newline tetap ikut dibaca.
//! - `raw` hanya ikut kalau `answer` DAN `reasoning` kosong (`undefined`
//!   tidak ada di `JSON.stringify`).
//! - `history_messages`: `history.slice(-5)`, lalu tiap item jadi dua pesan
//!   (user lalu assistant). Kunci `item.user`/`item.assistant` yang `undefined`
//!   tidak ikut.
//!
//! Beda yang diketahui:
//! - `timeout: 60000` milik axios tidak ditiru (batas mesin, bukan tampilan);
//!   `UreqHttp` memakai batas global 20 detik. Untuk SSE yang lambat ini bisa
//!   jadi bedanya.
//! - Header `Accept-Encoding: gzip, deflate, br, zstd` TIDAK dikirim.
//!   `ureq` yang mengurus pemampatan sesuai namanya sendiri; menyebut header
//!   itu lewat `HttpCall` berisiko_body terkompresi tidak terurai.
//! - `res.data.on("error")` (stream putus) tidak ada padanannya: di sini body
//!   dibaca utuh, jadi kegagalan stream jadi `Err` dari `http.call`.
//! - `history` yang bukan array: JS melempar `TypeError` dari `.slice`; di sini
//!   diperlakukan sebagai daftar kosong (beda yang diketahui).

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, truthy};

const BASE: &str = "https://notegpt.io";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// `crypto.randomUUID()` (`deepseek.js:9-11`).
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

/// `Math.floor(Date.now() / 1000)` (`deepseek.js:20,26`).
fn detik_sekarang() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `randomNumber(length)` (`deepseek.js:13-17`) — `length` digit acak 0..9,
/// bukan angka acak sepanjang `length`.
fn angka_acak(jumlah: usize) -> String {
    (0..jumlah)
        .map(|_| char::from(b'0' + rand::random_range(0..10)))
        .collect()
}

/// `Buffer.from(x).toString("base64")` (`deepseek.js:23`) — tidak ada crate
/// base64 di `Cargo.toml`, jadi tabel RFC 4648 ditulis di sini.
fn base64(bytes: &[u8]) -> String {
    const TABEL: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for blok in bytes.chunks(3) {
        let b0 = blok[0] as u32;
        let b1 = *blok.get(1).unwrap_or(&0) as u32;
        let b2 = *blok.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABEL[(n >> 18 & 63) as usize] as char);
        out.push(TABEL[(n >> 12 & 63) as usize] as char);
        // Satu byte terakhir tidak punya dua digit basis-64 yang sah.
        out.push(if blok.len() > 1 {
            TABEL[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if blok.len() > 2 {
            TABEL[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// `makeSboxGuid()` (`deepseek.js:19-23`).
fn sbox_guid() -> String {
    let raw = format!("{}|762|{}", detik_sekarang(), angka_acak(9));
    base64(raw.as_bytes())
}

/// `makeCookieHeader()` (`deepseek.js:25-37`) — enam cookie digabung `"; "`.
fn cookie_header() -> String {
    let now = detik_sekarang();
    [
        format!("_ga_PFX3BRW5RQ=GS2.1.s{now}$o1$g0$t{now}$j60$l0$h{}", angka_acak(9)),
        format!("_ga=GA1.2.{}.{now}", angka_acak(9)),
        format!("_gid=GA1.2.{}.{now}", angka_acak(9)),
        "_gat_gtag_UA_252982427_14=1".to_string(),
        format!("sbox-guid={}", encode_uri_component(&sbox_guid())),
        format!("anonymous_user_id={}", uuid_v4()),
    ]
    .join("; ")
}

/// `toHistoryMessages(history)` (`deepseek.js:39-50`) — `history.slice(-5)`
/// lalu tiap item jadi dua pesan.
fn ke_pesan_history(history: &[Value]) -> Vec<Value> {
    let mulai = history.len().saturating_sub(5);
    let mut out = Vec::new();
    for item in &history[mulai..] {
        for kunci in ["user", "assistant"] {
            // `{role, content}` — `item.user` yang `undefined` tidak ikut.
            let mut pesan = Map::new();
            pesan.insert("role".to_string(), Value::String(kunci.to_string()));
            if let Some(v) = item.get(kunci) {
                pesan.insert("content".to_string(), v.clone());
            }
            out.push(Value::Object(pesan));
        }
    }
    out
}

/// `String(v)` untuk nilai yang sudah dipastikan truthy (`if (json.text)`),
/// jadi hanya string/angka/boolean yang mungkin muncul.
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

/// `parseSSE(rawBody)` (`deepseek.js:52-76`) — `(answer, reasoning)`.
fn parse_sse(raw: &str) -> (String, String) {
    let mut answer = String::new();
    let mut reasoning = String::new();
    for baris in raw.split('\n') {
        let clean = baris.trim();
        let Some(data) = clean.strip_prefix("data:") else {
            continue;
        };
        // `clean.replace(/^data:\s*/, "").trim()`.
        let data = data.trim_start().trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        // `catch {}` di JS: baris yang bukan JSON dilewati diam-diam.
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if let Some(r) = v.get("reasoning").filter(|r| truthy(r)) {
            reasoning.push_str(&teks_js(r));
        }
        if let Some(t) = v.get("text").filter(|t| truthy(t)) {
            answer.push_str(&teks_js(t));
        }
        // `if (json.done) break` — `false`/`null`/`0` tidak menghentikan.
        if v.get("done").map(truthy).unwrap_or(false) {
            break;
        }
    }
    (answer, reasoning)
}

/// `DeepSeekThinking(prompt, history = [])` (`deepseek.js:78-161`).
pub fn deepseek_thinking(
    http: &dyn HttpGet,
    prompt: &str,
    history: &[Value],
) -> Result<Value, HttpError> {
    let conversation_id = uuid_v4();
    let payload = json!({
        "message": prompt,
        "language": "auto",
        "model": "deepseek-v4-flash",
        "tone": "default",
        "length": "moderate",
        "conversation_id": conversation_id,
        "image_urls": [],
        "history_messages": ke_pesan_history(history),
        "chat_mode": "deep_think",
    });

    let res = http.call(
        &HttpCall::post(format!("{BASE}/api/v2/chat/stream"))
            .header("sec-ch-ua-platform", "\"Android\"")
            .header("user-agent", UA)
            .header(
                "sec-ch-ua",
                "\"Google Chrome\";v=\"147\", \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"147\"",
            )
            .header("content-type", "application/json")
            .header("sec-ch-ua-mobile", "?1")
            .header("accept", "*/*")
            .header("origin", BASE)
            .header("sec-fetch-site", "same-origin")
            .header("sec-fetch-mode", "cors")
            .header("sec-fetch-dest", "empty")
            .header("referer", format!("{BASE}/chat-deepseek"))
            .header("accept-language", "id-ID,id;q=0.9")
            .header("cookie", cookie_header())
            .header("priority", "u=1, i")
            .json(&payload)
            // `validateStatus: () => true`.
            .terima_semua_status(),
    )?;

    let raw_body = res.text();
    let (answer, reasoning) = parse_sse(&raw_body);
    // `raw` hanya ikut kalau `answer` DAN `reasoning` kosong — dicek sebelum
    // kedua string dipindahkan ke hasil.
    let kosong = answer.is_empty() && reasoning.is_empty();

    let mut out = Map::new();
    out.insert("status".to_string(), Value::from(res.status));
    out.insert(
        "success".to_string(),
        Value::Bool(!kosong),
    );
    out.insert("conversation_id".to_string(), Value::String(conversation_id));
    out.insert("model".to_string(), Value::String("deepseek-v4-flash".into()));
    out.insert("chat_mode".to_string(), Value::String("deep_think".into()));
    out.insert("prompt".to_string(), Value::String(prompt.to_string()));
    out.insert("answer".to_string(), Value::String(answer));
    out.insert("reasoning".to_string(), Value::String(reasoning));
    out.insert(
        "history_used".to_string(),
        // `Math.min(history.length, 5)` — jumlah riwayatnya, bukan jumlah pesan.
        Value::from(std::cmp::min(history.len(), 5)),
    );
    // `raw: answer || reasoning ? undefined : rawBody`.
    if kosong {
        out.insert("raw".to_string(), Value::String(raw_body));
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan, membalas status +
    /// body yang ditentukan test.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<(u16, String)>) -> Self {
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
                    mimetype: Some("text/event-stream".into()),
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(status: u16, body: &str) -> StubHttp {
        StubHttp::baru(vec![(status, body.to_string())])
    }

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn body_json(p: &oktz_plugin_api::HttpCall) -> Value {
        match &p.body {
            Some(oktz_plugin_api::HttpBody::Bytes { data, .. }) => {
                serde_json::from_slice(data).expect("body JSON")
            }
            lain => panic!("body bukan JSON: {lain:?}"),
        }
    }

    #[test]
    fn base64_sama_dengan_buffer_node() {
        // `Buffer.from(x).toString("base64")` — termasuk Padding `=` dan
        // panjang yang bukan kelipatan 3.
        assert_eq!(
            base64(b"1750000000|762|123456789"),
            "MTc1MDAwMDAwMHw3NjJ8MTIzNDU2Nzg5"
        );
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn cookie_punya_enam_bagian_dengan_pola_yg_diperapkan() {
        let c = cookie_header();
        let bagian: Vec<&str> = c.split("; ").collect();
        assert_eq!(bagian.len(), 6, "cookie: {c}");
        assert!(bagian[0].starts_with("_ga_PFX3BRW5RQ=GS2.1.s"), "{}", bagian[0]);
        assert!(bagian[0].contains("$o1$g0$t"), "{}", bagian[0]);
        assert!(bagian[1].starts_with("_ga=GA1.2."), "{}", bagian[1]);
        assert!(bagian[2].starts_with("_gid=GA1.2."), "{}", bagian[2]);
        assert_eq!(bagian[3], "_gat_gtag_UA_252982427_14=1");
        // `sbox-guid` = base64 lalu `encodeURIComponent`; `=` jadi `%3D`,
        // `|` (kalau ada) jadi `%7C`.
        let sbox = bagian[4].trim_start_matches("sbox-guid=");
        assert!(!sbox.contains('='), "padding harus ter-escape: {sbox}");
        assert!(!sbox.contains('|'), "garis tegak harus ter-escape: {sbox}");
        assert!(bagian[5].starts_with("anonymous_user_id="), "{}", bagian[5]);
        assert_eq!(bagian[5].trim_start_matches("anonymous_user_id=").len(), 36);
    }

    #[test]
    fn parse_sse_mengumpulkan_reasoning_dan_text_lalu_berhenti_saat_done() {
        let (answer, reasoning) = parse_sse(
            "data: {\"reasoning\":\"mikir \"}\n\
             \n\
             : komentar diabaikan\n\
             data: {\"text\":\"jawab \"}\n\
             data: {\"text\":\"dua\"}\n\
             data: {\"done\":true}\n\
             data: {\"text\":\"tidak dibaca\"}\n\
             data: [DONE]\n",
        );
        assert_eq!(answer, "jawab dua");
        assert_eq!(reasoning, "mikir ");
        // Potongan terakhir tanpa newline tetap ikut (tidak ada `lines.pop()`).
        assert_eq!(parse_sse("data: {\"text\":\"akhir\"}").0, "akhir");
        // `done:false` tidak menghentikan, baris rusak dilewati.
        assert_eq!(
            parse_sse("data: {\"text\":\"a\",\"done\":false}\ndata: {\"text\":\"b\"}\n").0,
            "ab"
        );
        assert_eq!(parse_sse("data: bukan json\ndata: {\"text\":\"x\"}\n").0, "x");
        // `text` non-string tapi truthy tetap disambung lewat `ToString`.
        assert_eq!(parse_sse("data: {\"text\":5}\n").0, "5");
        assert_eq!(parse_sse("data: {\"text\":\"\"}\n").0, "");
    }

    #[test]
    fn alur_sukses_mengembalikan_status_hasil_dan_body_sse() {
        let http = stub(
            200,
            "data: {\"reasoning\":\"langkah \"}\n\
             data: {\"text\":\"Jawaban\"}\n\
             data: [DONE]\n",
        );
        let hasil = deepseek_thinking(&http, "Jelaskan black hole", &[]).unwrap();
        assert_eq!(hasil["status"], 200);
        assert_eq!(hasil["success"], true);
        assert_eq!(hasil["model"], "deepseek-v4-flash");
        assert_eq!(hasil["chat_mode"], "deep_think");
        assert_eq!(hasil["prompt"], "Jelaskan black hole");
        assert_eq!(hasil["answer"], "Jawaban");
        assert_eq!(hasil["reasoning"], "langkah ");
        assert_eq!(hasil["history_used"], 0);
        assert!(hasil.get("raw").is_none(), "raw tidak ikut saat ada jawaban");
        assert_eq!(
            hasil["conversation_id"].as_str().unwrap().len(),
            36,
            "conversation_id harus UUID"
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://notegpt.io/api/v2/chat/stream");
        assert!(p[0].terima_semua_status, "validateStatus: () => true");
        assert_eq!(cari_header(&p[0], "content-type").as_deref(), Some("application/json"));
        assert_eq!(cari_header(&p[0], "origin").as_deref(), Some("https://notegpt.io"));
        assert_eq!(
            cari_header(&p[0], "referer").as_deref(),
            Some("https://notegpt.io/chat-deepseek")
        );
        assert_eq!(cari_header(&p[0], "priority").as_deref(), Some("u=1, i"));
        assert_eq!(
            cari_header(&p[0], "sec-ch-ua-platform").as_deref(),
            Some("\"Android\"")
        );
        assert_eq!(
            cari_header(&p[0], "sec-fetch-dest").as_deref(),
            Some("empty")
        );
        assert_eq!(cari_header(&p[0], "accept-language").as_deref(), Some("id-ID,id;q=0.9"));
        // `Accept-Encoding` sengaja tidak ikut: ureq yang mengurus pemampatan.
        assert!(cari_header(&p[0], "accept-encoding").is_none());
        let cookie = cari_header(&p[0], "cookie").expect("cookie wajib ada");
        assert!(cookie.contains("anonymous_user_id="), "{cookie}");

        let body = body_json(&p[0]);
        assert_eq!(body["message"], "Jelaskan black hole");
        assert_eq!(body["language"], "auto");
        assert_eq!(body["tone"], "default");
        assert_eq!(body["length"], "moderate");
        assert_eq!(body["chat_mode"], "deep_think");
        assert_eq!(body["image_urls"].as_array().unwrap().len(), 0);
        assert_eq!(body["history_messages"].as_array().unwrap().len(), 0);
        assert_eq!(
            body["conversation_id"],
            Value::String(hasil["conversation_id"].as_str().unwrap().to_string())
        );
    }

    #[test]
    fn history_dipotong_lima_item_terakhir_dan_jadi_dua_pesan() {
        let http = stub(200, "data: {\"text\":\"ok\"}\n");
        let history: Vec<Value> = (0..8)
            .map(|i| json!({"user": format!("u{i}"), "assistant": format!("a{i}")}))
            .collect();
        let hasil = deepseek_thinking(&http, "p", &history).unwrap();
        assert_eq!(hasil["history_used"], 5, "Math.min(8, 5)");

        let p = http.permintaan.lock().unwrap();
        let pesan = body_json(&p[0])["history_messages"].clone();
        let pesan = pesan.as_array().unwrap();
        assert_eq!(pesan.len(), 10, "lima item terakhir, dua pesan tiap item");
        assert_eq!(pesan[0], json!({"role": "user", "content": "u3"}));
        assert_eq!(pesan[1], json!({"role": "assistant", "content": "a3"}));
        assert_eq!(pesan[8], json!({"role": "user", "content": "u7"}));
        assert_eq!(pesan[9], json!({"role": "assistant", "content": "a7"}));
    }

    #[test]
    fn item_history_tanpa_kunci_menghasilkan_pesan_tanpa_content() {
        let http = stub(200, "data: {\"text\":\"ok\"}\n");
        deepseek_thinking(&http, "p", &[json!({}), json!({"user": "u"})]).unwrap();
        let p = http.permintaan.lock().unwrap();
        let pesan = body_json(&p[0])["history_messages"].clone();
        let pesan = pesan.as_array().unwrap();
        assert_eq!(pesan[0], json!({"role": "user"}), "content undefined hilang");
        assert_eq!(pesan[1], json!({"role": "assistant"}));
        assert_eq!(pesan[2], json!({"role": "user", "content": "u"}));
    }

    #[test]
    fn body_sse_kosong_mengembalikan_raw_dan_success_false() {
        let http = stub(200, "data: [DONE]\n");
        let hasil = deepseek_thinking(&http, "p", &[]).unwrap();
        assert_eq!(hasil["success"], false);
        assert_eq!(hasil["answer"], "");
        assert_eq!(hasil["reasoning"], "");
        assert_eq!(hasil["raw"], "data: [DONE]\n");
    }

    #[test]
    fn status_4xx_tetap_dibaca_karena_validate_status_true() {
        // Tanpa `.terima_semua_status()` permintaan ini jadi `Err`.
        let http = stub(403, "{\"error\":\"ditolak\"}");
        let hasil = deepseek_thinking(&http, "p", &[]).unwrap();
        assert_eq!(hasil["status"], 403);
        assert_eq!(hasil["success"], false);
        assert_eq!(hasil["raw"], "{\"error\":\"ditolak\"}");
    }
}