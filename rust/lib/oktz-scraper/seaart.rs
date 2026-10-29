//! `src/scraper/seaart.js` — unggah gambar ke `api.imgupscaler.ai`, sunting
//! lewat nanobanana (`kyzznekoo.zone.id`), dan pembuat gambar flux
//! (`api.yuulabs.web.id`).
//!
//! `seaart.js` mengimpor `config.js` tapi tidak memakai satu pun isinya
//! (nilai `config.APIkey` maupun env tidak pernah dirujuk di file itu), jadi
//! tidak ada kunci yang perlu jadi argumen tambahan.
//!
//! Yang ikut disalin dari JS:
//! - `live3d` di JS mengembalikan `{ image: Buffer }`; di Rust balikannya byte
//!   gambar mentah (`Vec<u8>`) karena `serde_json::Value` tidak bisa membawa
//!   buffer dan pemanggil JS memakai `.image` langsung sebagai media.
//! - Pesan `Gagal mengedit gambar menggunakan nanobanana.` dan jatuhan
//!   `data?.message || data?.error || "Gagal membuat gambar"` huruf per huruf.
//! - `form.append("object_name", object_name)` di `form-data` meng-`String()`-kan
//!   nilai non-string; nilai yang benar-benar hilang jadi `"undefined"`.
//! - `timeout: 120000` di axios tidak ditiru (batas mesin, bukan tampilan);
//!   `UreqHttp` memakai timeout global 20 detik per permintaan.
//!
//! Argumen yang perlu diisi plugin: `prompt` di JS punya nilai bawaan
//! (`"ubah warna rambut jadi biru"`) dan `ratio` bawaan `"1:1"`; Rust tidak
//! punya argumen bawaan, jadi pemanggil yang mengirim nilai bawaannya.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, MultipartPart};
use serde_json::{Value, json};

use crate::common::{encode_uri_component, truthy};

/// UA `hdrs` (`seaart.js:9`). BEDA dari UA request `live3d` (`seaart.js:57`,
/// Chrome/150) yang ditulis inline di sana — jangan disamakan.
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36";

/// `hdrs` (`seaart.js:8-19`).
fn header_dasar(req: HttpCall) -> HttpCall {
    req.header("User-Agent", UA)
        .header("Accept", "*/*")
        .header("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Origin", "https://imgupscaler.ai")
        .header("Referer", "https://imgupscaler.ai/")
        .header("sec-ch-ua", "\"Chromium\";v=\"137\", \"Not/A)Brand\";v=\"24\"")
        .header("sec-ch-ua-mobile", "?0")
        .header("sec-ch-ua-platform", "\"Linux\"")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-mode", "cors")
}

/// `randomUUID()` dari `crypto` — UUID v4 huruf kecil.
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

/// `String(v)` untuk nilai JSON (`form-data` meng-`String()`-kan nilai yang
/// bukan string/buffer/stream).
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

/// `data?.message || data?.error || "Gagal membuat gambar"` — nilai truthy
/// pertama yang menang, lalu di-`String()`-kan oleh `new Error(...)`.
fn pesan_galat(data: &Value) -> String {
    for kunci in ["message", "error"] {
        if let Some(v) = data.get(kunci)
            && truthy(v) {
                return teks_js(v);
            }
    }
    "Gagal membuat gambar".to_string()
}

/// `uploadBuf(buf)` (`seaart.js:23-45`) → `signed.result.url`.
fn upload_buf(http: &dyn HttpGet, buf: &[u8]) -> Result<String, HttpError> {
    // `const ext = "jpg"`; `form1.append("file_name", `${randomUUID()}.${ext}`)`.
    let nama = format!("{}.jpg", uuid_v4());
    let res = http.call(
        &header_dasar(HttpCall::post(
            "https://api.imgupscaler.ai/api/common/upload/upload-image",
        ))
        .header("sec-fetch-site", "same-site")
        .multipart(vec![MultipartPart::field("file_name", nama)]),
    )?;
    let reg = res.json().ok_or_else(|| HttpError {
        message: "respons upload-image bukan JSON".to_string(),
    })?;
    // `const { url: uploadUrl, object_name } = reg.result;` — di JS `result`
    // yang hilang melempar TypeError; di sini jadi `Err`.
    let url_unggah = reg
        .get("result")
        .and_then(|r| r.get("url"))
        .and_then(Value::as_str)
        .ok_or_else(|| HttpError {
            message: "respons upload-image tanpa result.url".to_string(),
        })?;
    let object_name = reg
        .get("result")
        .and_then(|r| r.get("object_name"))
        .map(teks_js)
        .unwrap_or_else(|| "undefined".to_string());

    // `axios.put(uploadUrl, buf, { headers: { "Content-Type": "image/jpeg" } })`
    http.call(&HttpCall::put(url_unggah).raw(buf.to_vec(), "image/jpeg"))?;

    let res = http.call(
        &header_dasar(HttpCall::post(
            "https://api.imgupscaler.ai/api/common/upload/sign-object",
        ))
        .header("sec-fetch-site", "same-site")
        .multipart(vec![MultipartPart::field("object_name", object_name)]),
    )?;
    let signed = res.json().ok_or_else(|| HttpError {
        message: "respons sign-object bukan JSON".to_string(),
    })?;
    signed
        .get("result")
        .and_then(|r| r.get("url"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| HttpError {
            message: "respons sign-object tanpa result.url".to_string(),
        })
}

/// `live3d(imageBuffer, prompt = "ubah warna rambut jadi biru")`
/// (`seaart.js:47-70`) — balikan byte gambar, lihat catatan di atas.
pub fn live3d(http: &dyn HttpGet, image_buffer: &[u8], prompt: &str) -> Result<Vec<u8>, HttpError> {
    let img_url = upload_buf(http, image_buffer)?;

    let alamat = format!(
        "https://kyzznekoo.zone.id/api/img/v2/nanobanana?url={}&prompt={}",
        encode_uri_component(&img_url),
        encode_uri_component(prompt)
    );
    let res = http.call(
        &HttpCall::get(alamat)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
            )
            .header("Content-Type", "application/json"),
    )?;

    // Body yang bukan JSON membuat `data` jadi string di axios; `data?.status`
    // tetap falsy dan berakhir di pesan galat yang sama.
    let data = res.json().unwrap_or(Value::Null);
    let kandidat = data.get("data").and_then(|d| d.get("result_url"));
    if !data.get("status").map(truthy).unwrap_or(false) || !kandidat.map(truthy).unwrap_or(false) {
        return Err(HttpError {
            message: "Gagal mengedit gambar menggunakan nanobanana.".to_string(),
        });
    }
    let url_hasil = kandidat.and_then(Value::as_str).unwrap_or_default();

    // `axios.get(data.data.result_url, { responseType: "arraybuffer" })`
    let gambar = http.call(&HttpCall::get(url_hasil))?;
    Ok(gambar.body.bytes().to_vec())
}

/// `fluxImage(message, ratio = "1:1")` (`seaart.js:72-91`) → `data.result`.
pub fn flux_image(http: &dyn HttpGet, message: &str, ratio: &str) -> Result<Value, HttpError> {
    let res = http.call(
        &HttpCall::post("https://api.yuulabs.web.id/api/ai/flux-img")
            .header("Content-Type", "application/json")
            .json(&json!({ "message": message, "ratio": ratio })),
    )?;

    let data = res.json().unwrap_or(Value::Null);
    if !data.get("status").map(truthy).unwrap_or(false)
        || !data
            .get("result")
            .and_then(|r| r.get("url"))
            .map(truthy)
            .unwrap_or(false)
    {
        return Err(HttpError {
            message: pesan_galat(&data),
        });
    }
    Ok(data.get("result").cloned().unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpCall, HttpResponse};
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body) berurutan; setelah habis, balasan terakhir dipakai lagi.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn new(balasan: Vec<(u16, String)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
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
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, teks) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: std::sync::Arc::from(teks.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn cari_header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn uuid_berpola(s: &str) -> bool {
        let re = regex::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$",
        )
        .unwrap();
        re.is_match(s)
    }

    fn alur_unggah() -> Vec<(u16, String)> {
        vec![
            (
                200,
                r#"{"result":{"url":"https://unggah.example/x","object_name":"obj/1"}}"#.to_string(),
            ),
            (200, String::new()),
            (
                200,
                r#"{"result":{"url":"https://signed.example/x.jpg"}}"#.to_string(),
            ),
        ]
    }

    #[test]
    fn live3d_mengunggah_menandatangani_lalu_mengambil_gambar() {
        let mut balasan = alur_unggah();
        balasan.push((
            200,
            r#"{"status":true,"data":{"result_url":"https://hasil.example/y.png"}}"#.to_string(),
        ));
        balasan.push((200, "GAMBAR".to_string()));
        let http = StubHttp::new(balasan);

        let gambar = live3d(&http, &[1u8, 2, 3], "ubah warna rambut jadi biru").unwrap();
        assert_eq!(gambar, b"GAMBAR".to_vec());

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 5);

        // 1. upload-image: multipart `file_name` = `<uuid>.jpg`.
        assert_eq!(
            p[0].url,
            "https://api.imgupscaler.ai/api/common/upload/upload-image"
        );
        let Some(HttpBody::Multipart(parts)) = &p[0].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "file_name");
        assert_eq!(parts[0].file_name, None);
        let nama = String::from_utf8(parts[0].body.bytes().to_vec()).unwrap();
        assert!(nama.ends_with(".jpg"), "nama berkas aneh: {nama}");
        assert!(uuid_berpola(&nama[..nama.len() - 4]), "bukan UUID v4: {nama}");
        assert_eq!(
            cari_header(&p[0], "sec-fetch-site"),
            Some("same-site".to_string())
        );
        assert_eq!(
            cari_header(&p[0], "origin"),
            Some("https://imgupscaler.ai".to_string())
        );
        assert!(cari_header(&p[0], "user-agent").unwrap().contains("Chrome/137"));

        // 2. PUT byte gambar mentah dengan content-type image/jpeg.
        assert_eq!(p[1].method, oktz_plugin_api::HttpMethod::Put);
        assert_eq!(p[1].url, "https://unggah.example/x");
        let Some(HttpBody::Bytes { data, mimetype }) = &p[1].body else {
            panic!("body PUT bukan byte");
        };
        assert_eq!(data.as_ref(), &[1u8, 2, 3][..]);
        assert_eq!(mimetype.as_deref(), Some("image/jpeg"));

        // 3. sign-object: multipart `object_name`.
        assert_eq!(
            p[2].url,
            "https://api.imgupscaler.ai/api/common/upload/sign-object"
        );
        let Some(HttpBody::Multipart(parts)) = &p[2].body else {
            panic!("body bukan multipart");
        };
        assert_eq!(parts[0].name, "object_name");
        assert_eq!(parts[0].body.bytes(), &b"obj/1"[..]);

        // 4. nanobanana: url & prompt di-encode `encodeURIComponent`.
        assert_eq!(
            p[3].url,
            "https://kyzznekoo.zone.id/api/img/v2/nanobanana?url=https%3A%2F%2Fsigned.example%2Fx.jpg&prompt=ubah%20warna%20rambut%20jadi%20biru"
        );
        assert_eq!(
            cari_header(&p[3], "content-type"),
            Some("application/json".to_string())
        );

        // 5. GET gambar hasil tanpa header kustom.
        assert_eq!(p[4].url, "https://hasil.example/y.png");
        assert_eq!(p[4].headers.len(), 0);
    }

    #[test]
    fn live3d_galat_nanobanana_memakai_pesan_js() {
        let mut balasan = alur_unggah();
        balasan.push((200, r#"{"status":false}"#.to_string()));
        let http = StubHttp::new(balasan);

        let err = live3d(&http, &[1u8], "ubah").unwrap_err();
        assert_eq!(err.message, "Gagal mengedit gambar menggunakan nanobanana.");
        // Percobaan berhenti sebelum mengunduh gambar hasil.
        assert_eq!(http.permintaan.lock().unwrap().len(), 4);
    }

    #[test]
    fn upload_gagal_tanpa_result_url_jadi_error() {
        let http = StubHttp::new(vec![(200, r#"{"result":{}}"#.to_string())]);
        let err = live3d(&http, &[1u8], "ubah").unwrap_err();
        assert_eq!(err.message, "respons upload-image tanpa result.url");
    }

    #[test]
    fn flux_image_mengirim_body_json_dan_mengembalikan_result() {
        let http = StubHttp::new(vec![(
            200,
            r#"{"status":true,"result":{"url":"https://x/gambar.png","id":7}}"#.to_string(),
        )]);
        let hasil = flux_image(&http, "kucing", "1:1").unwrap();
        assert_eq!(hasil["url"], "https://x/gambar.png");
        assert_eq!(hasil["id"], 7);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, "https://api.yuulabs.web.id/api/ai/flux-img");
        assert_eq!(
            cari_header(&p[0], "content-type"),
            Some("application/json".to_string())
        );
        let Some(HttpBody::Bytes { data, .. }) = &p[0].body else {
            panic!("body bukan JSON");
        };
        assert_eq!(data.as_ref(), &br#"{"message":"kucing","ratio":"1:1"}"#[..]);
    }

    #[test]
    fn flux_image_memakai_pesan_dari_data_lalu_jatuh_ke_pesan_bawaan() {
        let http = StubHttp::new(vec![(
            200,
            r#"{"status":false,"message":"kuota habis"}"#.to_string(),
        )]);
        assert_eq!(
            flux_image(&http, "kucing", "1:1").unwrap_err().message,
            "kuota habis"
        );

        let http = StubHttp::new(vec![(
            200,
            r#"{"status":false,"error":"kunci salah"}"#.to_string(),
        )]);
        assert_eq!(
            flux_image(&http, "kucing", "1:1").unwrap_err().message,
            "kunci salah"
        );

        let http = StubHttp::new(vec![(200, r#"{"status":false}"#.to_string())]);
        assert_eq!(
            flux_image(&http, "kucing", "1:1").unwrap_err().message,
            "Gagal membuat gambar"
        );

        // `status` benar tapi `result.url` kosong → tetap pesan bawaan.
        let http = StubHttp::new(vec![(
            200,
            r#"{"status":true,"result":{"url":""}}"#.to_string(),
        )]);
        assert_eq!(
            flux_image(&http, "kucing", "1:1").unwrap_err().message,
            "Gagal membuat gambar"
        );
    }

    #[test]
    fn balasan_bukan_json_dianggap_gagal() {
        let http = StubHttp::new(vec![(200, "<html>gerbang</html>".to_string())]);
        assert_eq!(
            flux_image(&http, "kucing", "1:1").unwrap_err().message,
            "Gagal membuat gambar"
        );
    }
}
