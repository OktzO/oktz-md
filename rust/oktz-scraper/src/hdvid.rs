//! `src/scraper/hdvid.js` — penajam video lewat `fgsi.dpdns.org`.
//!
//! Tiga lapis: `createEnhanceTask` (unggah multipart), `pollEnhanceTask` (tunggu
//! sampai selesai), dan `videoenhancer` yang merangkai keduanya. Yang disalin
//! apa adanya:
//!
//! - `form.append("file", fs.createReadStream(filePath), path.basename(filePath))`
//!   → part bernama `file` dengan `filename` = basename dan content-type dari
//!   `mime.lookup` berkas itu (`_getContentType` di `form-data` memakai
//!   `options.filename` kalau ada). Tidak ada part field lain.
//! - `if (!payload?.status || !payload?.data?.pollUrl)` melempar dengan
//!   `payload?.message || "Gagal membuat task HD video"`.
//! - `throw new Error("HD video selesai tetapi url hasil tidak ditemukan")`
//!   **bukan** terminal: ia dilempar di dalam `try` setelah respons terbaca.
//!   Karena `transientErrors = 0` dijalankan SEBELUM status dinilai, tiap
//!   putaran menyetel ulang penghitung lalu menaikkan lagi — jadi penghitung
//!   tidak pernah mencapai `maxTransientErrors`. Polling berjalan sampai
//!   `timeoutMs`, lalu melempar `lastPayload?.message || "Timeout menunggu
//!   hasil HD video"`. Kekhasan ini sengaja ditiru.
//! - Status `failed`/`error`/`cancelled`/`canceled` dan respons tanpa
//!   `payload.status` dengan status di luar daftar `PENDING_STATUSES` menandai
//!   `isTerminal`, jadi langsung dilempar tanpa dihitung sebagai transient.
//! - `while (Date.now() - startedAt < timeoutMs)` dan `delay(pollIntervalMs)`
//!   setelah tiap percobaan, termasuk setelah percobaan terakhir.
//!
//! Beda yang diketahui karena scraper tidak boleh melakukan I/O:
//! - `fs.existsSync` + `fs.createReadStream` + tulis/`unlink` berkas temporer
//!   dihapus. Byte video masuk sebagai argumen `video`, dan `filename` masuk
//!   sebagai argumen terpisah, jadi `path.basename` tetap sama tanpa perlu
//!   `os.tmpdir()`.
//! - `videoenhancer(video)` di JS menerima Buffer ATAU string path. Bentuk
//!   path itu tidak ada di sini: pemanggil yang membaca berkas dan mengirim
//!   byte-nya, lalu menulis ke `os.tmpdir()` sendiri kalau memang perlu.
//! - `apiKey = process.env.FGSI_API_KEY` jadi argumen: crate scraper tidak membaca
//!   environment. Nilai kosong tetap dikirim sebagai header `apikey: ""`,
//!   persis seperti JS.
//! - `timeout: 120000/30000` dan `maxBodyLength: Infinity` milik axios tidak
//!   ditiru — batas mesin, bukan tampilan.
//! - `error?.response?.data?.message` tidak ada di `HttpError` (hanya `message`),
//!   jadi kegagalan HTTP/transport memakai pesan klien itu sendiri.

use std::sync::Arc;
use std::time::{Duration, Instant};

use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, MultipartPart};
use serde_json::{Map, Value};

use crate::common::truthy;

const API_URL: &str = "https://fgsi.dpdns.org/api/tools/enchantVideo";

/// `PENDING_STATUSES` (`hdvid.js:10-15`).
const PENDING: [&str; 4] = ["pending", "processing", "queued", "running"];

/// Status yang langsung-stop polling (`hdvid.js:88-98`).
const GAGAL: [&str; 4] = ["failed", "error", "cancelled", "canceled"];

/// Kegagalan di dalam polling: `isTerminal` pada objek error di JS.
///
/// `respons_terbaca` meniru posisi `transientErrors = 0` di JS: penugasan itu
/// ada SEBELUM status dinilai, jadi kegagalan setelah respons terbaca ikut
/// menyetel ulang hitungan — bukan hanya yang "masuk antrean".
struct PollingGagal {
    pesan: String,
    terminal: bool,
    respons_terbaca: bool,
}

impl PollingGagal {
    /// `isTerminal = true` — langsung dilempar oleh `catch`, tanpa dihitung.
    fn terminal(pesan: String) -> Self {
        Self {
            pesan,
            terminal: true,
            respons_terbaca: true,
        }
    }

    /// Kegagalan sementara yang_thrown setelah respons terbaca.
    fn sementara(pesan: String) -> Self {
        Self {
            pesan,
            terminal: false,
            respons_terbaca: true,
        }
    }

    /// Kegagalan transport: `lastPayload` dan `transientErrors` tidak berubah.
    fn transport(pesan: String) -> Self {
        Self {
            pesan,
            terminal: false,
            respons_terbaca: false,
        }
    }
}

/// `payload?.message` sebagai teks; nilai non-string ikut di-`String()` seperti
/// `new Error(nonString)` di JS.
fn pesan_teks(v: Option<&Value>) -> Option<String> {
    v.filter(|x| truthy(x)).map(|x| match x {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

/// `String(payload?.data?.status || "")` lalu `.trim()`.
fn status_poll(payload: &Value) -> String {
    let mentah = payload
        .get("data")
        .and_then(|d| d.get("status"))
        .filter(|v| truthy(v))
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_default();
    mentah.trim().to_string()
}

/// `crypto.randomUUID()` (`hdvid.js:140`; fallback `randomBytes(16)` tidak
/// pernah terpakai di Node yang memakai `randomUUID?.()`).
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

/// `createEnhanceTask(filePath, apiKey)` (`hdvid.js:19-47`).
///
/// Byte video dan `file_path` masuk sebagai argumen (scraper tidak menyentuh
/// disk); `path.basename(file_path)` tetap dipakai untuk `filename`.
pub fn create_enhance_task(
    http: &dyn HttpGet,
    file_path: &str,
    video: &[u8],
    api_key: &str,
) -> Result<Value, HttpError> {
    let nama = basename(file_path);
    let res = http.call(
        &HttpCall::post(API_URL)
            // `...form.getHeaders()` tidak disalin: boundary dibuat klien Rust,
            // dan `Content-Type: multipart/form-data` dari pemanggil dibuang
            // untuk body multipart (sama seperti `hd.rs`).
            .header("apikey", api_key)
            .multipart(vec![MultipartPart {
                name: "file".to_string(),
                file_name: Some(nama.clone()),
                mimetype: Some(mime_dari_nama(&nama).to_string()),
                body: HttpBody::Bytes {
                    data: Arc::from(video.to_vec()),
                    mimetype: None,
                },
            }]),
    )?;
    let payload = res.json().unwrap_or(Value::Null);

    let poll_url = payload
        .get("data")
        .and_then(|d| d.get("pollUrl"))
        .filter(|v| truthy(v));
    if !payload.get("status").map(truthy).unwrap_or(false) || poll_url.is_none() {
        return Err(HttpError {
            message: pesan_teks(payload.get("message"))
                .unwrap_or_else(|| "Gagal membuat task HD video".to_string()),
        });
    }

    // Kunci yang `undefined` di JS tidak ikut `JSON.stringify`.
    let mut out = Map::new();
    for kunci in ["taskId", "createdAt", "pollUrl"] {
        if let Some(v) = payload.get("data").and_then(|d| d.get(kunci)) {
            out.insert(kunci.to_string(), v.clone());
        }
    }
    Ok(Value::Object(out))
}

/// `pollEnhanceTask(pollUrl, {…})` (`hdvid.js:49-125`).
///
/// Mengembalikan `{ taskId, createdAt, ...result }` persis seperti objek yang
/// dikembalikan JS. `poll_interval_ms` dan `timeout_ms` adalah argumen supaya
/// test bisa memakai nilai kecil.
pub fn poll_enhance_task(
    http: &dyn HttpGet,
    poll_url: &str,
    api_key: &str,
    poll_interval_ms: u64,
    timeout_ms: u64,
    max_transient_errors: u32,
) -> Result<Value, HttpError> {
    let mulai = Instant::now();
    let batas = Duration::from_millis(timeout_ms);
    let mut transient = 0u32;
    // `lastPayload` hanya diisi setelah respons berhasil dibaca; kegagalan
    // transport tidak menyentuhnya.
    let mut last_payload: Option<Value> = None;

    while mulai.elapsed() < batas {
        match percobaan(http, poll_url, api_key) {
            // `lastPayload = payload; transientErrors = 0` terjadi begitu
            // respons terbaca, sebelum status dinilai — jadi respons yang
            // menunggu pun menyetel ulang hitungan transient.
            Ok((hasil, payload)) => {
                last_payload = Some(payload);
                transient = 0;
                if !hasil.is_null() {
                    return Ok(hasil);
                }
            }
            Err(e) if e.terminal => return Err(HttpError { message: e.pesan }),
            Err(e) => {
                // `transientErrors = 0` di JS sudah dieksekusi sebelum status
                // dinilai, jadi kegagalan yang terjadi sesudah respons terbaca
                // ikut menyetel ulang penghitung.
                if e.respons_terbaca {
                    transient = 0;
                }
                transient += 1;
                if transient >= max_transient_errors {
                    // `error?.response?.data?.message || error?.message ||
                    //  "Polling HD video gagal"` — `HttpError` tidak punya
                    // `response`, jadi yang dipakai adalah `message`.
                    return Err(HttpError {
                        message: if e.pesan.is_empty() {
                            "Polling HD video gagal".to_string()
                        } else {
                            e.pesan
                        },
                    });
                }
            }
        }
        // `await delay(pollIntervalMs)` — juga setelah percobaan terakhir.
        std::thread::sleep(Duration::from_millis(poll_interval_ms));
    }

    Err(HttpError {
        message: last_payload
            .as_ref()
            .and_then(|p| pesan_teks(p.get("message")))
            .unwrap_or_else(|| "Timeout menunggu hasil HD video".to_string()),
    })
}

/// Satu putaran polling: satu GET plus penilaian status.
///
/// Balikan `Ok((hasil, payload))`: `hasil` adalah objek yang dikembalikan JS
/// kalau task selesai, atau `Value::Null` kalau masih menunggu; `payload` adalah
/// respons mentah untuk `lastPayload`.
fn percobaan(
    http: &dyn HttpGet,
    poll_url: &str,
    api_key: &str,
) -> Result<(Value, Value), PollingGagal> {
    // Kegagalan transport (dan 4xx/5xx, karena `HttpGet` melempar seperti axios)
    // masuk `catch` sebagai transient — `lastPayload` tidak berubah.
    let payload = match http.call(&HttpCall::get(poll_url).header("apikey", api_key)) {
        Ok(res) => res.json().unwrap_or(Value::Null),
        Err(e) => return Err(PollingGagal::transport(e.message)),
    };

    let status = status_poll(&payload);
    let normalized = status.to_lowercase();
    let data = payload.get("data").cloned().unwrap_or(Value::Null);

    if normalized == "success" {
        let result = data.get("result").cloned().unwrap_or(Value::Null);
        if !result.get("res_url").map(truthy).unwrap_or(false) {
            // Dilempar di dalam `try` JS: transient, bukan terminal. Karena
            // `transientErrors = 0` sudah berjalan, penghitung selalu kembali
            // ke 1 dan batasnya tidak tercapai.
            return Err(PollingGagal::sementara(
                "HD video selesai tetapi url hasil tidak ditemukan".to_string(),
            ));
        }
        let mut out = Map::new();
        for kunci in ["taskId", "createdAt"] {
            if let Some(v) = data.get(kunci) {
                out.insert(kunci.to_string(), v.clone());
            }
        }
        // `...result` menimpa kunci di atas kalau punya nama yang sama.
        if let Some(obj) = result.as_object() {
            for (k, v) in obj {
                out.insert(k.clone(), v.clone());
            }
        }
        return Ok((Value::Object(out), payload));
    }

    if GAGAL.contains(&normalized.as_str()) {
        return Err(PollingGagal::terminal(
            pesan_teks(payload.get("message"))
                .or_else(|| pesan_teks(data.get("message")))
                .unwrap_or_else(|| format!("HD video gagal dengan status {status}")),
        ));
    }

    if !payload.get("status").map(truthy).unwrap_or(false) && !PENDING.contains(&normalized.as_str())
    {
        return Err(PollingGagal::terminal(
            pesan_teks(payload.get("message"))
                .unwrap_or_else(|| "Polling HD video gagal".to_string()),
        ));
    }

    Ok((Value::Null, payload))
}

/// `videoenhancer(video, { filename, apiKey, pollIntervalMs, timeoutMs })`
/// (`hdvid.js:127-172`).
///
/// `video` adalah byte video, bukan path: `None` berarti `!video` di JS.
/// `filename` `None`/kosong memunculkan nama bawaan `hdvid-<uuid>.mp4`.
pub fn video_enhancer(
    http: &dyn HttpGet,
    video: Option<&[u8]>,
    filename: Option<&str>,
    api_key: &str,
    poll_interval_ms: u64,
    timeout_ms: u64,
) -> Result<Value, HttpError> {
    let Some(video) = video else {
        return Err(HttpError {
            message: "video is required".to_string(),
        });
    };
    let safe_name = match filename.filter(|n| !n.is_empty()) {
        Some(n) => n.to_string(),
        None => format!("hdvid-{}.mp4", uuid_v4()),
    };

    let task = create_enhance_task(http, &safe_name, video, api_key)?;
    let poll_url = task
        .get("pollUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| HttpError {
            message: "Gagal membuat task HD video".to_string(),
        })?
        .to_string();
    let result = poll_enhance_task(http, &poll_url, api_key, poll_interval_ms, timeout_ms, 5)?;

    let mut out = Map::new();
    for kunci in ["taskId", "pollUrl", "createdAt"] {
        if let Some(v) = task.get(kunci) {
            out.insert(kunci.to_string(), v.clone());
        }
    }
    if let Some(obj) = result.as_object() {
        for (k, v) in obj {
            out.insert(k.clone(), v.clone());
        }
    }
    // `resultUrl: result.res_url` — kunci `undefined` tidak ikut `JSON.stringify`.
    if let Some(v) = result.get("res_url") {
        out.insert("resultUrl".to_string(), v.clone());
    }
    Ok(Value::Object(out))
}

/// `path.basename(filePath)` (POSIX): garis miring di ujung diabaikan.
fn basename(jalur: &str) -> String {
    let mut s = jalur;
    while s.len() > 1 && s.ends_with('/') {
        s = &s[..s.len() - 1];
    }
    if s == "/" {
        return String::new();
    }
    match s.rfind('/') {
        Some(i) => s[i + 1..].to_string(),
        None => s.to_string(),
    }
}

/// `mime.lookup(filename)` untuk berkas video; di luar daftar ini `form-data`
/// memakai `DEFAULT_CONTENT_TYPE` (`application/octet-stream`).
fn mime_dari_nama(nama: &str) -> &'static str {
    let ext = nama
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mp4" => "video/mp4",
        "m4v" => "video/x-m4v",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "flv" => "video/x-flv",
        "mpg" | "mpeg" => "video/mpeg",
        "3gp" => "video/3gpp",
        "ogv" => "video/ogg",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::HttpResponse;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: balasan berurutan, balasan terakhir diulang
    /// setelah habis (bikin percobaan polling yang sama terulang).
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<VecDeque<(u16, String)>>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<(u16, &str)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(
                    balasan.into_iter().map(|(s, b)| (s, b.to_string())).collect(),
                ),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
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
            unreachable!("semua lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let (status, body) = {
                let mut q = self.balasan.lock().unwrap();
                if q.len() > 1 {
                    q.pop_front().unwrap()
                } else {
                    q.front().cloned().unwrap_or_default()
                }
            };
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

    fn cari<'a>(p: &'a HttpCall, nama: &str) -> Option<&'a str> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    /// Poll membaca `payload.data.status`, bukan `payload.status`
    /// (`hdvid.js:70`) — jadi fixture harus menaruh status di dalam `data`.
    const SUKSES: &str = r#"{"status":true,"data":{"taskId":"t1","createdAt":"2024-01-01",
        "status":"success",
        "pollUrl":"https://fgsi.dpdns.org/api/poll/t1",
        "result":{"res_url":"https://cdn/hasil.mp4","durasi":"12s"}}}"#;

    #[test]
    fn create_task_mengirim_multipart_dengan_basename_dan_mime() {
        let http = StubHttp::baru(vec![(
            200,
            r#"{"status":true,"data":{"taskId":"t1","createdAt":"2024-01-01",
                "pollUrl":"https://fgsi/poll/t1"}}"#,
        )]);
        let hasil =
            create_enhance_task(&http, "/tmp/vid/klip.mp4", &[1u8, 2, 3], "kunci-rahasia").unwrap();
        assert_eq!(hasil["taskId"], "t1");
        assert_eq!(hasil["pollUrl"], "https://fgsi/poll/t1");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, API_URL);
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart: {:?}", p[0].body);
        };
        assert_eq!(parts.len(), 1, "tidak ada part field lain di JS");
        assert_eq!(parts[0].name, "file");
        assert_eq!(parts[0].file_name.as_deref(), Some("klip.mp4"));
        assert_eq!(parts[0].mimetype.as_deref(), Some("video/mp4"));
        assert_eq!(parts[0].body.bytes(), &[1u8, 2, 3][..]);
        assert_eq!(cari(&p[0], "apikey"), Some("kunci-rahasia"));
    }

    #[test]
    fn create_task_gagal_memakai_pesan_api_lalu_bawaan() {
        let dari_api = StubHttp::baru(vec![(
            200,
            r#"{"status":false,"message":"File terlalu besar"}"#,
        )]);
        assert_eq!(
            create_enhance_task(&dari_api, "a.mp4", &[1], "k")
                .unwrap_err()
                .message,
            "File terlalu besar"
        );

        let bawaan = StubHttp::baru(vec![(200, r#"{"status":true}"#)]);
        assert_eq!(
            create_enhance_task(&bawaan, "a.mp4", &[1], "k")
                .unwrap_err()
                .message,
            "Gagal membuat task HD video"
        );
    }

    #[test]
    fn kunci_undefined_tidak_masuk_hasil_create_task() {
        let http = StubHttp::baru(vec![(
            200,
            r#"{"status":true,"data":{"pollUrl":"https://fgsi/poll/x"}}"#,
        )]);
        let hasil = create_enhance_task(&http, "a.mp4", &[1], "k").unwrap();
        let obj = hasil.as_object().unwrap();
        assert_eq!(obj.len(), 1, "{hasil}");
        assert_eq!(obj["pollUrl"], "https://fgsi/poll/x");
    }

    #[test]
    fn polling_berhenti_di_status_pending_lalu_sukses() {
        let http = StubHttp::baru(vec![
            (200, r#"{"status":true,"data":{"status":"pending"}}"#),
            (200, SUKSES),
        ]);
        let hasil = poll_enhance_task(
            &http,
            "https://fgsi/poll/t1",
            "k",
            0,
            60_000,
            5,
        )
        .unwrap();
        assert_eq!(hasil["taskId"], "t1");
        assert_eq!(hasil["res_url"], "https://cdn/hasil.mp4");
        assert_eq!(hasil["durasi"], "12s");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(p[0].url, "https://fgsi/poll/t1");
        assert_eq!(cari(&p[0], "apikey"), Some("k"));
    }

    #[test]
    fn status_gagal_langsung_melempar_tanpa_menghitung_transient() {
        for status in ["failed", "error", "cancelled", "canceled"] {
            let http = StubHttp::baru(vec![(
                200,
                &format!(r#"{{"status":true,"data":{{"status":"{status}"}}}}"#),
            )]);
            let err = poll_enhance_task(&http, "u", "k", 0, 60_000, 5).unwrap_err();
            assert_eq!(err.message, format!("HD video gagal dengan status {status}"));
            assert_eq!(
                http.permintaan.lock().unwrap().len(),
                1,
                "terminal tidak diulang: {status}"
            );
        }
    }

    #[test]
    fn pesan_terminal_mengutamakan_pesan_api() {
        let http = StubHttp::baru(vec![(
            200,
            r#"{"status":true,"message":"dana habis","data":{"status":"failed","message":"kurang"}}"#,
        )]);
        assert_eq!(
            poll_enhance_task(&http, "u", "k", 0, 60_000, 5)
                .unwrap_err()
                .message,
            "dana habis"
        );
    }

    #[test]
    fn tanpa_status_luar_daftar_pending_jadi_terminal_default() {
        let http = StubHttp::baru(vec![(200, r#"{"data":{"status":"unknown"}}"#)]);
        assert_eq!(
            poll_enhance_task(&http, "u", "k", 0, 60_000, 5)
                .unwrap_err()
                .message,
            "Polling HD video gagal"
        );
    }

    #[test]
    fn url_hilang_saat_sukses_jatuh_ke_timeout_bukan_ke_batas_transient() {
        // `transientErrors = 0` terjadi sebelum `res_url` dicek, jadi setiap
        // putaran: reset ke 0 lalu naik ke 1. Tidak pernah mencapai 3.
        let http = StubHttp::baru(vec![(
            200,
            r#"{"status":true,"data":{"status":"success","result":{}}}"#,
        )]);
        let err = poll_enhance_task(&http, "u", "k", 0, 5, 3).unwrap_err();
        assert_eq!(
            err.message,
            "Timeout menunggu hasil HD video",
            "batas transient tidak terjangkau untuk error ini"
        );
        assert!(
            http.permintaan.lock().unwrap().len() > 1,
            "polling harus tetap berputar sampai waktu habis"
        );
    }

    #[test]
    fn kegagalan_transport_sampai_batas_baru_keluar() {
        #[derive(Debug, Default)]
        struct Offline;
        impl HttpGet for Offline {
            fn get(&self, _u: &str) -> Result<HttpResponse, HttpError> {
                unreachable!("tidak dipakai")
            }
            fn post_json(&self, _u: &str, _b: &Value) -> Result<HttpResponse, HttpError> {
                unreachable!("tidak dipakai")
            }
            fn post_form(&self, _u: &str, _f: &str) -> Result<HttpResponse, HttpError> {
                unreachable!("tidak dipakai")
            }
            fn post_multipart(
                &self,
                _u: &str,
                _p: &[(&str, HttpBody)],
            ) -> Result<HttpResponse, HttpError> {
                unreachable!("tidak dipakai")
            }
            fn call(&self, _r: &HttpCall) -> Result<HttpResponse, HttpError> {
                Err(HttpError {
                    message: "ECONNREFUSED".to_string(),
                })
            }
        }
        let err = poll_enhance_task(&Offline, "u", "k", 0, 60_000, 2).unwrap_err();
        assert_eq!(err.message, "ECONNREFUSED");
    }

    #[test]
    fn batas_waktu_memakai_pesan_payload_terakhir() {
        // `timeout_ms: 0` membuat `while` tidak pernah jalan, jadi `lastPayload`
        // masih `null` dan pesan bawaannya yang keluar.
        let http = StubHttp::baru(vec![(200, r#"{"status":true,"data":{"status":"pending"}}"#)]);
        let err = poll_enhance_task(&http, "u", "k", 0, 0, 5).unwrap_err();
        assert_eq!(err.message, "Timeout menunggu hasil HD video");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn video_enhancer_menggabungkan_task_poll_dan_result_url() {
        let http = StubHttp::baru(vec![
            (
                200,
                r#"{"status":true,"data":{"taskId":"t1","createdAt":"2024-01-01",
                    "pollUrl":"https://fgsi/poll/t1"}}"#,
            ),
            (200, SUKSES),
        ]);
        // Batas waktu pendek supaya fixture yang salah bentuk cepat gagal, bukan
        // berputar sampai proses dibunuh.
        let hasil = video_enhancer(&http, Some(&[7u8, 8]), None, "kunci", 0, 2_000).unwrap();
        assert_eq!(hasil["taskId"], "t1");
        assert_eq!(hasil["pollUrl"], "https://fgsi/poll/t1");
        assert_eq!(hasil["createdAt"], "2024-01-01");
        assert_eq!(hasil["res_url"], "https://cdn/hasil.mp4");
        assert_eq!(hasil["resultUrl"], "https://cdn/hasil.mp4");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("unggahan harus multipart");
        };
        // Nama bawaan `hdvid-<uuid>.mp4` — bentuknya, bukan nilainya.
        let nama = parts[0].file_name.clone().unwrap();
        assert!(nama.starts_with("hdvid-") && nama.ends_with(".mp4"), "{nama}");
        assert_eq!(nama.len(), "hdvid-".len() + 36 + ".mp4".len());
    }

    #[test]
    fn filename_kosong_jatuh_ke_nama_bawaan() {
        let http = StubHttp::baru(vec![(200, SUKSES), (200, SUKSES)]);
        video_enhancer(&http, Some(&[1]), Some(""), "k", 0, 60_000).unwrap();
        let p = http.permintaan.lock().unwrap();
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("unggahan harus multipart")
        };
        assert!(parts[0].file_name.as_deref().unwrap().starts_with("hdvid-"));
    }

    #[test]
    fn tanpa_video_melempar_pesan_js_tanpa_request() {
        let http = StubHttp::baru(vec![(200, SUKSES)]);
        let err = video_enhancer(&http, None, None, "k", 0, 60_000).unwrap_err();
        assert_eq!(err.message, "video is required");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn basename_dan_mime_mengikuti_form_data() {
        assert_eq!(basename("/tmp/a/b.mp4"), "b.mp4");
        assert_eq!(basename("b.mp4"), "b.mp4");
        assert_eq!(basename("/tmp/a/b/"), "b");
        assert_eq!(basename("/"), "");
        assert_eq!(mime_dari_nama("a.MP4"), "video/mp4");
        assert_eq!(mime_dari_nama("a.mov"), "video/quicktime");
        assert_eq!(mime_dari_nama("a.unknownext"), "application/octet-stream");
    }

    #[test]
    fn api_key_kosong_tetap_dikirim_sebagai_header() {
        let http = StubHttp::baru(vec![(
            200,
            r#"{"status":true,"data":{"pollUrl":"https://fgsi/poll/x"}}"#,
        )]);
        create_enhance_task(&http, "a.mp4", &[1], "").unwrap();
        let p = http.permintaan.lock().unwrap();
        assert_eq!(cari(&p[0], "apikey"), Some(""));
    }
}