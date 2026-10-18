//! `src/scraper/ytdl.js` — konversi YouTube lewat `d.ymcdn.org`.
//!
//! Yang ikut disalin dari JS:
//! - `ytdl` dibungkus `try/catch` yang mengembalikan
//!   `{ status: false, mess: "System Error: " + e.message }` — jadi semua
//!   kegagalan, termasuk yang tidak diharapkan, TIDAK jadi `Err`.
//! - `params` axios diserialisasi gaya `URLSearchParams` (spasi `+`), urutan
//!   kunci mengikuti urutan objek `params` di JS.
//! - Polling: `while (progress < 3 && attempts < 20)`, jeda 250 ms hanya kalau
//!   progress masih di bawah 3, lalu cek `attempts >= 20 && progress < 3`.
//! - `Number(data?.progress || 0)` bisa `NaN`; `NaN < 3` false di JS, jadi
//!   polling berhenti lebih awal dan hasilnya tetap dianggap sukses.
//!
//! Beda yang diketahui:
//! - `timeout: 60000` milik instance axios tidak ditiru (batas mesin, bukan
//!   tampilan); `UreqHttp` memakai batas global 20 detik.
//! - `setTimeout(250)` jadi `std::thread::sleep`, jadi fungsi ini memblokir
//!   thread selama polling.
//! - `Math.random()` jadi `rand::random::<f64>()`; bentuk keduanya angka desimal
//!   terpendek yang bolak-balik tepat, jadi query `_=` punya bentuk sama.

use std::sync::OnceLock;
use std::time::Duration;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::truthy;

const UA: &str = "Mozilla/5.0 (Linux; Android 16; NX729J) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.7271.123 Mobile Safari/537.36";

const REFERER: &str = "https://id.ytmp3.mobi/";

const INIT_URL: &str = "https://d.ymcdn.org/api/v1/init";

/// `CONVERT_FAILURE_MESSAGE` (`ytdl.js:10`).
const GAGAL_KONVERSI: &str = "Gagal mendapatkan data konversi.";

/// `this.CREATED_BY` / `this.NOTE` (`ytdl.js:112-113`).
const CREATED_BY: &str = "Ditzzy";
const CATATAN: &str = "Thank you for using this scrape";

/// `axios.create({ headers })` — dipakai ketiga permintaan.
fn header_klient(req: HttpCall) -> HttpCall {
    req.header("user-agent", UA).header("referer", REFERER)
}

/// `String(v)` untuk nilai JSON — dipakai di templat pesan error.
fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `Number(v)` — hanya bentuk yang dipakai `progress` dan `error`. Nilai lain
/// (objek/array) mengembalikan `NaN` seperti JS untuk kasus yang muncul di sini.
fn angka_js(v: Option<&Value>) -> f64 {
    match v {
        None | Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => {
            let s = s.trim();
            if s.is_empty() {
                0.0
            } else {
                s.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Some(Value::Array(_)) | Some(Value::Object(_)) => f64::NAN,
    }
}

/// `encodeURIComponent` gaya `URLSearchParams` axios: alfanumerik + `*-._`
/// dibiarkan, spasi `+`, sisanya persen huruf besar.
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

/// `axios` `buildURL`: `?` kalau URL belum punya query, `&` kalau sudah punya.
fn tambah_query(base: &str, bagian: &[(&str, String)]) -> String {
    let query = bagian
        .iter()
        .map(|(k, v)| format!("{}={}", encode_query(k), encode_query(v)))
        .collect::<Vec<_>>()
        .join("&");
    if query.is_empty() {
        return base.to_string();
    }
    if base.contains('?') {
        format!("{base}&{query}")
    } else {
        format!("{base}?{query}")
    }
}

/// `Math.random()` untuk parameter `_`.
fn acak() -> String {
    rand::random::<f64>().to_string()
}

/// `YOUTUBE_ID_REGEX` (`ytdl.js:3-4`).
fn pola_id_youtube() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| {
        Regex::new(
            r#"(?:youtube\.com/(?:[^/]+\/.+/|(?:v|e(?:mbed)?)/|.*[?&]v=)|youtu\.be/)([^"&?/\s]{11})"#,
        )
        .expect("pola id youtube")
    })
}

/// `extractVideoId(url)`.
fn ambil_id_youtube(url: &str) -> Option<String> {
    pola_id_youtube()
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// `v` yang dipakai sebagai URL: falsy → `None`, non-string truthy → error
/// (di JS axios melempar karena URL-nya bukan string).
fn ambil_url(v: Option<&Value>, nama: &str) -> Result<Option<String>, HttpError> {
    match v {
        Some(x) if truthy(x) => x.as_str().map(|s| Some(s.to_string())).ok_or_else(|| HttpError {
            message: format!("{nama} bukan string"),
        }),
        _ => Ok(None),
    }
}

/// `describeConvertFailure(convert)` (`ytdl.js:12-18`) tanpa argumen HTTP.
fn deskripsi_gagal(convert: &Value) -> String {
    let kode = convert.get("error");
    let kosong = match kode {
        None | Some(Value::Null) => true,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s == "0",
        Some(_) => false,
    };
    if kosong {
        return GAGAL_KONVERSI.to_string();
    }
    format!(
        "{GAGAL_KONVERSI} (upstream error {})",
        teks_js(kode.unwrap_or(&Value::Null))
    )
}

/// `describeConvertFailure(convert)` — diekspor JS; tidak melakukan permintaan
/// apa pun, tapi tetap menerima `&dyn HttpGet` supaya semua scraper punya
/// bentuk argumen yang sama.
pub fn describe_convert_failure(_http: &dyn HttpGet, convert: &Value) -> String {
    deskripsi_gagal(convert)
}

/// `ytdl(url, format = "mp3")` tanpa `try/catch` — `ytdl_inti` yang dibungkus.
fn ytdl_inti(http: &dyn HttpGet, url: &str, format: Option<&str>) -> Result<Value, HttpError> {
    let Some(video_id) = ambil_id_youtube(url) else {
        return Ok(json!({
            "status": false,
            "mess": "Format URL tidak dikenali atau bukan link YouTube yang valid.",
        }));
    };

    // `String(format || "mp3")` — string kosong ikut jatuh ke bawaan.
    let format = format.filter(|f| !f.is_empty()).unwrap_or("mp3");
    let normalized = if format.eq_ignore_ascii_case("mp4") {
        "mp4"
    } else {
        "mp3"
    };

    let init_url = tambah_query(
        INIT_URL,
        &[
            ("p", "y".to_string()),
            ("23", "1llum1n471".to_string()),
            ("_", acak()),
        ],
    );
    let init = http
        .call(&header_klient(HttpCall::get(&init_url)))?
        .json()
        .unwrap_or(Value::Null);
    let convert_url = match ambil_url(init.get("convertURL"), "init.convertURL")? {
        Some(u) => u,
        None => {
            return Ok(json!({
                "status": false,
                "mess": "Gagal menginisialisasi server (Init failed).",
            }));
        }
    };

    let convert_url = tambah_query(
        &convert_url,
        &[
            ("v", video_id),
            ("f", normalized.to_string()),
            ("_", acak()),
        ],
    );
    let convert = http
        .call(&header_klient(HttpCall::get(&convert_url)))?
        .json()
        .unwrap_or(Value::Null);
    let progress_url = ambil_url(convert.get("progressURL"), "convert.progressURL")?;
    let download_url = ambil_url(convert.get("downloadURL"), "convert.downloadURL")?;
    let (Some(progress_url), Some(download_url)) = (progress_url, download_url) else {
        return Ok(json!({
            "status": false,
            "mess": deskripsi_gagal(&convert),
        }));
    };

    let mut progress = 0.0f64;
    let mut title = match convert.get("title") {
        Some(t) if truthy(t) => t.clone(),
        _ => Value::String(String::new()),
    };
    let mut attempts = 0;
    let max_attempts = 20;

    while progress < 3.0 && attempts < max_attempts {
        let data = http
            .call(&header_klient(HttpCall::get(&progress_url)))?
            .json()
            .unwrap_or(Value::Null);
        // `(data?.error || 0) > 0` — `|| 0` dulu, baru dibandingkan.
        if angka_js(data.get("error").filter(|v| truthy(v))) > 0.0 {
            return Ok(json!({
                "status": false,
                "mess": format!("Error dari server: {}", teks_js(data.get("error").unwrap_or(&Value::Null))),
            }));
        }
        progress = angka_js(data.get("progress"));
        if let Some(t) = data.get("title").filter(|v| truthy(v)) {
            title = t.clone();
        }
        if progress < 3.0 {
            attempts += 1;
            std::thread::sleep(Duration::from_millis(250));
        }
    }

    if attempts >= max_attempts && progress < 3.0 {
        return Ok(json!({
            "status": false,
            "mess": "Request timeout (proses terlalu lama).",
        }));
    }

    let mut out = Map::new();
    out.insert("status".to_string(), Value::Bool(true));
    out.insert("title".to_string(), title);
    out.insert("dl".to_string(), Value::String(download_url));
    Ok(Value::Object(out))
}

/// `ytdl(url, format = "mp3")` (`ytdl.js:20-108`) — `catch` mengubah semua
/// kegagalan jadi `{ status: false, mess: "System Error: …" }`.
pub fn ytdl(http: &dyn HttpGet, url: &str, format: Option<&str>) -> Result<Value, HttpError> {
    match ytdl_inti(http, url, format) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({
            "status": false,
            "mess": format!("System Error: {}", e.message),
        })),
    }
}

/// `new Youtube().download(url, format = "audio")` (`ytdl.js:110-143`) —
/// `wrapResponse` digabung ke sini karena hanya dipakai sekali.
pub fn youtube_download(
    http: &dyn HttpGet,
    url: &str,
    format: Option<&str>,
) -> Result<Value, HttpError> {
    let format = format.filter(|f| !f.is_empty()).unwrap_or("audio");
    let output_format = if matches!(format.to_lowercase().as_str(), "video" | "mp4") {
        "mp4"
    } else {
        "mp3"
    };
    let result = ytdl(http, url, Some(output_format))?;

    let ada = result.get("status").map(truthy).unwrap_or(false)
        && result.get("dl").map(truthy).unwrap_or(false);
    if !ada {
        let mess = match result.get("mess") {
            Some(m) if truthy(m) => teks_js(m),
            _ => "Gagal mengunduh konten YouTube".to_string(),
        };
        return Err(HttpError { message: mess });
    }

    let dl = result.get("dl").cloned().unwrap_or(Value::Null);
    let mut results = Map::new();
    // `title: result.title` — `undefined` tidak ikut di `JSON.stringify`.
    if let Some(t) = result.get("title") {
        results.insert("title".to_string(), t.clone());
    }
    results.insert("download".to_string(), dl.clone());
    results.insert("url".to_string(), dl);
    results.insert(
        "format".to_string(),
        Value::String(output_format.to_string()),
    );

    let mut out = Map::new();
    out.insert("created_by".to_string(), Value::String(CREATED_BY.to_string()));
    out.insert("note".to_string(), Value::String(CATATAN.to_string()));
    out.insert("results".to_string(), Value::Object(results));
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` dengan antrean balasan: scraper ini memanggil beberapa
    /// URL berbeda dalam satu alur, jadi balasan tidak bisa satu goolector.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<String>>,
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
            let mut antrean = self.balasan.lock().unwrap();
            let isi = if antrean.is_empty() {
                String::new()
            } else {
                antrean.remove(0)
            };
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(isi.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(daftar: &[&str]) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(daftar.iter().map(|s| (*s).to_string()).collect()),
        }
    }

    #[test]
    fn alur_sukses_menampilkan_judul_dan_url_unduhan() {
        let http = stub(&[
            r#"{"convertURL":"https://d.ymcdn.org/api/v1/convert?token=abc"}"#,
            r#"{"progressURL":"https://d.ymcdn.org/api/v1/progress?token=abc",
                "downloadURL":"https://d.ymcdn.org/dl/lagu.mp3","title":"Lagu"}"#,
            r#"{"progress":1,"title":"Lagu YT"}"#,
            r#"{"progress":3,"title":"Lagu YT"}"#,
        ]);
        let hasil = ytdl(&http, "https://youtu.be/dQw4w9WgXcQ", None).unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["title"], "Lagu YT");
        assert_eq!(hasil["dl"], "https://d.ymcdn.org/dl/lagu.mp3");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 4);
        assert!(p[0].url.starts_with("https://d.ymcdn.org/api/v1/init?p=y&23=1llum1n471&_="));
        // `convertURL` dari fixture sudah punya `?`, jadi axios `buildURL`
        // menambah `&` — bukan `?` kedua.
        assert!(
            p[1].url.starts_with(
                "https://d.ymcdn.org/api/v1/convert?token=abc&v=dQw4w9WgXcQ&f=mp3&_="
            ),
            "p1 = {}",
            p[1].url
        );
        assert_eq!(p[3].url, "https://d.ymcdn.org/api/v1/progress?token=abc");
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("referer").as_deref(), Some(REFERER));
        assert_eq!(cari("user-agent").as_deref(), Some(UA));
    }

    #[test]
    fn url_bukan_youtube_mengembalikan_mess_tetap() {
        let http = stub(&[]);
        let hasil = ytdl(&http, "https://vimeo.com/12345", None).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(
            hasil["mess"],
            "Format URL tidak dikenali atau bukan link YouTube yang valid."
        );
        assert!(http.permintaan.lock().unwrap().is_empty(), "tidak ada request");
    }

    #[test]
    fn init_gagal_dan_konversi_gagal_punya_teks_masing_masing() {
        let tanpa_init = stub(&[r#"{"convertURL":""}"#]);
        let hasil = ytdl(&tanpa_init, "https://youtu.be/dQw4w9WgXcQ", None).unwrap();
        assert_eq!(hasil["mess"], "Gagal menginisialisasi server (Init failed).");

        let tanpa_url = stub(&[r#"{"convertURL":"https://konv/convert"}"#]);
        let hasil = ytdl(&tanpa_url, "https://youtu.be/dQw4w9WgXcQ", None).unwrap();
        assert_eq!(hasil["mess"], "Gagal mendapatkan data konversi.");

        // `error: "0"` (string) tetap dianggap gagal; `error: 5` memakai
        // keterangan upstream. `describeConvertFailure` sendiri tidak
        // melakukan permintaan apa pun.
        assert_eq!(deskripsi_gagal(&json!({"error": "0"})), GAGAL_KONVERSI);
        assert_eq!(deskripsi_gagal(&json!({"error": null})), GAGAL_KONVERSI);
        assert_eq!(deskripsi_gagal(&json!({})), GAGAL_KONVERSI);
        assert_eq!(
            deskripsi_gagal(&json!({"error": 5})),
            "Gagal mendapatkan data konversi. (upstream error 5)"
        );
        assert_eq!(
            describe_convert_failure(&oktz_plugin_api::NoHttp, &json!({"error": "9"})),
            "Gagal mendapatkan data konversi. (upstream error 9)"
        );
    }

    #[test]
    fn error_dari_server_dan_kegagalan_jaringan_ikut_format_js() {
        let http = stub(&[
            r#"{"convertURL":"https://konv/convert"}"#,
            r#"{"progressURL":"https://konv/p","downloadURL":"https://konv/d","title":"x"}"#,
            r#"{"error":3}"#,
        ]);
        let hasil = ytdl(&http, "https://youtu.be/dQw4w9WgXcQ", Some("MP4")).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["mess"], "Error dari server: 3");
        // Format `mp4` diteruskan apa adanya ke upstream.
        assert!(http.permintaan.lock().unwrap()[1].url.contains("&f=mp4&_="));

        // Balasan bukan JSON = `data` berupa string di axios → `convertURL`
        // kosong → init gagal.
        let bukan_json = stub(&["bukan json sama sekali"]);
        let hasil = ytdl(&bukan_json, "https://youtu.be/dQw4w9WgXcQ", None).unwrap();
        assert_eq!(hasil["mess"], "Gagal menginisialisasi server (Init failed).");
    }

    #[test]
    fn youtube_download_membungkus_dengan_created_by_dan_note() {
        let http = stub(&[
            r#"{"convertURL":"https://konv/convert"}"#,
            r#"{"progressURL":"https://konv/p","downloadURL":"https://konv/d.mp4","title":"Vidi"}"#,
            r#"{"progress":3}"#,
        ]);
        let hasil = youtube_download(&http, "https://youtu.be/dQw4w9WgXcQ", Some("video")).unwrap();
        assert_eq!(hasil["created_by"], "Ditzzy");
        assert_eq!(hasil["note"], "Thank you for using this scrape");
        assert_eq!(hasil["results"]["title"], "Vidi");
        assert_eq!(hasil["results"]["download"], "https://konv/d.mp4");
        assert_eq!(hasil["results"]["url"], "https://konv/d.mp4");
        assert_eq!(hasil["results"]["format"], "mp4");

        // Kegagalan `ytdl` diteruskan sebagai `Err` dengan teks `mess`.
        let gagal = stub(&[r#"{}"#]);
        let err = youtube_download(&gagal, "https://vimeo.com/1", None).unwrap_err();
        assert_eq!(
            err.message,
            "Format URL tidak dikenali atau bukan link YouTube yang valid."
        );
    }
}