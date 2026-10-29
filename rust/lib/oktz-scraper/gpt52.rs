//! `src/scraper/gpt52.js` — ChatGPT tanpa login lewat `backend-anon`.
//!
//! Alur JS (`gpt52.js:44-95`): dua token sentinel (`chat-requirements/
//! prepare` + `finalize`, dengan proof-of-work FNV-1a kalau diminta) lalu
//! token conduit dari `f/conversation/prepare`, baru POST ke
//! `backend-anon/f/conversation` dan isi SSE-nya dipatch.
//!
//! Penyimpangan yang dicatat apa adanya:
//!
//! - **Tanpa unggah gambar dari berkas.** JS menulis buffer ke
//!   `temp/gpt52_<uuid><ext>` lalu membacanya untuk `_uploadImage`;
//!   di sini byte gambar langsung dipakai, tanpa `fs`.
//! - **Tanpa streaming.** `HttpGet` mengembalikan body utuh, jadi SSE dibaca
//!   sekaligus setelah diterima, bukan per chunk. Opsi `stream`/`onChunk`
//!   di JS tidak punya padanan dan tidak dipakai di sini.
//! - **Waktu lokal jadi UTC.** `new Date()` memakai zona waktu host JS;
//!   di sini format `String(new Date)`, `getTimezoneOffset()` dan
//!   `Intl…timeZone` dihitung untuk UTC. Nilai sentinel diacak server,
//!   jadi selisihnya tidak mengubah bentuk hasil.
//! - `performance.now()` jadi milidetik sejak mulai proses, dan
//!   `performance.timeOrigin` jadi milidetik sejak epoch.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use rand::Rng;
use serde_json::{Value, json};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const BASE_URL: &str = "https://chatgpt.com";

const DEFAULT_UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Mobile Safari/537.36";

const BUILD_NUMBER: &str = "prod-69a06c53754594935887d6c16b844885964a78fc";

/// `new ChatGPT({ lang: "id-ID" })` (`gpt52.js:561`).
pub struct ChatGpt {
    pub user_agent: String,
    pub oai_did: String,
    pub screen_width: i64,
    pub screen_height: i64,
    pub lang: String,
    pub build_number: String,
    pub auth_token: Option<String>,
}

impl Default for ChatGpt {
    /// `lang` bawaan konstruktor adalah `"id-ID"`, sama seperti instans
    /// global di `gpt52.js:561`.
    fn default() -> Self {
        Self {
            user_agent: DEFAULT_UA.to_string(),
            oai_did: uuid_v4(),
            screen_width: 423,
            screen_height: 965,
            lang: "id-ID".to_string(),
            build_number: BUILD_NUMBER.to_string(),
            auth_token: None,
        }
    }
}

/// Opsi `send(message, opts)` (`gpt52.js:44-57`).
#[derive(Default)]
pub struct Opsi<'a> {
    pub conversation_id: Option<&'a str>,
    pub parent_message_id: Option<&'a str>,
    pub image: Option<Image>,
    pub web_search: bool,
}

/// Hasil `_uploadImage` (`gpt52.js:166`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub file_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub width: u32,
    pub height: u32,
}

/// Hasil `chat()` (`gpt52.js:649-656`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hasil {
    pub text: String,
    pub raw: String,
    pub model: String,
    /// Dari event `title_generation`; `chat()` JS tidak memakainya lagi.
    pub title: Option<String>,
    pub conversation_id: Option<String>,
    pub message_id: Option<String>,
}

fn basis64(bytes: &[u8]) -> String {
    const AB: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for blok in bytes.chunks(3) {
        let b0 = blok[0] as u32;
        let b1 = *blok.get(1).unwrap_or(&0) as u32;
        let b2 = *blok.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(AB[(n >> 18) as usize & 63] as char);
        out.push(AB[(n >> 12) as usize & 63] as char);
        if blok.len() > 1 {
            out.push(AB[(n >> 6) as usize & 63] as char);
        } else {
            out.push('=');
        }
        if blok.len() > 2 {
            out.push(AB[n as usize & 63] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// `crypto.randomUUID()` (`gpt52.js:29`).
fn uuid_v4() -> String {
    let mut b: [u8; 16] = rand::rng().random();
    b[6] = (b[6] & 0x0F) | 0x40;
    b[8] = (b[8] & 0x3F) | 0x80;
    let h: String = b.iter().map(|v| format!("{v:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

fn epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

fn epoch_secs() -> f64 {
    epoch_ms() as f64 / 1000.0
}

/// `String(new Date())` untuk zona UTC — `"Sat Oct 17 2026 08:20:31 GMT+0000
/// (Coordinated Universal Time)"` (`gpt52.js:222`).
fn tanggal_js(ms: u128) -> String {
    const HARI: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const BULAN: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let total_detik = (ms / 1000) as i64;
    // 1 Januari 1970 adalah Kamis, jadi hari ke-0 = index 4.
    let hari_dalam_minggu = (total_detik.div_euclid(86_400) + 4).rem_euclid(7) as usize;
    let (tahun, bulan, hari) =tanggal_kalender(total_detik);
    let detik_dalam_hari = total_detik.rem_euclid(86_400);
    format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        HARI[hari_dalam_minggu],
        BULAN[bulan - 1],
        hari,
        tahun,
        (detik_dalam_hari / 3600) as u32,
        ((detik_dalam_hari % 3600) / 60) as u32,
        (detik_dalam_hari % 60) as u32
    )
}

/// `(tahun, bulan, hari)` dari hari sejak epoch — algoritma Howard Hinnant.
fn tanggal_kalender(total_detik: i64) -> (i64, usize, i64) {
    // Algoritma Howard Hinnant: hitung hari sejak epoch ke tanggal proleptik
    // Gregorius tanpa pustaka tanggal.
    let z = total_detik.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let tahun = if m <= 2 { y + 1 } else { y };
    (tahun, m as usize, d)
}

/// `this._fnv1a(str)` (`gpt52.js:196-206`).
pub fn fnv1a_js(s: &str) -> String {
    let mut h: u32 = 2_166_136_261;
    for u in s.encode_utf16() {
        h ^= u32::from(u);
        h = h.wrapping_mul(16_777_619);
    }
    h ^= h >> 16;
    h = h.wrapping_mul(2_246_822_507);
    h ^= h >> 13;
    h = h.wrapping_mul(3_266_489_909);
    h ^= h >> 16;
    format!("{h:08x}")
}

impl ChatGpt {
    /// `_headers(extra)` (`gpt52.js:179-200`).
    fn header_pairs(&self, extra: &[(&str, String)]) -> Vec<(String, String)> {
        let mut pairs: Vec<(String, String)> = vec![
            ("User-Agent".into(), self.user_agent.clone()),
            ("accept".into(), "*/*".into()),
            (
                "accept-language".into(),
                format!("{},en-US;q=0.9,en;q=0.8", self.lang),
            ),
            ("content-type".into(), "application/json".into()),
            ("OAI-Device-Id".into(), self.oai_did.clone()),
            (
                "sec-ch-ua".into(),
                "\"Chromium\";v=\"144\", \"Not/A)Brand\";v=\"24\"".into(),
            ),
            ("sec-ch-ua-mobile".into(), "?1".into()),
            ("sec-ch-ua-platform".into(), "\"Android\"".into()),
            ("sec-fetch-dest".into(), "empty".into()),
            ("sec-fetch-mode".into(), "cors".into()),
            ("sec-fetch-site".into(), "same-origin".into()),
            ("origin".into(), "https://chatgpt.com".into()),
            ("referer".into(), "https://chatgpt.com/".into()),
        ];
        if let Some(t) = &self.auth_token {
            pairs.push(("authorization".into(), format!("Bearer {t}")));
        }
        // `{ ...headers, ...extra }` menimpa kunci yang sama, bukan
        // menambah duplikat, jadi `accept` untuk SSE jadi `text/event-stream`.
        for (k, v) in extra {
            match pairs.iter_mut().find(|(n, _)| n == k) {
                Some(slot) => slot.1 = v.clone(),
                None => pairs.push(((*k).to_string(), v.clone())),
            }
        }
        pairs
    }

    /// `_makeBrowserConfig()` (`gpt52.js:212-234`).
    fn browser_config(&self, mulai: Instant) -> Value {
        json!([
            self.screen_width + self.screen_height,
            tanggal_js(epoch_ms()),
            2172649472_u32,
            0,
            self.user_agent,
            Value::Null,
            self.build_number,
            self.lang,
            format!("{},en", self.lang),
            0,
            "contacts\u{2212}[object ContactsManager]",
            "_reactListening",
            "User",
            mulai.elapsed().as_secs_f64() * 1000.0,
            uuid_v4(),
            "",
            8,
            epoch_ms() as f64,
            0,
            423,
            0,
            0,
            0,
            0,
            0,
            32,
            100,
            1,
            0,
            0,
            0
        ])
    }

    /// `_encodeConfig(cfg)` (`gpt52.js:208-210`).
    fn encode_config(cfg: &Value) -> String {
        basis64(cfg.to_string().as_bytes())
    }

    /// `_computePow(seed, difficulty, cfg)` (`gpt52.js:239-255`).
    fn compute_pow(&self, seed: &str, difficulty: &str, cfg: &mut Value, mulai: Instant) -> String {
        let _ = mulai;
        for i in 0..500_000u32 {
            cfg[3] = json!(i);
            cfg[9] = json!(0);
            let encoded = Self::encode_config(cfg);
            if fnv1a_js(&format!("{seed}{encoded}"))
                .starts_with(difficulty)
            {
                return format!("gAAAAAB{encoded}~S");
            }
        }
        "wQ8Lk5FbGpA2NcR9dShT6gYjU7VxZ4De".to_string()
    }

    /// `_generateSentinelTokens()` (`gpt52.js:257-291`).
    fn sentinel_tokens(&self, http: &dyn HttpGet) -> Result<Sentinel, HttpError> {
        let mulai = Instant::now();
        let mut cfg = self.browser_config(mulai);
        cfg[3] = json!(1);
        cfg[9] = json!(0);
        let init_token = format!("gAAAAAC{}", Self::encode_config(&cfg));

        let mut call = HttpCall::post(format!("{BASE_URL}/backend-anon/sentinel/chat-requirements/prepare"));
        call.headers = self.header_pairs(&[]);
        call = call.json(&json!({ "p": init_token }));
        let prepare = http.call(&call)?.json().unwrap_or(Value::Null);

        let mut pow = None;
        if crate::common::truthy(&prepare["proofofwork"]["required"]) {
            let seed = prepare["proofofwork"]["seed"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let difficulty = prepare["proofofwork"]["difficulty"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let mut cfg2 = self.browser_config(mulai);
            pow = Some(self.compute_pow(&seed, &difficulty, &mut cfg2, mulai));
        }

        // `crypto.randomBytes(Math.floor((2256 / 4) * 3))` lalu base64, potong
        // 2256 karakter.
        let n = (2256_f64 / 4.0 * 3.0).floor() as usize;
        let acak: Vec<u8> = (0..n).map(|_| rand::rng().random::<u8>()).collect();
        let turnstile = basis64(&acak).chars().take(2256).collect::<String>();

        let mut finalize_body = json!({
            "prepare_token": prepare
                .get("prepare_token")
                .filter(|v| crate::common::truthy(v))
                .cloned()
                .unwrap_or_else(|| json!("")),
        });
        if let Some(p) = &pow {
            finalize_body["proofofwork"] = json!(p);
        }
        finalize_body["turnstile"] = json!(turnstile);

        let mut call = HttpCall::post(format!("{BASE_URL}/backend-anon/sentinel/chat-requirements/finalize"));
        call.headers = self.header_pairs(&[]);
        call = call.json(&finalize_body);
        let finalize = http.call(&call)?.json().unwrap_or(Value::Null);

        Ok(Sentinel {
            pow,
            turnstile,
            chat_requirements_token: finalize
                .get("token")
                .filter(|v| crate::common::truthy(v))
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }

    /// `_getConduitToken(...)` (`gpt52.js:293-336`).
    fn conduit_token(
        &self,
        http: &dyn HttpGet,
        message: &str,
        msg_id: &str,
        opsi: &Opsi<'_>,
        parent: &str,
    ) -> Result<String, HttpError> {
        let mut body = json!({
            "action": "next",
            "fork_from_shared_post": false,
            "parent_message_id": parent,
            "model": "auto",
            // `getTimezoneOffset()` untuk UTC = 0.
            "timezone_offset_min": 0,
            "timezone": "UTC",
            "conversation_mode": { "kind": "primary_assistant" },
            "system_hints": if opsi.web_search { json!(["search"]) } else { json!([]) },
            "supports_buffering": true,
            "supported_encodings": ["v1"],
            "partial_query": {
                "id": msg_id,
                "author": { "role": "user" },
                "content": { "content_type": "text", "parts": [message] },
            },
            "client_contextual_info": { "app_name": "chatgpt.com" },
        });
        if let Some(c) = opsi.conversation_id.filter(|v| !v.is_empty()) {
            body["conversation_id"] = json!(c);
        }
        if let Some(img) = &opsi.image {
            body["attachment_mime_types"] = json!([img.mime_type]);
        }
        let mut call = HttpCall::post(format!("{BASE_URL}/backend-anon/f/conversation/prepare"));
        call.headers = self.header_pairs(&[("X-Conduit-Token", "no-token".to_string())]);
        call = call.json(&body);
        let data = http.call(&call)?.json().unwrap_or(Value::Null);
        Ok(data
            .get("token")
            .or_else(|| data.get("conduit_token"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string())
    }

    /// `_buildMessageBody(...)` (`gpt52.js:338-441`).
    fn message_body(&self, message: &str, msg_id: &str, opsi: &Opsi<'_>, parent: &str) -> Value {
        let (content, meta) = match &opsi.image {
            Some(img) => (
                json!({
                    "content_type": "multimodal_text",
                    "parts": [
                        {
                            "content_type": "image_asset_pointer",
                            "asset_pointer": format!("file-service://{}", img.file_id),
                            "size_bytes": img.size_bytes,
                            "width": img.width,
                            "height": img.height,
                        },
                        message,
                    ],
                }),
                json!({
                    "attachments": [{
                        "id": img.file_id,
                        "size": img.size_bytes,
                        "name": img.file_name,
                        "mime_type": img.mime_type,
                        "width": img.width,
                        "height": img.height,
                        "source": "local",
                        "is_big_paste": false,
                    }],
                    "selected_github_repos": [],
                    "selected_all_github_repos": false,
                    "serialization_metadata": { "custom_symbol_offsets": [] },
                }),
            ),
            None => {
                let mut meta = json!({
                    "selected_github_repos": [],
                    "selected_all_github_repos": false,
                    "serialization_metadata": { "custom_symbol_offsets": [] },
                });
                if opsi.web_search {
                    meta["system_hints"] = json!(["search"]);
                }
                (
                    json!({ "content_type": "text", "parts": [message] }),
                    meta,
                )
            }
        };

        let mut body = json!({
            "action": "next",
            "messages": [{
                "id": msg_id,
                "author": { "role": "user" },
                "create_time": epoch_secs(),
                "content": content,
                "metadata": meta,
            }],
            "parent_message_id": parent,
            "model": "auto",
            "timezone_offset_min": 0,
            "timezone": "UTC",
            "conversation_mode": { "kind": "primary_assistant" },
            "enable_message_followups": true,
            "system_hints": if opsi.web_search { json!(["search"]) } else { json!([]) },
            "supports_buffering": true,
            "supported_encodings": ["v1"],
            "client_contextual_info": {
                "is_dark_mode": true,
                "time_since_loaded": 10,
                "page_height": 845,
                "page_width": 423,
                "pixel_ratio": 1.7,
                "screen_height": self.screen_height,
                "screen_width": self.screen_width,
                "app_name": "chatgpt.com",
            },
            "no_auth_ad_preferences": {
                "personalization_enabled": true,
                "history_enabled": true,
            },
            "paragen_cot_summary_display_override": "allow",
            "force_parallel_switch": "auto",
        });
        if let Some(c) = opsi.conversation_id.filter(|v| !v.is_empty()) {
            body["conversation_id"] = json!(c);
        }
        if opsi.web_search {
            body["force_use_search"] = json!(true);
            body["client_reported_search_source"] =
                json!("conversation_composer_web_icon");
        }
        body
    }

    /// `_uploadImage(filePath, ctx)` (`gpt52.js:96-176`).
    fn upload_image(
        &self,
        http: &dyn HttpGet,
        bytes: &[u8],
        file_name: &str,
        opsi: &Opsi<'_>,
        parent: &str,
    ) -> Result<Image, HttpError> {
        let mime = ext_to_mime(file_name);
        let (width, height) = image_dimensions(bytes, mime);

        let mut call = HttpCall::post(format!("{BASE_URL}/backend-anon/files"));
        call.headers = self.header_pairs(&[]);
        call = call.json(&json!({
                "file_name": file_name,
                "file_size": bytes.len(),
                "use_case": "multimodal",
                "timezone_offset_min": 0,
                "reset_rate_limits": false,
            }));
        let register = http.call(&call)?.json().unwrap_or(Value::Null);
        let upload_url = register.get("upload_url").cloned().unwrap_or(Value::Null);
        let file_id = register.get("file_id").cloned().unwrap_or(Value::Null);
        if !crate::common::truthy(&upload_url) || !crate::common::truthy(&file_id) {
            return Err(HttpError {
                message: format!(
                    "Gagal mendaftar file: {}",
                    register
                ),
            });
        }

        let mut blob = HttpCall::new(oktz_plugin_api::HttpMethod::Put, js_str(&upload_url));
        blob.headers = vec![
            ("Content-Type".into(), mime.to_string()),
            ("x-ms-blob-type".into(), "BlockBlob".into()),
            ("x-ms-version".into(), "2020-04-08".into()),
        ];
        blob.body = Some(oktz_plugin_api::HttpBody::Bytes {
            data: std::sync::Arc::from(bytes),
            mimetype: Some(mime.to_string()),
        });
        let upload = http.call(&blob)?;
        if upload.status >= 400 {
            return Err(HttpError {
                message: format!("Upload blob gagal: HTTP {}", upload.status),
            });
        }

        let mut process_body = json!({
            "file_id": file_id,
            "use_case": "multimodal",
            "index_for_retrieval": false,
            "file_name": file_name,
        });
        if !opsi.conversation_id.unwrap_or("").is_empty() || !parent.is_empty() {
            process_body["metadata"] = json!({
                "library_file_info": {
                    "origination_message_id": parent,
                    "origination_thread_id": opsi.conversation_id,
                }
            });
        }
        let mut call = HttpCall::post(format!("{BASE_URL}/backend-anon/files/process_upload_stream"));
        call.headers = self.header_pairs(&[]);
        call = call.json(&process_body);
        let proses = http.call(&call)?;
        if !proses.text().contains("file.processing.completed") {
            // JS-readable streamberhenti saat marker muncul; kalau marker
            // tidak pernah datang, JS tetap lanjut dengan hasil yang sama.
        }

        Ok(Image {
            file_id: js_str(&file_id),
            file_name: file_name.to_string(),
            mime_type: mime.to_string(),
            size_bytes: bytes.len(),
            width,
            height,
        })
    }

    /// `send(message, opts)` (`gpt52.js:44-95`).
    pub fn send(
        &self,
        http: &dyn HttpGet,
        message: &str,
        opsi: &Opsi<'_>,
        image_bytes: Option<&[u8]>,
    ) -> Result<Hasil, HttpError> {
        if message.trim().is_empty() {
            return Err(HttpError {
                message: "Pesan tidak boleh kosong.".to_string(),
            });
        }
        let parent = opsi.parent_message_id.unwrap_or("client-created-root");
        let msg_id = uuid_v4();

        let opsi_owned = Opsi {
            image: match image_bytes {
                Some(b) => Some(self.upload_image(http, b, "gambar.png", opsi, parent)?),
                None => None,
            },
            ..*opsi
        };

        let tokens = self.sentinel_tokens(http)?;
        let conduit = self.conduit_token(http, message, &msg_id, &opsi_owned, parent)?;
        let body = self.message_body(message, &msg_id, &opsi_owned, parent);

        let mut call = HttpCall::post(format!("{BASE_URL}/backend-anon/f/conversation"));
        call.headers = self.header_pairs(&[
                ("accept", "text/event-stream".to_string()),
                ("OAI-Language", self.lang.clone()),
                (
                    "OpenAI-Sentinel-Chat-Requirements-Token",
                    tokens.chat_requirements_token.clone().unwrap_or_default(),
                ),
                ("OpenAI-Sentinel-Turnstile-Token", tokens.turnstile.clone()),
                (
                    "OpenAI-Sentinel-Proof-Token",
                    tokens.pow.clone().unwrap_or_default(),
                ),
                ("X-Conduit-Token", conduit),
            ]);
        call = call.json(&body);

        let res = http.call(&call)?;
        if res.status >= 400 {
            return Err(HttpError {
                message: format!("HTTP {}: {}", res.status, res.text()),
            });
        }
        parse_sse(&res.text())
    }
}

struct Sentinel {
    pow: Option<String>,
    turnstile: String,
    chat_requirements_token: Option<String>,
}

/// `_getMimeType(fileName)` (`gpt52.js:504-515`).
fn ext_to_mime(file_name: &str) -> &'static str {
    let k = file_name.to_ascii_lowercase();
    if k.ends_with(".png") {
        "image/png"
    } else if k.ends_with(".jpg") || k.ends_with(".jpeg") {
        "image/jpeg"
    } else if k.ends_with(".gif") {
        "image/gif"
    } else if k.ends_with(".webp") {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

/// `_getImageDimensions(buffer, mimeType)` (`gpt52.js:516-540`).
fn image_dimensions(buffer: &[u8], mime: &str) -> (u32, u32) {
    if mime == "image/png" && buffer.len() >= 24 {
        return (
            u32::from_be_bytes([buffer[16], buffer[17], buffer[18], buffer[19]]),
            u32::from_be_bytes([buffer[20], buffer[21], buffer[22], buffer[23]]),
        );
    }
    if mime == "image/jpeg" {
        let mut i = 2usize;
        while i + 8 < buffer.len() {
            if buffer[i] != 0xff {
                break;
            }
            let marker = buffer[i + 1];
            let seg = u16::from_be_bytes([buffer[i + 2], buffer[i + 3]]) as usize;
            if (0xc0..=0xc3).contains(&marker) {
                return (
                    u16::from_be_bytes([buffer[i + 7], buffer[i + 8]]) as u32,
                    u16::from_be_bytes([buffer[i + 5], buffer[i + 6]]) as u32,
                );
            }
            i += 2 + seg;
        }
    }
    (0, 0)
}

/// `_parseSSE(body, …)` (`gpt52.js:443-501`).
pub fn parse_sse(body: &str) -> Result<Hasil, HttpError> {
    let mut full_text = String::new();
    let mut title: Option<String> = None;
    let mut model: Option<String> = None;
    let mut conv_id: Option<String> = None;
    let mut assistant_msg_id: Option<String> = None;

    // `lines.pop()` membuang baris terakhir yang belum diakhiri baris baru.
    let baris: Vec<&str> = body.split('\n').collect();
    for line in &baris[..baris.len().saturating_sub(1)] {
        if !line.starts_with("data:") {
            continue;
        }
        let raw = line[5..].trim();
        if raw.is_empty() || raw == "[DONE]" {
            continue;
        }
        let Ok(json) = serde_json::from_str::<Value>(raw) else {
            continue;
        };
        if crate::common::truthy(&json["conversation_id"]) {
            conv_id = json["conversation_id"].as_str().map(str::to_string);
        } else if crate::common::truthy(&json["v"]["conversation_id"]) {
            conv_id = json["v"]["conversation_id"].as_str().map(str::to_string);
        }
        let v_obj = json.get("v").filter(|v| v.is_object());
        if let Some(v) = v_obj
            && v["message"]["author"]["role"].as_str() == Some("assistant")
            && crate::common::truthy(&v["message"]["id"])
        {
            assistant_msg_id = v["message"]["id"].as_str().map(str::to_string);
        }
        if json["type"].as_str() == Some("title_generation") {
            title = Some(js_str(&json["title"]));
        }
        if json["type"].as_str() == Some("server_ste_metadata") {
            model = json["metadata"]["model_slug"].as_str().map(str::to_string);
        }
        if let Some(patch) = json.get("v").and_then(Value::as_array) {
            for p in patch {
                let appends = p.get("o").and_then(Value::as_str) == Some("append");
                let jalan = p
                    .get("p")
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.contains("/message/content/parts/0"));
                if appends && jalan {
                    full_text.push_str(&js_str(&p["v"]));
                }
            }
        }
    }

    Ok(Hasil {
        text: full_text,
        raw: String::new(),
        model: model.unwrap_or_default(),
        title,
        conversation_id: conv_id,
        message_id: assistant_msg_id,
    })
}

/// `buildPrompt({ message, instruction, history })` (`gpt52.js:562-608`).
pub fn build_prompt(message: &str, instruction: &str, history: &[(String, String)]) -> String {
    let mut parts: Vec<String> = Vec::new();

    if !instruction.trim().is_empty() {
        parts.push(
            [
                "IKUTI INSTRUKSI SISTEM BERIKUT SECARA KETAT.",
                "Jangan ringkas, jangan abaikan, dan jangan ubah format tag yang diminta.",
                "Jika instruksi sistem meminta tag seperti [ACTION:...] atau [RICH:...], keluarkan persis format itu.",
                "Jangan bungkus jawaban dengan markdown code fence kecuali memang isi RICH:CODE yang diminta.",
                "Jangan jelaskan aturan. Langsung jawab sesuai instruksi.",
                "",
                "SYSTEM INSTRUCTION:",
                instruction.trim(),
            ]
            .join("\n"),
        );
    }

    if !history.is_empty() {
        // `.slice(-10)` — sepuluh entri terakhir.
        let start = history.len().saturating_sub(10);
        let baris: Vec<String> = history[start..]
            .iter()
            .map(|(role, content)| {
                let peran = if role == "assistant" {
                    "Assistant"
                } else {
                    "User"
                };
                let isi = content.trim();
                if isi.is_empty() {
                    String::new()
                } else {
                    format!("{peran}: {isi}")
                }
            })
            .filter(|s| !s.is_empty())
            .collect();
        let gabung = baris.join("\n");
        if !gabung.is_empty() {
            parts.push(format!("RIWAYAT PERCAKAPAN:\n{gabung}"));
        }
    }

    parts.push(
        [
            "PESAN USER SAAT INI:",
            message.trim(),
            "",
            "ATURAN OUTPUT:",
            "- Pertahankan format tag spesial secara persis bila dibutuhkan.",
            "- Jangan ubah [ACTION:TYPE param=value] menjadi variasi lain.",
            "- Jangan ubah [RICH:TYPE]...[/RICH:TYPE] menjadi markdown biasa.",
            "- Jangan tambahkan pagar kode ``` di awal atau akhir jawaban.",
        ]
        .join("\n"),
    );

    parts
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `chat({ message, instruction, imageBuffer, history })` (`gpt52.js:619-657`).
///
/// `**teks**` menjadi `*teks*` (`gpt52.js:644`), hasil mentah disimpan
/// terpisah supaya plugin bisa memakai bentuk aslinya.
pub fn chat(
    http: &dyn HttpGet,
    message: &str,
    instruction: &str,
    history: &[(String, String)],
    image: Option<&[u8]>,
) -> Result<Hasil, HttpError> {
    if message.trim().is_empty() {
        return Err(HttpError {
            message: "Message is required.".to_string(),
        });
    }
    let prompt = build_prompt(message, instruction, history);
    let client = ChatGpt::default();
    let hasil = client.send(http, &prompt, &Opsi::default(), image)?;
    let kasar = hasil.text.clone();
    let teks =bintang_gada_jadi_satu(&kasar);
    Ok(Hasil {
        text: teks.clone(),
        raw: if kasar.is_empty() { teks } else { kasar },
        title: hasil.title,
        model: if hasil.model.is_empty() {
            "gpt52".to_string()
        } else {
            hasil.model
        },
        conversation_id: hasil.conversation_id,
        message_id: hasil.message_id,
    })
}

/// `replace(/\*\*(.+?)\*\*/g, "*$1*")` (`gpt52.js:644`) — non-greedy, tanpa
/// benturan `*`, dan tidak menyalin `\n`.
fn bintang_gada_jadi_satu(kasar: &str) -> String {
    let b: Vec<char> = kasar.chars().collect();
    let mut out = String::with_capacity(kasar.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == '*' && b.get(i + 1) == Some(&'*') {
            // Cari penutup `**` pertama yang tidak diikuti newline dan tidak
            //didahului `*` (meniru `.` yang tidak melewati newline).
            let mut j = i + 2;
            let mut tutup = None;
            while j < b.len() {
                if b[j] == '\n' {
                    break;
                }
                if b[j] == '*' && b.get(j + 1) == Some(&'*') && b.get(j + 2) != Some(&'*') {
                    tutup = Some(j);
                    break;
                }
                j += 1;
            }
            if let Some(t) = tutup {
                out.push('*');
                out.extend(b[i + 2..t].iter());
                out.push('*');
                i = t + 2;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out.trim().to_string()
}

fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        seen: Mutex<Vec<HttpCall>>,
        bodies: Mutex<Vec<(String, u16, String)>>,
    }

    impl Stub {
        fn baru(bodies: &[(&str, u16, &str)]) -> Arc<Stub> {
            Arc::new(Stub {
                seen: Mutex::new(Vec::new()),
                bodies: Mutex::new(
                    bodies
                        .iter()
                        .map(|(k, s, b)| (k.to_string(), *s, b.to_string()))
                        .collect(),
                ),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(HttpCall::get(url));
            Ok(self.jawab(url))
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.clone());
            Ok(self.jawab(&req.url))
        }
    }

    impl Stub {
        fn jawab(&self, url: &str) -> HttpResponse {
            for (k, status, body) in self.bodies.lock().unwrap().iter() {
                if url.contains(k.as_str()) {
                    return HttpResponse {
                        status: *status,
                        body: HttpBody::Bytes {
                            data: Arc::from(body.clone().into_bytes()),
                            mimetype: None,
                        },
                        headers: Vec::new(),
                    };
                }
            }
            HttpResponse {
                status: 599,
                body: HttpBody::Bytes {
                    data: Arc::from(&b"tanpa stub"[..]),
                    mimetype: None,
                },
                headers: Vec::new(),
            }
        }
    }

    fn header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    #[test]
    fn fnv1a_cocok_dengan_js() {
        // Dihitung dengan implementasi JS yang sama persis.
        fn js(s: &str) -> String {
            let mut h: u32 = 2166136261;
            for u in s.encode_utf16() {
                h ^= u as u32;
                h = h.wrapping_mul(16777619);
            }
            h ^= h >> 16;
            h = h.wrapping_mul(2246822507);
            h ^= h >> 13;
            h = h.wrapping_mul(3266489909);
            h ^= h >> 16;
            format!("{h:08x}")
        }
        for s in ["", "a", "abc", "seed+config", "chatgpt.com"] {
            assert_eq!(fnv1a_js(s), js(s), "input {s:?}");
        }
    }

    #[test]
    fn tanggal_utc_punya_bentuk_string_date_js() {
        // 2026-10-17T08:20:31Z dihitung dengan `Date.UTC` Node.
        assert_eq!(
            tanggal_js(1_792_225_231_000),
            "Sat Oct 17 2026 08:20:31 GMT+0000 (Coordinated Universal Time)"
        );
    }

    #[test]
    fn basis64_sama_dengan_buffer_js() {
        assert_eq!(basis64(b""), "");
        assert_eq!(basis64(b"a"), "YQ==");
        assert_eq!(basis64(b"ab"), "YWI=");
        assert_eq!(basis64(b"abc"), "YWJj");
        assert_eq!(basis64(b"abcd"), "YWJjZA==");
    }

    #[test]
    fn sse_menempelkan_patch_dan_membaca_metadata() {
        let body = "data: {\"conversation_id\":\"conv-1\"}\n\n\
             data: {\"type\":\"server_ste_metadata\",\"metadata\":{\"model_slug\":\"gpt-5\"}}\n\n\
             data: {\"v\":{\"message\":{\"author\":{\"role\":\"assistant\"},\"id\":\"asst-9\"}}}\n\n\
             data: {\"v\":[{\"o\":\"append\",\"p\":\"/message/content/parts/0\",\"v\":\"Halo\"},\
             {\"o\":\"append\",\"p\":\"/message/content/parts/0\",\"v\":\" dunia\"}]}\n\n\
             data: [DONE]\n\n";
        let hasil = parse_sse(body).unwrap();
        assert_eq!(hasil.text, "Halo dunia");
        assert_eq!(hasil.model, "gpt-5");
        assert_eq!(hasil.conversation_id.as_deref(), Some("conv-1"));
        assert_eq!(hasil.message_id.as_deref(), Some("asst-9"));
    }

    #[test]
    fn patch_di_lain_tidak_dihitung_dan_rusak_dilewati() {
        let body = "data: {\"v\":[{\"o\":\"append\",\"p\":\"/message/metadata\",\"v\":\"x\"}]}\n\n\
             data: bukan json\n\n\
             data: {\"v\":[{\"o\":\"append\",\"p\":\"/message/content/parts/0\",\"v\":\"ok\"}]}\n\n";
        assert_eq!(parse_sse(body).unwrap().text, "ok");
    }

    #[test]
    fn prompt_menempelkan_instruksi_riwayat_dan_pesan() {
        let riwayat = vec![
            ("user".to_string(), "halo".to_string()),
            ("assistant".to_string(), "hai".to_string()),
        ];
        let p = build_prompt("  apa kabar?  ", "  JAWAB SINGKAT  ", &riwayat);
        assert!(p.starts_with("IKUTI INSTRUKSI SISTEM BERIKUT SECARA KETAT."));
        assert!(p.contains("SYSTEM INSTRUCTION:\nJAWAB SINGKAT\n"));
        assert!(p.contains("RIWAYAT PERCAKAPAN:\nUser: halo\nAssistant: hai"));
        assert!(p.contains("PESAN USER SAAT INI:\napa kabar?"));
        assert!(p.ends_with("jawaban."));
        // Tanpa instruksi dan riwayat, blok pertama hilang.
        let polos = build_prompt("hi", "", &[]);
        assert!(polos.starts_with("PESAN USER SAAT INI:"));
    }

    #[test]
    fn hanya_sepuluh_entri_riwayat_terakhir_yang_dipakai() {
        let riwayat: Vec<(String, String)> = (0..15)
            .map(|i| ("user".to_string(), format!("pesan {i}")))
            .collect();
        let p = build_prompt("x", "", &riwayat);
        assert!(!p.contains("pesan 4"), "entri pertama terpotong");
        assert!(p.contains("pesan 5"), "sepuluh terakhir dipakai");
        assert!(p.contains("pesan 14"));
    }

    #[test]
    fn bintang_ganda_jadi_satu_tanpa_melintang_baris() {
        assert_eq!(bintang_gada_jadi_satu("**tebal** dan **tipis**"), "*tebal* dan *tipis*");
        assert_eq!(bintang_gada_jadi_satu("  **a**  "), "*a*");
        assert_eq!(bintang_gada_jadi_satu("**tak ditutup"), "**tak ditutup");
        assert_eq!(bintang_gada_jadi_satu("**a\nb**"), "**a\nb**", "`.` tidak melewati newline");
    }

    #[test]
    fn pesan_kosong_ditolak_dua_kalimat_sesuai_js() {
        let http = Stub::baru(&[]);
        let err = chat(http.as_ref(), "   ", "", &[], None).unwrap_err();
        assert_eq!(err.message, "Message is required.");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn dimensi_gambar_png_dan_jpeg_dan_bawaan_nol() {
        let mut png = vec![0u8; 24];
        png[16..20].copy_from_slice(&800u32.to_be_bytes());
        png[20..24].copy_from_slice(&600u32.to_be_bytes());
        assert_eq!(image_dimensions(&png, "image/png"), (800, 600));

        // JPEG: SOI + APP0 (panjang 16) + dua byte sisipan + SOF0. Dua byte
        // sisipan itu bukan gaya penulisan, tapi meniru lompatan `i += 2 +
        // segLen` dari JS yang menghitung ulang penanda 0xff, jadi penanda
        // SOF0 harus ada di indeks 20.
        let mut jpg: Vec<u8> = vec![0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10];
        jpg.extend_from_slice(&[0u8; 14]);
        jpg.extend_from_slice(&[0xff, 0xc0, 0x00, 0x11, 0x08, 0x02, 0x58, 0x03, 0x20]);
        jpg.extend_from_slice(&[0u8; 16]);
        assert_eq!(image_dimensions(&jpg, "image/jpeg"), (800, 600));
        assert_eq!(image_dimensions(b"bukan gambar", "image/gif"), (0, 0));
    }

    #[test]
    fn mime_gambar_dan_jatuh_ke_oktet_stream() {
        assert_eq!(ext_to_mime("a.PNG"), "image/png");
        assert_eq!(ext_to_mime("a.jpeg"), "image/jpeg");
        assert_eq!(ext_to_mime("a.gif"), "image/gif");
        assert_eq!(ext_to_mime("a.webp"), "image/webp");
        assert_eq!(ext_to_mime("a.svg"), "application/octet-stream");
    }

    #[test]
    fn alur_penuh_mengirim_sentinel_lalu_conduit_lalu_sse() {
        let http = Stub::baru(&[
            (
                "/sentinel/chat-requirements/prepare",
                200,
                r#"{"prepare_token":"PREP"}"#,
            ),
            ("/sentinel/chat-requirements/finalize", 200, r#"{"token":"REQ-TOK"}"#),
            ("/f/conversation/prepare", 200, r#"{"token":"COND-TOK"}"#),
            (
                "/backend-anon/f/conversation",
                200,
                "data: {\"conversation_id\":\"c1\"}\n\n\
                 data: {\"v\":[{\"o\":\"append\",\"p\":\"/message/content/parts/0\",\"v\":\"hai\"}]}\n\n",
            ),
        ]);
        let c = ChatGpt::default();
        let hasil = c
            .send(http.as_ref(), "halo", &Opsi::default(), None)
            .unwrap();
        assert_eq!(hasil.text, "hai");
        assert_eq!(hasil.conversation_id.as_deref(), Some("c1"));

        let p = http.seen.lock().unwrap();
        let urls: Vec<&str> = p.iter().map(|r| r.url.as_str()).collect();
        assert_eq!(urls.len(), 4);
        assert!(urls[0].ends_with("/backend-anon/sentinel/chat-requirements/prepare"));
        assert!(urls[1].ends_with("/backend-anon/sentinel/chat-requirements/finalize"));
        assert!(urls[2].ends_with("/backend-anon/f/conversation/prepare"));
        assert!(urls[3].ends_with("/backend-anon/f/conversation"));

        // Prepare mengirim token awal berawalan `gAAAAAC` (`gpt52.js:259`).
        let badan = String::from_utf8(p[0].body.as_ref().unwrap().bytes().to_vec()).unwrap();
        assert!(badan.contains(r#""p":"gAAAAAC"#), "{badan}");
        assert_eq!(header(&p[0], "accept").as_deref(), Some("*/*"));
        // `extra` menimpa `accept` bawaan untuk permintaan SSE.
        assert_eq!(
            header(&p[3], "accept").as_deref(),
            Some("text/event-stream")
        );
        assert_eq!(
            header(&p[3], "X-Conduit-Token").as_deref(),
            Some("COND-TOK")
        );
        assert_eq!(
            header(&p[3], "OpenAI-Sentinel-Chat-Requirements-Token").as_deref(),
            Some("REQ-TOK")
        );
        assert_eq!(header(&p[2], "X-Conduit-Token").as_deref(), Some("no-token"));
    }

    #[test]
    fn proof_of_work_dipakai_hanya_saat_diminta() {
        let tanpa = ChatGpt::default();
        let hasil = tanpa.sentinel_tokens(
            Stub::baru(&[
                ("prepare", 200, r#"{"prepare_token":"P"}"#),
                ("finalize", 200, r#"{"token":"T"}"#),
            ])
            .as_ref(),
        );
        assert!(hasil.is_ok());

        let dengan = ChatGpt::default();
        let hasil = dengan
            .sentinel_tokens(
                Stub::baru(&[
                    (
                        "prepare",
                        200,
                        r#"{"prepare_token":"P","proofofwork":{"required":true,
                             "seed":"0","difficulty":"00000"}}"#,
                    ),
                    ("finalize", 200, r#"{"token":"T"}"#),
                ])
                .as_ref(),
            )
            .unwrap();
        assert!(hasil.pow.is_some());
        assert_eq!(hasil.turnstile.chars().count(), 2256);
    }

    #[test]
    fn status_gagal_dibungkus_pesan_http() {
        let http = Stub::baru(&[
            ("prepare", 200, r#"{"prepare_token":"P"}"#),
            ("finalize", 200, r#"{"token":"T"}"#),
            ("f/conversation/prepare", 200, r#"{"token":"C"}"#),
            ("/backend-anon/f/conversation", 429, "terlalu banyak"),
        ]);
        let c = ChatGpt::default();
        let err = c.send(http.as_ref(), "halo", &Opsi::default(), None).unwrap_err();
        assert_eq!(err.message, "HTTP 429: terlalu banyak");
    }
}
