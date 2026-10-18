//! `src/scraper/wink.js` — pipeline Wink AI: enhancement video lewat
//! `wink.ai`, unggah ke Qiniu (`strategy.app.meitudata.com`), transcode,
//! lalu_polling hasil akhir.
//!
//! Alur JS-nya (`wink.js:504-573`): `getMaatSign` -> `getUploadPolicy` ->
//! `uploadToQiniu` -> `getVideoInfo` -> `startTranscode` -> `waitTranscode` ->
//! `delivery` -> `waitResult`.
//!
//! Penyimpangan yang dicatat apa adanya:
//!
//! - **Tidak ada berkas temporer.** JS menulis buffer video ke `os.tmpdir()`
//!   lalu membacanya sebagai `fs.createReadStream`; di sini byte-nya langsung
//!   jadi isi part multipart. Nama berkas tetap melalui `safe_temp_name`
//!   supaya nama yang dikirim ke Qiniu sama seperti JS.
//! - **Cookie tidak butuh jar.** JS mengisi `CookieJar` dengan dua cookie
//!   lokal (`_sm`, `meitustat`) sebelum ada permintaan dan tidak pernah
//!   membaca `set-cookie` dari respons, jadi header `Cookie` dihitung langsung.
//!   Cookie `wink.ai` juga tidak dikirim ke `strategy.app.meitudata.com`,
//!   jadi `getUploadPolicy` dan `uploadToQiniu` tidak memakainya.
//! - **Jeda polling** memakai `std::thread::sleep`. `Rencana` diambil dari
//!   pemanggil supaya test bisa memakai `try = 1` tanpa menunggu.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, MultipartPart};
use rand::Rng;
use serde_json::{Value, json};

const BASE_URL: &str = "https://wink.ai";
const STRATEGY_URL: &str = "https://strategy.app.meitudata.com";

const CLIENT_ID: &str = "1189857605";
const VERSION: &str = "5.1.2";
const COUNTRY_CODE: &str = "ID";
const CLIENT_LANGUAGE: &str = "en_US";
const CLIENT_TIMEZONE: &str = "Asia/Jakarta";

const TASK_TYPE: &str = "11";
const CONTENT_TYPE: &str = "2";

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";
const SEC_CH_UA: &str = "\"Google Chrome\";v=\"147\", \"Not.A/Brand\";v=\"8\", \"Chromium\";v=\"147\"";

/// Anggaran polling (`wink.js:589-604`). Default lama 80x3s + 120x5s = 840
/// detik (14 menit) padahal plugin menjanjikan "1-5 menit". Sekarang
/// dibatasi total 4 menit 30 detik: transcode 150s, result 120s.
pub struct Rencana {
    pub transcode_tries: usize,
    pub transcode_delay_ms: u64,
    pub result_tries: usize,
    pub result_delay_ms: u64,
}

/// `planPollBudget()` (`wink.js:595-604`).
pub fn plan_poll_budget() -> Rencana {
    const TRANSCODE_MS: u64 = 150_000;
    const RESULT_MS: u64 = 120_000;
    let transcode_delay_ms = 3000;
    let result_delay_ms = 4000;
    Rencana {
        transcode_tries: TRANSCODE_MS.div_ceil(transcode_delay_ms) as usize,
        transcode_delay_ms,
        result_tries: RESULT_MS.div_ceil(result_delay_ms) as usize,
        result_delay_ms,
    }
}

/// `totalWorstCaseMs()` (`wink.js:606-609`) — angka default lama, dipakai
/// test untuk mengunci janji "1-5 menit".
pub fn total_worst_case_ms() -> u64 {
    80 * 3000 + 120 * 5000
}

/// `isUsableUrl(value)` (`wink.js:406-408`).
pub fn is_usable_url(value: &str) -> bool {
    value.starts_with("https://")
}

/// `isStaleResponse(responseMsgId, currentMsgId)` (`wink.js:612-614`).
pub fn is_stale_response(response_msg_id: &str, current_msg_id: &str) -> bool {
    !response_msg_id.is_empty() && response_msg_id != current_msg_id
}

/// `safeTempName(input)` (`wink.js:577-587`). Tanpa sanitasi, nama dari luar
/// bisa keluar dari direktori temp (path traversal).
pub fn safe_temp_name(input: &str) -> String {
    let mut base = String::new();
    let mut pendingslash = false;
    for c in input.chars() {
        if c == '/' || c == '\\' {
            // `replace(/[/\\]+/g, "-")` — runtuhn digabung jadi satu `-`.
            pendingslash = true;
        } else if !pendingslash {
            base.push(c);
        } else {
            base.push('-');
            base.push(c);
            pendingslash = false;
        }
    }
    if pendingslash {
        base.push('-');
    }
    // `replace(/\.\./g, "")` lebih dulu, jadi ini setelah penggabungan slash.
    let base = base.replace("..", "");
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let tanpa_garis_awal = cleaned.trim_start_matches('-').to_string();
    let dipotong = if tanpa_garis_awal.chars().count() > 80 {
        tanpa_garis_awal.chars().take(80).collect::<String>()
    } else {
        tanpa_garis_awal
    };
    let stem = if dipotong.to_ascii_lowercase().ends_with(".mp4") {
        dipotong[..dipotong.len() - 4].to_string()
    } else {
        dipotong.clone()
    };
    let stem = if stem.is_empty() {
        "wink-video".to_string()
    } else {
        stem
    };
    format!("{stem}.mp4")
}

/// `extractResultUrl(data)` (`wink.js:410-424`).
///
/// Meitu kadang mengisi `media_data` dengan JSON dimensi, bukan URL. Kandidat
/// yang bukan `https://` dilewati apa adanya.
pub fn extract_result_url(data: &Value) -> String {
    let item = &data["item_list"][0];
    let media = &item["result"]["media_info_list"][0];
    let kandidat = [
        media.get("media_data").and_then(Value::as_str),
        item["result"].get("result_url").and_then(Value::as_str),
        item["result"].get("url").and_then(Value::as_str),
        item["client_ext_params"]
            .get("video_transcoded")
            .and_then(Value::as_str),
    ];
    for k in kandidat.into_iter().flatten() {
        if is_usable_url(k) {
            return k.to_string();
        }
    }
    String::new()
}

/// `extractNextMsgId(data, currentMsgId)` (`wink.js:426-445`).
pub fn extract_next_msg_id(data: &Value, current_msg_id: &str) -> String {
    let item = &data["item_list"][0];
    let nilai = item["result"].get("result").and_then(Value::as_str).unwrap_or("");
    let asli = item["result"]
        .get("msg_id")
        .and_then(Value::as_str)
        .or_else(|| item.get("msg_id").and_then(Value::as_str))
        .unwrap_or("");

    if !nilai.is_empty()
        && nilai != current_msg_id
        && !nilai.starts_with("http")
        && !nilai.starts_with("https")
    {
        return nilai.to_string();
    }
    if !asli.is_empty() && asli != current_msg_id && !asli.starts_with("wpr_") {
        return asli.to_string();
    }
    String::new()
}

/// Enkode `application/x-www-form-urlencoded` seperti `URLSearchParams`
/// (spasi jadi `+`, `*` tidak di-escape).
fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '*' | '-' | '.' | '_') {
            out.push(c);
        } else if c == ' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn qs(pasangan: &[(&str, String)]) -> String {
    pasangan
        .iter()
        .map(|(k, v)| format!("{}={}", form_encode(k), form_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// `crypto.randomUUID()` (`wink.js:28`).
fn uuid_v4() -> String {
    let mut bytes: [u8; 16] = rand::rng().random();
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `makeTrace()` (`wink.js:93-95`).
fn make_trace() -> String {
    let mut rng = rand::rng();
    let a: [u8; 16] = rng.random();
    let b: [u8; 8] = rng.random();
    let hex = |x: &[u8]| -> String { x.iter().map(|v| format!("{v:02x}")).collect() };
    format!("{}-{}-1", hex(&a), hex(&b))
}

/// `traceHeaders()` (`wink.js:97-113`).
fn trace_headers() -> Vec<(String, String)> {
    let trace = make_trace();
    let trace_id = trace.split('-').next().unwrap_or("").to_string();
    vec![
        ("sentry-trace".to_string(), trace),
        (
            "baggage".to_string(),
            [
                "sentry-environment=release".to_string(),
                "sentry-release=5.1.2%20(b60d25c477f43c6dfac4107810f26d442320f4f1)".to_string(),
                "sentry-public_key=e1bf914f3448d9bc8a10c7e499d17d54".to_string(),
                format!("sentry-trace_id={trace_id}"),
                "sentry-sampled=true".to_string(),
                "sentry-sample_rate=0.75".to_string(),
            ]
            .join(","),
        ),
    ]
}

/// Header default `axios.create` di `getApi()` (`wink.js:44-60`) ditambah
/// cookie jar lokal (`wink.js:30-37`).
fn header_api(gnum: &str) -> Vec<(String, String)> {
    let meitustat = crate::common::encode_uri_component(&json!({ "wgid": gnum }).to_string());
    vec![
        ("accept".to_string(), "*/*".to_string()),
        ("origin".to_string(), BASE_URL.to_string()),
        ("referer".to_string(), format!("{BASE_URL}/video-enhancer/upload")),
        ("user-agent".to_string(), UA.to_string()),
        ("sec-ch-ua".to_string(), SEC_CH_UA.to_string()),
        ("sec-ch-ua-mobile".to_string(), "?1".to_string()),
        ("sec-ch-ua-platform".to_string(), "\"Android\"".to_string()),
        (
            "ab_info".to_string(),
            json!({ "ab_codes": [], "version": "1.4.4" }).to_string(),
        ),
        (
            "cookie".to_string(),
            format!("_sm={gnum}; meitustat={meitustat}"),
        ),
    ]
}

/// Header untuk `httpAxios` — domain `strategy.app.meitudata.com`, tanpa
/// cookie `wink.ai` (`wink.js:141-151`).
fn header_strategy(referer: &str) -> Vec<(String, String)> {
    vec![
        ("accept".to_string(), "*/*".to_string()),
        ("origin".to_string(), BASE_URL.to_string()),
        ("referer".to_string(), referer.to_string()),
        ("user-agent".to_string(), UA.to_string()),
        ("sec-ch-ua".to_string(), SEC_CH_UA.to_string()),
        ("sec-ch-ua-mobile".to_string(), "?1".to_string()),
        ("sec-ch-ua-platform".to_string(), "\"Android\"".to_string()),
    ]
}

/// `baseParams(extra)` (`wink.js:115-129`).
fn base_params(gnum: &str, extra: &[(&str, String)]) -> String {
    let mut pasangan: Vec<(&str, String)> = vec![
        ("client_id", CLIENT_ID.to_string()),
        ("version", VERSION.to_string()),
        ("country_code", COUNTRY_CODE.to_string()),
        ("gnum", gnum.to_string()),
        ("client_language", CLIENT_LANGUAGE.to_string()),
        ("client_channel_id", String::new()),
        ("client_timezone", CLIENT_TIMEZONE.to_string()),
    ];
    pasangan.extend_from_slice(extra);
    qs(&pasangan)
}

/// Header tambahan untuk permintaan form `POST` ke `wink.ai`.
fn tambah(mut base: Vec<(String, String)>, tambahan: Vec<(String, String)>) -> Vec<(String, String)> {
    base.extend(tambahan);
    base
}

fn cek(res: Result<oktz_plugin_api::HttpResponse, HttpError>) -> Result<Value, HttpError> {
    let res = res?;
    if res.status >= 400 {
        return Err(HttpError {
            message: format!("HTTP {}", res.status),
        });
    }
    Ok(res.json().unwrap_or(Value::Null))
}

/// `getMaatSign()` (`wink.js:131-147`).
fn get_maat_sign(http: &dyn HttpGet, gnum: &str) -> Result<Value, HttpError> {
    let params = base_params(
        gnum,
        &[
            ("suffix", ".mp4".to_string()),
            ("type", "temp".to_string()),
            ("count", "1".to_string()),
        ],
    );
    let mut call = HttpCall::get(format!("{BASE_URL}/api/file/get_maat_sign.json?{params}"))
        .terima_semua_status();
    for (k, v) in tambah(header_api(gnum), trace_headers()) {
        call = call.header(&k, &v);
    }
    let data = cek(http.call(&call))?;
    let ok = data.get("code") == Some(&json!(0));
    if !ok {
        return Err(HttpError {
            message: format!("get_maat_sign gagal: {}", json_teks(&data)),
        });
    }
    Ok(data["data"].clone())
}

/// `getUploadPolicy(sign)` (`wink.js:149-178`).
fn get_upload_policy(http: &dyn HttpGet, sign: &Value) -> Result<Value, HttpError> {
    let params = qs(&[
        ("app", js_str(&sign["app"])),
        ("count", js_str(&sign["count"])),
        ("sig", js_str(&sign["sig"])),
        ("sigTime", js_str(&sign["sig_time"])),
        ("sigVersion", js_str(&sign["sig_version"])),
        ("suffix", js_str(&sign["suffix"])),
        ("type", js_str(&sign["type"])),
    ]);
    let mut call = HttpCall::get(format!("{STRATEGY_URL}/upload/policy?{params}"))
        .terima_semua_status();
    for (k, v) in header_strategy(&format!("{BASE_URL}/")) {
        call = call.header(&k, &v);
    }
    let data = cek(http.call(&call))?;
    let ada_qiniu = data
        .as_array()
        .and_then(|a| a.first())
        .and_then(|v| v.get("qiniu"))
        .is_some();
    if !ada_qiniu {
        return Err(HttpError {
            message: format!("upload policy gagal: {}", json_teks(&data)),
        });
    }
    Ok(data[0]["qiniu"].clone())
}

/// `extToMime(file)` (`wink.js:74-82`).
fn ext_to_mime(nama: &str) -> &'static str {
    let huruf_kecil = nama.to_ascii_lowercase();
    if huruf_kecil.ends_with(".mp4") {
        "video/mp4"
    } else if huruf_kecil.ends_with(".mov") {
        "video/quicktime"
    } else if huruf_kecil.ends_with(".webm") {
        "video/webm"
    } else if huruf_kecil.ends_with(".mkv") {
        "video/x-matroska"
    } else {
        "application/octet-stream"
    }
}

/// `uploadToQiniu(policy, filePath)` (`wink.js:180-217`).
fn upload_qiniu(
    http: &dyn HttpGet,
    policy: &Value,
    video: &[u8],
    nama: &str,
) -> Result<Value, HttpError> {
    let url = js_str(&policy["url"]);
    let key = js_str(&policy["key"]);
    let token = js_str(&policy["token"]);
    let fallback = policy.get("data").cloned().unwrap_or(Value::Null);

    let mut berkas = MultipartPart::file("file", nama.to_string(), video.to_vec());
    // `contentType` dari `extToMime` ikut pada part berkas.
    berkas.mimetype = Some(ext_to_mime(nama).to_string());

    let mut call = HttpCall::post(&url)
        .terima_semua_status()
        .multipart(vec![
            berkas,
            MultipartPart::field("token", token),
            MultipartPart::field("key", key.clone()),
            MultipartPart::field("fname", nama.to_string()),
        ]);
    for (k, v) in header_strategy(&format!("{BASE_URL}/")) {
        call = call.header(&k, &v);
    }
    let data = cek(http.call(&call))?;
    let ada_url = truthy_str(&js_str(&data["url"])) || truthy_str(&js_str(&data["data"]));
    if !ada_url {
        return Err(HttpError {
            message: format!("upload qiniu response tidak valid: {}", json_teks(&data)),
        });
    }
    Ok(json!({
        "file_key": key,
        "source_url": pilih(&data["url"], &data["data"], &fallback),
        "raw_url": pilih(&data["data"], &fallback, &fallback),
        "raw": data,
    }))
}

fn pilih(a: &Value, b: &Value, bawaan: &Value) -> Value {
    if crate::common::truthy(a) {
        a.clone()
    } else if crate::common::truthy(b) {
        b.clone()
    } else {
        bawaan.clone()
    }
}

fn truthy_str(s: &str) -> bool {
    !s.is_empty()
}

/// `getVideoInfo(fileKey)` (`wink.js:219-239`).
fn get_video_info(http: &dyn HttpGet, gnum: &str, file_key: &str) -> Result<Value, HttpError> {
    let body = base_params(gnum, &[("file_key", file_key.to_string())]);
    let mut call = HttpCall::post(format!("{BASE_URL}/api/file/video_cover_and_display_info_ext.json"))
        .terima_semua_status()
        .form(body);
    for (k, v) in tambah(header_api(gnum), trace_headers()) {
        call = call.header(&k, &v);
    }
    call = call.header("content-type", "application/x-www-form-urlencoded;charset=UTF-8");
    let data = cek(http.call(&call))?;
    if data.get("code") != Some(&json!(0)) {
        return Err(HttpError {
            message: format!("video info gagal: {}", json_teks(&data)),
        });
    }
    Ok(data["data"].clone())
}

/// `startTranscode(fileKey)` (`wink.js:241-259`).
fn start_transcode(http: &dyn HttpGet, gnum: &str, file_key: &str) -> Result<String, HttpError> {
    let body = base_params(gnum, &[("file_key", file_key.to_string())]);
    let mut call =
        HttpCall::post(format!("{BASE_URL}/api/file/video_trans_start.json"))
            .terima_semua_status()
            .form(body);
    for (k, v) in tambah(header_api(gnum), trace_headers()) {
        call = call.header(&k, &v);
    }
    call = call.header("content-type", "application/x-www-form-urlencoded;charset=UTF-8");
    let data = cek(http.call(&call))?;
    let id = data["data"]["id"].clone();
    if data.get("code") != Some(&json!(0)) || !crate::common::truthy(&id) {
        return Err(HttpError {
            message: format!("transcode start gagal: {}", json_teks(&data)),
        });
    }
    Ok(js_str(&id))
}

/// `queryTranscode(id)` (`wink.js:261-277`).
fn query_transcode(http: &dyn HttpGet, gnum: &str, id: &str) -> Result<Value, HttpError> {
    let params = base_params(gnum, &[("id", id.to_string())]);
    let mut call = HttpCall::get(format!("{BASE_URL}/api/file/video_trans_query.json?{params}"))
        .terima_semua_status();
    for (k, v) in tambah(header_api(gnum), trace_headers()) {
        call = call.header(&k, &v);
    }
    let data = cek(http.call(&call))?;
    if data.get("code") != Some(&json!(0)) {
        return Err(HttpError {
            message: format!("transcode query gagal: {}", json_teks(&data)),
        });
    }
    Ok(data["data"].clone())
}

/// `waitTranscode(...)` (`wink.js:294-334`). Kalau lewat batas waktu, THROW
/// — jangan kembalikan URL asli seolah-olah itu hasil enhance.
pub fn wait_transcode(
    http: &dyn HttpGet,
    gnum: &str,
    id: &str,
    fallback_source_url: &str,
    rencana: &Rencana,
) -> Result<Value, HttpError> {
    let mut last = Value::Null;
    for i in 1..=rencana.transcode_tries {
        let data = query_transcode(http, gnum, id)?;
        last = data.clone();

        let video = [&data["video"], &data["url"], &data["source_url"]]
            .into_iter()
            .map(js_str)
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        let transcoded = [
            &data["video_transcoded"],
            &data["transcoded_video"],
            &data["transcoded_url"],
            &data["video_url"],
        ]
        .into_iter()
        .map(js_str)
        .find(|s| !s.is_empty())
        .unwrap_or_default();

        if is_usable_url(&transcoded) && transcoded != fallback_source_url {
            let sumber = if video.is_empty() {
                fallback_source_url.to_string()
            } else {
                video
            };
            return Ok(json!({
                "source_url": sumber,
                "video_transcoded": transcoded,
                "raw": data,
            }));
        }

        if i < rencana.transcode_tries {
            std::thread::sleep(std::time::Duration::from_millis(rencana.transcode_delay_ms));
        }
    }
    Err(HttpError {
        message: format!(
            "Transcode tidak selesai dalam {} detik. Respons terakhir: {}",
            (rencana.transcode_tries as u64) * rencana.transcode_delay_ms / 1000,
            json_teks(&last).chars().take(200).collect::<String>()
        ),
    })
}

/// `delivery(sourceUrl, videoTranscoded, taskName)` (`wink.js:336-374`).
fn delivery(
    http: &dyn HttpGet,
    gnum: &str,
    source_url: &str,
    video_transcoded: &str,
    task_name: &str,
) -> Result<Value, HttpError> {
    let body = base_params(
        gnum,
        &[
            ("type", TASK_TYPE.to_string()),
            ("content_type", CONTENT_TYPE.to_string()),
            ("source_url", source_url.to_string()),
            (
                "type_params",
                json!({
                    "is_mirror": 0,
                    "orientation_tag": 1,
                    "j_420_trans": "1",
                    "return_ext": "2",
                })
                .to_string(),
            ),
            (
                "right_detail",
                json!({
                    "source": "1",
                    "touch_type": "4",
                    "function_id": "630",
                    "material_id": "63011",
                    "url": "https://wink.ai/video-enhancer/upload",
                })
                .to_string(),
            ),
            (
                "ext_params",
                json!({
                    "task_name": task_name,
                    "records": TASK_TYPE,
                    "video_transcoded": video_transcoded,
                })
                .to_string(),
            ),
            ("with_prepare", "1".to_string()),
        ],
    );
    let mut call = HttpCall::post(format!("{BASE_URL}/api/meitu_ai/delivery.json"))
        .terima_semua_status()
        .form(body);
    for (k, v) in tambah(header_api(gnum), trace_headers()) {
        call = call.header(&k, &v);
    }
    call = call.header("content-type", "application/x-www-form-urlencoded;charset=UTF-8");
    let data = cek(http.call(&call))?;
    if data.get("code") != Some(&json!(0)) {
        return Err(HttpError {
            message: format!("delivery gagal: {}", json_teks(&data)),
        });
    }
    let d = &data["data"];
    Ok(json!({
        "msg_id": js_str(&d["msg_id"]),
        "prepare_msg_id": js_str(&d["prepare_msg_id"]),
        "raw": d,
    }))
}

/// `queryBatch(msgId)` (`wink.js:376-399`).
fn query_batch(http: &dyn HttpGet, gnum: &str, msg_id: &str) -> Result<Value, HttpError> {
    let params = base_params(gnum, &[("msg_ids", msg_id.to_string())]);
    let mut call = HttpCall::get(format!("{BASE_URL}/api/meitu_ai/query_batch.json?{params}"))
        .terima_semua_status();
    // `referer` di `queryBatch` sama dengan default, jadi tidak ada penimpaan
    // yang perlu disalin (lihat `wink.js:376-392`).
    for (k, v) in tambah(header_api(gnum), trace_headers()) {
        call = call.header(&k, &v);
    }
    let data = cek(http.call(&call))?;
    if data.get("code") != Some(&json!(0)) {
        return Err(HttpError {
            message: format!("query batch gagal: {}", json_teks(&data)),
        });
    }
    Ok(data["data"].clone())
}

/// `waitResult(firstMsgId, maxTry, delayMs)` (`wink.js:458-502`).
pub fn wait_result(
    http: &dyn HttpGet,
    gnum: &str,
    first_msg_id: &str,
    rencana: &Rencana,
) -> Result<String, HttpError> {
    let mut msg_id = first_msg_id.to_string();
    let mut last = Value::Null;
    for i in 1..=rencana.result_tries {
        let data = query_batch(http, gnum, &msg_id)?;
        last = data.clone();

        let next = extract_next_msg_id(&data, &msg_id);
        if !next.is_empty() {
            msg_id = next;
            if i < rencana.result_tries {
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
            continue;
        }

        let hasil = &data["item_list"][0]["result"];
        let kode = hasil.get("error_code").cloned().unwrap_or(Value::Null);
        let pesan = js_str(&hasil["error_msg"]);
        let kode_nilai = kode.as_i64().or_else(|| kode.as_f64().map(|f| f as i64));
        // 29901 = masih diproses, jadi bukan error.
        if crate::common::truthy(&kode) && kode_nilai != Some(29901) && kode_nilai != Some(0) {
            return Err(HttpError {
                message: format!("task gagal: {} {}", js_str(&kode), pesan),
            });
        }

        let url = extract_result_url(&data);
        if is_usable_url(&url) && kode_nilai == Some(0) {
            return Ok(url);
        }

        if i < rencana.result_tries {
            std::thread::sleep(std::time::Duration::from_millis(rencana.result_delay_ms));
        }
    }
    Err(HttpError {
        message: format!(
            "Result belum selesai dalam {} detik. Respons terakhir: {}",
            (rencana.result_tries as u64) * rencana.result_delay_ms / 1000,
            json_teks(&last).chars().take(200).collect::<String>()
        ),
    })
}

/// `winkEnhance(video, { filename })` (`wink.js:504-573`). Balikan bentuk
/// `{ resultUrl }` dari JS disederhanakan jadi `String` URL-nya saja.
pub fn wink_enhance(
    http: &dyn HttpGet,
    video: &[u8],
    nama_berkas: Option<&str>,
    rencana: &Rencana,
) -> Result<String, HttpError> {
    if video.is_empty() {
        return Err(HttpError {
            message: "video is required".to_string(),
        });
    }
    let bawaan = format!("wink-{}", uuid_v4());
    let nama = safe_temp_name(nama_berkas.unwrap_or(&bawaan));
    let gnum = uuid_v4();
    // `path.parse(filePath).name` — `safe_temp_name` selalu berakhiran `.mp4`.
    let stem = nama[..nama.len() - 4].to_string();
    let task_name = format!("Enhancer-Ultra HD-{stem}");

    let sign = get_maat_sign(http, &gnum)?;
    let policy = get_upload_policy(http, &sign)?;
    let uploaded = upload_qiniu(http, &policy, video, &nama)?;

    let file_key = js_str(&uploaded["file_key"]);
    let source_url = js_str(&uploaded["source_url"]);
    get_video_info(http, &gnum, &file_key)?;

    let transcode_id = start_transcode(http, &gnum, &file_key)?;
    let transcode = wait_transcode(http, &gnum, &transcode_id, &source_url, rencana)?;

    let task = delivery(
        http,
        &gnum,
        &js_str(&transcode["source_url"]),
        &js_str(&transcode["video_transcoded"]),
        &task_name,
    )?;
    let mut first_msg_id = js_str(&task["msg_id"]);
    if first_msg_id.is_empty() {
        first_msg_id = js_str(&task["prepare_msg_id"]);
    }
    if first_msg_id.is_empty() {
        return Err(HttpError {
            message: format!(
                "delivery tidak mengembalikan msg_id: {}",
                json_teks(&task["raw"])
            ),
        });
    }

    wait_result(http, &gnum, &first_msg_id, rencana)
}

/// `String(v)` JS untuk nilai JSON: objek jadi teks JSON-nya, bukan `[object Object]`.
fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// `JSON.stringify(v)` untuk pesan galat.
fn json_teks(v: &Value) -> String {
    v.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet` berbasis pola URL: mencatat permintaan, lalu membalas
    /// dengan bodies yang cocok paling awal. `urutan` Override untuk satu
    /// pola kalau polling perlu balasan berbeda tiap percobaan.
    #[derive(Debug, Default)]
    struct Stub {
        seen: Mutex<Vec<HttpCall>>,
        pola: Mutex<Vec<(String, u16, String)>>,
        urutan: Mutex<Vec<(String, Vec<String>)>>,
        /// Berapa kali tiap entri `urutan` sudah dipakai.
        pakai: Mutex<Vec<usize>>,
    }

    impl Stub {
        fn baru(pola: &[(&str, u16, &str)]) -> Arc<Stub> {
            Arc::new(Stub {
                seen: Mutex::new(Vec::new()),
                pola: Mutex::new(
                    pola.iter()
                        .map(|(k, s, b)| (k.to_string(), *s, b.to_string()))
                        .collect(),
                ),
                urutan: Mutex::new(Vec::new()),
                pakai: Mutex::new(Vec::new()),
            })
        }

        fn jawab(&self, url: &str) -> HttpResponse {
            let urutan = self.urutan.lock().unwrap();
            if let Some(i) = urutan.iter().position(|(p, _)| url.contains(p.as_str())) {
                let n = urutan[i].1.len();
                let ke = self.pakai.lock().unwrap()[i];
                let body = urutan[i].1[ke.min(n.saturating_sub(1))].clone();
                self.pakai.lock().unwrap()[i] += 1;
                return balas(200, &body);
            }
            let pola = self.pola.lock().unwrap();
            for (k, status, body) in pola.iter() {
                if url.contains(k.as_str()) {
                    return balas(*status, body);
                }
            }
            balas(599, "tidak ada stub untuk URL ini")
        }
    }

    fn balas(status: u16, body: &str) -> HttpResponse {
        HttpResponse {
            status,
            body: HttpBody::Bytes {
                data: Arc::from(body.as_bytes()),
                mimetype: None,
            },
            headers: Vec::new(),
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(HttpCall::get(url));
            Ok(self.jawab(url))
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_multipart(&self, _: &str, _: &[(&str, HttpBody)]) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.clone());
            Ok(self.jawab(&req.url))
        }
    }

    fn header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    fn tanpa_jeda() -> Rencana {
        Rencana {
            transcode_tries: 1,
            transcode_delay_ms: 0,
            result_tries: 1,
            result_delay_ms: 0,
        }
    }

    #[test]
    fn nama_temp_aman_dan_mengakhiri_mp4() {
        assert_eq!(safe_temp_name("../../../etc/passwd"), "etc-passwd.mp4");
        assert_eq!(safe_temp_name("klip.mp4"), "klip.mp4");
        assert_eq!(safe_temp_name("a/b\\c.mp4"), "a-b-c.mp4");
        assert_eq!(safe_temp_name("...."), "wink-video.mp4");
        assert_eq!(safe_temp_name(""), "wink-video.mp4");
        assert_eq!(safe_temp_name("video.MP4"), "video.mp4");
        let panjang = safe_temp_name(&"x".repeat(200));
        assert_eq!(panjang.chars().count(), 84, "dipotong 80 + `.mp4`");
    }

    #[test]
    fn anggaran_polling_sejati_kalandar_janji_promo() {
        let r = plan_poll_budget();
        assert_eq!(r.transcode_tries, 50);
        assert_eq!(r.result_tries, 30);
        assert_eq!(total_worst_case_ms(), 840_000, "angka default lama");
    }

    #[test]
    fn kandidat_url_hanya_https() {
        assert!(is_usable_url("https://a/b.mp4"));
        assert!(!is_usable_url("http://a/b.mp4"));
        assert!(!is_usable_url(""));
        assert!(is_stale_response("wpr_1", "wpr_2"));
        assert!(!is_stale_response("", "wpr_2"));
        assert!(!is_stale_response("wpr_1", "wpr_1"));
    }

    #[test]
    fn url_hasil_melewati_media_data_yang_bukan_url() {
        let data = json!({
            "item_list": [{
                "result": {
                    "media_info_list": [{ "media_data": "{\"w\":1920,\"h\":1080}" }],
                    "result_url": "http://bukan-https",
                    "url": "https://akhir/hasil.mp4"
                }
            }]
        });
        assert_eq!(extract_result_url(&data), "https://akhir/hasil.mp4");
        let kosong = json!({ "item_list": [{}] });
        assert_eq!(extract_result_url(&kosong), "");
    }

    #[test]
    fn msg_id_berikut_melewati_yang_mirip_url_atau_wpr() {
        let dari_result = json!({ "item_list": [{ "result": { "result": "msg_baru" } }] });
        assert_eq!(extract_next_msg_id(&dari_result, "msg_lama"), "msg_baru");
        let url = json!({ "item_list": [{ "result": { "result": "https://x/y" } }] });
        assert_eq!(extract_next_msg_id(&url, "msg_lama"), "");
        let sama = json!({ "item_list": [{ "result": { "result": "msg_lama" } }] });
        assert_eq!(extract_next_msg_id(&sama, "msg_lama"), "");
        let dari_msg_id = json!({ "item_list": [{ "msg_id": "wpr_1", "result": {} }] });
        assert_eq!(extract_next_msg_id(&dari_msg_id, "msg_lama"), "");
        let asli = json!({ "item_list": [{ "msg_id": "msg_asli", "result": {} }] });
        assert_eq!(extract_next_msg_id(&asli, "msg_lama"), "msg_asli");
    }

    #[test]
    fn parameter_dasar_ikut_urutan_js_dan_spasi_jadi_plus() {
        // `URLSearchParams` menyandikan `/` jadi `%2F` dan spasi jadi `+`.
        let s = base_params("G-1", &[("file_key", "a b.mp4".to_string())]);
        assert_eq!(
            s,
            "client_id=1189857605&version=5.1.2&country_code=ID&gnum=G-1\
             &client_language=en_US&client_channel_id=&client_timezone=Asia%2FJakarta\
             &file_key=a+b.mp4"
        );
    }

    #[test]
    fn alur_sampai_hasil_akhir_dengan_satu_percobaan() {
        let http = Stub::baru(&[
            (
                "/api/file/get_maat_sign.json",
                200,
                r#"{"code":0,"data":{"app":"A","count":3,"sig":"S","sig_time":"T",
                     "sig_version":"V","suffix":".mp4","type":"temp"}}"#,
            ),
            (
                "/upload/policy",
                200,
                r#"[{"qiniu":{"url":"https://upload.qiniu.com/","key":"K","token":"TOK"}}]"#,
            ),
            ("https://upload.qiniu.com/", 200, r#"{"url":"https://cdn/asli.mp4"}"#),
            ("video_cover_and_display_info_ext", 200, r#"{"code":0,"data":{}}"#),
            ("video_trans_start", 200, r#"{"code":0,"data":{"id":"T1"}}"#),
            (
                "video_trans_query",
                200,
                r#"{"code":0,"data":{"video":"https://cdn/asli.mp4",
                     "video_transcoded":"https://cdn/ultra.mp4"}}"#,
            ),
            ("delivery.json", 200, r#"{"code":0,"data":{"msg_id":"wpr_1"}}"#),
            (
                "query_batch",
                200,
                r#"{"code":0,"data":{"item_list":[{"result":{"error_code":0,
                     "media_info_list":[{"media_data":"https://cdn/hasil.mp4"}]}}]}}"#,
            ),
        ]);
        let hasil = wink_enhance(
            http.as_ref(),
            b"VIDEO",
            Some("klip liar.mp4"),
            &tanpa_jeda(),
        )
        .unwrap();
        assert_eq!(hasil, "https://cdn/hasil.mp4");

        let p = http.seen.lock().unwrap();
        let urls: Vec<&str> = p.iter().map(|r| r.url.as_str()).collect();
        assert_eq!(urls.len(), 8);
        assert!(urls[0].starts_with("https://wink.ai/api/file/get_maat_sign.json?"));
        assert!(urls[1].contains("sig=S"), "policy ikut param sign: {}", urls[1]);
        assert_eq!(urls[2], "https://upload.qiniu.com/");
        assert_eq!(urls[6], "https://wink.ai/api/meitu_ai/delivery.json");
        // Cookie jar lokal ikut pada setiap permintaan ke wink.ai.
        let cookie = header(&p[0], "cookie").expect("cookie");
        assert!(cookie.starts_with("_sm="), "{cookie}");
        assert!(cookie.contains("meitustat=%7B%22wgid%22%3A"), "{cookie}");
        // Upload multipart: file, token, key, fname.
        let Some(HttpBody::Multipart(bagian)) = &p[2].body else {
            panic!("upload harus multipart");
        };
        assert_eq!(bagian.len(), 4);
        assert_eq!(bagian[0].name, "file");
        assert_eq!(bagian[0].body.bytes(), b"VIDEO");
        assert_eq!(bagian[0].file_name.as_deref(), Some("klip-liar.mp4"));
        assert_eq!(bagian[3].name, "fname");
        // Task name memakai nama berkas tanpa ekstensi.
        let badan = String::from_utf8(p[6].body.as_ref().unwrap().bytes().to_vec()).unwrap();
        assert!(badan.contains("Enhancer-Ultra+HD-klip-liar"), "{badan}");
    }

    #[test]
    fn video_kosong_ditolak_tanpa_permintaan() {
        let http = Stub::baru(&[]);
        assert_eq!(
            wink_enhance(http.as_ref(), b"", None, &tanpa_jeda())
                .unwrap_err()
                .message,
            "video is required"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn transcode_berulang_melewati_url_sama_dengan_sumber() {
        // URL hasil yang sama dengan sumber belum dianggap jadi enhance.
        let http = Stub::baru(&[
            (
                "video_trans_query",
                200,
                r#"{"code":0,"data":{"video":"https://cdn/asli.mp4",
                     "video_transcoded":"https://cdn/asli.mp4"}}"#,
            ),
        ]);
        let err = wait_transcode(http.as_ref(), "G", "T1", "https://cdn/asli.mp4", &tanpa_jeda())
            .unwrap_err();
        assert!(
            err.message.starts_with("Transcode tidak selesai dalam 0 detik."),
            "{}",
            err.message
        );
    }

    #[test]
    fn error_code_lain_dari_pending_langsung_berhenti() {
        let http = Stub::baru(&[(
            "query_batch",
            200,
            r#"{"code":0,"data":{"item_list":[{"result":{"error_code":500,
                 "error_msg":"rusak"}}]}}"#,
        )]);
        let err = wait_result(http.as_ref(), "G", "wpr_1", &tanpa_jeda()).unwrap_err();
        assert_eq!(err.message, "task gagal: 500 rusak");
    }

    #[test]
    fn http_bukan_2xx_dan_kode_bukan_nol_melempar_pesannya() {
        let http = Stub::baru(&[("/api/file/get_maat_sign.json", 500, r#"{"code":1}"#)]);
        let err = get_maat_sign(http.as_ref(), "G").unwrap_err();
        assert_eq!(err.message, "HTTP 500");

        let http = Stub::baru(&[("/api/file/get_maat_sign.json", 200, r#"{"code":7,"data":{}}"#)]);
        assert_eq!(
            get_maat_sign(http.as_ref(), "G").unwrap_err().message,
            r#"get_maat_sign gagal: {"code":7,"data":{}}"#
        );
    }

    #[test]
    fn mime_video_dari_ekstensi_dan_bukan_lainnya() {
        assert_eq!(ext_to_mime("a.MP4"), "video/mp4");
        assert_eq!(ext_to_mime("a.mov"), "video/quicktime");
        assert_eq!(ext_to_mime("a.webm"), "video/webm");
        assert_eq!(ext_to_mime("a.mkv"), "video/x-matroska");
        assert_eq!(ext_to_mime("a.avi"), "application/octet-stream");
    }

    #[test]
    fn transcode_batal_melewati_tugas_yang_masih_antre() {
        // 29901 = pending, jadi bukan error; tanpa URL finalYD harus habis percobaan.
        let http = Stub::baru(&[(
            "query_batch",
            200,
            r#"{"code":0,"data":{"item_list":[{"result":{"error_code":29901}}]}}"#,
        )]);
        let err = wait_result(http.as_ref(), "G", "wpr_1", &tanpa_jeda()).unwrap_err();
        assert!(
            err.message.starts_with("Result belum selesai dalam 0 detik."),
            "{}",
            err.message
        );
    }
}
