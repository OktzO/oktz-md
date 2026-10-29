//! `src/scraper/txt2img2.js` — text to image lewat Gradio (Flux Klein 4B).
//!
//! Tiga langkah, semuanya ikut disalin apa adanya dari JS:
//! 1. `checkPrompt` — POST ke server action `upsampler.com`; kegagalan apa pun
//!    di dalamnya ditelan `catch` dan dibaca sebagai "tidak flagged, tidak rate
//!    limited" (`txt2img2.js:36-38`), jadi request yang gagal tidak menghentikan
//!    scraper.
//! 2. `join` — POST body JSON ke `gradio_api/queue/join?`, ambil `event_id`.
//! 3. `getResult` — GET stream SSE `gradio_api/queue/data`, blok demi blok
//!    sampai `process_completed` memberi URL.
//!
//! `Txt2Img2` sendiri tidak pernah melempar: `catch` terakhir mengubah semua
//! kegagalan jadi objek `{ status: false, code, prompt, error }`. Karena itu
//! fungsi publik ini selalu `Ok` — `Err` tidak pernah terjadi, sama seperti
//! `return` JS yang tidak pernah menolak.
//!
//! Beda yang diketahui:
//! - `responseType: "stream"` tidak ada padanannya di `HttpGet`: SSE dibaca
//!   utuh setelah respons selesai, bukan sedikit demi sedikit. Hasil yang
//!   dipakai tetap sama; yang hilang hanya Kemampuan berhenti di event pertama
//!   (`res.data.destroy()`), dan `setTimeout` 180 detik.
//! - `timeout: 30000/180000` milik axios tidak ditiru — batas mesin, bukan
//!   tampilan; `UreqHttp` punya batas global sendiri.
//! - `e.response?.status` pada cabang `catch` tidak selalu bisa dipulihkan:
//!   status yang bukan 2xx di sini dibuat sendiri dari respons yang sudah
//!   diterima (`.terima_semua_status()`), sedangkan kegagalan transport
//!   (`e.response` undefined) selalu jadi `500` seperti di JS.

use std::sync::LazyLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Value, json};

use crate::common::truthy;

const UPSAMPLER_URL: &str = "https://upsampler.com/free-image-generator-no-signup";
const SPACE_URL: &str = "https://black-forest-labs-flux-2-klein-4b.hf.space";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// `next-action` server action yang dipakai `checkPrompt` (`txt2img2.js:26`).
const NEXT_ACTION: &str = "315bc26dade9ed14e1a168a4d9f7cea08869133d";

/// `crypto.randomBytes(8).toString("hex")` — 16 digit heksa.
fn session_hash() -> String {
    (0..8).map(|_| format!("{:02x}", rand::random::<u8>())).collect()
}

/// `extractUrl(output)` (`txt2img2.js:125-143`). `JSON.stringify(output || "")`
/// diganti `serde_json::to_string`; keduanya rapat tanpa spasi, dan itu yang
/// dibaca kedua regex.
fn extract_url(output: &Value) -> String {
    let text = if truthy(output) {
        serde_json::to_string(output).unwrap_or_default()
    } else {
        // `JSON.stringify("")` — dua tanda kutip, bukan string kosong.
        "\"\"".to_string()
    };

    if let Some(m) = RE_URL_PENUH.find(&text) {
        return m.as_str().replace("\\u0026", "&").replace("\\/", "/");
    }
    if let Some(m) = RE_TMP_GRADIO.find(&text) {
        return format!("{SPACE_URL}/gradio_api/file={}", m.as_str());
    }
    String::new()
}

static RE_URL_PENUH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"https://black-forest-labs-flux-2-klein-4b\.hf\.space/gradio_api/file=[^"'\\\s]+"#)
        .expect("regex URL penuh")
});

static RE_TMP_GRADIO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"/tmp/gradio/[^"'\\\s]+?\.(webp|png|jpg|jpeg)"#).expect("regex /tmp/gradio")
});

/// Pembaca SSE `getResult` (`txt2img2.js:41-123`).
///
/// `buffer.split("\n\n")` lalu `blocks.pop()`: blok terakhir yang tidak diakhiri
/// baris kosong TIDAK diproses. Ditiru apa adanya di sini.
fn read_queue_sse(raw: &str, event_id: &Value) -> Result<String, String> {
    let blocks: Vec<&str> = raw.split("\n\n").collect();
    for block in blocks.iter().take(blocks.len().saturating_sub(1)) {
        let Some(line) = block.split('\n').find(|l| l.starts_with("data: ")) else {
            continue;
        };
        // `replace("data: ", "")` di JS hanya melepas kemunculan pertama.
        let payload = line.strip_prefix("data: ").unwrap_or(line).trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        let Ok(json) = serde_json::from_str::<Value>(payload) else {
            // `JSON.parse` melempar SyntaxError, dan `catch` di dalam
            // `on("data")` meneruskannya apa adanya ke promise yang ditolak.
            return Err(format!("SyntaxError: JSON tidak terbaca: {payload}"));
        };
        if json
            .get("event_id")
            .is_some_and(truthy)
            && json.get("event_id") != Some(event_id)
        {
            continue;
        }
        match json.get("msg").and_then(Value::as_str) {
            Some("process_completed") => {
                let url = extract_url(json.get("output").unwrap_or(&Value::Null));
                if url.is_empty() {
                    return Err("URL hasil tidak ditemukan".to_string());
                }
                return Ok(url);
            }
            Some("process_failed") => return Err("Generate gagal".to_string()),
            _ => {}
        }
    }
    // `res.data.on("end")` menolak kalau stream selesai tanpa hasil.
    Err("Stream selesai tanpa hasil".to_string())
}

/// `checkPrompt(prompt)` (`txt2img2.js:16-39`) — `(flagged, rateLimited)`.
fn check_prompt(http: &dyn HttpGet, prompt: &str) -> (bool, bool) {
    let body = serde_json::to_string(&json!([prompt])).unwrap_or_else(|_| "[]".to_string());
    let Ok(res) = http.call(
        &HttpCall::post(UPSAMPLER_URL)
            .header("user-agent", UA)
            .header("accept", "text/x-component")
            .header("content-type", "text/plain;charset=UTF-8")
            .header("origin", "https://upsampler.com")
            .header("referer", UPSAMPLER_URL)
            .header("next-action", NEXT_ACTION)
            .raw(body.into_bytes(), "text/plain;charset=UTF-8"),
    ) else {
        return (false, false);
    };
    // `String(res.data || "")` — respons dibaca sebagai teks apa adanya.
    let text = res.text();
    (
        text.contains("\"flagged\":true"),
        text.contains("\"rateLimited\":true"),
    )
}

/// Bentuk `catch` di `Txt2Img2`: `{ status: false, code, prompt, error }`.
fn gagal(prompt: &str, code: u16, error: String) -> Value {
    json!({ "status": false, "code": code, "prompt": prompt, "error": error })
}

/// Pesan axios untuk status di luar 2xx (`Request failed with status code N`).
fn pesan_status(status: u16) -> String {
    format!("Request failed with status code {status}")
}

/// `Txt2Img2(prompt)` (`txt2img2.js:145-195`) — tidak pernah `Err`.
pub fn txt2img2(http: &dyn HttpGet, prompt: &str) -> Result<Value, HttpError> {
    Ok(alur(http, prompt))
}

fn alur(http: &dyn HttpGet, prompt: &str) -> Value {
    let (flagged, rate_limited) = check_prompt(http, prompt);
    if flagged {
        return gagal(prompt, 400, "Prompt terdeteksi tidak aman".to_string());
    }
    if rate_limited {
        return gagal(prompt, 429, "Rate limited, coba lagi nanti".to_string());
    }

    let session = session_hash();
    let join = http.call(
        &HttpCall::post(format!("{SPACE_URL}/gradio_api/queue/join?"))
            .header("user-agent", UA)
            .header("accept", "*/*")
            .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
            .header("origin", "https://upsampler.com")
            .header("referer", "https://upsampler.com/")
            .header("content-type", "application/json")
            .header("x-gradio-user", "api")
            .terima_semua_status()
            .json(&json!({
                "data": [prompt, [], "Distilled (4 steps)", 0, true, 1024, 1024, 4, 1, false],
                "event_data": Value::Null,
                "fn_index": 6,
                "trigger_id": Value::Null,
                "session_hash": session,
            })),
    );
    let join = match join {
        Ok(r) => r,
        Err(e) => return gagal(prompt, 500, pesan_teks_atau(e.message, "Unknown error")),
    };
    if !(200..300).contains(&join.status) {
        return gagal(prompt, join.status, pesan_status(join.status));
    }

    // `joinRes.data?.event_id` lalu `if (!eventId)` — nilai falsy (termasuk
    // string kosong) ikut memicu cabang 500.
    let body = join.json().unwrap_or(Value::Null);
    let Some(event_id) = body.get("event_id").filter(|v| truthy(v)).cloned() else {
        // `event_id` string kosong juga falsy, jadi ikut ke sini.
        return gagal(prompt, 500, "event_id tidak ditemukan".to_string());
    };

    let antrean = http.call(
        &HttpCall::get(format!(
            "{SPACE_URL}/gradio_api/queue/data?session_hash={session}"
        ))
        .header("user-agent", UA)
        .header("accept", "text/event-stream")
        .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("origin", "https://upsampler.com")
        .header("referer", "https://upsampler.com/")
        .header("content-type", "application/json")
        .terima_semua_status(),
    );
    let antrean = match antrean {
        Ok(r) => r,
        Err(e) => return gagal(prompt, 500, pesan_teks_atau(e.message, "Unknown error")),
    };
    if !(200..300).contains(&antrean.status) {
        return gagal(prompt, antrean.status, pesan_status(antrean.status));
    }

    match read_queue_sse(&antrean.text(), &event_id) {
        Ok(url) => json!({ "status": true, "code": 200, "prompt": prompt, "url": url }),
        Err(e) => gagal(prompt, 500, e),
    }
}

/// `e.message || "Unknown error"` — pesan kosong jadi teks bawaan.
fn pesan_teks_atau(pesan: String, bawaan: &str) -> String {
    if pesan.is_empty() {
        bawaan.to_string()
    } else {
        pesan
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet` tanpa jaringan: memilih balasan lewat potongan URL, jadi
    /// tiga langkah scraper bisa punya balasan berbeda.
    #[derive(Debug)]
    struct StubHttp {
        rute: Vec<(String, u16, String)>,
        permintaan: Mutex<Vec<HttpCall>>,
    }

    impl StubHttp {
        fn baru(rute: Vec<(&str, u16, &str)>) -> Self {
            Self {
                rute: rute
                    .into_iter()
                    .map(|(f, s, b)| (f.to_string(), s, b.to_string()))
                    .collect(),
                permintaan: Mutex::new(Vec::new()),
            }
        }

        fn balas(&self, url: &str) -> Result<HttpResponse, HttpError> {
            let Some((_, status, body)) = self.rute.iter().find(|(f, _, _)| url.contains(f.as_str()))
            else {
                return Err(HttpError {
                    message: "koneksi ditolak".to_string(),
                });
            };
            Ok(HttpResponse {
                status: *status,
                body: HttpBody::Bytes {
                    data: Arc::from(body.clone().into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(HttpCall::get(url));
            self.balas(url)
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
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
            self.balas(&req.url)
        }
    }

    /// Rute lengkap: cek prompt bersih, join dengan `event_id`, lalu SSE.
    fn stub_sse(sse: &str) -> StubHttp {
        StubHttp::baru(vec![
            (UPSAMPLER_URL, 200, r#"{"ok":1}"#),
            ("queue/join", 200, r#"{"event_id":"evt-1"}"#),
            ("queue/data", 200, sse),
        ])
    }

    const SSE_SUKSES: &str = concat!(
        "data: {\"msg\":\"estimation_started\",\"event_id\":\"evt-1\"}\n\n",
        "data: {\"msg\":\"process_completed\",\"event_id\":\"evt-1\",\
         \"output\":{\"url\":\"/tmp/gradio/aa/out.webp\"}}\n\n",
        "data: [DONE]\n\n"
    );

    fn cari<'a>(p: &'a HttpCall, nama: &str) -> Option<&'a str> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn sukses_mengembalikan_status_code_prompt_dan_url() {
        let http = stub_sse(SSE_SUKSES);
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["prompt"], "kucing");
        assert_eq!(
            hasil["url"],
            format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/aa/out.webp")
        );
        assert!(hasil.get("error").is_none(), "kunci sukses tidak ada error");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 3, "cek prompt, join, baca antrean");

        // Langkah 1: POST server action dengan header khasnya.
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, UPSAMPLER_URL);
        assert_eq!(p[0].body.as_ref().unwrap().bytes(), br#"["kucing"]"#);
        assert_eq!(cari(&p[0], "accept"), Some("text/x-component"));
        assert_eq!(
            cari(&p[0], "content-type"),
            Some("text/plain;charset=UTF-8")
        );
        assert_eq!(cari(&p[0], "next-action"), Some(NEXT_ACTION));
        assert_eq!(cari(&p[0], "referer"), Some(UPSAMPLER_URL));
        assert_eq!(cari(&p[0], "origin"), Some("https://upsampler.com"));

        // Langkah 2: body join — urutan kunci dan nilai sama seperti objek JS.
        assert_eq!(p[1].url, format!("{SPACE_URL}/gradio_api/queue/join?"));
        let body: Value = serde_json::from_slice(p[1].body.as_ref().unwrap().bytes()).unwrap();
        assert_eq!(
            body["data"],
            json!([
                "kucing",
                [],
                "Distilled (4 steps)",
                0,
                true,
                1024,
                1024,
                4,
                1,
                false
            ])
        );
        assert_eq!(body["fn_index"], 6);
        assert_eq!(body["event_data"], Value::Null);
        assert_eq!(body["trigger_id"], Value::Null);
        let hash = body["session_hash"].as_str().unwrap().to_string();
        assert_eq!(hash.len(), 16, "randomBytes(8) = 16 hex");
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "{hash}");
        assert_eq!(cari(&p[1], "x-gradio-user"), Some("api"));
        assert_eq!(cari(&p[1], "content-type"), Some("application/json"));

        // Langkah 3: GET antrean memakai session hash yang sama dengan join.
        assert_eq!(
            p[2].url,
            format!("{SPACE_URL}/gradio_api/queue/data?session_hash={hash}")
        );
        assert_eq!(cari(&p[2], "accept"), Some("text/event-stream"));
    }

    #[test]
    fn prompt_flagged_menghentikan_sebelum_join() {
        let http = StubHttp::baru(vec![(UPSAMPLER_URL, 200, r#"{"flagged":true}"#)]);
        let hasil = txt2img2(&http, "bahaya").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["code"], 400);
        assert_eq!(hasil["prompt"], "bahaya");
        assert_eq!(hasil["error"], "Prompt terdeteksi tidak aman");
        assert_eq!(http.permintaan.lock().unwrap().len(), 1, "tidak boleh join");
    }

    #[test]
    fn rate_limited_menghentikan_sebelum_join() {
        let http = StubHttp::baru(vec![(UPSAMPLER_URL, 200, r#"{"rateLimited":true}"#)]);
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["code"], 429);
        assert_eq!(hasil["error"], "Rate limited, coba lagi nanti");
    }

    #[test]
    fn kegagalan_cek_prompt_ditelan_seperti_js() {
        // `catch` di `checkPrompt` membuat request yang gagal terbaca sebagai
        // "tidak flagged, tidak rate limited", jadi alur tetap jalan.
        let mut http = stub_sse(SSE_SUKSES);
        http.rute.retain(|(f, _, _)| f != UPSAMPLER_URL);
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["status"], true, "cek prompt gagal tidak boleh menggagalkan");
    }

    #[test]
    fn join_tanpa_event_id_mengembalikan_500_dengan_pesan_js() {
        let http = StubHttp::baru(vec![
            (UPSAMPLER_URL, 200, "{}"),
            ("queue/join", 200, r#"{"event_id":""}"#),
        ]);
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["code"], 500);
        assert_eq!(hasil["error"], "event_id tidak ditemukan");
        assert_eq!(http.permintaan.lock().unwrap().len(), 2, "tidak baca antrean");
    }

    #[test]
    fn stream_gagal_dan_kosong_memakai_pesan_js_masing_masing() {
        let gagal = StubHttp::baru(vec![
            (UPSAMPLER_URL, 200, "{}"),
            ("queue/join", 200, r#"{"event_id":"evt-1"}"#),
            (
                "queue/data",
                200,
                "data: {\"msg\":\"process_failed\",\"event_id\":\"evt-1\"}\n\n",
            ),
        ]);
        let hasil = txt2img2(&gagal, "kucing").unwrap();
        assert_eq!(hasil["code"], 500);
        assert_eq!(hasil["error"], "Generate gagal");

        let kosong = stub_sse("data: [DONE]\n\n");
        let hasil = txt2img2(&kosong, "kucing").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Stream selesai tanpa hasil");
    }

    #[test]
    fn proses_selesai_tanpa_url_melempar_pesan_js() {
        let http = stub_sse(concat!(
            "data: {\"msg\":\"process_completed\",\"event_id\":\"evt-1\",\
             \"output\":{\"url\":\"/tmp/gradio/tidak-ada-ekstensi\"}}\n\n",
            "data: [DONE]\n\n"
        ));
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["code"], 500);
        assert_eq!(hasil["error"], "URL hasil tidak ditemukan");
    }

    #[test]
    fn status_bukan_2xx_membawa_kode_status_seperti_axios() {
        let http = StubHttp::baru(vec![
            (UPSAMPLER_URL, 200, "{}"),
            ("queue/join", 502, "<html>bad gateway</html>"),
        ]);
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["code"], 502, "e.response.status ikut");
        assert_eq!(hasil["error"], "Request failed with status code 502");
    }

    #[test]
    fn kegagalan_transport_jadi_500_dengan_pesan_klien() {
        let mut http = stub_sse(SSE_SUKSES);
        http.rute.retain(|(f, _, _)| f != "queue/join");
        let hasil = txt2img2(&http, "kucing").unwrap();
        assert_eq!(hasil["code"], 500, "tanpa response selalu 500");
        assert_eq!(hasil["error"], "koneksi ditolak");
    }

    #[test]
    fn sse_dari_event_id_lain_dilewati() {
        let sse = concat!(
            "data: {\"msg\":\"process_completed\",\"event_id\":\"lain\",\
             \"output\":{\"url\":\"/tmp/gradio/salah.webp\"}}\n\n",
            "data: {\"msg\":\"process_completed\",\"event_id\":\"evt-1\",\
             \"output\":{\"url\":\"/tmp/gradio/benar.webp\"}}\n\n"
        );
        let url = read_queue_sse(sse, &json!("evt-1")).unwrap();
        assert_eq!(
            url,
            format!("{SPACE_URL}/gradio_api/file=/tmp/gradio/benar.webp")
        );
    }

    #[test]
    fn blok_sse_terakhir_tanpa_baris_kosong_dibuang_seperti_pop() {
        let sse = "data: {\"msg\":\"process_completed\",\"output\":{\"url\":\"/tmp/gradio/a.png\"}}";
        assert_eq!(
            read_queue_sse(sse, &json!("evt-1")).unwrap_err(),
            "Stream selesai tanpa hasil"
        );
    }

    #[test]
    fn url_penuh_menang_dan_escape_json_dibuka() {
        let keluaran = json!({
            "cdn": format!("{SPACE_URL}/gradio_api/file=aaa.webp?x=1&y=2"),
            "tmp": "/tmp/gradio/bbb.png",
        });
        assert_eq!(
            extract_url(&keluaran),
            format!("{SPACE_URL}/gradio_api/file=aaa.webp?x=1&y=2")
        );
        // Slash yang di-escape tidak bisa kena. `JSON.stringify` menggandakan
        // tiap backslash, jadi teks berisi `https:\\/\\/` sedangkan regex JS
        // mencari `https://`; `replaceAll("\/","/")` di `txt2img2.js:132` juga
        // tidak menolong karena `"\/"` sudah sama dengan `"/"`. Hasilnya kosong,
        // sama seperti yang diberi Node untuk masukan ini.
        let terbalik = json!("https:\\/\\/black-forest-labs-flux-2-klein-4b.hf.space\\/gradio_api\\/file=z.webp");
        assert_eq!(extract_url(&terbalik), "");
        assert_eq!(extract_url(&json!("tidak ada url")), "");
    }

    #[test]
    fn session_hash_enam_awal_dan_enam_belas_panjang() {
        let h = session_hash();
        assert_eq!(h.len(), 16);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()), "{h}");
    }
}