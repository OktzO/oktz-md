//! `src/scraper/imgdrop.js` — unggah gambar ke `imgdrop.web.id`.
//!
//! Alur JS (`imgdrop.js:175-204`):
//! 1. ambil cookie sesi (dari `IMGDROP_COOKIE`, atau dari halaman `?i=1`
//!    digabung dengan cookie challenge),
//! 2. unggah lewat multipart ke `upload.php`,
//! 3. kalau respons memuat challenge (`slowAES.decrypt` atau `__test=`),
//!    selesaikan challenge, gabungkan ke cookie, lalu unggah sekali lagi.
//!
//! Beda yang diketahui dari JS:
//! - **TERBLOKIR — dekripsi AES-128-CBC.** `solveJsChallenge` memakai
//!   `crypto.createDecipheriv("aes-128-cbc", …)`. Crate `oktz-scraper` hanya
//!   boleh memakai `oktz-plugin-api`, `serde_json`, `rand`, `regex`, `md-5`,
//!   `sha1` (`Cargo.toml`) — tidak ada implementasi AES, dan menambah
//!   dependency bukan wewenang modul ini. Jadi `solve_js_challenge` bisa
//!   MENDETEKSI challenge (regex berjalan) tapi tidak bisa menyelesaikannya,
//!   dan itu dilaporkan sebagai galat, bukan diam-diam mengembalikan cookie
//!   palsu. Konsekuensi: imgdrop hanya bisa dipakai kalau situs tidak
//!   accordion challenge — yaitu ketika operator sudah menyetel
//!   `IMGDROP_COOKIE`, atau kalau challenge hanya muncul di respons POST
//!   (itu tetap fatal di JS juga). Jalan lain: sediakan dependency AES lalu
//!   isi fungsi ini.
//! - `fetch(..., { redirect: "manual" })` tidak punya padanan di `HttpCall`
//!   (field redirect tidak ada). Permintaan tetap dikirim; kalau server
//!   membalas 3xx, `UreqHttp` yang memutuskan.
//! - `process.env.IMGDROP_COOKIE` dibaca lewat `std::env::var` di
//!   `upload_image`. Supaya test tidak bergantung pada lingkungan, fungsi
//!   privatnya menerima nilai cookie manual sebagai argumen.
//! - Header `content-length` dihitung ulang dari body yang benar-benar dikirim
//!   (JS menghitungnya dari `body.length`); angkanya sama karena
//!   body multipart dibangun manual, sama seperti di JS — bukan lewat
//!   `MultipartPart`.
//! - `FILE_PATH` (`imgdrop.js:8`) tidak pernah dipakai di file JS itu sendiri,
//!   jadi tidak ikut.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value};

use crate::common::truthy;

const BASE_URL: &str = "https://imgdrop.web.id";
const UPLOAD_URL: &str = "https://imgdrop.web.id/upload.php";
const REFERER: &str = "https://imgdrop.web.id/?i=1";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// `randomBoundary()` (`imgdrop.js:10-13`) — 12 byte acak jadi base64url,
/// karakter non-alfanumerik dibuang, lalu 16 karakter pertama.
fn random_boundary() -> String {
    const AB: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mentah: Vec<u8> = (0..16)
        .map(|_| AB[rand::random_range(0..64usize)])
        .collect();
    let acak: String = String::from_utf8_lossy(&mentah)
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    let potong: String = acak.chars().take(16).collect();
    format!("----WebKitFormBoundary{potong}")
}

/// `guessMime(filename)` (`imgdrop.js:15-29`) — ekstensi huruf kecil, daftar
/// persis seperti di JS (tanpa `heic`/`tiff` yang ada di `imglarger.rs`).
fn guess_mime(filename: &str) -> &'static str {
    let ext = extname(filename);
    match ext.as_str() {
        ".jpg" | ".jpeg" => "image/jpeg",
        ".png" => "image/png",
        ".webp" => "image/webp",
        ".gif" => "image/gif",
        ".bmp" => "image/bmp",
        ".svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

/// `path.extname` — bagian setelah titik terakhir, titik di depan tidak
/// dihitung (`.bashrc` -> `""`).
fn extname(filename: &str) -> String {
    let dasar = filename.rsplit('/').next().unwrap_or(filename);
    match dasar.rfind('.') {
        Some(i) if i > 0 => dasar[i..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// Pola challenge (`imgdrop.js:37`).
fn pola_challenge() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| {
        Regex::new(
            r#"var a=toNumbers\("([^"]+)"\),b=toNumbers\("([^"]+)"\),c=toNumbers\("([^"]+)"\)"#,
        )
        .expect("pola challenge")
    })
}

/// `solveJsChallenge(html)` (`imgdrop.js:35-55`).
///
/// Hasil JS: `__test=<hex>` kalau challenge ditemukan, `""` kalau tidak.
///
/// Beda yang diketahui: bagian dekripsi tidak ada (lihat catatan modul soal
/// AES-128-CBC). Kalau challenge DIDAPATKAN, fungsi ini mengembalikan `Err`
/// dengan alasan — bukan `Ok("")`, karena `""` berarti "tidak ada challenge"
/// dan itu akan membuat unggahan gagal diam-diam dengan pesan yang menyesatkan.
fn solve_js_challenge(html: &str) -> Result<String, HttpError> {
    if pola_challenge().is_match(html) {
        return Err(HttpError {
            message: "challenge imgdrop (AES-128-CBC) tidak bisa diselesaikan: \
                      crate ini tidak punya implementasi AES"
                .to_string(),
        });
    }
    Ok(String::new())
}

/// `parseSetCookie(headers)` (`imgdrop.js:57-72`) — jalur `getSetCookie()`:
/// setiap nilai dipotong sebelum `;` lalu disambung `; `.
fn parse_set_cookie(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.split(';').next().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// `mergeCookies(...)` (`imgdrop.js:74-87`) — nama cookie jadi kunci, nilai
/// terakhir menang; urutan sisipan pertama dipertahankan (perilaku `Map` JS).
fn merge_cookies(terpisah: &[&str]) -> String {
    let mut urutan: Vec<String> = Vec::new();
    let mut nilai: Vec<String> = Vec::new();
    for s in terpisah {
        if s.is_empty() {
            continue;
        }
        for item in s.split("; ") {
            let nama = item.split('=').next().unwrap_or("");
            if nama.is_empty() {
                continue;
            }
            match urutan.iter().position(|n| n == nama) {
                Some(i) => nilai[i] = item.to_string(),
                None => {
                    urutan.push(nama.to_string());
                    nilai.push(item.to_string());
                }
            }
        }
    }
    nilai.join("; ")
}

/// `getSessionCookie()` (`imgdrop.js:89-110`), dengan cookie manual
/// (`process.env.IMGDROP_COOKIE`) dibaca di luar supaya bisa diuji.
fn get_session_cookie(
    http: &dyn HttpGet,
    cookie_manual: Option<&str>,
) -> Result<String, HttpError> {
    if let Some(c) = cookie_manual.filter(|c| !c.is_empty()) {
        return Ok(c.to_string());
    }
    let res = http.call(
        &HttpCall::get(REFERER)
            .header("user-agent", UA)
            .header(
                "accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
            )
            .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7"),
    )?;
    let header_cookie = parse_set_cookie(&res.headers);
    let challenge = solve_js_challenge(&res.text())?;
    Ok(merge_cookies(&[&header_cookie, &challenge]))
}

/// `createMultipartBody(...)` (`imgdrop.js:112-123`) — body multipart
/// dirakit tangan supaya `boundary` yang sama bisa dipakai di header dan di
/// body, persis seperti JS.
fn create_multipart_body(
    field: &str,
    filename: &str,
    mime: &str,
    file: &[u8],
    boundary: &str,
) -> Vec<u8> {
    let kepala = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\nContent-Type: {mime}\r\n\r\n"
    );
    let ekor = format!("\r\n--{boundary}--\r\n");
    let mut out = Vec::with_capacity(kepala.len() + file.len() + ekor.len());
    out.extend_from_slice(kepala.as_bytes());
    out.extend_from_slice(file);
    out.extend_from_slice(ekor.as_bytes());
    out
}

/// Hasil `requestUpload` (`imgdrop.js:168-172`) — butuh `res.ok`, status, dan
/// teks respons. Field `cookie` di JS juga tidak pernah dibaca oleh
/// `uploadImage`, jadi tidak ikut.
struct HasilUnggah {
    ok: bool,
    status: u16,
    teks: String,
}

/// `requestUpload(fileBuffer, filename, cookie)` (`imgdrop.js:125-173`).
fn request_upload(
    http: &dyn HttpGet,
    file_buffer: &[u8],
    filename: &str,
    cookie: &str,
) -> Result<HasilUnggah, HttpError> {
    let mime = guess_mime(filename);
    let boundary = random_boundary();
    let body = create_multipart_body("file", filename, mime, file_buffer, &boundary);

    let mut req = HttpCall::post(UPLOAD_URL)
        .header("user-agent", UA)
        .header("accept", "*/*")
        .header("accept-language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header("content-length", body.len().to_string())
        .header("origin", BASE_URL)
        .header("referer", REFERER)
        .header("sec-ch-ua-platform", "\"Android\"")
        .header(
            "sec-ch-ua",
            "\"Google Chrome\";v=\"147\", \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"147\"",
        )
        .header("sec-ch-ua-mobile", "?1")
        .header("sec-fetch-site", "same-origin")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-dest", "empty")
        .header("priority", "u=1, i")
        .raw(body, "multipart/form-data");
    // `if (cookie) headers.cookie = cookie`
    if !cookie.is_empty() {
        req = req.header("cookie", cookie);
    }

    let res = http.call(&req)?;
    let status = res.status;
    Ok(HasilUnggah {
        // `res.ok` = status 200..299.
        ok: (200..300).contains(&status),
        status,
        teks: res.text(),
    })
}

/// `uploadImage(buffer, filename)` (`imgdrop.js:175-204`).
pub fn upload_image(
    http: &dyn HttpGet,
    buffer: &[u8],
    filename: &str,
) -> Result<Value, HttpError> {
    let cookie_manual = std::env::var("IMGDROP_COOKIE").ok();
    upload_image_dengan_cookie(http, buffer, filename, cookie_manual.as_deref())
}

/// `uploadImage` dengan cookie manual yang disuntik (bikin `process.env`
/// bisa diuji tanpa bergantung pada lingkungan).
fn upload_image_dengan_cookie(
    http: &dyn HttpGet,
    buffer: &[u8],
    filename: &str,
    cookie_manual: Option<&str>,
) -> Result<Value, HttpError> {
    let mut cookie = get_session_cookie(http, cookie_manual)?;
    let mut result = request_upload(http, buffer, filename, &cookie)?;

    // Kalau respons masih challenge: selesaikan, gabungkan, unggah sekali lagi.
    if result.teks.contains("slowAES.decrypt") || result.teks.contains("__test=") {
        let challenge_cookie = solve_js_challenge(&result.teks)?;
        if !challenge_cookie.is_empty() {
            cookie = merge_cookies(&[&cookie, &challenge_cookie]);
            result = request_upload(http, buffer, filename, &cookie)?;
        }
    }

    let json = serde_json::from_str::<Value>(&result.teks).ok();

    let mut out = Map::new();
    out.insert(
        "status".to_string(),
        Value::Bool(
            result.ok
                && json
                    .as_ref()
                    .and_then(|j| j.get("success"))
                    .map(truthy)
                    .unwrap_or(false),
        ),
    );
    out.insert(
        "code".to_string(),
        Value::Number(serde_json::Number::from(result.status)),
    );
    // `json?.url || ""` — nilai bukan string juga ikut apa adanya kalau truthy.
    for kunci in ["url", "size", "filename"] {
        let nilai = json
            .as_ref()
            .and_then(|j| j.get(kunci))
            .filter(|v| truthy(v))
            .cloned()
            .unwrap_or(Value::String(String::new()));
        out.insert(kunci.to_string(), nilai);
    }
    Ok(Value::Object(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpMethod, HttpResponse};
    use std::sync::Mutex;

    /// `(status, body, headers)` untuk stub HTTP — alias biar tipe
    /// balancer-nya tidak memicu `clippy::type_complexity`.
    type Balasan = (u16, String, Vec<(String, String)>);

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body, headers) berurutan; setelah habis, balasan terakhir
    /// dipakai lagi.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<Balasan>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<Balasan>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }

        fn tunggal(body: &str) -> Self {
            Self::baru(vec![(200, body.to_string(), Vec::new())])
        }

        fn dengan_headers(body: &str, headers: Vec<(&str, &str)>) -> Self {
            Self::baru(vec![
                (
                    200,
                    body.to_string(),
                    headers
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                ),
            ])
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua permintaan lewat HttpCall::call")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("semua permintaan lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("imgdrop mengirim multipart, bukan form")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("imgdrop merakit body multipart sendiri")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, teks, headers) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: std::sync::Arc::from(teks.into_bytes()),
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

    #[test]
    fn cookie_dari_lingkungan_menghapus_permintaan_get() {
        // `IMGDROP_COOKIE` = return langsung, tidak ada GET halaman.
        let http = StubHttp::tunggal("{}");
        let hasil = upload_image_dengan_cookie(
            &http,
            &[1, 2, 3],
            "foto.png",
            Some("saya=manual"),
        )
        .unwrap();
        assert_eq!(hasil["status"], false, "body `{{}}` tidak punya success");
        assert_eq!(hasil["code"], 200);
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1, "hanya POST upload");
        assert_eq!(p[0].method, HttpMethod::Post);
        assert_eq!(cari_header(&p[0], "cookie").as_deref(), Some("saya=manual"));
    }

    #[test]
    fn upload_mengirim_multipart_dengan_boundary_dan_header_lengkap() {
        let http = StubHttp::tunggal(
            r#"{"success":true,"url":"https://imgdrop.web.id/a.png","size":123,"filename":"a.png"}"#,
        );
        let hasil =
            upload_image_dengan_cookie(&http, &[0xAA, 0xBB], "foto.PNG", Some("c=1")).unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["url"], "https://imgdrop.web.id/a.png");
        assert_eq!(hasil["size"], 123, "angka ikut apa adanya");
        assert_eq!(hasil["filename"], "a.png");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, UPLOAD_URL);
        let ctype = cari_header(&p[0], "content-type").unwrap();
        assert!(
            ctype.starts_with("multipart/form-data; boundary=----WebKitFormBoundary"),
            "{ctype}"
        );
        let boundary = ctype.split("boundary=").nth(1).unwrap().to_string();
        assert!(boundary.len() > "----WebKitFormBoundary".len());
        assert_eq!(cari_header(&p[0], "origin").as_deref(), Some(BASE_URL));
        assert_eq!(cari_header(&p[0], "referer").as_deref(), Some(REFERER));
        assert_eq!(cari_header(&p[0], "sec-fetch-mode").as_deref(), Some("cors"));
        assert_eq!(cari_header(&p[0], "priority").as_deref(), Some("u=1, i"));

        // Body: header part + byte berkas + penutup, persis seperti JS.
        let body = p[0].body.as_ref().unwrap().bytes().to_vec();
        let kepala = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"foto.PNG\"\r\nContent-Type: image/png\r\n\r\n"
        );
        let ekor = format!("\r\n--{boundary}--\r\n");
        let handelt = body.starts_with(kepala.as_bytes()) && body.ends_with(ekor.as_bytes());
        assert!(handelt, "bentuk body harus sama dengan createMultipartBody");
        let isi = &body[kepala.len()..body.len() - ekor.len()];
        assert_eq!(isi, &[0xAA, 0xBB][..]);
        assert_eq!(
            cari_header(&p[0], "content-length"),
            Some(body.len().to_string())
        );
    }

    #[test]
    fn cookie_kosong_tidak_mengirim_header_cookie() {
        let http = StubHttp::tunggal(r#"{"success":false}"#);
        upload_image_dengan_cookie(&http, &[1], "a.png", None).unwrap();
        let p = http.permintaan.lock().unwrap();
        // GET halaman dulu (tanpa set-cookie → cookie kosong), lalu POST.
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].method, HttpMethod::Get);
        assert_eq!(p[0].url, REFERER);
        assert_eq!(p[1].method, HttpMethod::Post);
        assert!(
            cari_header(&p[1], "cookie").is_none(),
            "cookie kosong tidak boleh jadi header"
        );
    }

    #[test]
    fn set_cookie_dari_halaman_dipakai_di_unggahan() {
        let http = StubHttp::baru(vec![
            (
                200,
                "<p>halama</p>".to_string(),
                vec![
                    ("set-cookie".to_string(), "cf=1; Path=/; HttpOnly".to_string()),
                    ("set-cookie".to_string(), "id=2; Secure".to_string()),
                ],
            ),
            (200, r#"{"success":true,"url":"u"}"#.to_string(), Vec::new()),
        ]);
        upload_image_dengan_cookie(&http, &[1], "a.png", None).unwrap();
        let p = http.permintaan.lock().unwrap();
        assert_eq!(cari_header(&p[1], "cookie").as_deref(), Some("cf=1; id=2"));
    }

    #[test]
    fn challenge_tanpa_pola_tonumbers_tidak_mengulang_unggahan() {
        // JS: `solveJsChallenge` balas `""` kalau regex `toNumbers` tidak
        // cocok, dan `if (challengeCookie)` falsehood → tidak ada percobaan
        // ulang. Persis itu yang harus terjadi di port.
        let http = StubHttp::baru(vec![
            (
                200,
                "<p>bukan challenge</p>".to_string(),
                vec![("set-cookie".to_string(), "a=1".to_string())],
            ),
            (200, "var x = slowAES.decrypt(…);".to_string(), Vec::new()),
        ]);
        let hasil = upload_image_dengan_cookie(&http, &[1], "a.png", None).unwrap();
        assert_eq!(hasil["status"], false, "body bukan JSON");
        assert_eq!(hasil["code"], 200);
        assert_eq!(
            http.permintaan.lock().unwrap().len(),
            2,
            "tanpa percobaan ulang"
        );
    }

    #[test]
    fn challenge_di_respons_post_yang_punya_pola_menghasilkan_galat() {
        // Bentuk respons challenge sungguhan: halaman yang memanggil
        // `slowAES.decrypt` (itu juga syarat JS `imgdrop.js:180`) dan memuat
        // variabel `toNumbers`. Tanpa `slowAES.decrypt`/`__test=` di body,
        // JS pun tidak akan mencoba challenge — jadi fixture lama salah.
        // Di sini pola `toNumbers` cocok, jadi JS benar-benar mendekripsi
        // (dan mengunggah ulang). Port tidak punya AES, jadi dilaporkan
        // sebagai galat — bukan cookie palsu.
        let pola = r#"<script>slowAES.decrypt();var a=toNumbers("00"),b=toNumbers("11"),c=toNumbers("22")</script>"#;
        let http = StubHttp::baru(vec![
            (
                200,
                "<p>bukan challenge</p>".to_string(),
                vec![("set-cookie".to_string(), "a=1".to_string())],
            ),
            (200, pola.to_string(), Vec::new()),
        ]);
        let err = upload_image_dengan_cookie(&http, &[1], "a.png", None).unwrap_err();
        assert!(
            err.message.contains("AES-128-CBC"),
            "pesan harus menyebut blokirnya: {}",
            err.message
        );
        assert_eq!(
            http.permintaan.lock().unwrap().len(),
            2,
            "tanpa percobaan ulang"
        );
    }

    #[test]
    fn challenge_di_halaman_sesi_juga_menghasilkan_galat() {
        let http = StubHttp::dengan_headers(
            r#"var a=toNumbers("00"),b=toNumbers("00"),c=toNumbers("00")"#,
            vec![("set-cookie", "a=1")],
        );
        let err = upload_image_dengan_cookie(&http, &[1], "a.png", None).unwrap_err();
        assert!(err.message.contains("AES-128-CBC"));
    }

    #[test]
    fn respons_bukan_json_memberikan_nilai_kosong_bukan_galat() {
        let http = StubHttp::tunggal("<html>antrian</html>");
        let hasil = upload_image_dengan_cookie(&http, &[1], "a.png", Some("c=1")).unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["code"], 200);
        assert_eq!(hasil["url"], "");
        assert_eq!(hasil["size"], "");
        assert_eq!(hasil["filename"], "");
    }

    #[test]
    fn status_http_gagal_tetap_mengembalikan_objek_bukan_err() {
        // `fetch` tidak melempar untuk 4xx, dan `status` dibentuk dari
        // `res.ok && …` — jadi objek lengkap tetap dikembalikan.
        let http = StubHttp::baru(vec![(
            503,
            r#"{"success":true,"url":"u"}"#.to_string(),
            Vec::new(),
        )]);
        let hasil = upload_image_dengan_cookie(&http, &[1], "a.png", Some("c=1")).unwrap();
        assert_eq!(hasil["status"], false, "503 bukan res.ok");
        assert_eq!(hasil["code"], 503);
        assert_eq!(hasil["url"], "u", "payload tetap dibaca");
    }

    #[test]
    fn merge_cookies_menang_di_nama_dan_jaga_urutan_sisipan() {
        assert_eq!(merge_cookies(&["a=1; b=2", "b=3", "a=4"]), "a=4; b=3");
        assert_eq!(merge_cookies(&["", "x=1"]), "x=1");
        assert_eq!(merge_cookies(&["=takbernama", "y=2"]), "y=2");
    }

    #[test]
    fn parse_set_cookie_memotong_setiap_nilai_di_titik_koma() {
        let headers = vec![
            ("Set-Cookie".to_string(), "a=1; Path=/".to_string()),
            ("set-cookie".to_string(), "b=2; HttpOnly".to_string()),
            ("content-type".to_string(), "text/html".to_string()),
        ];
        assert_eq!(parse_set_cookie(&headers), "a=1; b=2");
        assert_eq!(parse_set_cookie(&[]), "");
    }

    #[test]
    fn extname_dan_mime_mengikuti_js() {
        assert_eq!(guess_mime("a/b/foto.JPG"), "image/jpeg");
        assert_eq!(guess_mime("foto.jpeg"), "image/jpeg");
        assert_eq!(guess_mime("foto.png"), "image/png");
        assert_eq!(guess_mime("foto.webp"), "image/webp");
        assert_eq!(guess_mime("foto.gif"), "image/gif");
        assert_eq!(guess_mime("foto.bmp"), "image/bmp");
        assert_eq!(guess_mime("foto.svg"), "image/svg+xml");
        assert_eq!(guess_mime("foto.xyz"), "application/octet-stream");
        assert_eq!(guess_mime("tanpa_ekstensi"), "application/octet-stream");
        // Titik di awal bukan ekstensi.
        assert_eq!(guess_mime(".png"), "application/octet-stream");
    }

    #[test]
    fn boundary_mengikuti_bentuk_webkit_dan_panjang_maksimal_16() {
        for _ in 0..20 {
            let b = random_boundary();
            assert!(b.starts_with("----WebKitFormBoundary"));
            let sufiks = &b["----WebKitFormBoundary".len()..];
            assert!(!sufiks.is_empty());
            assert!(sufiks.len() <= 16, "slice(0, 16): {}", sufiks);
            assert!(
                sufiks.chars().all(|c| c.is_ascii_alphanumeric()),
                "base64url lalu filter non-alnum: {sufiks}"
            );
        }
    }
}