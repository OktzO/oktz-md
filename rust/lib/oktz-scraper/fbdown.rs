//! `src/scraper/fbdown.js` — ambil tautan video Facebook dari agregator nexray.
//!
//! satu GET, tanpa API key. Yang disalin apa adanya:
//! - `params: { url }` axios memakai `encode` sendiri (`axios/lib/helpers/buildURL.js`):
//!   `encodeURIComponent` lalu `%3A`/`%24`/`%2C` dikembalikan ke `:`/`$`/`,`
//!   dan `%20` jadi `+`. BEDA dari `encodeURIComponent` polos di `common.rs`.
//! - `res.data?.status === true && res.data?.result` — perbandingan `=== true`
//!   (bukan truthy) dan `result` harus ada.
//! - `if (r.video_hd)` / `if (r.video_sd)` diuji truthy, jadi string kosong
//!   tidak pernah masuk `medias`. Urutan push `hd` lalu `sd` menentukan urutan.
//! - `title: r.title ?? ""` memakai `??`, jadi hanya `null`/`undefined` yang jadi
//!   string kosong — `0` atau `false` tetap ikut apa adanya.
//! - Seluruh badan dibungkus `try/catch`, jadi fungsi ini tidak pernah melempar:
//!   kegagalan jaringan berakhir sebagai `{ status: false, message }` dengan
//!   pesan dari klien HTTP, bukan `Err`.
//!
//! Catatan asal fungsi: `fbdown` dari paket npm `btch-downloader` adalah fungsi
//! LAIN (request ke `backend1.tioo.eu.org/fbdown`, bentuk hasil
//! `{ status, HD, Normal_video }`) dan hanya dipakai `src/scraper/aio.js`.
//! File ini tidak menyentuh `btch-downloader` sama sekali, jadi tidak ada
//! header `btch/<versi>` di sini.
//!
//! `httpAxios` memasang timeout 15 detik, `maxRedirects: 3`, dan
//! `maxContentLength: 25 MB` — batas mesin, bukan tampilan, jadi tidak ditiru.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const ENDPOINT: &str = "https://api.nexray.web.id/api/downloader/facebook";

/// `fbdown(url)` (`fbdown.js:3-19`) — tidak pernah `Err`.
pub fn fbdown(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    match alur(http, url) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": false, "message": e.message })),
    }
}

fn alur(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let target = format!("{ENDPOINT}?url={}", encode_axios(url));
    let res = http.call(&HttpCall::get(target))?;
    let body = res.json().unwrap_or(Value::Null);

    if body.get("status").and_then(Value::as_bool) != Some(true) {
        return Err(HttpError {
            message: "Gagal mengambil data dari API".to_string(),
        });
    }
    // `res.data?.result` — harus truthy, jadi `null`, `0`, `""`, dan `false`
    // semuanya ikut ke cabang gagal.
    let Some(result) = body.get("result").filter(|r| truthy(r)) else {
        return Err(HttpError {
            message: "Gagal mengambil data dari API".to_string(),
        });
    };

    let mut medias = Vec::new();
    // Urutan append di `fbdown.js:11-12` menentukan urutan `medias`.
    for (kunci, kualitas) in [("video_hd", "hd"), ("video_sd", "sd")] {
        if let Some(v) = result.get(kunci).filter(|v| crate::common::truthy(v)) {
            let mut m = Map::new();
            m.insert("quality".to_string(), Value::String(kualitas.to_string()));
            m.insert("url".to_string(), v.clone());
            medias.push(Value::Object(m));
        }
    }

    let mut hasil = Map::new();
    hasil.insert("status".to_string(), Value::Bool(true));
    let mut isi = Map::new();
    // `r.title ?? ""` — `??` hanya menyentuh null/undefined.
    isi.insert(
        "title".to_string(),
        match result.get("title") {
            None | Some(Value::Null) => Value::String(String::new()),
            Some(v) => v.clone(),
        },
    );
    isi.insert("medias".to_string(), Value::Array(medias));
    hasil.insert("result".to_string(), Value::Object(isi));
    Ok(Value::Object(hasil))
}

/// `encode` axios untuk `params` (`axios/lib/helpers/buildURL.js:14-21`).
///
/// `encodeURIComponent` dulu, lalu `%3A`/`%24`/`%2C` dikembalikan menjadi
/// `:`/`$`/`,` dan `%20` menjadi `+`. Huruf kecil ikut karena axios memakai
/// flag `i`.
fn encode_axios(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
        {
            out.push(c);
        } else {
            // Persis seperti `encodeURIComponent`; fours replace di bawah
            // bekerja pada hasil yang sama karena Huruf besarnya sudah sama.
            match b {
                b':' => out.push(':'),
                b'$' => out.push('$'),
                b',' => out.push(','),
                b' ' => out.push('+'),
                _ => out.push_str(&format!("%{b:02X}")),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet` tanpa jaringan: mencatat URL dan membalas JSON tetap.
    #[derive(Debug)]
    struct StubHttp {
        dilihat: Mutex<Vec<String>>,
        balasan: Mutex<String>,
    }

    impl StubHttp {
        fn baru(balasan: &str) -> Self {
            Self {
                dilihat: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan.to_string()),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.dilihat.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.lock().unwrap().clone().into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.dilihat.lock().unwrap().push(req.url.clone());
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

    #[test]
    fn hd_dan_sd_masuk_urutan_yang_sama_dengan_js() {
        let http = StubHttp::baru(
            r#"{"status":true,"result":{"title":"Uji","video_hd":"https://cdn/hd.mp4",
                "video_sd":"https://cdn/sd.mp4"}}"#,
        );
        let hasil = fbdown(&http, "https://www.facebook.com/watch?v=xxx").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["result"]["title"], "Uji");
        let medias = hasil["result"]["medias"].as_array().unwrap();
        assert_eq!(medias.len(), 2);
        assert_eq!(medias[0]["quality"], "hd");
        assert_eq!(medias[0]["url"], "https://cdn/hd.mp4");
        assert_eq!(medias[1]["quality"], "sd");
        assert_eq!(medias[1]["url"], "https://cdn/sd.mp4");

        assert_eq!(
            http.dilihat.lock().unwrap()[0],
            "https://api.nexray.web.id/api/downloader/facebook\
             ?url=https:%2F%2Fwww.facebook.com%2Fwatch%3Fv%3Dxxx"
        );
    }

    #[test]
    fn hanya_satu_kualitas_tetap_valid() {
        let sd = StubHttp::baru(r#"{"status":true,"result":{"video_sd":"https://cdn/s.mp4"}}"#);
        let hasil = fbdown(&sd, "u").unwrap();
        assert_eq!(hasil["result"]["medias"].as_array().unwrap().len(), 1);
        assert_eq!(hasil["result"]["title"], "", "title hilang jadi kosong");

        // Dua-duanya kosong: `medias` kosong, tapi `status` tetap true.
        let kosong = StubHttp::baru(r#"{"status":true,"result":{"video_hd":"","video_sd":""}}"#);
        let hasil = fbdown(&kosong, "u").unwrap();
        assert_eq!(hasil["status"], true);
        assert!(hasil["result"]["medias"].as_array().unwrap().is_empty());
    }

    #[test]
    fn title_nol_dan_false_ikut_apa_adanya_karena_operator_ku_dua_tanda() {
        for (body, hope) in [
            (r#"{"status":true,"result":{"title":0}}"#, json!(0)),
            (r#"{"status":true,"result":{"title":false}}"#, json!(false)),
            (r#"{"status":true,"result":{"title":""}}"#, json!("")),
            (r#"{"status":true,"result":{"title":null}}"#, json!("")),
            (r#"{"status":true,"result":{}}"#, json!("")),
        ] {
            let http = StubHttp::baru(body);
            let hasil = fbdown(&http, "u").unwrap();
            assert_eq!(hasil["result"]["title"], hope, "{body}");
        }
    }

    #[test]
    fn status_bukan_true_menghasilkan_pesan_bawaan() {
        for body in [
            r#"{"status":false,"result":{"video_hd":"https://cdn/hd.mp4"}}"#,
            r#"{"status":"true","result":{}}"#,
            r#"{"status":1,"result":{}}"#,
            r#"{"status":true}"#,
            r#"{"status":true,"result":null}"#,
            "{}",
        ] {
            let http = StubHttp::baru(body);
            let hasil = fbdown(&http, "u").unwrap();
            assert_eq!(hasil["status"], false, "{body}");
            assert_eq!(hasil["message"], "Gagal mengambil data dari API", "{body}");
            assert!(hasil.get("result").is_none(), "{body}");
        }
    }

    #[test]
    fn kegagalan_jaringan_juga_status_false_bukan_error() {
        // `catch` di JS menelan kegagalan transport, jadi `Ok` dengan pesan
        // klien — `Err` tidak pernah keluar dari fungsi ini.
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
                    message: "ECONNRESET".to_string(),
                })
            }
        }
        let hasil = fbdown(&Mati, "u").unwrap();
        assert_eq!(hasil, json!({ "status": false, "message": "ECONNRESET" }));
    }

    #[test]
    fn body_bukan_json_menghasilkan_pesan_bawaan() {
        let http = StubHttp::baru("<html>gateway</html>");
        let hasil = fbdown(&http, "u").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["message"], "Gagal mengambil data dari API");
    }

    #[test]
    fn encode_axios_meloloskan_kolom_dan_menaruh_spasi_sebagai_plus() {
        assert_eq!(
            encode_axios("https://www.facebook.com/watch?v=xxx"),
            "https:%2F%2Fwww.facebook.com%2Fwatch%3Fv%3Dxxx"
        );
        assert_eq!(encode_axios("a b"), "a+b");
        assert_eq!(encode_axios("a:b,c$d"), "a:b,c$d");
        // `!'()*-._~` tetap lolos seperti `encodeURIComponent`.
        assert_eq!(encode_axios("!'()*-._~"), "!'()*-._~");
    }
}