//! `src/scraper/img2prompt.js` — kirim gambar ke `imageprompt.org` untuk
//! mendapat prompt.
//!
//! Yang ikut disalin dari JS:
//! - `base64Url` SELALU diawali `data:image/webp;base64,` apa pun tipe
//!   berkas sebenarnya (`img2prompt.js:10`) — plugin menulis `.webp` ke disk
//!   supaya ini kebetulan benar.
//! - `imageModelId: 0` dan `language: 'en'` ikut, dan urutan kunci body itu.
//! - `catch` mengembalikan `{ status: 'eror', msg: e.message }` — ejaan
//!   `'eror'` itu milik JS dan plugin Rails membacanya (`imgtoprompt.js:57`),
//!   jadi tidak boleh dibetulkan.
//! - Kunci `prompt`/`generatedAt` yang `undefined` tidak ikut hasil, persis
//!   seperti `JSON.stringify`.
//!
//! Beda yang diketahui: JS membaca berkas dari disk (`fs.readFileSync(media)`)
//! dan scraper Rust tidak menyentuh berkas. Byte gambar masuk sebagai argumen
//! (`gambar`), base64-nya dihitung di sini. Kalau pemanggil salah handing MIME,
//! body-nya tetap menyebut `image/webp` — sama seperti JS.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

/// `imgtoprompt(media)` (`img2prompt.js:3-32`).
pub fn imgtoprompt(http: &dyn HttpGet, gambar: &[u8]) -> Result<Value, HttpError> {
    match alur(http, gambar) {
        Ok(v) => Ok(v),
        // `catch(e){ return { status:'eror', msg: e.message } }`.
        Err(e) => Ok(json!({ "status": "eror", "msg": e.message })),
    }
}

fn alur(http: &dyn HttpGet, gambar: &[u8]) -> Result<Value, HttpError> {
    let body = json!({
        "base64Url": format!("data:image/webp;base64,{}", base64(gambar)),
        "imageModelId": 0,
        "language": "en",
    });
    let res = http.call(
        &HttpCall::post("https://imageprompt.org/api/ai/prompts/image")
            .header("User-Agent", "Mozilla/5.0 (Linux; Android 10)")
            .header("Content-Type", "application/json")
            .header("origin", "https://imageprompt.org")
            .header("referer", "https://imageprompt.org/image-to-prompt")
            .json(&body),
    )?;
    // Body non-JSON di axios menjadi string; `r.data.prompt` lalu `undefined`,
    // jadi hasilnya objek KOSONG — bukan error.
    let data = res.json().unwrap_or(Value::Null);

    // `r.data.prompt` / `r.data.generatedAt` — `undefined` tidak ikut hasil.
    let mut o = Map::new();
    for kunci in ["prompt", "generatedAt"] {
        if let Some(v) = data.get(kunci) {
            o.insert(kunci.to_string(), v.clone());
        }
    }
    Ok(Value::Object(o))
}

/// `Buffer.toString('base64')` — abjad standar dengan padding `=`.
fn base64(data: &[u8]) -> String {
    const ABJAD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for blok in data.chunks(3) {
        let b = [blok[0], *blok.get(1).unwrap_or(&0), *blok.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ABJAD[(n >> 18) as usize & 63] as char);
        out.push(ABJAD[(n >> 12) as usize & 63] as char);
        out.push(if blok.len() > 1 {
            ABJAD[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if blok.len() > 2 {
            ABJAD[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet` — mencatat `HttpCall`, membalas body tetap; tanpa
    /// jaringan.
    #[derive(Debug)]
    struct Stub {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: String,
        galat: Option<String>,
    }

    impl Stub {
        fn baru(balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: balasan.to_string(),
                galat: None,
            }
        }
        fn gagal(pesan: &str) -> Self {
            Self {
                galat: Some(pesan.to_string()),
                ..Self::baru("")
            }
        }
        fn jawab(&self) -> Result<HttpResponse, HttpError> {
            if let Some(pesan) = &self.galat {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini kirim JSON")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            self.jawab()
        }
    }

    fn cari_header<'a>(req: &'a HttpCall, nama: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    fn badan(p: &HttpCall) -> String {
        String::from_utf8(p.body.as_ref().unwrap().bytes().to_vec()).unwrap()
    }

    #[test]
    fn base64_sama_dengan_buffer_node() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"a"), "YQ==");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"abc"), "YWJj");
        assert_eq!(base64(b"Mozilla/5.0"), "TW96aWxsYS81LjA=");
    }

    #[test]
    fn sukses_mengembalikan_prompt_dan_generated_at() {
        let http = Stub::baru(r#"{"prompt":"a cat","generatedAt":"2024-01-01"}"#);
        let hasil = imgtoprompt(&http, b"abc").unwrap();
        assert_eq!(hasil["prompt"], "a cat");
        assert_eq!(hasil["generatedAt"], "2024-01-01");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(p[0].url, "https://imageprompt.org/api/ai/prompts/image");
        assert_eq!(cari_header(&p[0], "user-agent"), Some("Mozilla/5.0 (Linux; Android 10)"));
        assert_eq!(cari_header(&p[0], "content-type"), Some("application/json"));
        assert_eq!(cari_header(&p[0], "origin"), Some("https://imageprompt.org"));
        assert_eq!(
            cari_header(&p[0], "referer"),
            Some("https://imageprompt.org/image-to-prompt")
        );
        // Awalan `image/webp` tetap walau byte-nya apa pun.
        assert_eq!(
            badan(&p[0]),
            r#"{"base64Url":"data:image/webp;base64,YWJj","imageModelId":0,"language":"en"}"#
        );
    }

    #[test]
    fn field_hilang_tidak_menghasilkan_kunci_kosong() {
        let http = Stub::baru(r#"{"prompt":"x"}"#);
        let hasil = imgtoprompt(&http, b"a").unwrap();
        assert_eq!(hasil["prompt"], "x");
        assert!(hasil.get("generatedAt").is_none());
    }

    #[test]
    fn galat_transport_kembali_sebagai_objek_eror() {
        let http = Stub::gagal("offline");
        let hasil = imgtoprompt(&http, b"a").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert_eq!(hasil["msg"], "offline");
    }

    #[test]
    fn galat_4xx_kembali_sebagai_objek_eror_bukan_lempar() {
        // axios melempar untuk 4xx dan `catch` hanya membungkusnya jadi objek.
        let http = Stub::gagal("Request failed with status code 400");
        let hasil = imgtoprompt(&http, b"a").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert_eq!(hasil["msg"], "Request failed with status code 400");
    }

    #[test]
    fn badan_bukan_json_memberi_objek_kosong_bukan_error() {
        // `r.data` jadi string, `r.data.prompt` jadi `undefined`, dan
        // `JSON.stringify` membuangnya -> `{}`.
        let http = Stub::baru("<html>halo</html>");
        let hasil = imgtoprompt(&http, b"a").unwrap();
        assert_eq!(hasil, json!({}));
        assert!(hasil.get("status").is_none());
    }
}