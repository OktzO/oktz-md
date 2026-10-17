//! `src/scraper/videy.js` — uploader ke `videy.co` plus pembaca tautan berbagi.
//!
//! File JS punya tiga export, jadi tiga `pub fn` di sini:
//! - `videy(file)` (default export) — unggah video lokal ke `videy.co`.
//! - `idDari(url)` — id video dari tautan berbagi `https://videy.co/v?id=…`.
//! - `videyShare(url, { signal })` — tautan berbagi jadi URL video langsung,
//!   setelah memprobe CDN supaya tautan yang sudah dihapus tidak lolos.
//!
//! Yang disalin apa adanya:
//! - `form.append("file", stream, { filename, contentType: "video/mp4" })`
//!   → satu part bernama `file`, `filename` = `path.basename(file)`, dan
//!   content-type dipaksa `video/mp4` (bukan hasil `mime.lookup`, karena
//!   `options.contentType` menang di `_getContentType`).
//! - Seluruh badan `videy` dibungkus `try/catch` yang mengembalikan
//!   `{ status: "error", msg }` — jadi fungsi ini tidak pernah `Err`.
//! - `ekstensiVidey`: `mov` hanya untuk id 9 karakter yang berakhir "2".
//! - `ID_BENTUK = /^[A-Za-z0-9_-]{1,64}$/`; id yang tidak cocok berarti `null`,
//!   bukan error.
//!
//! Beda yang diketahui:
//! - `signal` (AbortController) tidak punya padanan di `HttpGet`, jadi
//!   `videy_share` tidak menerimanya. Di JS `signal` hanya menghentikan
//!   pemanggil yang menunggu; probe-nya sendiri tetap berjalan sampai timeout
//!   15 detik milik `httpAxios`.
//! - `maxContentLength: 64 * 1024` pada probe tidak ditiru (batas memori,
//!   bukan tampilan). `HttpGet` tidak punya batas ukuran respons.
//! - `new URL(...)` memakai parser WHATWG lengkap. Yang di sini hanya Host+query
//!   minimal: tanpa itu `URLSearchParams` tidak punya sumber. `@napi` tidak
//!   tersedia; yang dijaga adalah sifat yang dipakai JS: URL tanpa skema
//!   ditolak, dan `id` diambil dari kemunculan pertama.
//! - `httpAxios` memasang timeout 15 detik dan `maxRedirects: 3` — batas mesin,
//!   bukan tampilan, jadi tidak ditiru.

use std::sync::Arc;

use oktz_plugin_api::{HttpBody, HttpCall, HttpError, HttpGet, MultipartPart};
use serde_json::{Map, Value, json};

const UPLOAD_URL: &str = "https://videy.co/api/upload?visitorId=";
const CDN: &str = "https://cdn.videy.co";

const UA: &str = "Mozilla/5.0 (Linux; Android 10)";

/// `PROBE_RANGE` (`videy.js:69`).
const PROBE_RANGE: &str = "bytes=0-0";

/// `crypto.randomUUID()` untuk `visitorId` (`videy.js:23`).
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

/// `videy(file)` (`videy.js:7-45`) — tidak pernah `Err`, hasilnya `Ok` selalu.
pub fn videy(http: &dyn HttpGet, file: &str) -> Result<Value, HttpError> {
    match alur(http, file) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": "error", "msg": e.message })),
    }
}

fn alur(http: &dyn HttpGet, file: &str) -> Result<Value, HttpError> {
    if file.is_empty() {
        return Err(HttpError {
            message: "input required".to_string(),
        });
    }
    // `fs.existsSync(file)` — pesan error-nya ikut dipakai plugin.
    let byte = std::fs::read(file).map_err(|_| HttpError {
        message: "file not found".to_string(),
    })?;

    let nama = basename(file);
    let res = http.call(
        &HttpCall::post(format!("{UPLOAD_URL}{}", uuid_v4()))
            // `...form.getHeaders()` tidak disalin: boundary dibuat klien Rust.
            .header("user-agent", UA)
            .header("origin", "https://videy.co")
            .header("referer", "https://videy.co/")
            .header("accept", "application/json")
            .multipart(vec![MultipartPart {
                name: "file".to_string(),
                file_name: Some(nama),
                mimetype: Some("video/mp4".to_string()),
                body: HttpBody::Bytes {
                    data: Arc::from(byte),
                    mimetype: None,
                },
            }]),
    )?;
    Ok(json!({ "output": res.json().unwrap_or(Value::Null) }))
}

/// `ekstensiVidey(id)` (`videy.js:75-77`) — disalin dari frontend videy.
fn ekstensi_videy(id: &str) -> &'static str {
    if id.chars().count() == 9 && id.ends_with('2') {
        "mov"
    } else {
        "mp4"
    }
}

/// `idDari(url)` (`videy.js:84-93`) — `null` kalau tautannya tidak terbaca.
///
/// Tidak melakukan HTTP sama sekali; parameter `_http` ada hanya supaya semua
/// scraper memakai bentuk argumen yang sama.
pub fn id_dari(_http: &dyn HttpGet, url: &str) -> Option<String> {
    let id = parameter_id(url.trim())?;
    // `id && ID_BENTUK.test(id)` — string kosong juga ditolak.
    if id.is_empty() || id.chars().count() > 64 {
        return None;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        .then_some(id)
}

/// `videyShare(url, { signal })` (`videy.js:101-127`).
pub fn videy_share(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let Some(id) = id_dari(http, url) else {
        return Err(HttpError {
            message: "tautan berbagi videy tanpa id video yang bisa dibaca".to_string(),
        });
    };
    let ext = ekstensi_videy(&id);
    let address = format!("{CDN}/{id}.{ext}");

    // 2xx (dan 206 parsial dari `Range`) berarti objeknya ada; 404 berarti
    // tautannya sudah mati dan axios melempar. Perilaku bawaan `HttpGet`
    // persis seperti itu: status >= 400 jadi `Err`.
    http.call(
        &HttpCall::get(&address)
            .header("user-agent", UA)
            .header("referer", "https://videy.co/")
            .header("accept", "video/mp4,video/quicktime,*/*")
            .header("range", PROBE_RANGE),
    )?;

    let mut out = Map::new();
    out.insert("id".to_string(), Value::String(id));
    out.insert("ext".to_string(), Value::String(ext.to_string()));
    out.insert("url".to_string(), Value::String(address));
    Ok(Value::Object(out))
}

/// `new URL(...).searchParams.get("id")` — nilai pertama dari query.
///
/// Dipisah dari fragment (`#`) lebih dulu, seperti `URL` yang menaruh query
/// sebelum fragment. `None` kalau URL-nya tidak punya skema http(s) atau tidak
/// punya query.
fn parameter_id(url: &str) -> Option<String> {
    let (scheme, sisa) = url.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    // Authority berakhir di `/`, `?`, atau `#` — `new URL` menolak host kosong
    // (`https:///v?id=x`).
    let authority = sisa.split(['/', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return None;
    }
    // Query dibaca dari `?` pertama yang muncul, dan berhenti sebelum `#`.
    let setelah_authority = sisa
        .strip_prefix(authority)?
        .split('#')
        .next()
        .unwrap_or("");
    let (_, query) = setelah_authority.split_once('?')?;
    for bagian in query.split('&') {
        let (k, v) = bagian.split_once('=').unwrap_or((bagian, ""));
        if decode_komponen(k) == "id" {
            return Some(decode_komponen(v));
        }
    }
    None
}

/// Decode satu komponen `application/x-www-form-urlencoded`.
fn decode_komponen(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 3 <= b.len() => match std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok())
            {
                Some(byte) => {
                    out.push(byte);
                    i += 3;
                }
                None => {
                    out.push(b'%');
                    i += 1;
                }
            },
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

/// `path.basename(file)` (POSIX): garis miring di ujung diabaikan.
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

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::HttpResponse;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas tetap.
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<String>,
    }

    impl StubHttp {
        fn baru(balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan.to_string()),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
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
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.lock().unwrap().clone().into_bytes()),
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

    /// Berkas sementara untuk `fs.existsSync`/`createReadStream` di JS.
    fn berkas(nama: &str, isi: &[u8]) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("oktz-videy-{}-{nama}", std::process::id()));
        std::fs::write(&p, isi).unwrap();
        p
    }

    #[test]
    fn unggah_mengirim_multipart_dengan_nama_dan_video_mp4() {
        let path = berkas("klip.mp4", &[1u8, 2, 3]);
        let http = StubHttp::baru(r#"{"status":"success","url":"https://videy.co/v?id=abc"}"#);
        let hasil = videy(&http, path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(hasil["output"]["status"], "success");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        let prefix = UPLOAD_URL;
        assert!(p[0].url.starts_with(prefix), "{}", p[0].url);
        // `crypto.randomUUID()` — bentuknya, bukan nilainya.
        let visitor = p[0].url.strip_prefix(prefix).unwrap();
        assert_eq!(visitor.len(), 36, "{visitor}");
        assert_eq!(visitor.matches('-').count(), 4, "{visitor}");

        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart: {:?}", p[0].body);
        };
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "file");
        assert!(
            parts[0]
                .file_name
                .as_deref()
                .unwrap()
                .ends_with("klip.mp4")
        );
        assert_eq!(
            parts[0].mimetype.as_deref(),
            Some("video/mp4"),
            "contentType eksplisit menang atas mime.lookup"
        );
        assert_eq!(parts[0].body.bytes(), &[1u8, 2, 3][..]);
        assert_eq!(cari(&p[0], "origin"), Some("https://videy.co"));
        assert_eq!(cari(&p[0], "referer"), Some("https://videy.co/"));
        assert_eq!(cari(&p[0], "accept"), Some("application/json"));
        assert_eq!(cari(&p[0], "user-agent"), Some(UA));
    }

    #[test]
    fn input_kosong_dan_berkas_hilang_keduanya_status_error() {
        let http = StubHttp::baru("{}");
        assert_eq!(
            videy(&http, "").unwrap(),
            json!({ "status": "error", "msg": "input required" })
        );
        assert_eq!(
            videy(&http, "/tmp/oktz-tidak-ada-aja/klip.mp4").unwrap(),
            json!({ "status": "error", "msg": "file not found" })
        );
        assert!(
            http.permintaan.lock().unwrap().is_empty(),
            "tidak boleh ada request"
        );
    }

    #[test]
    fn id_dari_membaca_kemunculan_pertama_dan_memangkas_id_berbahaya() {
        let http = StubHttp::baru("{}");
        let id = |u: &str| id_dari(&http, u);
        assert_eq!(id("https://videy.co/v?id=7ZH1ZRIF"), Some("7ZH1ZRIF".to_string()));
        assert_eq!(
            id("https://videy.co/v?id=abc&id=lain"),
            Some("abc".to_string())
        );
        assert_eq!(
            id("https://videy.co/v?x=1&id=zz#frag"),
            Some("zz".to_string())
        );
        // Bentuk id: hanya A-Z a-z 0-9 _ -, maksimal 64.
        assert_eq!(id("https://videy.co/v?id=a-b_c9"), Some("a-b_c9".to_string()));
        let batas = format!("https://videy.co/v?id={}", "a".repeat(65));
        for buruk in [
            "https://videy.co/v?id=",
            "https://videy.co/v?id=../rahasia",
            "https://videy.co/v?id=ada%20spasi",
            "https://videy.co/v?id=a+b",
            batas.as_str(),
        ] {
            assert_eq!(id(buruk), None, "{buruk} harus ditolak");
        }
        // Tanpa skema, host kosong, atau tanpa query.
        for buruk in [
            "videy.co/v?id=abc",
            "ftp://videy.co/v?id=abc",
            "https:///v?id=abc",
            "https://videy.co/v",
            "",
            "halo dunia",
        ] {
            assert_eq!(id(buruk), None, "{buruk} harus ditolak");
        }
    }

    #[test]
    fn probe_mengirim_range_satu_byte_dan_mengembalikan_tiga_kunci() {
        let http = StubHttp::baru("{}");
        let hasil = videy_share(&http, "https://videy.co/v?id=7ZH1ZRIF").unwrap();
        assert_eq!(hasil["id"], "7ZH1ZRIF");
        assert_eq!(hasil["ext"], "mp4");
        assert_eq!(hasil["url"], "https://cdn.videy.co/7ZH1ZRIF.mp4");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(p[0].url, "https://cdn.videy.co/7ZH1ZRIF.mp4");
        assert_eq!(cari(&p[0], "range"), Some("bytes=0-0"));
        assert_eq!(cari(&p[0], "accept"), Some("video/mp4,video/quicktime,*/*"));
        assert_eq!(cari(&p[0], "referer"), Some("https://videy.co/"));
    }

    #[test]
    fn ekstensi_mov_hanya_saat_id_sembilan_karakter_berakhir_dua() {
        let http = StubHttp::baru("{}");
        assert_eq!(ekstensi_videy("123456789"), "mp4", "9 karakter tapi bukan akhiran 2");
        assert_eq!(ekstensi_videy("abcdefgh2"), "mov");
        assert_eq!(ekstensi_videy("abcdefghi2"), "mp4", "10 karakter");
        assert_eq!(ekstensi_videy(""), "mp4");
        // Regex JS menghitung karakter UTF-16; `chars().count()` menghitung
        // scalar. Untuk id yang lolos `ID_BENTUK` keduanya sama.
        assert_eq!(videy_share(&http, "https://videy.co/v?id=abcdefgh2").unwrap()["ext"], "mov");
    }

    #[test]
    fn tautan_tanpa_id_melempar_pesan_js_tanpa_probe() {
        let http = StubHttp::baru("{}");
        let err = videy_share(&http, "https://videy.co/v").unwrap_err();
        assert_eq!(
            err.message,
            "tautan berbagi videy tanpa id video yang bisa dibaca"
        );
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn probe_gagal_meneruskan_error_http() {
        // `httpAxios` melempar untuk 404, jadi tautan mati tidak pernah sampai
        // ke `return { id, ext, url }`.
        #[derive(Debug, Default)]
        struct Mati;
        impl HttpGet for Mati {
            fn get(&self, _u: &str) -> Result<HttpResponse, HttpError> {
                unreachable!("lewat HttpCall::call")
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
                    message: "Request failed with status code 404".to_string(),
                })
            }
        }
        let err = videy_share(&Mati, "https://videy.co/v?id=abc").unwrap_err();
        assert_eq!(err.message, "Request failed with status code 404");
    }

    #[test]
    fn output_bukan_json_tetap_dibungkus_output() {
        // axios selalu punya `r.data`; body non-JSON di sini jadi `null`.
        let path = berkas("x.mp4", &[1]);
        let http = StubHttp::baru("<html>bukan json</html>");
        let hasil = videy(&http, path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(hasil["output"], Value::Null);
    }

    #[test]
    fn basename_mengikuti_path_basename() {
        assert_eq!(basename("/tmp/a/b.mp4"), "b.mp4");
        assert_eq!(basename("b.mp4"), "b.mp4");
        assert_eq!(basename("/tmp/a/b/"), "b");
        assert_eq!(basename("/"), "");
    }
}