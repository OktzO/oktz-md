//! `src/scraper/upscaler.js` — kirim satu gambar ke aienhancer.ai, lalu polling
//! hasilnya sepuluh kali.
//!
//! Semantik yang disalin dari JS:
//! - Dua POST JSON dengan header tetap (`origin`, `referer`, `user-agent`,
//!   `content-type`).
//! - `for (let i = 0; i < 10; i++)` dengan `await sleep(2500)` DI AWAL tiap
//!   percobaan — jadi funciones sleep 2,5 detik walau percobaan pertama sudah
//!   berisi hasil.
//! - `if (data && data.output)` — cek truthy, bukan `=== true`.
//! - `id`, `output`, `input` disalin apa adanya; `input` yang `undefined`
//!   membuat kuncinya tidak ikut (JSON.stringify).
//! - Falls through tanpa `return` kalau sepuluh percobaan habis: JS
//!   mengembalikan `undefined`, di sini `Value::Null`.
//! - Seluruh badan dalam `try`, jadi kegagalan apa pun (termasuk membaca
//!   berkas) jadi `{ status: "error", msg: e.message }`, bukan `Err`.
//!
//! Beda yang diketahui:
//! - `fs.readFileSync(path)` jadi `std::fs::read(path)` (crate ini sudah
//!   memakainya di `hd.rs`); byte-nya di-encode base64 sendiri karena tidak ada
//!   dependency base64 di crate ini. Pesan gagalnya berbeda dari Node
//!   (`No such file or directory (os error 2)`, bukan `ENOENT: ...`).
//! - `axios` melempar untuk di luar 2xx; agar `msg` sama dengan axios,
//!   permintaan memakai `.terima_semua_status()` dan teks
//!   `Request failed with status code NNN` dibuat sendiri.
//! - Blok IIFE di akhir JS (`console.log(await upscaler("./z.png"))`) tidak
//!   ikut — file ini hanya Exports fungsi.
//! - Jeda 2500 ms dibuat parameter privat `jeda_ms` supaya test tidak menunggu
//!   25 detik; jalur produksi tetap 2500.

use std::time::Duration;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const CREATE_URL: &str = "https://aienhancer.ai/api/v1/r/image-enhance/create";
const RESULT_URL: &str = "https://aienhancer.ai/api/v1/r/image-enhance/result";
const SETTINGS: &str = "kRpBbpnRCD2nL2RxnnuoMo7MBc0zHndTDkWMl9aW+Gw=";
const PERCOBAAN: u32 = 10;

/// Header tetap kedua POST (`upscaler.js:7-12`).
fn header_dasar(req: HttpCall) -> HttpCall {
    req.header("User-Agent", "Mozilla/5.0 (Linux; Android 10)")
        .header("Content-Type", "application/json")
        .header("origin", "https://aienhancer.ai")
        .header("referer", "https://aienhancer.ai/ai-image-upscaler")
}

/// Base64 standar dengan padding — `Buffer#toString("base64")`. Ditulis tangan
/// karena crate ini tidak punya dependency base64.
fn base64_encode(data: &[u8]) -> String {
    const TABEL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for blok in data.chunks(3) {
        let ambil = |i: usize| u32::from(blok.get(i).copied().unwrap_or(0));
        let n = (ambil(0) << 16) | (ambil(1) << 8) | ambil(2);
        let jumlah = match blok.len() {
            3 => 4,
            2 => 3,
            _ => 2,
        };
        for i in 0..4 {
            if i < jumlah {
                out.push(TABEL[((n >> (18 - i * 6)) & 0x3F) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// `a.b.c` — kalau ada ruas yang nullish, JS melempar TypeError dengan teks
/// yang disalin huruf per huruf.
///
/// Batas: body yang bukan JSON jadi `Value::Null` di sini, jadi pesannya
/// `null` walaupun di JS body string membuat `undefined`.
fn berantai<'a>(v: &'a Value, jalur: &[&str]) -> Result<&'a Value, HttpError> {
    let mut cur = v;
    for kunci in jalur {
        match cur.get(kunci) {
            Some(baru) => cur = baru,
            None => {
                let apa = if cur.is_null() { "null" } else { "undefined" };
                return Err(HttpError {
                    message: format!("Cannot read properties of {apa} (reading '{kunci}')"),
                });
            }
        }
    }
    Ok(cur)
}

/// `upscaler(path)` (`upscaler.js:3-48`) — seluruh badan dibungkus `catch`.
pub fn upscaler(http: &dyn HttpGet, path: &str) -> Result<Value, HttpError> {
    match alur(http, path, 2500) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": "error", "msg": e.message })),
    }
}

/// Alur tanpa pembungkus; `jeda_ms` hanya supaya test tidak benar-benar
/// menunggu (nilai produksi 2500).
fn alur(http: &dyn HttpGet, path: &str, jeda_ms: u64) -> Result<Value, HttpError> {
    let img = std::fs::read(path).map_err(|e| HttpError {
        message: e.to_string(),
    })?;

    let res = http.call(
        &header_dasar(HttpCall::post(CREATE_URL)).json(&json!({
            "model": 3,
            "image": format!("data:image/jpeg;base64,{}", base64_encode(&img)),
            "settings": SETTINGS,
        })),
    )?;
    if !(200..300).contains(&res.status) {
        return Err(HttpError {
            message: format!("Request failed with status code {}", res.status),
        });
    }
    let body = res.json().unwrap_or(Value::Null);
    let id = berantai(&body, &["data", "id"])?.clone();

    for _ in 0..PERCOBAAN {
        // `await new Promise(r => setTimeout(r, 2500))` di awal perulangan.
        std::thread::sleep(Duration::from_millis(jeda_ms));
        let res = http.call(
            &header_dasar(HttpCall::post(RESULT_URL)).json(&json!({ "task_id": id })),
        )?;
        if !(200..300).contains(&res.status) {
            return Err(HttpError {
                message: format!("Request failed with status code {}", res.status),
            });
        }
        // `result.data.data` — body harus disimpan dulu, jangan dipinjam dari
        // nilai sementara.
        let body = res.json().unwrap_or(Value::Null);
        let data = berantai(&body, &["data"])?;
        if let Some(output) = data.get("output").filter(|v| truthy(v)) {
            let mut out = Map::new();
            out.insert("id".to_string(), id.clone());
            out.insert("output".to_string(), output.clone());
            if let Some(v) = data.get("input") {
                out.insert("input".to_string(), v.clone());
            }
            return Ok(Value::Object(out));
        }
    }

    // `return` tidak terjangkau di JS → `undefined`.
    Ok(Value::Null)
}

/// Bantu test: balas `HttpResponse` tetap tanpa jaringan.
#[cfg(test)]
fn dummy_res(status: u16, body: &str) -> oktz_plugin_api::HttpResponse {
    oktz_plugin_api::HttpResponse {
        status,
        body: oktz_plugin_api::HttpBody::Bytes {
            data: std::sync::Arc::from(body.as_bytes()),
            mimetype: None,
        },
        headers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::HttpBody;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan, membalas status +
    /// body berurutan (balasan terakhir dipakai ulang setelah habis).
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: &[(u16, &str)]) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(
                    balasan
                        .iter()
                        .map(|(s, b)| (*s, b.to_string()))
                        .collect::<Vec<_>>(),
                ),
                hitung: Mutex::new(0),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma POST")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, body) = balasan[i].clone();
            Ok(dummy_res(status, &body))
        }
    }

    /// Berkas semu berisi satu piksel JPEG — isinya tidak penting, yang penting
    /// terbaca lewat `std::fs::read` dan jadi base64.
    ///
    /// Namanya diberi nomor urut per test: nama tetap dipakai bersama akan
    /// saling menimpa karena `std::fs::write` memotong berkas, dan test jalan
    /// paralel. Akibatnya satu test kadang membaca berkas 0 byte dan base64-nya
    /// kosong — gangguan yang hanya muncul sesekali.
    fn berkas_semu() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static URUT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "oktz-upscaler-test-{}-{}.jpg",
            std::process::id(),
            URUT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::write(&p, [0xFF, 0xD8, 0xFF, 0xD9]).expect("tulis berkas uji");
        p
    }

    #[test]
    fn membuat_tugas_lalu_mengembalikan_output_pada_percobaan_pertama() {
        let http = StubHttp::baru(&[
            (200, r#"{"data":{"id":"tugas-1"}}"#),
            (200, r#"{"data":{"output":"https://out/1.png","input":"https://in/1.png"}}"#),
        ]);
        let hasil = alur(&http, berkas_semu().to_str().unwrap(), 0).unwrap();

        assert_eq!(hasil["id"], "tugas-1");
        assert_eq!(hasil["output"], "https://out/1.png");
        assert_eq!(hasil["input"], "https://in/1.png");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].url, CREATE_URL);
        assert_eq!(p[1].url, RESULT_URL);
        let cari = |req: &HttpCall, nama: &str| {
            req.headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari(&p[0], "origin"), Some("https://aienhancer.ai".to_string()));
        assert_eq!(
            cari(&p[0], "referer"),
            Some("https://aienhancer.ai/ai-image-upscaler".to_string())
        );
        assert_eq!(cari(&p[0], "User-Agent"), Some("Mozilla/5.0 (Linux; Android 10)".to_string()));
        // Body create: model, data URI base64, settings apa adanya.
        let badan = String::from_utf8(p[0].body.as_ref().unwrap().bytes().to_vec()).unwrap();
        assert!(badan.contains(r#""model":3"#), "{badan}");
        assert!(badan.contains(r#""image":"data:image/jpeg;base64,/9j/2Q=="#), "{badan}");
        assert!(badan.contains(r#""settings":"kRpBbpnRCD2nL2RxnnuoMo7MBc0zHndTDkWMl9aW+Gw=""#), "{badan}");
        assert_eq!(
            String::from_utf8(p[1].body.as_ref().unwrap().bytes().to_vec()).unwrap(),
            r#"{"task_id":"tugas-1"}"#
        );
    }

    #[test]
    fn sepuluh_percobaan_tanpa_output_kembali_null_seperti_undefined() {
        let http = StubHttp::baru(&[
            (200, r#"{"data":{"id":7}}"#),
            (200, r#"{"data":{"output":null}}"#),
        ]);
        let hasil = alur(&http, berkas_semu().to_str().unwrap(), 0).unwrap();
        assert_eq!(hasil, Value::Null);
        // 1 create + 10 polling.
        assert_eq!(http.permintaan.lock().unwrap().len(), 11);
    }

    #[test]
    fn id_hilang_dibungkus_catch_seperti_js() {
        let http = StubHttp::baru(&[(200, r#"{"data":{}}"#)]);
        let hasil = upscaler(&http, berkas_semu().to_str().unwrap()).unwrap();
        assert_eq!(hasil["status"], "error");
        assert_eq!(
            hasil["msg"],
            "Cannot read properties of undefined (reading 'id')"
        );
        // Tidak ada permintaan polling kalau `create` gagal.
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }

    #[test]
    fn status_bukan_2xx_dan_berkas_hilang_menjadi_objek_error() {
        let http = StubHttp::baru(&[(503, "{}")]);
        let hasil = upscaler(&http, berkas_semu().to_str().unwrap()).unwrap();
        assert_eq!(hasil["status"], "error");
        assert_eq!(hasil["msg"], "Request failed with status code 503");

        let http = StubHttp::baru(&[(200, "{}")]);
        let hasil = upscaler(&http, "/tidak/ada/z.png").unwrap();
        assert_eq!(hasil["status"], "error");
        assert!(
            hasil["msg"].as_str().unwrap().contains("No such file"),
            "{}",
            hasil["msg"]
        );
        assert_eq!(http.permintaan.lock().unwrap().len(), 0);
    }

    #[test]
    fn base64_polos_dan_padding_sesuai_node() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"a"), "YQ==");
        assert_eq!(base64_encode(b"ab"), "YWI=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
        assert_eq!(base64_encode(&[0xFF, 0xD8, 0xFF, 0xD9]), "/9j/2Q==");
    }
}