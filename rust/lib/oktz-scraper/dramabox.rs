//! `src/scraper/dramabox.js` — cari drama lewat halaman `dramabox.com`.
//!
//! Yang ikut disalin dari JS:
//! - Halaman Next.js dibaca dengan regex yang sama persis, termasuk `[\s\S]*?`
//!   (perlaku-`[\s\S]` = sebaris) dan `[^>]*` pada atribut `<script>`.
//! - Seluruh alur dibungkus `try/catch` yang mengembalikan
//!   `{ status: 'eror', msg: e.message }` — termasuk `TypeError` dari akses
//!   properti halamannya sendiri, bukan hanya dari HTTP. Tulisan `eror` yang
//!   salah ketik itu milik JS dan ikut apa adanya karena plugin membacanya.
//! - `json.props.pageProps.bookList || []` — `bookList` yang falsy jadi daftar
//!   kosong, jadi `total: 0` dan `results: []`.
//! - `v.coverCutWap || v.coverWap` memakai jatuhan `||`, jadi string kosong di
//!   sampul pertama ikut jatuh ke sampul kedua.
//! - `play_url` memakai template literal: nilai yang `undefined` menjadi teks
//!   `"undefined"`, bukan hilang.
//!
//! Beda yang diketahui:
//! - Pesan `SyntaxError` dari `JSON.parse` tidak bisa direproduksi persis
//!   (teksnya ikut versi V8/Node), jadi di sini teks dari parser `serde_json`.
//! - Pesan kegagalan HTTP milik axios (`"Request failed with status code 500"`)
//!   digantikan pesan dari `UreqHttp` (`"HTTP 500"`).
//! - Tidak ada helper yang dipakai file lain: `String(v)` dan pola regex
//!   sengaja privat di modul ini.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{encode_uri_component, truthy};

/// `/<script id="__NEXT_DATA__"[^>]*>([\s\S]*?)<\/script>/` (`dramabox.js:7`).
fn pola_next_data() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| {
        Regex::new(r#"<script id="__NEXT_DATA__"[^>]*>([\s\S]*?)</script>"#)
            .expect("pola __NEXT_DATA__")
    })
}

/// `String(v)` di dalam template literal — `undefined` jadi `"undefined"`.
fn teks_js(v: Option<&Value>) -> String {
    match v {
        None => "undefined".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(a)) => a.iter().map(|v| teks_js(Some(v))).collect::<Vec<_>>().join(","),
        Some(Value::Object(_)) => "[object Object]".to_string(),
    }
}

/// `dramaboxsearch(q)` tanpa `try/catch` — bagian yang dibungkus `err`.
fn dramabox_inti(http: &dyn HttpGet, q: &str) -> Result<Value, HttpError> {
    let html = http.call(&HttpCall::get(format!(
        "https://www.dramabox.com/search?searchValue={}",
        encode_uri_component(q)
    )))?.text();

    // `.match(...)[1]` di JS: kalau tidak cocok, `null[1]` melempar TypeError.
    let Some(captures) = pola_next_data().captures(&html) else {
        return Err(HttpError {
            message: "Cannot read properties of null (reading '1')".to_string(),
        });
    };
    let isi = captures.get(1).map(|m| m.as_str()).unwrap_or("");
    let halaman: Value = serde_json::from_str(isi).map_err(|e| HttpError {
        message: format!("JSON.parse gagal: {e}"),
    })?;

    // Akses properti berantai. Nama di pesan error diambil dari objek yang
    // sedang dibaca dan properti yang dibaca, jadi `null` dan `undefined`
    // harus dibedakan (V8 menulis keduanya secara berbeda).
    let Some(props) = halaman.get("props") else {
        return Err(HttpError {
            message: "Cannot read properties of undefined (reading 'pageProps')".to_string(),
        });
    };
    if props.is_null() {
        return Err(HttpError {
            message: "Cannot read properties of null (reading 'pageProps')".to_string(),
        });
    }
    let Some(page_props) = props.get("pageProps") else {
        return Err(HttpError {
            message: "Cannot read properties of undefined (reading 'bookList')".to_string(),
        });
    };
    if page_props.is_null() {
        return Err(HttpError {
            message: "Cannot read properties of null (reading 'bookList')".to_string(),
        });
    }
    // `bookList` truthy tapi bukan array → `list.length` undefined lalu
    // `list.map` bukan fungsi.
    let daftar: &[Value] = match page_props.get("bookList").filter(|v| truthy(v)) {
        None => &[],
        Some(v) => match v.as_array() {
            Some(a) => a,
            None => {
                return Err(HttpError {
                    message: "list.map is not a function".to_string(),
                });
            }
        },
    };

    let mut results: Vec<Value> = Vec::with_capacity(daftar.len());
    for v in daftar {
        let mut item = Map::new();
        for (sumber, kunci) in [
            (v.get("bookId"), "id"),
            (v.get("bookName"), "title"),
            (v.get("totalChapterNum"), "episodes"),
            (v.get("introduction"), "description"),
        ] {
            if let Some(x) = sumber {
                item.insert(kunci.to_string(), x.clone());
            }
        }
        // `v.coverCutWap || v.coverWap`.
        let cover = v
            .get("coverCutWap")
            .filter(|c| truthy(c))
            .or_else(|| v.get("coverWap"));
        if let Some(c) = cover {
            item.insert("cover".to_string(), c.clone());
        }
        item.insert(
            "play_url".to_string(),
            Value::String(format!(
                "https://www.dramabox.com/video/{}_{}/{}_Episode-1",
                teks_js(v.get("bookId")),
                teks_js(v.get("bookNameEn")),
                teks_js(v.get("chapterId")),
            )),
        );
        results.push(Value::Object(item));
    }

    Ok(json!({
        "query": q,
        "total": daftar.len(),
        "results": results,
    }))
}

/// `dramaboxsearch(q)` (`dramabox.js:2-26`) — semua kegagalan jadi
/// `{ status: 'eror', msg }`, tidak pernah `Err`.
pub fn dramabox_search(http: &dyn HttpGet, q: &str) -> Result<Value, HttpError> {
    match dramabox_inti(http, q) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({
            "status": "eror",
            "msg": e.message,
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat URL dan membalas HTML tetap.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: String,
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
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(html: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: html.to_string(),
        }
    }

    fn halaman(data_json: &str) -> String {
        format!(r#"<!DOCTYPE html><html><body><script id="__NEXT_DATA__" type="application/json">{data_json}</script></body></html>"#)
    }

    #[test]
    fn mencari_menampilkan_daftar_beserta_play_url() {
        let http = stub(&halaman(
            r#"{"props":{"pageProps":{"bookList":[
                 {"bookId":"bk1","bookName":"Drama Satu","bookNameEn":"Drama One",
                  "totalChapterNum":10,"introduction":"Cerita satu",
                  "coverCutWap":"https://cdn/wap.jpg","coverWap":"https://cdn/cover.jpg",
                  "chapterId":"ch1"},
                 {"bookId":"bk2","bookName":"Drama Dua","bookNameEn":"Drama Two",
                  "totalChapterNum":0,"introduction":"","coverCutWap":"",
                  "coverWap":"https://cdn/dua.jpg","chapterId":"ch2"}
               ]}}}"#,
        ));
        let hasil = dramabox_search(&http, "drama dua").unwrap();
        assert_eq!(hasil["query"], "drama dua");
        assert_eq!(hasil["total"], 2);
        assert!(hasil.get("status").is_none(), "jalur sukses tidak punya status");

        let hasil = hasil.as_object().unwrap();
        let results = hasil["results"].as_array().unwrap();
        assert_eq!(results[0]["id"], "bk1");
        assert_eq!(results[0]["title"], "Drama Satu");
        assert_eq!(results[0]["episodes"], 10);
        assert_eq!(results[0]["description"], "Cerita satu");
        assert_eq!(results[0]["cover"], "https://cdn/wap.jpg");
        assert_eq!(
            results[0]["play_url"],
            "https://www.dramabox.com/video/bk1_Drama One/ch1_Episode-1"
        );
        // `coverCutWap` string kosong ikut jatuh ke `coverWap`.
        assert_eq!(results[1]["cover"], "https://cdn/dua.jpg");
        assert_eq!(results[1]["episodes"], 0);
        assert_eq!(results[1]["description"], "");
        assert_eq!(
            results[1]["play_url"],
            "https://www.dramabox.com/video/bk2_Drama Two/ch2_Episode-1"
        );

        // Query di-encode dengan `encodeURIComponent` (spasi `%20`, bukan `+`).
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://www.dramabox.com/search?searchValue=drama%20dua"
        );
        assert!(p[0].body.is_none(), "tanpa body");
    }

    #[test]
    fn field_yang_hilang_tidak_ikut_tapi_play_url_jadi_teks_undefined() {
        let http = stub(&halaman(r#"{"props":{"pageProps":{"bookList":[{"bookId":"bk9"}]}}}"#));
        let hasil = dramabox_search(&http, "x").unwrap();
        let item = &hasil["results"][0];
        assert_eq!(item["id"], "bk9");
        assert!(item.get("title").is_none(), "bookName tidak ada");
        assert!(item.get("episodes").is_none());
        assert!(item.get("description").is_none());
        assert!(item.get("cover").is_none(), "dua cover kosong");
        assert_eq!(
            item["play_url"],
            "https://www.dramabox.com/video/bk9_undefined/undefined_Episode-1"
        );
    }

    #[test]
    fn book_list_kosong_atau_hilang_memberi_total_nol() {
        for data in [
            r#"{"props":{"pageProps":{"bookList":[]}}}"#,
            r#"{"props":{"pageProps":{}}}"#,
            r#"{"props":{"pageProps":{"bookList":null}}}"#,
        ] {
            let http = stub(&halaman(data));
            let hasil = dramabox_search(&http, "x").unwrap();
            assert_eq!(hasil["total"], 0, "data: {data}");
            assert_eq!(hasil["results"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    fn tidak_ada_script_next_data_memberi_pesan_typeerror_js() {
        let http = stub("<html><body>halaman lain</body></html>");
        let hasil = dramabox_search(&http, "x").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert_eq!(
            hasil["msg"],
            "Cannot read properties of null (reading '1')"
        );
    }

    #[test]
    fn rantai_properti_hilang_membawa_pesan_yang_sama_dengan_js() {
        let tanpa_props = stub(&halaman(r#"{"pageProps":{"bookList":[]}}"#));
        let hasil = dramabox_search(&tanpa_props, "x").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert_eq!(
            hasil["msg"],
            "Cannot read properties of undefined (reading 'pageProps')"
        );

        let tanpa_page_props = stub(&halaman(r#"{"props":null}"#));
        let hasil = dramabox_search(&tanpa_page_props, "x").unwrap();
        assert_eq!(
            hasil["msg"],
            "Cannot read properties of null (reading 'pageProps')"
        );

        let page_props_null = stub(&halaman(r#"{"props":{"pageProps":null}}"#));
        let hasil = dramabox_search(&page_props_null, "x").unwrap();
        assert_eq!(
            hasil["msg"],
            "Cannot read properties of null (reading 'bookList')"
        );

        let book_list_bukan_array = stub(&halaman(r#"{"props":{"pageProps":{"bookList":"abc"}}}"#));
        let hasil = dramabox_search(&book_list_bukan_array, "x").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert_eq!(hasil["msg"], "list.map is not a function");
    }

    #[test]
    fn json_next_data_rusak_juga_menghasilkan_objek_error() {
        let http = stub(&halaman("{bukan json"));
        let hasil = dramabox_search(&http, "x").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert!(
            hasil["msg"]
                .as_str()
                .unwrap()
                .starts_with("JSON.parse gagal:"),
            "{}",
            hasil["msg"]
        );
    }

    #[test]
    fn kegagalan_jaringan_ikut_menjadi_objek_error() {
        #[derive(Debug)]
        struct Mati;
        impl oktz_plugin_api::HttpGet for Mati {
            fn get(&self, _: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                Err(HttpError { message: "HTTP 503".into() })
            }
            fn post_json(
                &self,
                _: &str,
                _: &Value,
            ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_form(
                &self,
                _: &str,
                _: &str,
            ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                unreachable!()
            }
            fn post_multipart(
                &self,
                _: &str,
                _: &[(&str, oktz_plugin_api::HttpBody)],
            ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                unreachable!()
            }
            fn call(
                &self,
                _: &oktz_plugin_api::HttpCall,
            ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
                Err(HttpError { message: "HTTP 503".into() })
            }
        }
        let hasil = dramabox_search(&Mati, "x").unwrap();
        assert_eq!(hasil["status"], "eror");
        assert_eq!(hasil["msg"], "HTTP 503");
    }
}