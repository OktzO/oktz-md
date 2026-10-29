//! `src/scraper/removebackground.js` — hapus background lewat dua API
//! (pixelcut lalu bgninja sebagai cadangan).
//!
//! **Bagian yang terblokir: `pixa()` dan `normalizeToJpeg()`.** Keduanya butuh
//! `sharp`: `normalizeToJpeg(img)` menjalankan `sharp(img).metadata()` dan
//! `sharp(img).jpeg().toBuffer()` untuk menormalkan gambar (webp, heic, png,
//! apa pun) menjadi JPEG sebelum dikirim. `sharp` adalah binding native
//! libvips; crate `oktz-scraper` tidak punya dependency itu dan tidak boleh
//! menambahkannya, jadi port ini **tidak** membuat stub yang mengarang JPEG.
//! Yang diport di sini hanya jalur HTTP dua API — pemanggil wajib mengirim
//! buffer yang sudah JPEG (lihat laporan).
//!
//! Yang ikut disalin dari JS:
//! - `fetch` tidak melempar untuk 4xx/5xx, jadi `res.ok` dicek sendiri —
//!   permintaan memakai `.terima_semua_status()`.
//! - Syarat gagal: `!res.ok || !contentType.includes("image/")`. Header
//!   `content-type` kosong juga membuat gagal.
//! - Cabang JSON: `json.error || json.message || "pixelcut failed: {status}"`
//!   (`json.error || json.detail || ...` untuk bgninja). Kalau body-nya bukan
//!   JSON, yang kedua yang terpilih: `"{prefix} failed: {status} {teks 200
//!   karakter}"`.
//! - `text.slice(0, 200)` memakai karakter, bukan byte.
//! - Balikan JS adalah `Buffer`; di sini `Vec<u8>`, karena `serde_json::Value`
//!   tidak bisa membawa buffer gambar.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, HttpResponse, MultipartPart};

/// Header pixelcut (`removebackground.js:17-30`).
fn header_pixelcut(req: HttpCall) -> HttpCall {
    req.header(
        "User-Agent",
        "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36",
    )
    .header("Accept", "application/json, text/plain, */*")
    .header("sec-ch-ua", "\"Chromium\";v=\"139\", \"Not;A=Brand\";v=\"99\"")
    .header("x-locale", "en")
    .header("x-client-version", "web:pixa.com:4a5b0af2")
    .header("sec-ch-ua-mobile", "?1")
    .header("sec-ch-ua-platform", "\"Android\"")
    .header("origin", "https://www.pixelcut.ai")
    .header("sec-fetch-site", "cross-site")
    .header("sec-fetch-mode", "cors")
    .header("sec-fetch-dest", "empty")
    .header("referer", "https://www.pixelcut.ai/")
    .header(
        "accept-language",
        "id-ID,id;q=0.9,en-AU;q=0.8,en;q=0.7,en-US;q=0.6",
    )
}

/// `res.headers.get("content-type") || ""`.
fn content_type(res: &HttpResponse) -> String {
    res.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// `String(v)` — `new Error(objek)` memakai `String(objek)`.
fn teks_js(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        serde_json::Value::Object(_) => "[object Object]".to_string(),
        serde_json::Value::Null => "null".to_string(),
    }
}

/// Gabungan dua cabang error `removebackground.js:36-45` (dan `:61-70`):
/// kembalikan pesan dari JSON kalau bisa, kalau tidak tempel 200 karakter teks.
fn pesan_gagal(res: &HttpResponse, prefix: &str, kunci: [&str; 2]) -> String {
    let teks = res.text();
    let status = res.status;
    match serde_json::from_str::<serde_json::Value>(&teks) {
        Ok(json) => {
            let terpilih = kunci
                .iter()
                .find_map(|k| json.get(*k).filter(|v| crate::common::truthy(v)))
                .map(teks_js);
            terpilih.unwrap_or_else(|| format!("{prefix} failed: {status}"))
        }
        // `SyntaxError` -> `"{prefix} failed: {status} {teks.slice(0, 200)}"`.
        Err(_) => format!(
            "{prefix} failed: {status} {}",
            teks.chars().take(200).collect::<String>()
        ),
    }
}

/// `pixelcutRemove(buf)` (`removebackground.js:9-47`) — balikan byte PNG hasil
/// matte, atau `Err` dengan pesan yang sama seperti JS.
pub fn pixelcut_remove(http: &dyn HttpGet, jpeg: &[u8]) -> Result<Vec<u8>, HttpError> {
    let res = http.call(
        &header_pixelcut(HttpCall::post("https://api2.pixelcut.app/image/matte/v1")).multipart(
            vec![
                MultipartPart::file("image", "image.jpg", jpeg.to_vec())
                    .with_mimetype("image/jpeg"),
                MultipartPart::field("format", "png"),
                MultipartPart::field("model", "v1"),
            ],
        )
        .terima_semua_status(),
    )?;

    let gagal = !(200..300).contains(&res.status) || !content_type(&res).contains("image/");
    if gagal {
        return Err(HttpError {
            message: pesan_gagal(&res, "pixelcut", ["error", "message"]),
        });
    }
    Ok(res.body.bytes().to_vec())
}

/// `bgninjaRemove(buf)` (`removebackground.js:49-72`).
pub fn bgninja_remove(http: &dyn HttpGet, jpeg: &[u8]) -> Result<Vec<u8>, HttpError> {
    let res = http.call(
        &HttpCall::post("https://bgninja.com/api/remove")
            .header("User-Agent", "Mozilla/5.0")
            .multipart(vec![
                MultipartPart::file("file", "image.jpg", jpeg.to_vec())
                    .with_mimetype("image/jpeg"),
                MultipartPart::field("src", "wa-bot"),
            ])
            .terima_semua_status(),
    )?;

    let gagal = !(200..300).contains(&res.status) || !content_type(&res).contains("image/");
    if gagal {
        return Err(HttpError {
            message: pesan_gagal(&res, "bgninja", ["error", "detail"]),
        });
    }
    Ok(res.body.bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::HttpBody;
    use std::sync::Mutex;

    /// `(status, body, headers)` untuk stub HTTP — alias biar tipe
    /// balancer-nya tidak memicu `clippy::type_complexity`.
    type Balasan = (u16, String, Vec<(String, String)>);

    /// Stub `HttpGet` yang membalas berdasar URL dan bisa menyertakan header
    /// respons (dipakai untuk menguji syarat `content-type`).
    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<Balasan>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: &[(u16, &str, &str)]) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(
                    balasan
                        .iter()
                        .map(|(s, t, ct)| {
                            let headers = if ct.is_empty() {
                                Vec::new()
                            } else {
                                vec![("content-type".to_string(), ct.to_string())]
                            };
                            (*s, t.to_string(), headers)
                        })
                        .collect(),
                ),
                hitung: Mutex::new(0),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("removebackground cuma POST")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &serde_json::Value,
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
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
                body: HttpBody::Bytes {
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

    #[test]
    fn pixelcut_mengirim_multipart_lengkap_dan_mengembalikan_byte() {
        let http = StubHttp::baru(&[(200, "PNG-BYTES", "image/png")]);
        let hasil = pixelcut_remove(&http, b"JPEG-BYTES").unwrap();
        assert_eq!(hasil, b"PNG-BYTES".to_vec());

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, "https://api2.pixelcut.app/image/matte/v1");
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0].name, "image");
        assert_eq!(parts[0].file_name.as_deref(), Some("image.jpg"));
        assert_eq!(parts[0].mimetype.as_deref(), Some("image/jpeg"));
        assert_eq!(parts[0].body.bytes(), b"JPEG-BYTES");
        assert_eq!(parts[1].name, "format");
        assert_eq!(parts[1].body.bytes(), b"png");
        assert_eq!(parts[2].name, "model");
        assert_eq!(parts[2].body.bytes(), b"v1");
        assert_eq!(cari_header(&p[0], "x-client-version"), Some("web:pixa.com:4a5b0af2".to_string()));
        assert_eq!(cari_header(&p[0], "origin"), Some("https://www.pixelcut.ai".to_string()));
        assert!(cari_header(&p[0], "sec-fetch-mode").is_some());
    }

    #[test]
    fn bgninja_mengirim_field_src_dan_tanpa_header_pixelcut() {
        let http = StubHttp::baru(&[(200, "PNG", "image/png")]);
        bgninja_remove(&http, b"JPEG").unwrap();
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, "https://bgninja.com/api/remove");
        assert_eq!(cari_header(&p[0], "user-agent"), Some("Mozilla/5.0".to_string()));
        assert_eq!(p[0].headers.len(), 1, "hanya User-Agent");
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts[0].name, "file");
        assert_eq!(parts[1].name, "src");
        assert_eq!(parts[1].body.bytes(), b"wa-bot");
    }

    #[test]
    fn status_gagal_tetap_gagal_walaupun_content_type_gambar() {
        // `removebackground.js:36` — syaratnya `!res.ok || !ct.includes("image/")`.
        // Status 502 membuat `!res.ok` benar, jadi content-type gambar
        // tidak menolong: JS melempar di sini juga.
        let http = StubHttp::baru(&[(502, r#"{"error":"server tumbang"}"#, "image/png")]);
        assert_eq!(
            pixelcut_remove(&http, b"J").unwrap_err().message,
            "server tumbang"
        );

        // Tanpa `error`/`message` -> bawaan, teks JSON ikut gagal parse.
        let http = StubHttp::baru(&[(404, "PNG", "image/png")]);
        assert_eq!(
            pixelcut_remove(&http, b"J").unwrap_err().message,
            "pixelcut failed: 404 PNG"
        );
    }

    #[test]
    fn content_type_bukan_gambar_membaca_pesan_dari_json() {
        let http = StubHttp::baru(&[(400, r#"{"error":"gambar terlalu besar"}"#, "application/json")]);
        assert_eq!(
            pixelcut_remove(&http, b"J").unwrap_err().message,
            "gambar terlalu besar"
        );

        let http = StubHttp::baru(&[(400, r#"{"message":"kunci salah"}"#, "application/json")]);
        assert_eq!(pixelcut_remove(&http, b"J").unwrap_err().message, "kunci salah");

        // Tidak ada `error`/`message` -> bawaan.
        let http = StubHttp::baru(&[(422, r#"{"detail":"x"}"#, "application/json")]);
        assert_eq!(
            pixelcut_remove(&http, b"J").unwrap_err().message,
            "pixelcut failed: 422"
        );
    }

    #[test]
    fn body_bukan_json_menggunakan_teks_200_karakter() {
        let panjang = "a".repeat(500);
        let http = StubHttp::baru(&[(500, &panjang, "text/html")]);
        let salah = pixelcut_remove(&http, b"J").unwrap_err();
        assert_eq!(salah.message, format!("pixelcut failed: 500 {}", "a".repeat(200)));
        assert!(!salah.message.contains(&"a".repeat(201)));
    }

    #[test]
    fn content_type_kosong_ikut_gagal_walaupun_status_200() {
        let http = StubHttp::baru(&[(200, r#"{"error":"ditolak"}"#, "")]);
        assert_eq!(pixelcut_remove(&http, b"J").unwrap_err().message, "ditolak");
    }

    #[test]
    fn bgninja_memakai_kunci_detail() {
        let http = StubHttp::baru(&[(403, r#"{"detail":"terlalu sering"}"#, "application/json")]);
        assert_eq!(bgninja_remove(&http, b"J").unwrap_err().message, "terlalu sering");

        let http = StubHttp::baru(&[(403, "ditolak", "text/plain")]);
        assert_eq!(
            bgninja_remove(&http, b"J").unwrap_err().message,
            "bgninja failed: 403 ditolak"
        );
    }
}