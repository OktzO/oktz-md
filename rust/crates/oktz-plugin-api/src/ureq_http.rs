//! Implementasi sinkron [`HttpGet`] di atas `ureq`.
//!
//! Host sync, bukan async, karena handler plugin bertipe `Fn(Ctx) -> Reply`
//! dan tidak async — mengubahnya jadi async berarti membongkar seluruh
//! registry. `ponytail:` konsekuensinya, request memblokir worker thread
//! tokio selama timeout; kalau class B jadi terlalu berat, naikkan ke reqwest
//! async + ubah `HandlerFn` jadi future.

use std::sync::Arc;

use serde_json::Value;

use crate::registry::{HttpBody, HttpError, HttpGet, HttpResponse};

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
}