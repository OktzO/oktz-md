//! `src/scraper/hdvid2.js` — unggah video ke `api.unwatermark.ai`, buat job
//! penajam, lalu polling sampai selesai.
//!
//! Yang disalin dari JS:
//! - Tiga permintaan semuanya memakai `validateStatus: () => true`, jadi status
//!   4xx/5xx **tidak** jadi error dan body-nya tetap dibaca —
//!   `.terima_semua_status()` di ketiga permintaan.
//! - `form.append(key, value)` tanpa opsi berkas, jadi `MultipartPart::field`.
//!   `post_multipart` lama menamai part dengan NAMA FIELD, bukan `filename`,
//!   jadi tidak boleh dipakai di sini.
//! - `form.getHeaders()` berisi `content-type` multipart + boundary; boundary
//!   dibuat klien Rust, jadi header itu tidak ikut (preseden `hd.rs`).
//! - `maxBodyLength/maxContentLength: Infinity` dan `timeout` axios adalah batas
//!   mesin, bukan tampilan — tidak ditiru.
//! - `res.data` bukan `Option`: `transformResponse` axios mencoba `JSON.parse`
//!   dan, kalau gagal (`silentJSONParsing`), mengembalikan string mentah. Jadi
//!   pesan galat yang menyisipkan body respons tidak pernah berbunyi `undefined`
//!   hanya karena body-nya bukan JSON. `data_axios()` menirukan itu.
//! - Dua ekspresi pesan galat memang BERBEDA di JS dan tidak boleh disamakan:
//!   `Gagal ambil upload url`/`Gagal create job`/`Job belum selesai` memakai
//!   `JSON.stringify(result.data)` polos, sedangkan pesan PUT memakai
//!   `typeof put.data === "string" ? put.data : JSON.stringify(put.data)`
//!   (`hdvid2.js:204`) — string mentah tidak diapit tanda kutip.
//! - `waitJob` 80 percobaan jeda 5 detik. Tidur ditiru apa adanya lewat
//!   `std::thread::sleep`; ini memblokir worker selama polling, sama seperti
//!   `imglarger.rs`.
//!
//! Beda yang disengaja: JS menerima `Buffer`, menulisnya ke `os.tmpdir()`, lalu
//! menghapusnya di `finally`; port ini menerima path berkas. Penulisan dan
//! penghapusan berkas lokal tidak ada di API scraper, dan pemanggil (plugin)
//! sudah memegang path — lihat `hd.rs`.

use std::time::Duration;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, HttpResponse, MultipartPart};
use serde_json::{Map, Value};

use crate::common::{field, truthy};

const API: &str = "https://api.unwatermark.ai";
const WEB: &str = "https://unblurimage.ai";

/// `RESOLUTION` (`hdvid2.js:12`).
const RESOLUTION: &str = "2k";
/// `IS_PREVIEW` (`hdvid2.js:13`).
const IS_PREVIEW: &str = "false";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// `videoEnhancer(video, { filename })` (`hdvid2.js:178-224`).
///
/// `file_path` = `filePath` di JS setelah `Buffer` ditulis ke tmp. Option
/// `filename` bawaan `hdvid2-<uuid>.mp4` hanya dipakai di cabang `Buffer`, jadi
/// tidak ada parameterinya di sini.
pub fn video_enhancer(http: &dyn HttpGet, file_path: &str) -> Result<Value, HttpError> {
    // `fsp.stat(filePath)` + `fs.existsSync(filePath)`
    let stat = std::fs::metadata(file_path).map_err(|_| HttpError {
        message: format!("File tidak ditemukan: {file_path}"),
    })?;
    let data = std::fs::read(file_path).map_err(|e| HttpError {
        message: e.to_string(),
    })?;

    let upload = create_upload_url(http, file_path)?;
    let Some(signed_url) = upload.get("url").and_then(Value::as_str) else {
        // Di JS `upload.url` yang `undefined` membuat `axios.put` melempar
        // "Invalid URL"; teks itu milik axios, jadi port menolak lebih awal
        // dengan alasan sendiri yang di sini dicatat sebagai beda.
        return Err(HttpError {
            message: "Gagal ambil upload url: respons tidak punya result.url".to_string(),
        });
    };
    let signed_url = signed_url.to_string();
    let public_url = clean_public_url(&signed_url);

    let put = put_file(
        http,
        &signed_url,
        &data,
        ext_to_mime(file_path),
        stat.len() as usize,
    )?;
    if put.status >= 400 {
        return Err(HttpError {
            message: format!(
                "Upload file gagal HTTP {}: {}",
                put.status,
                data_atau_teks(&put)
            ),
        });
    }

    let job = create_job(http, &public_url)?;
    let job_id = field(&job, "job_id").cloned().unwrap_or(Value::Null);
    let done = wait_job(http, &job_id)?;

    // `done.result?.output_url?.[0] || ""`
    let result_url = field(&done, "result.output_url")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .filter(|v| truthy(v))
        .cloned()
        .unwrap_or_else(|| Value::String(String::new()));

    let mut out = Map::new();
    out.insert("jobId".to_string(), job_id);
    out.insert("resultUrl".to_string(), result_url);
    Ok(Value::Object(out))
}

/// `createUploadUrl(filePath)` (`hdvid2.js:117-129`).
fn create_upload_url(http: &dyn HttpGet, file_path: &str) -> Result<Value, HttpError> {
    // `path.basename(filePath)`
    let nama = std::path::Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let res = post_form(
        http,
        "/api/web/common/upload/video",
        &[("video_file_name", nama)],
    )?;

    // `result.data` — body non-JSON jadi string, dan `("x").code` di JS juga
    // `undefined`, jadi jalur berakhir sama saja di `Value::Null`.
    let body = data_axios(&res);

    // `result.status >= 400 || result.data?.code !== 100000`
    let code_ok = field(&body, "code").and_then(Value::as_i64) == Some(100000);
    if res.status >= 400 || !code_ok {
        return Err(HttpError {
            message: format!("Gagal ambil upload url: {}", json_dari_body(&res)),
        });
    }
    Ok(field(&body, "result").cloned().unwrap_or(Value::Null))
}

/// `createJob(originalVideoUrl)` (`hdvid2.js:131-146`).
fn create_job(http: &dyn HttpGet, original_video_url: &str) -> Result<Value, HttpError> {
    let res = post_form(
        http,
        "/api/web/unblurimage/v1/video-enhancer/create-job",
        &[
            ("original_video_url", original_video_url),
            ("resolution", RESOLUTION),
            ("is_preview", IS_PREVIEW),
        ],
    )?;

    let body = data_axios(&res);

    // `result.status >= 400 || !result.data?.result?.job_id`
    let ada_job = field(&body, "result.job_id").map(truthy).unwrap_or(false);
    if res.status >= 400 || !ada_job {
        return Err(HttpError {
            message: format!("Gagal create job: {}", json_dari_body(&res)),
        });
    }
    Ok(field(&body, "result").cloned().unwrap_or(Value::Null))
}

/// `waitJob(jobId, maxTry = 80, delayMs = 5000)` (`hdvid2.js:154-176`).
fn wait_job(http: &dyn HttpGet, job_id: &Value) -> Result<Value, HttpError> {
    let max_try = 80;
    let delay_ms = 5000;
    let id = teks_dalam_template(job_id);
    // `let last = null` di JS (`hdvid2.js:155`).
    let mut last = Value::Null;

    for _ in 1..=max_try {
        let res = get_json(
            http,
            &format!("/api/web/unblurimage/v1/video-enhancer/get-job/{id}"),
        )?;
        // `last = result.data` — body non-JSON tetap string, bukan `undefined`.
        let data = data_axios(&res);
        last = data.clone();

        // `Array.isArray(outputUrl) && outputUrl.length > 0`
        if let Some(output) = field(&data, "result.output_url").and_then(Value::as_array)
            && !output.is_empty()
        {
            return Ok(data);
        }
        // `status === 1`
        if field(&data, "result.status").and_then(Value::as_i64) == Some(1) {
            return Ok(data);
        }
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    Err(HttpError {
        message: format!(
            "Job belum selesai: {}",
            serde_json::to_string(&last).unwrap_or_default()
        ),
    })
}

/// `postForm(endpoint, fields)` (`hdvid2.js:57-73`).
fn post_form(
    http: &dyn HttpGet,
    endpoint: &str,
    fields: &[(&str, &str)],
) -> Result<HttpResponse, HttpError> {
    let parts: Vec<MultipartPart> = fields
        .iter()
        .map(|(k, v)| MultipartPart::field(*k, *v))
        .collect();
    let call = base_headers(
        HttpCall::post(format!("{API}{endpoint}"))
            .multipart(parts)
            .terima_semua_status(),
    );
    http.call(&call)
}

/// `getJson(endpoint)` (`hdvid2.js:75-87`).
fn get_json(http: &dyn HttpGet, endpoint: &str) -> Result<HttpResponse, HttpError> {
    let call = base_headers(HttpCall::get(format!("{API}{endpoint}")).terima_semua_status())
        .header("content-type", "application/json; charset=UTF-8");
    http.call(&call)
}

/// `putFileToSignedUrl(uploadUrl, filePath)` (`hdvid2.js:89-107`) — body biner
/// mentah. `baseHeaders` tidak dipakai di sini, sama seperti JS.
fn put_file(
    http: &dyn HttpGet,
    upload_url: &str,
    data: &[u8],
    mime: &str,
    size: usize,
) -> Result<HttpResponse, HttpError> {
    let call = HttpCall::put(upload_url)
        .header("content-type", mime)
        .header("content-length", size.to_string())
        .terima_semua_status()
        .raw(data.to_vec(), mime);
    http.call(&call)
}

/// `baseHeaders(extra)` (`hdvid2.js:40-55`).
fn base_headers(call: HttpCall) -> HttpCall {
    call.header("accept", "*/*")
        .header("origin", WEB)
        .header("referer", format!("{WEB}/"))
        .header("user-agent", UA)
        .header("product-code", "067003")
        .header("product-serial", random_product_serial())
        .header("x-request-id", uuid_v4())
        .header("sec-ch-ua-platform", r#""Android""#)
        .header(
            "sec-ch-ua",
            r#""Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147""#,
        )
        .header("sec-ch-ua-mobile", "?1")
}

/// `randomProductSerial()` (`hdvid2.js:18-27`).
fn random_product_serial() -> String {
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..6)
        .map(|_| CHARS[rand::random_range(0..CHARS.len())] as char)
        .collect()
}

/// `crypto.randomUUID()` untuk header `x-request-id` dan nama berkas bawaan.
fn uuid_v4() -> String {
    let mut b: [u8; 16] = rand::random();
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let h: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// `cleanPublicUrl(url)` (`hdvid2.js:109-111`) — buang setelah `?`.
fn clean_public_url(url: &str) -> String {
    url.split_once('?').map_or(url, |(a, _)| a).to_string()
}

/// `extToMime(file)` (`hdvid2.js:29-38`) — hanya empat ekstensi video.
fn ext_to_mime(file: &str) -> &'static str {
    let ext = std::path::Path::new(file)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        _ => "application/octet-stream",
    }
}

/// Nilai `v` di dalam template literal JS — `undefined` jadi teksnya, bukan
/// JSON `null`.
fn teks_dalam_template(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "undefined".to_string(),
        other => other.to_string(),
    }
}

/// `res.data` versi `transformResponse` axios: `JSON.parse` dicoba lebih dulu,
/// dan kalau GAGAL respons itu tetap string mentah — body HTML atau pesan teks
/// server tidak pernah berubah jadi `undefined`.
fn data_axios(res: &HttpResponse) -> Value {
    match res.json() {
        Some(v) => v,
        None => Value::String(res.text()),
    }
}

/// `JSON.stringify(res.data)` (`hdvid2.js:125`, `:142`, `:175`) — nilai string
/// ikut diapit tanda kutip, persis seperti `JSON.stringify` JS.
fn json_dari_body(res: &HttpResponse) -> String {
    serde_json::to_string(&data_axios(res)).unwrap_or_default()
}

/// `typeof put.data === "string" ? put.data : JSON.stringify(put.data)`
/// (`hdvid2.js:204`) — string mentah dipakai apa adanya, tanpa tanda kutip.
fn data_atau_teks(res: &HttpResponse) -> String {
    match data_axios(res) {
        Value::String(s) => s,
        other => serde_json::to_string(&other).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::HttpBody;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<VecDeque<(u16, String)>>,
        dipakai: Mutex<usize>,
    }

    impl Stub {
        fn baru(balasan: Vec<(u16, String)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(VecDeque::from(balasan)),
                dipakai: Mutex::new(0),
            }
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_json(&self, _u: &str, _b: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(&self, _u: &str, _f: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _u: &str,
            _p: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut dipakai = self.dipakai.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*dipakai).min(balasan.len().saturating_sub(1));
            *dipakai += 1;
            let (status, body) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn cari_header<'a>(req: &'a HttpCall, nama: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    fn berkas_sementara(nama: &str, isi: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("oktz-hdvid2-{}-{nama}", std::process::id()));
        std::fs::write(&p, isi).unwrap();
        p
    }

    #[test]
    fn alur_lengkap_mengirim_multipart_put_dan_polling() {
        let path = berkas_sementara("klip.mp4", &[1u8, 2, 3, 4]);
        // Urutan balasan mengikuti urutan permintaan JS (`hdvid2.js:196-209`):
        // upload url, PUT, create-job, get-job.
        let http = Stub::baru(vec![
            (200, r#"{"code":100000,"result":{"url":"https://cdn/x/v.mp4?sig=abc"}}"#.to_string()),
            // Balasan PUT dari signed URL: body kosong, yang penting statusnya.
            (200, String::new()),
            (200, r#"{"code":100000,"result":{"job_id":"job-7"}}"#.to_string()),
            (200, r#"{"code":100000,"result":{"status":1,"output_url":["https://cdn/out.mp4"]}}"#.to_string()),
        ]);
        let hasil = video_enhancer(&http, path.to_str().unwrap()).unwrap();
        assert_eq!(hasil["jobId"], "job-7");
        assert_eq!(hasil["resultUrl"], "https://cdn/out.mp4");
        let _ = std::fs::remove_file(&path);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 4, "empat permintaan: upload, put, create-job, get-job");

        // 1) POST multipart upload, satu field, tanpa `filename`.
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(
            p[0].url,
            "https://api.unwatermark.ai/api/web/common/upload/video"
        );
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "video_file_name");
        assert_eq!(parts[0].file_name, None, "bukan part berkas");
        let nama_terkirim = String::from_utf8_lossy(parts[0].body.bytes()).to_string();
        assert!(
            nama_terkirim.starts_with("oktz-hdvid2-") && nama_terkirim.ends_with("klip.mp4"),
            "path.basename, bukan path penuh: {nama_terkirim}"
        );

        // 2) PUT biner ke signed URL lengkap dengan query — JADI SEBELUM
        //    create-job, mengikuti `hdvid2.js:200` lalu `:208`.
        assert_eq!(p[1].method, oktz_plugin_api::HttpMethod::Put);
        assert_eq!(p[1].url, "https://cdn/x/v.mp4?sig=abc");
        assert_eq!(cari_header(&p[1], "content-type"), Some("video/mp4"));
        assert_eq!(cari_header(&p[1], "content-length"), Some("4"));
        assert_eq!(p[1].body.as_ref().unwrap().bytes(), &[1u8, 2, 3, 4][..]);

        // 3) POST multipart create-job, tiga field berurutan; query URL sudah dibuang.
        assert_eq!(
            p[2].url,
            "https://api.unwatermark.ai/api/web/unblurimage/v1/video-enhancer/create-job"
        );
        let Some(HttpBody::Multipart(parts)) = &p[2].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].name, "original_video_url");
        assert_eq!(parts[0].body.bytes(), b"https://cdn/x/v.mp4");
        assert_eq!(parts[1].body.bytes(), b"2k");
        assert_eq!(parts[2].body.bytes(), b"false");

        // 4) GET polling dengan job_id dari create-job.
        assert_eq!(p[3].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[3].url,
            "https://api.unwatermark.ai/api/web/unblurimage/v1/video-enhancer/get-job/job-7"
        );

        // `validateStatus: () => true` di keempat permintaan.
        assert!(p.iter().all(|c| c.terima_semua_status));

        // Header `baseHeaders` hanya di POST/GET; PUT punya dua header sendiri.
        let h = &p[0];
        assert_eq!(cari_header(h, "origin"), Some("https://unblurimage.ai"));
        assert_eq!(cari_header(h, "referer"), Some("https://unblurimage.ai/"));
        assert_eq!(cari_header(h, "accept"), Some("*/*"));
        assert_eq!(cari_header(h, "product-code"), Some("067003"));
        assert_eq!(cari_header(h, "sec-ch-ua-mobile"), Some("?1"));
        assert_eq!(
            cari_header(h, "sec-ch-ua"),
            Some(r#""Google Chrome";v="147", "Not.A/Brand";v="8", "Chromium";v="147""#)
        );
        let serial = cari_header(h, "product-serial").unwrap();
        assert_eq!(serial.len(), 6);
        assert!(serial.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(cari_header(&p[1], "x-request-id"), None);
    }

    #[test]
    fn job_yang_sudah_selesai_tidak_perlu_menunggu() {
        // `output_url` terisi di percobaan pertama, jadi tidak ada yang perlu
        // ditidurkan — test tetap cepat.
        let path = berkas_sementara("cepat.mp4", &[0]);
        // Empat balasan: upload url, PUT, create-job, get-job.
        let http = Stub::baru(vec![
            (200, r#"{"code":100000,"result":{"url":"https://cdn/v.mp4"}}"#.to_string()),
            (200, String::new()),
            (200, r#"{"code":100000,"result":{"job_id":"j1"}}"#.to_string()),
            (200, r#"{"code":100000,"result":{"output_url":["https://cdn/o1.mp4"]}}"#.to_string()),
        ]);
        let hasil = video_enhancer(&http, path.to_str().unwrap()).unwrap();
        assert_eq!(hasil["jobId"], "j1");
        assert_eq!(hasil["resultUrl"], "https://cdn/o1.mp4");
        assert_eq!(http.permintaan.lock().unwrap().len(), 4);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn code_upload_bukan_100000_melempar_dengan_pesan_js() {
        let path = berkas_sementara("a.mp4", &[0]);
        let http = Stub::baru(vec![(200, r#"{"code":1,"msg":"no"}"#.to_string())]);
        let galat = video_enhancer(&http, path.to_str().unwrap()).unwrap_err();
        assert_eq!(
            galat.message,
            r#"Gagal ambil upload url: {"code":1,"msg":"no"}"#
        );
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn status_400_dibaca_bukan_dilempar_klien() {
        let path = berkas_semporary("b.mp4", &[0]);
        let http = Stub::baru(vec![(400, r#"{"code":1}"#.to_string())]);
        let galat = video_enhancer(&http, path.as_str()).unwrap_err();
        assert_eq!(
            galat.message,
            r#"Gagal ambil upload url: {"code":1}"#,
            "status 400 bukan error transport, tapi memicu pesan upload"
        );
        let _ = std::fs::remove_file(&path);
    }

    fn berkas_semporary(nama: &str, isi: &[u8]) -> String {
        berkas_sementara(nama, isi)
            .to_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn put_gagal_memberi_pesanan_dengan_status_http() {
        let path = berkas_sementara("c.mp4", &[7, 7]);
        let http = Stub::baru(vec![
            (
                200,
                r#"{"code":100000,"result":{"url":"https://cdn/v.mp4?x=1"}}"#.to_string(),
            ),
            (500, "server meltdown".to_string()),
        ]);
        let galat = video_enhancer(&http, path.to_str().unwrap()).unwrap_err();
        assert_eq!(
            galat.message,
            "Upload file gagal HTTP 500: server meltdown",
            "body non-JSON: axios tinggallah string, `typeof` = string"
        );

        // Body JSON: axios mem-parse jadi objek, jadi yang ikut `JSON.stringify`
        // adalah seluruh objek — bukan hanya field `message`.
        let http = Stub::baru(vec![
            (
                200,
                r#"{"code":100000,"result":{"url":"https://cdn/v.mp4"}}"#.to_string(),
            ),
            (503, r#"{"message":"server meltdown"}"#.to_string()),
        ]);
        let galat = video_enhancer(&http, path.to_str().unwrap()).unwrap_err();
        assert_eq!(
            galat.message,
            r#"Upload file gagal HTTP 503: {"message":"server meltdown"}"#
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn create_job_tanpa_job_id_melempar_pesan_create_job() {
        let path = berkas_sementara("d.mp4", &[1]);
        let http = Stub::baru(vec![
            (
                200,
                r#"{"code":100000,"result":{"url":"https://cdn/v.mp4"}}"#.to_string(),
            ),
            (200, String::new()),
            (200, r#"{"code":100000,"result":{}}"#.to_string()),
        ]);
        let galat = video_enhancer(&http, path.to_str().unwrap()).unwrap_err();
        assert_eq!(
            galat.message,
            r#"Gagal create job: {"code":100000,"result":{}}"#
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn file_tidak_ada_melempar_sebelum_permintaan_apa_pun() {
        let http = Stub::baru(vec![]);
        let galat = video_enhancer(&http, "/tmp/oktz-hilang-hdvid2/zz.mp4").unwrap_err();
        assert_eq!(
            galat.message,
            "File tidak ditemukan: /tmp/oktz-hilang-hdvid2/zz.mp4"
        );
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn helper_menenuhi_aturan_js() {
        // `String(url||'').split('?')[0]`
        assert_eq!(
            clean_public_url("https://a/b.mp4?sig=1&x=2"),
            "https://a/b.mp4"
        );
        assert_eq!(clean_public_url("https://a/b.mp4"), "https://a/b.mp4");
        assert_eq!(clean_public_url("?x=1"), "");
        // `extToMime`
        assert_eq!(ext_to_mime("/x/A.MP4"), "video/mp4");
        assert_eq!(ext_to_mime("a.mov"), "video/quicktime");
        assert_eq!(ext_to_mime("a.webm"), "video/webm");
        assert_eq!(ext_to_mime("a.mkv"), "video/x-matroska");
        assert_eq!(ext_to_mime("a.avi"), "application/octet-stream");
        assert_eq!(ext_to_mime("tanpa-ekstensi"), "application/octet-stream");
        // `randomProductSerial` hanya huruf kecil dan angka.
        let s = random_product_serial();
        assert!(s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
        // `crypto.randomUUID()` untuk `x-request-id`.
        let u = uuid_v4();
        assert_eq!(u.len(), 36);
        assert_eq!(u.as_bytes()[14], b'4', "versi UUID v4: {u}");
    }
}