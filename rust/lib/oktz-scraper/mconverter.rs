//! `src/scraper/mconverter.js` — konversi berkas lewat mconverter.eu: init
//! session, unggah bertahap 10 MB, lalu polling sampai `finished`.
//!
//! Yang ikut disalin dari JS:
//! - Header tetap `CONFIG.HEADERS` (User-Agent, Origin, Referer,
//!   X-Requested-With) untuk semua permintaan AJAX; `initSession` Setting
//!   `"X-Requested-With": undefined` berarti header itu TIDAK dikirim pada
//!   GET beranda.
//! - `mime.lookup(inputPath) || "application/octet-stream"` — `mime.lookup`
//!   mengembalikan `false` untuk ekstensi tak dikenal.
//! - Unggah pertama hanya 1 byte (`start: 0, end: 0`), lalu potongan 10 MB
//!   mulai byte 1. `awaiting_chunks` yang falsy menghentikan loop lebih awal.
//! - Polling maksimal 100 kali dengan `await sleep(2000)` DI AWAL tiap
//!   percobaan; `status === "error"` melempar `error_reason` (atau
//!   `"Unknown error"`), `status === "finished"` mengembalikan URL unduhan.
//! - Kegagalan apa pun setelah file ada dikembalikan sebagai
//!   `{ error: e.message }`, bukan dilempar — hanya "File tidak ada" yang
//!   kembali tanpa menyentuh jaringan.
//!
//! Beda yang diketahui (bukan hal yang bisa disembunyikan):
//! - **Cookie jar disederhanakan.** `tough-cookie` + `axios-cookiejar-support`
//!   menyimpan atribut lengkap (domain, path, kedaluwarsa) dan memilih cookie
//!   per domain. Di sini cookie disimpan sebagai pasangan `k=v` per nama lalu
//!   dikirim sebagai satu header `k=v; k=v`, tanpa atribut.
//!
//!   Persisnya: satu cookie per nama (nilai terbaru menimpa), seluruhnya dikirim
//!   ke semua permintaan scraper ini — semuanya juga hanya ke `mconverter.eu`.
//!   Perbedaan nyata hanya cookie bertanda `Expires` yang sudah kedaluwarsa.
//! - **`sessionInitialized` jadi per-panggilan.** JS memakai flag modul jadi
//!   beranda hanya diambil sekali per proses; port ini mengambilnya di awal tiap
//!   panggilan karena crate ini tidak punya state global. Jalur jaringan jadi
//!   satu permintaan ekstra per pemanggilan.
//! - `fs.createReadStream(..., {start, end})` jadi potongan dari
//!   `std::fs::read`; seluruh isi berkas masuk memori. Tidak ada bedanya pada
//!   isi yang dikirim.
//! - `localeCompare` tidak ada di Rust; urutan target memakai perbandingan
//!   huruf-kecil lalu huruf-asing sebagai pemutus seri (lihat `urut_nama`).

use std::sync::Arc;

use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, HttpResponse, MultipartPart};
use serde_json::{Map, Value, json};

use crate::common::{field, truthy};

const BASE_URL: &str = "https://mconverter.eu";
const HOME: &str = "https://mconverter.eu/";
const UPLOAD: &str = "/cf_nocache/ajax/upload.php";
const PROGRESS: &str = "/cf_nocache/ajax/check_progress.php";
const GET_TARGETS: &str = "/cf_nocache/ajax/get_targets.php";
const DOWNLOAD: &str = "/cf_nocache/ajax/download.php";

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

const CHUNK_SIZE: u64 = 10 * 1024 * 1024;
const PERCOBAAN: u32 = 100;

/// Cookie jar disederhanakan — lihat catatan bedanya di kepala file.
#[derive(Debug, Default)]
struct CookieSederhana {
    pasangan: Vec<(String, String)>,
}

impl CookieSederhana {
    /// Serap seluruh header `set-cookie` dari satu respons.
    fn serap(&mut self, res: &HttpResponse) {
        for (nama, nilai) in &res.headers {
            if !nama.eq_ignore_ascii_case("set-cookie") {
                continue;
            }
            let pasangan = nilai.split(';').next().unwrap_or("").trim();
            let Some((k, v)) = pasangan.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim().to_string(), v.trim().to_string());
            match self.pasangan.iter_mut().find(|(nama, _)| *nama == k) {
                Some(slot) => slot.1 = v,
                None => self.pasangan.push((k, v)),
            }
        }
    }

    /// Nilai header `cookie`, atau `None` kalau jar masih kosong.
    fn header(&self) -> Option<String> {
        if self.pasangan.is_empty() {
            return None;
        }
        Some(
            self.pasangan
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("; "),
        )
    }
}

/// `CONFIG.HEADERS` — header yang dikirim ke setiap endpoint AJAX.
fn header_ajax(req: HttpCall, jar: &CookieSederhana) -> HttpCall {
    let req = req
        .header("User-Agent", UA)
        .header("Origin", BASE_URL)
        .header("Referer", HOME)
        .header("X-Requested-With", "XMLHttpRequest");
    match jar.header() {
        Some(k) => req.header("cookie", k),
        None => req,
    }
}

/// `new URLSearchParams({...}).toString()` — spasi `+`, hanya `*-._` lolos.
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

/// `initSession()` (`mconverter.js:43-58`) — GET beranda,umbnail cookie
/// sekalian. Kegagalan diabaikan (`catch` hanya `console.log`).
fn init_session(http: &dyn HttpGet, jar: &mut CookieSederhana) {
    let req = HttpCall::get(HOME)
        .header("User-Agent", UA)
        .header("Origin", BASE_URL)
        .header("Referer", HOME)
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Sec-Fetch-Mode", "navigate");
    if let Ok(res) = http.call(&req) {
        jar.serap(&res);
    }
}

/// `mime.lookup(path)` — ekstensi terakhir, huruf kecil. `None` berarti
/// `false` di JS, jadi pemanggil memakai `application/octet-stream`.
///
/// Daftar ini hanya format yang masuk `SUPPORTED_SOURCES` dan lazim dipakai;
/// `mime-types` asli punya database jauh lebih besar. Lihat laporan.
fn mime_lookup(path: &str) -> Option<&'static str> {
    let ext = path.rsplit('.').next()?.to_lowercase();
    Some(match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "heic" => "image/heic",
        "pdf" => "application/pdf",
        "psd" => "image/vnd.adobe.photoshop",
        "ai" | "eps" | "ps" => "application/postscript",
        "mp3" => "audio/mpeg",
        "m4a" | "aac" => "audio/mp4",
        "flac" => "audio/flac",
        "wav" => "audio/wav",
        "ogg" | "opus" => "audio/ogg",
        "wma" => "audio/x-ms-wma",
        "mid" => "audio/midi",
        "amv" => "video/amv",
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "flv" => "video/x-flv",
        "3gp" | "3gpp" => "video/3gpp",
        "mpeg" | "mpg" => "video/mpeg",
        "tsv" => "text/tab-separated-values",
        "csv" => "text/csv",
        "txt" | "srt" | "sub" | "vtt" => "text/plain",
        "md" | "markdown" => "text/markdown",
        "html" | "htm" => "text/html",
        "json" => "application/json",
        "rtf" => "application/rtf",
        "xml" => "text/xml",
        "epub" => "application/epub+zip",
        "fb2" => "application/x-fictionbook+xml",
        "zip" => "application/zip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        "bz2" => "application/x-bzip2",
        "xz" => "application/x-xz",
        "iso" => "application/x-iso9660-image",
        "jar" => "application/java-archive",
        "apk" => "application/vnd.android.package-archive",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "odp" => "application/vnd.oasis.opendocument.presentation",
        "ipynb" => "application/x-ipynb+json",
        "cab" => "application/vnd.ms-cab-compressed",
        "cbr" | "cbz" => "application/vnd.comicbook+zip",
        "cso" => "application/x-cso",
        _ => return None,
    })
}

/// `path.basename(p)` — komponen terakhir setelah `/`.
fn basename(p: &str) -> String {
    p.rsplit('/').next().unwrap_or(p).to_string()
}

/// Part `form.append("file", stream)` — stream tanpa opsi filename, jadi
/// `form-data` mengirim `application/octet-stream` dan TANPA `filename=`.
fn part_berkas(nama: &str, isi: Vec<u8>) -> MultipartPart {
    MultipartPart {
        name: nama.to_string(),
        file_name: None,
        mimetype: Some("application/octet-stream".to_string()),
        body: HttpBody::Bytes {
            data: Arc::from(isi),
            mimetype: None,
        },
    }
}

/// `a.name.localeCompare(b.name)` — tidak ada padanan persis di Rust.
/// Huruf-kecil dulu supaya `MP4` dan `mp4` tidak jauh, huruf-asing sebagai
/// pemutus seri supaya urutan tetap total (stabil).
fn urut_nama(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

/// Kumpulkan objek yang punya `name` dan `mime` truthy dari pohon apa pun
/// (`extractTargets`, `mconverter.js:76-85`). Array diproses berurutan.
fn kumpulkan_target(node: &Value, keluar: &mut Vec<Value>) {
    match node {
        Value::Array(a) => {
            for item in a {
                kumpulkan_target(item, keluar);
            }
        }
        Value::Object(o) => {
            let ada_nama = o.get("name").map(truthy).unwrap_or(false);
            let ada_mime = o.get("mime").map(truthy).unwrap_or(false);
            if ada_nama && ada_mime {
                keluar.push(node.clone());
            }
            for v in o.values() {
                kumpulkan_target(v, keluar);
            }
        }
        _ => {}
    }
}

/// `getAvailableTargets(filename)` (`mconverter.js:62-98`).
///
/// Balikan `array` target; `catch` di JS mengembalikan `[]`, jadi kegagalan
/// juga `Ok([])` — bukan `Err`.
pub fn get_available_targets(
    http: &dyn HttpGet,
    filename: &str,
) -> Result<Value, HttpError> {
    let mut jar = CookieSederhana::default();
    init_session(http, &mut jar);

    // `filename.split('.').pop().toLowerCase()`
    let ext = filename.rsplit('.').next().unwrap_or(filename).to_lowercase();

    let res = http.call(
        &header_ajax(HttpCall::post(format!("{BASE_URL}{GET_TARGETS}")), &jar).multipart(vec![
            MultipartPart::field("extensions", ext.clone()),
        ]),
    );

    let Ok(res) = res else {
        // `catch` JS: `console.error` lalu `return []`.
        return Ok(json!([]));
    };
    jar.serap(&res);

    let mut semua: Vec<Value> = Vec::new();
    if let Some(formats) = res.json().and_then(|d| d.get("formats").cloned()) {
        kumpulkan_target(&formats, &mut semua);
    }

    // `new Map(...)` menyimpan urutan kemunculan pertama tetapi nilai TERAKHIR
    // untuk kunci yang sama — jadi entri sama menimpa isinya, bukan diduplikasi.
    let mut unik: Vec<Value> = Vec::new();
    for item in semua {
        let Some(nama) = item.get("name").and_then(Value::as_str) else {
            continue;
        };
        match unik
            .iter_mut()
            .find(|s| s.get("name").and_then(Value::as_str) == Some(nama))
        {
            Some(slot) => *slot = item,
            None => unik.push(item),
        }
    }
    unik.sort_by(|a, b| {
        let ka = a.get("name").and_then(Value::as_str).unwrap_or("");
        let kb = b.get("name").and_then(Value::as_str).unwrap_or("");
        urut_nama(ka, kb)
    });

    Ok(Value::Array(unik))
}

/// `convert(inputPath, targetFormat)` (`mconverter.js:100-202`).
///
/// `inputPath` path lokal seperti di JS; seluruh isinya dibaca ke memori lalu
/// dipecah jadi potongan yang persis sama dengan `createReadStream`.
pub fn convert(
    http: &dyn HttpGet,
    input_path: &str,
    target_format: &str,
) -> Result<Value, HttpError> {
    // `await initSession()` berjalan SEBELUM cek berkas di JS, jadi beranda
    // tetap diambil walau berkasnya tidak ada.
    let mut jar = CookieSederhana::default();
    init_session(http, &mut jar);

    if !std::path::Path::new(input_path).exists() {
        return Ok(json!({ "error": "File tidak ada" }));
    }
    match alur(http, input_path, target_format, &mut jar) {
        Ok(v) => Ok(v),
        // `catch` JS mengembalikan `{ error: e.message }`.
        Err(e) => Ok(json!({ "error": e.message })),
    }
}

/// Badan `convert` tanpa pembungkus `catch`.
fn alur(
    http: &dyn HttpGet,
    input_path: &str,
    target_format: &str,
    jar: &mut CookieSederhana,
) -> Result<Value, HttpError> {
    let isi = std::fs::read(input_path).map_err(|e| HttpError {
        message: e.to_string(),
    })?;
    let ukuran = isi.len() as u64;
    let filename = basename(input_path);
    let mime_type = mime_lookup(input_path).unwrap_or("application/octet-stream");

    // Unggah pertama: hanya byte 0 (`start: 0, end: 0`).
    let params = format!(
        "target_format={}&total_size={}&source_mime={}&filename={}&abd=false&captcha=",
        encode_form(target_format),
        ukuran,
        encode_form(mime_type),
        encode_form(&filename),
    );
    let res = http.call(
        &header_ajax(HttpCall::post(format!("{BASE_URL}{UPLOAD}?{params}")), jar)
            .multipart(vec![part_berkas("file", isi.iter().take(1).copied().collect())]),
    )?;
    jar.serap(&res);
    let data = res.json().unwrap_or(Value::Null);
    if let Some(pesan) = field(&data, "error").filter(|v| truthy(v)) {
        return Err(HttpError {
            message: field(pesan, "message")
                .map(teks_js)
                .unwrap_or_default(),
        });
    }
    let token = data.get("token").cloned().unwrap_or(Value::Null);
    if !truthy(&token) {
        return Err(HttpError {
            message: "Gagal dapet token upload".to_string(),
        });
    }
    let token = teks_js(&token);

    // Potongan 10 MB mulai byte 1.
    let mut start_byte: u64 = 1;
    while start_byte < ukuran {
        let end_byte = (start_byte + CHUNK_SIZE).min(ukuran);
        let potong = isi[start_byte as usize..end_byte as usize].to_vec();
        let params = format!("token={}&start_byte={}", encode_form(&token), start_byte);
        let res = http.call(
            &header_ajax(HttpCall::post(format!("{BASE_URL}{UPLOAD}?{params}")), jar)
                .multipart(vec![part_berkas("file", potong)]),
        )?;
        jar.serap(&res);
        let data = res.json().unwrap_or(Value::Null);
        if let Some(pesan) = field(&data, "error").filter(|v| truthy(v)) {
            return Err(HttpError {
                message: field(pesan, "message")
                    .map(teks_js)
                    .unwrap_or_default(),
            });
        }
        if !data.get("awaiting_chunks").map(truthy).unwrap_or(false) {
            break;
        }
        start_byte = end_byte;
    }

    // Polling: `await sleep(2000)` di awal tiap percobaan.
    let url_progress = format!("{BASE_URL}{PROGRESS}?token={token}");
    for _ in 0..PERCOBAAN {
        jeda_polling();
        let res = http.call(&header_ajax(HttpCall::get(&url_progress), jar))?;
        jar.serap(&res);
        let data = res.json().unwrap_or(Value::Null);
        let status = field(&data, "conversion_data.status")
            .and_then(Value::as_str)
            .unwrap_or("");

        if status == "error" {
            return Err(HttpError {
                message: field(&data, "error_data.error_reason")
                    .filter(|v| truthy(v))
                    .map(teks_js)
                    .unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        if status == "finished" {
            let dl = format!(
                "token={}&file_idx=1&no_idx_in_name=1&orig_names=checked",
                encode_form(&token)
            );
            let mut out = Map::new();
            out.insert("success".to_string(), Value::Bool(true));
            out.insert(
                "url".to_string(),
                json!(format!("{BASE_URL}{DOWNLOAD}?{dl}")),
            );
            return Ok(Value::Object(out));
        }
    }

    Ok(json!({ "error": "Timeout waiting for conversion" }))
}

/// `String(v)` untuk nilai JSON.
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

/// `await new Promise(r => setTimeout(r, 2000))`.
#[cfg(not(test))]
fn jeda_polling() {
    std::thread::sleep(std::time::Duration::from_millis(2000));
}

/// Di test tidak menunggu — perilaku selain jeda tetap sama.
#[cfg(test)]
fn jeda_polling() {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// `(status, body, headers)` untuk stub HTTP — alias biar tipe
    /// balancer-nya tidak memicu `clippy::type_complexity`.
    type Balasan = (u16, String, Vec<(String, String)>);

    /// Stub `HttpGet` tanpa jaringan: membalas berdasar URL persis dan
    /// menyimpan header respons (dipakai untuk menguji cookie).
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<HashMap<String, Balasan>>,
    }

    impl StubHttp {
        fn baru(pasangan: &[(&str, &str)]) -> Self {
            let mut map = HashMap::new();
            for (k, v) in pasangan {
                map.insert(k.to_string(), (200u16, v.to_string(), Vec::new()));
            }
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(map),
            }
        }

        fn jawab(&self, url: &str) -> Result<HttpResponse, HttpError> {
            match self.balasan.lock().unwrap().get(url) {
                Some((status, body, headers)) => Ok(HttpResponse {
                    status: *status,
                    body: HttpBody::Bytes {
                        data: Arc::from(body.as_bytes()),
                        mimetype: None,
                    },
                    headers: headers.clone(),
                }),
                None => {
                    Err(HttpError {
                        message: format!("tidak ada balasan untuk {url}"),
                    })
                }
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(HttpCall::get(url));
            self.jawab(url)
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("mconverter lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("mconverter lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("mconverter lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            self.jawab(&req.url)
        }
    }

    fn cari_header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    /// Berkas uji di temp dir. Nomor urut dipakai supaya dua test dengan nama
/// yang sama tidak saling menimpa isi (test jalan paralel).
    fn nama_berkas_sementara(nama: &str, isi: &[u8]) -> std::path::PathBuf {
        static URUT: AtomicUsize = AtomicUsize::new(0);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "oktz-test-{}-{}-{nama}",
            std::process::id(),
            URUT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&path, isi).expect("tulis berkas uji");
        path
    }

    /// URL unggah pertama persis seperti yang dibangun `alur`.
    ///
    /// `filename` di query diambil dari `path.basename(path)` berkas yang
    /// BENAR-BENAR ada, dan berkas uji tinggal di temp dir dengan prefix
    /// `oktz-test-<pid>-<n>-`. Jadi nama di fixture harus dihitung dari
    /// `path`, bukan dari nama yang diminta test — kalau tidak, stub tidak
    /// pernah kena dan hasilnya "tidak ada balasan untuk ...".
    fn url_unggah_awal(
        path: &std::path::Path,
        target: &str,
        mime: &str,
        ukuran: usize,
    ) -> String {
        let nama = path.file_name().unwrap().to_string_lossy();
        format!(
            "{BASE_URL}{UPLOAD}?target_format={}&total_size={ukuran}&source_mime={}&filename={}&abd=false&captcha=",
            encode_form(target),
            encode_form(mime),
            encode_form(&nama),
        )
    }

    #[test]
    fn targets_diekstrak_dari_pohon_format_dan_disoret() {
        let http = StubHttp::baru(&[(
            "https://mconverter.eu/cf_nocache/ajax/get_targets.php",
            r#"{"formats":{"images":[{"name":"PNG","mime":"image/png"},
                                     {"name":"jpg","mime":"image/jpeg"}],
                       "other":{"name":"PNG","mime":"image/png","x":1},
                       "bukan":{"name":"","mime":"image/gif"}}}"#,
        )]);
        let hasil = get_available_targets(&http, "foto.JPG").unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 2, "duplikat nama hilang, yang tanpa nama dibuang");
        assert_eq!(arr[0]["name"], "jpg", "urutan locale: jpg sebelum PNG");
        assert_eq!(arr[1]["name"], "PNG");
        // `new Map` menyimpan nilai TERAKHIR untuk kunci kembar pada posisi
        // kemunculan pertama, jadi entri PNG adalah objek `other` yang punya `x`.
        assert_eq!(arr[1]["mime"], "image/png");
        assert_eq!(arr[1]["x"], 1, "penimpaan pakai objek terbaru");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2, "init session lalu get_targets");
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(p[0].url, HOME);
        assert!(
            cari_header(&p[0], "x-requested-with").is_none(),
            "X-Requested-With undefined tidak dikirim"
        );
        assert_eq!(cari_header(&p[0], "sec-fetch-mode"), Some("navigate".to_string()));

        let Some(HttpBody::Multipart(parts)) = &p[1].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "extensions");
        assert_eq!(parts[0].body.bytes(), b"jpg", "ekstensi huruf kecil");
        assert_eq!(cari_header(&p[1], "x-requested-with"), Some("XMLHttpRequest".to_string()));
        assert_eq!(cari_header(&p[1], "origin"), Some(BASE_URL.to_string()));
    }

    #[test]
    fn target_gagal_mengembalikan_daftar_kosong_bukan_error() {
        let http = StubHttp::baru(&[]);
        let hasil = get_available_targets(&http, "a.zip").unwrap();
        assert_eq!(hasil, json!([]));
    }

    #[test]
    fn konversi_satu_byte_mengambil_token_dan_url_unduhan() {
        let path = nama_berkas_sementara("kucing.png", b"ABCDEFGH");
        let up = url_unggah_awal(&path, "jpg", "image/png", 8);
        let http = StubHttp::baru(&[
            (up.as_str(), r#"{"token":"TOK"}"#),
            // Potongan sisanya: `awaiting_chunks` falsy menghentikan loop.
            (
                "https://mconverter.eu/cf_nocache/ajax/upload.php?token=TOK&start_byte=1",
                r#"{"awaiting_chunks":false}"#,
            ),
            (
                "https://mconverter.eu/cf_nocache/ajax/check_progress.php?token=TOK",
                r#"{"conversion_data":{"status":"finished"}}"#,
            ),
        ]);

        let hasil = convert(&http, path.to_str().unwrap(), "jpg").unwrap();
        assert_eq!(hasil["success"], true);
        assert_eq!(
            hasil["url"],
            "https://mconverter.eu/cf_nocache/ajax/download.php?token=TOK&file_idx=1&no_idx_in_name=1&orig_names=checked"
        );

        let p = http.permintaan.lock().unwrap();
        // Urutan JS: beranda (cookie) → unggah 1 byte → potongan → polling.
        assert_eq!(p.len(), 4, "beranda, unggah awal, 1 potongan, 1 polling");
        assert_eq!(p[0].url, HOME, "permintaan pertama ambil cookie");
        let unggah = &p[1];
        assert_eq!(unggah.url, up, "kunci stub sama persis dengan URL nyata");
        let Some(HttpBody::Multipart(parts)) = &unggah.body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts[0].name, "file");
        assert_eq!(parts[0].file_name, None, "createReadStream tanpa filename");
        assert_eq!(parts[0].body.bytes(), b"A", "hanya 1 byte pada unggah pertama");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn token_kosong_mengembalikan_pesan_js() {
        let path = nama_berkas_sementara("a.bin", b"xx");
        let up = url_unggah_awal(&path, "zip", "application/octet-stream", 2);
        let http = StubHttp::baru(&[(up.as_str(), r#"{"ok":true}"#)]);
        let hasil = convert(&http, path.to_str().unwrap(), "zip").unwrap();
        assert_eq!(hasil["error"], "Gagal dapet token upload");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn error_dari_server_dipakai_pesan_error_message() {
        let path = nama_berkas_sementara("a.bin", b"abc");
        let up = url_unggah_awal(&path, "jpg", "application/octet-stream", 3);
        let http = StubHttp::baru(&[(
            up.as_str(),
            r#"{"error":{"message":"format tidak didukung"}}"#,
        )]);
        let hasil = convert(&http, path.to_str().unwrap(), "jpg").unwrap();
        assert_eq!(hasil["error"], "format tidak didukung");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn status_error_mengakai_error_reason_lalu_unknown_error() {
        let path = nama_berkas_sementara("b.bin", b"abc");
        let up = url_unggah_awal(&path, "jpg", "application/octet-stream", 3);
        let poll = "https://mconverter.eu/cf_nocache/ajax/check_progress.php?token=T";
        let http = StubHttp::baru(&[
            (up.as_str(), r#"{"token":"T"}"#),
            (
                "https://mconverter.eu/cf_nocache/ajax/upload.php?token=T&start_byte=1",
                r#"{"awaiting_chunks":false}"#,
            ),
            (
                poll,
                r#"{"conversion_data":{"status":"error"},"error_data":{"error_reason":"server OUR"}}"#,
            ),
        ]);
        let hasil = convert(&http, path.to_str().unwrap(), "jpg").unwrap();
        assert_eq!(hasil["error"], "server OUR");

        let http = StubHttp::baru(&[
            (up.as_str(), r#"{"token":"T"}"#),
            (
                "https://mconverter.eu/cf_nocache/ajax/upload.php?token=T&start_byte=1",
                r#"{"awaiting_chunks":false}"#,
            ),
            (poll, r#"{"conversion_data":{"status":"error"}}"#),
        ]);
        let hasil = convert(&http, path.to_str().unwrap(), "jpg").unwrap();
        assert_eq!(hasil["error"], "Unknown error");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn seratus_polling_lalu_timeout() {
        let path = nama_berkas_sementara("c.bin", b"abc");
        let up = url_unggah_awal(&path, "jpg", "application/octet-stream", 3);
        let http = StubHttp::baru(&[
            (up.as_str(), r#"{"token":"T"}"#),
            (
                "https://mconverter.eu/cf_nocache/ajax/upload.php?token=T&start_byte=1",
                r#"{"awaiting_chunks":false}"#,
            ),
            (
                "https://mconverter.eu/cf_nocache/ajax/check_progress.php?token=T",
                r#"{"conversion_data":{"status":"processing"}}"#,
            ),
        ]);
        let hasil = convert(&http, path.to_str().unwrap(), "jpg").unwrap();
        assert_eq!(hasil["error"], "Timeout waiting for conversion");
        let p = http.permintaan.lock().unwrap();
        // Beranda (cookie, selalu di port) + unggah awal + 1 potongan + 100 polling.
        assert_eq!(p.len(), 103);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn berkas_tidak_ada_setelah_init_session() {
        // `initSession()` jalan lebih dulu di JS, jadi beranda tetap diambil
        // walau berkasnya tidak ada — tapi tidak ada permintaan unggah.
        let http = StubHttp::baru(&[]);
        let hasil = convert(&http, "/tmp/tidak-ada-oktz-12345.zip", "jpg").unwrap();
        assert_eq!(hasil["error"], "File tidak ada");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1, "hanya init session");
        assert_eq!(p[0].url, HOME);
    }

    #[test]
    fn cookie_dari_beranda_dikirim_ke_permintaan_berikutnya() {
        let http = StubHttp::baru(&[(
            "https://mconverter.eu/cf_nocache/ajax/get_targets.php",
            r#"{"formats":[]}"#,
        )]);
        http.balasan.lock().unwrap().insert(
            HOME.to_string(),
            (
                200,
                String::new(),
                vec![(
                    "set-cookie".to_string(),
                    "PHPSESSID=abc123; path=/; HttpOnly".to_string(),
                )],
            ),
        );
        get_available_targets(&http, "a.zip").unwrap();
        let p = http.permintaan.lock().unwrap();
        let cookie = cari_header(&p[1], "cookie").expect("cookie terkirim");
        assert_eq!(cookie, "PHPSESSID=abc123", "atribut di buang");
    }
}