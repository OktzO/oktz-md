//! Implementasi sinkron [`HttpGet`] di atas `ureq`.
//!
//! Host sync, bukan async, karena handler plugin bertipe `Fn(Ctx) -> Reply`
//! dan tidak async — mengubahnya jadi async berarti membongkar seluruh
//! registry. `ponytail:` konsekuensinya, request memblokir worker thread
//! tokio selama timeout; kalau class B jadi terlalu berat, naikkan ke reqwest
//! async + ubah `HandlerFn` jadi future.

use std::sync::Arc;

use serde_json::Value;

use crate::registry::{HttpBody, HttpCall, HttpError, HttpGet, HttpResponse};

/// User agent. API yang menolak UA bawaan `ureq` akan menolak bot dengan
/// `403`; plugin JS lewat axios/Node memakai UA seperti ini.
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

const TIMEOUT_SECS: u64 = 20;

#[derive(Debug)]
pub struct UreqHttp {
    agent: ureq::Agent,
}

impl Default for UreqHttp {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqHttp {
    pub fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(std::time::Duration::from_secs(TIMEOUT_SECS)))
            // `axios` menolak 4xx/5xx dan plugin menangkapnya di `catch`.
            // Status >= 400 jadi `Err` supaya alur plugin yang sama.
            .http_status_as_error(false)
            .build();
        Self {
            agent: config.into(),
        }
    }

    fn collect(res: ureq::http::Response<ureq::Body>) -> Result<HttpResponse, HttpError> {
        let status = res.status().as_u16();
        let headers = res
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_string(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let mut body = res.into_body();
        let data = body.read_to_vec().map_err(to_err)?;
        Ok(HttpResponse {
            status,
            body: HttpBody::Bytes {
                data: Arc::from(data),
                mimetype: None,
            },
            headers,
        })
    }

    /// Status >= 400 jadi error, meniru `axios` yang menolak response 4xx/5xx.
    fn guard(res: HttpResponse) -> Result<HttpResponse, HttpError> {
        if res.status >= 400 {
            return Err(HttpError {
                message: format!("HTTP {}", res.status),
            });
        }
        Ok(res)
    }

    /// Satu jalur kirim untuk [`HttpGet::call`], generik atas jenis body
    /// supaya permintaan tanpa body (GET/HEAD) tidak mengirim `Content-Length: 0`.
    fn send(&self, req: &HttpCall, body: impl ureq::AsSendBody) -> Result<HttpResponse, HttpError> {
        let mut permintaan = ureq::http::Request::builder()
            .method(req.method.as_str())
            .uri(req.url.as_str());
        let mut ada_ua = false;
        let mut ada_tipe = false;
        for (nama, nilai) in &req.headers {
            if nama.eq_ignore_ascii_case("user-agent") {
                ada_ua = true;
            }
            if nama.eq_ignore_ascii_case("content-type") {
                ada_tipe = true;
            }
            permintaan = permintaan.header(nama.as_str(), nilai.as_str());
        }
        // `axios` menimpa header bawaannya dengan milik pemanggil, jadi UA
        // browser ini cuma dipasang kalau pemanggil tidak menyebutnya sendiri.
        if !ada_ua {
            permintaan = permintaan.header("user-agent", UA);
        }
        // Content-type mengikuti bentuk body, kecuali pemanggil sudah menulisnya:
        // axios juga begitu (`data` string → form-urlencoded, objek → JSON).
        if !ada_tipe {
            match &req.body {
                Some(HttpBody::Form(_)) => {
                    permintaan = permintaan.header("content-type", "application/x-www-form-urlencoded");
                }
                Some(HttpBody::Bytes {
                    mimetype: Some(m), ..
                }) => {
                    permintaan = permintaan.header("content-type", m.as_str());
                }
                _ => {}
            }
        }
        let permintaan = permintaan.body(body).map_err(|e| HttpError {
            message: format!("permintaan HTTP tidak valid: {e}"),
        })?;
        let res = self.agent.run(permintaan).map_err(to_err)?;
        Self::guard(Self::collect(res)?)
    }
}

fn to_err(e: ureq::Error) -> HttpError {
    HttpError {
        message: e.to_string(),
    }
}

impl HttpGet for UreqHttp {
    fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
        let res = self.agent.get(url).header("user-agent", UA).call();
        Self::guard(Self::collect(res.map_err(to_err)?)?)
    }

    fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
        let payload = serde_json::to_string(body).map_err(|e| HttpError {
            message: format!("serialisasi JSON gagal: {e}"),
        })?;
        let res = self
            .agent
            .post(url)
            .header("user-agent", UA)
            .header("content-type", "application/json")
            .send(payload.as_bytes())
            .map_err(to_err)?;
        Self::guard(Self::collect(res)?)
    }

    fn post_form(&self, url: &str, form: &str) -> Result<HttpResponse, HttpError> {
        let res = self
            .agent
            .post(url)
            .header("user-agent", UA)
            .header("content-type", "application/x-www-form-urlencoded")
            .send(form.as_bytes())
            .map_err(to_err)?;
        Self::guard(Self::collect(res)?)
    }

    fn post_multipart(
        &self,
        url: &str,
        parts: &[(&str, HttpBody)],
    ) -> Result<HttpResponse, HttpError> {
        // `Form` menyimpan `&str`, jadi isi part harus hidup selama request.
        let owned: Vec<(String, Result<String, Vec<u8>>)> = parts
            .iter()
            .map(|(name, body)| {
                let bytes = body.bytes();
                (
                    name.to_string(),
                    String::from_utf8(bytes.to_vec()).map_err(|e| e.into_bytes()),
                )
            })
            .collect();
        let mut form = ureq::unversioned::multipart::Form::new();
        for (name, body) in &owned {
            form = match body {
                Ok(text) => form.text(name, text),
                Err(raw) => form.part(
                    name,
                    ureq::unversioned::multipart::Part::bytes(raw.as_slice()).file_name(name),
                ),
            };
        }
        let res = self
            .agent
            .post(url)
            .header("user-agent", UA)
            .send(form)
            .map_err(to_err)?;
        Self::guard(Self::collect(res)?)
    }

    fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
        match &req.body {
            None => self.send(req, ()),
            Some(HttpBody::Form(f)) => self.send(req, f.clone()),
            Some(HttpBody::Bytes { data, .. }) => self.send(req, data.to_vec()),
            Some(HttpBody::Multipart(parts)) => {
                // `Part` meminjam isi part, jadi semua byte ditampung dulu di
                // sini sampai request selesai dikirim.
                let owned: Vec<(String, Option<String>, Option<String>, Vec<u8>)> = parts
                    .iter()
                    .map(|p| {
                        (
                            p.name.clone(),
                            p.file_name.clone(),
                            p.mimetype.clone(),
                            p.body.bytes().to_vec(),
                        )
                    })
                    .collect();
                let mut form = ureq::unversioned::multipart::Form::new();
                for (name, file_name, mimetype, data) in &owned {
                    let part = match file_name {
                        // Field teks tanpa nama berkas: `form.append("type", 13)`
                        // dikirim sebagai field biasa, bukan lampiran.
                        None => match std::str::from_utf8(data) {
                            Ok(text) => ureq::unversioned::multipart::Part::text(text),
                            Err(_) => ureq::unversioned::multipart::Part::bytes(data),
                        },
                        Some(nama_berkas) => ureq::unversioned::multipart::Part::bytes(data)
                            .file_name(nama_berkas),
                    };
                    let part = match mimetype {
                        Some(m) => part.mime_str(m).map_err(to_err)?,
                        None => part,
                    };
                    form = form.part(name.as_str(), part);
                }
                // `Agent::run` TIDAK memindahkan content-type milik body ke
                // header (beda dari `RequestBuilder::send`), padahal multipart
                // tanpa boundary pasti ditolak server. Header pemanggil juga
                // dibuang: `form.getHeaders()` di JS memberi boundary milik
                // `form-data`, sedangkan di sini boundary dibuat oleh `Form`.
                let mut lokal = req.clone();
                lokal
                    .headers
                    .retain(|(k, _)| !k.eq_ignore_ascii_case("content-type"));
                lokal.headers.push((
                    "content-type".to_string(),
                    format!("multipart/form-data; boundary={}", form.boundary()),
                ));
                self.send(&lokal, form)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::MultipartPart;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    /// Baca satu permintaan HTTP utuh dari soket: header sampai `\r\n\r\n`,
    /// lalu body sebanyak `content-length`. Satu `read` saja tidak cukup —
    /// body sering datang di paket kedua.
    fn baca_permintaan(sock: &mut TcpStream) -> String {
        let mut data = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = sock.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            data.extend_from_slice(&buf[..n]);
            let teks = String::from_utf8_lossy(&data).to_string();
            if let Some(pos) = teks.find("\r\n\r\n") {
                let panjang = teks[..pos]
                    .lines()
                    .find_map(|l| {
                        let (k, v) = l.split_once(':')?;
                        k.eq_ignore_ascii_case("content-length")
                            .then(|| v.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                if data.len() >= pos + 4 + panjang {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&data).to_string()
    }

    /// Server loopback sekali-pakai: membalas `payload` JSON, dan mengembalikan
    /// permintaan mentah yang diterimanya untuk diperiksa test.
    fn server_sekali(payload: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let alamat = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            let req = baca_permintaan(&mut sock);
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            sock.write_all(resp.as_bytes()).unwrap();
            sock.flush().unwrap();
            req
        });
        (alamat, handle)
    }

    #[test]
    fn call_mengirim_header_kustom_dan_body_json() {
        let (alamat, handle) = server_sekali("{\"ok\":true}");
        let http = UreqHttp::new();
        let res = http
            .call(
                &HttpCall::post(format!("{alamat}/api"))
                    .header("X-CSRF-TOKEN", "abc")
                    .header("Cookie", "a=1")
                    .json(&serde_json::json!({"url": "https://x"})),
            )
            .unwrap();
        assert_eq!(res.status, 200);
        assert_eq!(res.json().unwrap()["ok"], true);
        let req = handle.join().unwrap();
        // Terminal bisa merusak tampilan string; periksa lewat teks mentah.
        let lower = req.to_lowercase();
        assert!(lower.starts_with("post /api "), "{req}");
        assert!(lower.contains("x-csrf-token: abc"), "{req}");
        assert!(lower.contains("cookie: a=1"), "{req}");
        assert!(lower.contains("content-type: application/json"), "{req}");
        assert!(req.contains("{\"url\":\"https://x\"}"), "{req}");
    }

    #[test]
    fn call_menghormati_ua_pemanggil_dan_metode_put() {
        let (alamat, handle) = server_sekali("{}");
        let http = UreqHttp::new();
        http.call(
            &HttpCall::put(format!("{alamat}/x"))
                .header("User-Agent", "klien-lain/1.0")
                .raw("a=1", "text/plain"),
        )
        .unwrap();
        let req = handle.join().unwrap();
        let lower = req.to_lowercase();
        assert!(lower.starts_with("put /x "), "{req}");
        assert!(lower.contains("user-agent: klien-lain/1.0"), "{req}");
        assert!(!lower.contains("chrome/120"), "UA bawaan tidak boleh ikut: {req}");
        assert!(lower.contains("content-type: text/plain"), "{req}");
    }

    #[test]
    fn call_tanpa_body_tidak_mengirim_content_length() {
        let (alamat, handle) = server_sekali("{}");
        let http = UreqHttp::new();
        http.call(&HttpCall::get(format!("{alamat}/y"))).unwrap();
        let req = handle.join().unwrap();
        let lower = req.to_lowercase();
        assert!(lower.starts_with("get /y "), "{req}");
        assert!(!lower.contains("content-length"), "{req}");
    }

    #[test]
    fn call_multipart_mengirim_nama_berkas_dan_field_biasa() {
        let (alamat, handle) = server_sekali("{\"ok\":1}");
        let http = UreqHttp::new();
        http.call(
            &HttpCall::post(format!("{alamat}/upload")).multipart(vec![
                MultipartPart::file("file", "foto.jpg", vec![1u8, 2, 3, 4])
                    .with_mimetype("image/jpeg"),
                MultipartPart::field("type", "13"),
            ]),
        )
        .unwrap();
        let req = handle.join().unwrap();
        let lower = req.to_lowercase();
        assert!(
            lower.contains("content-type: multipart/form-data; boundary="),
            "{req}"
        );
        assert!(lower.contains("filename=\"foto.jpg\""), "{req}");
        assert!(lower.contains("name=\"file\""), "{req}");
        assert!(lower.contains("name=\"type\""), "{req}");
        assert!(lower.contains("13"), "{req}");
    }
}