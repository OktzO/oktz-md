//! `src/scraper/tiktok.js` — unduh TikTok lewat empat provider berurutan.
//!
//! Urutan di `ttdown` (`tiktok.js:262-302`) disalin apa adanya: TikWM dulu,
//! lalu SaveTT (`savett.cc`) untuk tiap varian URL, lalu YuuLabs, lalu
//! MusicalDown. Semua kegagalan dikumpulkan jadi `errors.join(" | ")`.
//!
//! Perhatian khusus yang ikut disalin:
//! - `raw?.code !== undefined && raw.code !== 0` — `!==` itu PANJANG, jadi
//!   kode `"0"` (string) juga dianggap gagal.
//! - `res.duration == 0` memakai `==` LONGAR: `0`, `"0"`, `""`, `false`, dan
//!   `[]` semuanya kena; `null`/`undefined` tidak.
//! - `json.URL[0] || json.URL` — kalau `URL` berupa string, `URL[0]` adalah
//!   KARAKTER PERTAMA, bukan elemen pertama. Itu ikut disalin.
//! - `"https://www.tikwm.com" + res.hdplay` memakai `+`, jadi nilai angka di
//!   belakang jadi teks; semua penggabungan lewat `string_js`.
//! - `params` axios diserialkan dengan encoder bawaan axios 1.x
//!   (`encodeURIComponent`, tapi spasi jadi `+`) yang BERBEDA dari
//!   `encodeURIComponent` — lihat `encode_param_axios`.
//! - `raw.msg || "TikWM error"` dan `data?.message || "YuuLabs response invalid"`
//!   membawa teks ke `Error`, jadi teksnya harus sama persis.
//!
//! Beda yang diketahui dari JS:
//! - `resolveTikTokUrl` membaca `res.request?.res?.responseUrl` (URL akhir
//!   setelah redirect). `HttpResponse` tidak punya field itu, jadi `resolved`
//!   selalu sama dengan `url` — itu juga hasil yang running di Node, karena
//!   `response.request.res.responseUrl` di sana `undefined` sehingga JS jatuh
//!   ke `res.config?.url`. GET-nya tetap dikirim supaya urutan jaringan sama,
//!   hasilnya dibuang.
//! - `e.response.status` di blok `catch` tidak ada di `HttpError` (hanya
//!   `message`), jadi pesan `TikWM 404` / `saveTT 404` tidak bisa dibentuk;
//!   yang masuk adalah pesan lapisan HTTP. Bentuk daftarnya (`tikwm: …`,
//!   `savett(raw): …`, `yuulabs: …`, `musicaldown: …`) tetap sama.
//! - `timeout: 10000/15000/30000` tidak ditiru; `UreqHttp` punya timeout global
//!   20 detik per permintaan.
//! - `cheerio` diganti pengurai HTML mini privat di bawah file ini (bentuk yang
//!   sama dipakai `aio.rs`, `reddit.rs`, `sfiledl.rs`). Selector yang dipakai
//!   file JS ini: `.carousel-item[data-data]`, `#video-info h3`,
//!   `#formatselect option`, `#submit-form input`, `.video-header`,
//!   `a.download`, `.video-desc`, `.video-author b`, `.img-area img`.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{decode_entitas, encode_uri_component, field, truthy};

const YUULABS_API: &str = "https://api.yuulabs.web.id/api/downloader/tiktok?url=";

/// `REQUEST_HEADERS["user-agent"]` (`tiktok.js:6-7`) — dipakai TikWM, YuuLabs,
/// dan MusicalDown.
const UA: &str = "Mozilla/5.0 (Linux; Android 15; SM-F958 Build/AP3A.240905.015) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.6723.86 Mobile Safari/537.36";

/// `SAVETT_HEADERS["User-Agent"]` (`tiktok.js:73`).
const UA_SAVETT: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36";

/// `/v[mt]\.tiktok\.com/i` (`tiktok.js:12`).
fn pola_pendek() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r"(?i)v[mt]\.tiktok\.com").expect("pola tiktok pendek"))
}

/// `/name="csrf_token" value="([^"]+)"/` (`tiktok.js:81`).
fn pola_csrf() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r#"name="csrf_token" value="([^"]+)""#).expect("pola csrf"))
}

/// `/url\((.*?)\)/` (`tiktok.js:231`) — cover MusicalDown.
fn pola_url_css() -> &'static Regex {
    static P: OnceLock<Regex> = OnceLock::new();
    P.get_or_init(|| Regex::new(r"url\((.*?)\)").expect("pola url css"))
}

/// Serialisasi `params` axios (`lib/helpers/encode.js` di axios 1.x):
/// `encodeURIComponent` dengan tiga perubahan — spasi jadi `+`, dan
/// `!'()~` dibiarkan apa adanya.
///
/// BEDA dari `common::encode_uri_component` hanya pada spasi (`%20` vs `+`).
/// `:`/`,`/`$`/`[`/`]` TIDAK dikembalikan di axios 1.x (yang begitu axios 0.x),
/// jadi jangan ditambahkan. Salah pilih = query string berbeda dari JS.
fn encode_param_axios(s: &str) -> String {
    encode_uri_component(s).replace("%20", "+")
}

/// Serialisasi `new URLSearchParams(payload).toString()` (form-urlencoded).
///
/// BEDA dari `encode_uri_component`: spasi jadi `+` dan `!'()~` ikut di-encode;
/// yang dibiarkan hanya `*-._` dan alnum.
fn encode_form(s: &str) -> String {
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

/// `String(value)` JS untuk nilai JSON — dipakai di setiap penggabungan `+`.
fn string_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(items) => items
            .iter()
            .map(string_js)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `nilai || ""` — `0`, `""`, `false`, `null` semuanya jatuh ke `""`.
fn atau_kosong(nilai: Option<&Value>) -> String {
    match nilai {
        Some(v) if truthy(v) => string_js(v),
        _ => String::new(),
    }
}

/// `x == 0` versi JS (longgar). `null` / `undefined` TIDAK sama dengan nol.
fn sama_dengan_nol_loose(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => !*b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => {
            let s = s.trim();
            s.is_empty() || s.parse::<f64>().map(|n| n == 0.0).unwrap_or(false)
        }
        Some(Value::Array(a)) => a.is_empty(),
        Some(Value::Object(_)) => false,
    }
}

/// `json.URL[0] || json.URL` — indeks nol untuk array DAN string (karakter
/// pertama); jatuh ke nilai aslinya kalau indeksnya kosong atau falsy.
fn indeks_nol_atau_nilai(v: &Value) -> Value {
    let pertama: Option<Value> = match v {
        Value::Array(a) => a.first().cloned(),
        Value::String(s) => s.chars().next().map(|c| Value::String(c.to_string())),
        _ => None,
    };
    match pertama {
        Some(p) if truthy(&p) => p,
        _ => v.clone(),
    }
}

/// `res.author?.avatar ? "https://www.tikwm.com" + res.author.avatar : null`.
fn prefixed_atau_null(v: Option<&Value>) -> Value {
    match v {
        Some(v) if truthy(v) => Value::String(format!("https://www.tikwm.com{}", string_js(v))),
        _ => Value::Null,
    }
}

/// `downloads.push({ type, label, url })` — urutan kunci mengikuti JS.
fn unduhan(tipe: &str, label: &str, url: Value) -> Value {
    json!({ "type": tipe, "label": label, "url": url })
}

/// `resolveTikTokUrl(url)` (`tiktok.js:10-24`).
///
/// `res.request?.res?.responseUrl` tidak ada di `HttpResponse`, dan `catch {}`
/// menelan semua galat, jadi hasil port selalu `url` itu sendiri. Permintaan
/// tetap dikirim agar urutan jaringan sama dengan JS; balasan dibuang.
fn resolve_tiktok_url(http: &dyn HttpGet, url: &str) -> Result<String, HttpError> {
    if pola_pendek().is_match(url) {
        // Galat di sini ditelan `catch {}` JS — jangan diteruskan.
        let _ = http.call(&HttpCall::get(url).header("User-Agent", UA));
    }
    Ok(url.to_string())
}

/// `ttdownFromTikWM(url)` (`tiktok.js:26-66`).
pub fn ttdown_from_tikwm(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    // `params: { url, count: 12, cursor: 0, web: 1, hd: 1 }` — urutan kunci
    // mengikuti objek JS. `data: {}` jadi body `{}` walau content-type form.
    let alamat = format!(
        "https://www.tikwm.com/api/?url={}&count=12&cursor=0&web=1&hd=1",
        encode_param_axios(url)
    );
    let res = http.call(
        &HttpCall::post(&alamat)
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .header(
                "Content-Type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("Origin", "https://www.tikwm.com")
            .header("Referer", "https://www.tikwm.com/")
            .header("User-Agent", UA)
            .header("X-Requested-With", "XMLHttpRequest")
            .raw(
                b"{}".to_vec(),
                "application/x-www-form-urlencoded; charset=UTF-8",
            ),
    )?;
    // axios auto-parse JSON; kalau gagal, `data` tetap berupa string.
    let raw = res.json().unwrap_or_else(|| Value::String(res.text()));

    // `if (raw?.code !== undefined && raw.code !== 0) throw new Error(raw.msg || "TikWM error")`
    if let Some(kode) = raw.get("code")
        && !matches!(kode, Value::Number(n) if n.as_f64() == Some(0.0))
    {
        let pesan = match raw.get("msg") {
            Some(m) if truthy(m) => string_js(m),
            _ => "TikWM error".to_string(),
        };
        return Err(HttpError { message: pesan });
    }

    // `const res = raw?.data || raw`
    let data = match raw.get("data") {
        Some(d) if truthy(d) => d.clone(),
        _ => raw.clone(),
    };
    if !truthy(&data) {
        return Err(HttpError {
            message: "TikWM no data".to_string(),
        });
    }

    let mut downloads: Vec<Value> = Vec::new();
    if sama_dengan_nol_loose(data.get("duration"))
        && let Some(images) = data.get("images").and_then(Value::as_array)
    {
        for v in images {
            downloads.push(unduhan("photo", "Photo", Value::String(string_js(v))));
        }
    } else {
        for (tipe, label, kunci) in [
            ("nowatermark_hd", "Video HD", "hdplay"),
            ("nowatermark", "Video tanpa watermark", "play"),
            ("watermark", "Video watermark", "wmplay"),
        ] {
            if let Some(v) = data.get(kunci).filter(|v| truthy(v)) {
                downloads.push(unduhan(
                    tipe,
                    label,
                    Value::String(format!("https://www.tikwm.com{}", string_js(v))),
                ));
            }
        }
    }

    // `if (res.music) … else if (res.music_info?.play) …`
    match data.get("music") {
        Some(m) if truthy(m) => downloads.push(unduhan(
            "mp3",
            "Audio MP3",
            Value::String(format!("https://www.tikwm.com{}", string_js(m))),
        )),
        _ => {
            if let Some(p) = field(&data, "music_info.play").filter(|v| truthy(v)) {
                downloads.push(unduhan("mp3", "Audio MP3", p.clone()));
            }
        }
    }

    if downloads.is_empty() {
        return Err(HttpError {
            message: "TikWM tidak mengembalikan link download".to_string(),
        });
    }

    let mut penulis = Map::new();
    penulis.insert(
        "username".to_string(),
        Value::String(
            field(&data, "author.nickname")
                .filter(|v| truthy(v))
                .map(string_js)
                .unwrap_or_else(|| {
                    field(&data, "author.unique_id")
                        .filter(|v| truthy(v))
                        .map(string_js)
                        .unwrap_or_default()
                }),
        ),
    );
    penulis.insert(
        "avatar".to_string(),
        prefixed_atau_null(field(&data, "author.avatar")),
    );

    let mut out = Map::new();
    out.insert("title".to_string(), Value::String(atau_kosong(data.get("title"))));
    out.insert("author".to_string(), Value::Object(penulis));
    out.insert("cover".to_string(), prefixed_atau_null(data.get("cover")));
    out.insert("downloads".to_string(), Value::Array(downloads));
    Ok(Value::Object(out))
}

/// `getSaveTTToken()` (`tiktok.js:76-85`).
fn get_token_savett(http: &dyn HttpGet) -> Result<(String, String), HttpError> {
    let res = http.call(
        &HttpCall::get("https://savett.cc/en1/download").header("User-Agent", UA_SAVETT),
    )?;
    let html = res.text();
    let csrf = pola_csrf()
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_default();
    // `(res.headers["set-cookie"] || []).map(v => v.split(";")[0]).join("; ")`
    let cookie = res
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.split(';').next().unwrap_or("").to_string())
        .collect::<Vec<_>>()
        .join("; ");
    if csrf.is_empty() || cookie.is_empty() {
        return Err(HttpError {
            message: "Gagal mendapatkan token saveTT".to_string(),
        });
    }
    Ok((csrf, cookie))
}

/// `#formatselect option` — `option` yang punya leluhur `#formatselect`.
fn opsi_formatselect(akar: &Elemen) -> Vec<&Elemen> {
    semua_dalam_berlapis(
        akar,
        &|el| atribut(el, "id").as_deref() == Some("formatselect"),
        &|el| el.tag == "option",
    )
}

/// `ttdownFromSaveTT(url)` (`tiktok.js:87-135`).
pub fn ttdown_from_save_tt(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let (csrf, cookie) = get_token_savett(http)?;
    let body = format!(
        "csrf_token={}&url={}",
        encode_uri_component(&csrf),
        encode_uri_component(url)
    );
    let res = http.call(
        &HttpCall::post("https://savett.cc/en1/download")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Origin", "https://savett.cc")
            .header("Referer", "https://savett.cc/en1/download")
            .header("User-Agent", UA_SAVETT)
            .header("Cookie", &cookie)
            .form(body),
    )?;

    let akar = urai(&res.text());
    let mut downloads: Vec<Value> = Vec::new();

    // `$("#video-info h3").first().text().trim() || ""`
    let judul = semua_dalam_berlapis(
        &akar,
        &|el| atribut(el, "id").as_deref() == Some("video-info"),
        &|el| el.tag == "h3",
    )
    .first()
    .map(|el| teks_dari(el).trim().to_string())
    .unwrap_or_default();

    // `$(".carousel-item[data-data]")` — punya kelas DAN punya atribut itu.
    let slides = semua_dalam(&akar, &|el| {
        punya_kelas(el, "carousel-item") && atribut(el, "data-data").is_some()
    });

    if !slides.is_empty() {
        for el in &slides {
            let Some(isi) = atribut(el, "data-data") else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<Value>(&isi.replace("&quot;", "\"")) else {
                continue;
            };
            let Some(daftar) = json.get("URL").and_then(Value::as_array) else {
                continue;
            };
            if daftar.is_empty() {
                continue;
            }
            // `[...json.URL].sort((a, b) => String(b).length - String(a).length)[0]`
            // — `Array#sort` V8 stabil, `sort_by` juga stabil, jadi sama.
            let mut urut: Vec<&Value> = daftar.iter().collect();
            urut.sort_by_key(|v| std::cmp::Reverse(string_js(v).len()));
            downloads.push(unduhan("photo", "Photo", Value::String(string_js(urut[0]))));
        }

        for el in opsi_formatselect(&akar) {
            if !teks_dari(el).to_lowercase().contains("mp3") {
                continue;
            }
            let Some(nilai) = atribut(el, "value") else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<Value>(&nilai.replace("&quot;", "\"")) else {
                continue;
            };
            if let Some(u) = json.get("URL").filter(|u| truthy(u)) {
                downloads.push(unduhan("mp3", "Audio MP3", indeks_nol_atau_nilai(u)));
            }
        }

        if downloads.is_empty() {
            return Err(HttpError {
                message: "SaveTT photo no downloads".to_string(),
            });
        }
        return Ok(bentuk_savett(judul, downloads));
    }

    for el in opsi_formatselect(&akar) {
        let label_kecil = teks_dari(el).to_lowercase();
        // `if (!raw) return` — atribut kosong dianggap tidak ada.
        let Some(nilai) = atribut(el, "value").filter(|v| !v.is_empty()) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<Value>(&nilai.replace("&quot;", "\"")) else {
            continue;
        };
        // `if (!json.URL) return`, lalu `json.URL.map(...)` — `map` hanya ada di
        // array. Kalau `URL` berupa string, di JS itu `TypeError` yang
        // tertangkap `catch {}`, jadi hasilnya sama-sama dilewati.
        let Some(daftar) = json.get("URL").and_then(Value::as_array) else {
            continue;
        };
        let tipe = if label_kecil.contains("mp4") && !label_kecil.contains("watermark") {
            "nowatermark"
        } else if label_kecil.contains("watermark") {
            "watermark"
        } else if label_kecil.contains("mp3") {
            "mp3"
        } else {
            continue;
        };
        let label = teks_dari(el).trim().to_string();
        for u in daftar {
            downloads.push(unduhan(tipe, &label, u.clone()));
        }
    }

    if downloads.is_empty() {
        return Err(HttpError {
            message: "SaveTT tidak mengembalikan link download".to_string(),
        });
    }
    Ok(bentuk_savett(judul, downloads))
}

/// Bentuk hasil SaveTT: `avatar` dan `cover` selalu `null` (`tiktok.js:119,134`).
fn bentuk_savett(judul: String, downloads: Vec<Value>) -> Value {
    json!({
        "title": judul,
        "author": { "username": judul, "avatar": Value::Null },
        "cover": Value::Null,
        "downloads": downloads,
    })
}

/// `ttdownFromYuuLabs(url)` (`tiktok.js:137-187`).
pub fn ttdown_from_yuu_labs(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let alamat = format!("{YUULABS_API}{}", encode_uri_component(url));
    let res = http.call(&HttpCall::get(&alamat).header("user-agent", UA))?;
    let data = res.json().unwrap_or_else(|| Value::String(res.text()));

    // `if (!data?.status || !data?.result) throw new Error(data?.message || …)`
    let status_ada = data.get("status").map(truthy).unwrap_or(false);
    let result_ada = data.get("result").map(truthy).unwrap_or(false);
    if !status_ada || !result_ada {
        let pesan = match data.get("message") {
            Some(m) if truthy(m) => string_js(m),
            _ => "YuuLabs response invalid".to_string(),
        };
        return Err(HttpError { message: pesan });
    }
    let result = data.get("result").cloned().unwrap_or(Value::Null);

    let mut downloads = Vec::new();
    for (tipe, label, kunci) in [
        ("nowatermark", "Video tanpa watermark", "videoUrl"),
        ("nowatermark_hd", "Video HD", "hdVideo"),
        ("mp3", "Audio MP3", "audioUrl"),
    ] {
        if let Some(v) = result.get(kunci).filter(|v| truthy(v)) {
            downloads.push(unduhan(tipe, label, v.clone()));
        }
    }
    if downloads.is_empty() {
        return Err(HttpError {
            message: "YuuLabs tidak mengembalikan link download".to_string(),
        });
    }

    Ok(json!({
        "title": atau_kosong(result.get("description")),
        "author": { "username": atau_kosong(result.get("author")), "avatar": Value::Null },
        "cover": Value::Null,
        "downloads": downloads,
    }))
}

/// `ttdownFromMusicalDown(url)` (`tiktok.js:189-260`).
pub fn ttdown_from_musical_down(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let res = http.call(&HttpCall::get("https://musicaldown.com/en").header("user-agent", UA))?;
    let akar = urai(&res.text());

    // `payload[name] = value || ""`. Urutan objek JS dipakai
    // `Object.keys(...).find(...)` di bawah, jadi Map (preserve_order).
    let mut payload = Map::new();
    for el in semua_dalam_berlapis(
        &akar,
        &|el| atribut(el, "id").as_deref() == Some("submit-form"),
        &|el| el.tag == "input",
    ) {
        if let Some(name) = atribut(el, "name").filter(|v| !v.is_empty()) {
            let nilai = atribut(el, "value")
                .filter(|v| truthy(&Value::String(v.clone())))
                .unwrap_or_default();
            payload.insert(name, Value::String(nilai));
        }
    }

    // `Object.keys(payload).find(key => !payload[key])` — kunci PERTAMA yang
    // nilainya falsy, lalu diisi `url`.
    if let Some(kunci) = payload
        .iter()
        .find(|(_, v)| !truthy(v))
        .map(|(k, _)| k.clone())
    {
        payload.insert(kunci, Value::String(url.to_string()));
    }

    let body = payload
        .iter()
        .map(|(k, v)| format!("{}={}", encode_form(k), encode_form(&string_js(v))))
        .collect::<Vec<_>>()
        .join("&");

    // `Array.isArray(headers["set-cookie"]) ? join("; ") : ""` — di sini
    // nilai cookie UTUH (tidak dipotong di `;`), berbeda dari
    // `getSaveTTToken`.
    let cookie = res
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .map(|(_, v)| v.clone())
        .collect::<Vec<_>>()
        .join("; ");

    let res = http.call(
        &HttpCall::post("https://musicaldown.com/download")
            .header("user-agent", UA)
            .header(
                "content-type",
                "application/x-www-form-urlencoded; charset=UTF-8",
            )
            .header("Cookie", &cookie)
            .header("Origin", "https://musicaldown.com")
            .header("Referer", "https://musicaldown.com/")
            .form(body),
    )?;

    let akar = urai(&res.text());

    // `bgImage?.match(/url\((.*?)\)/)` dari `style` elemen `.video-header`
    // PERTAMA; tidak ada match berarti `null`.
    let cover = semua_dalam(&akar, &|el| punya_kelas(el, "video-header"))
        .first()
        .and_then(|el| atribut(el, "style"))
        .and_then(|s| {
            pola_url_css()
                .captures(&s)
                .and_then(|c| c.get(1))
                .map(|m| Value::String(m.as_str().to_string()))
        })
        .unwrap_or(Value::Null);

    let mut downloads = Vec::new();
    for el in semua_dalam(&akar, &|el| el.tag == "a" && punya_kelas(el, "download")) {
        // `$elem.data("event")` membaca atribut `data-event`, dan
        // `.replace("_download_click", "")` hanya mengganti kemunculan PERTAMA.
        let tipe = atribut(el, "data-event")
            .map(|v| Value::String(v.replacen("_download_click", "", 1)));
        let label = teks_dari(el).trim().to_string();
        let Some(url_unduh) = atribut(el, "href").filter(|v| !v.is_empty()) else {
            continue;
        };
        let mut item = Map::new();
        // `type: undefined` hilang di `JSON.stringify` — hanya disisipkan kalau
        // atributnya benar-benar ada.
        if let Some(t) = tipe {
            item.insert("type".to_string(), t);
        }
        item.insert("label".to_string(), Value::String(label));
        item.insert("url".to_string(), Value::String(url_unduh));
        downloads.push(Value::Object(item));
    }

    if downloads.is_empty() {
        return Err(HttpError {
            message: "MusicalDown tidak mengembalikan link download".to_string(),
        });
    }

    // `$$(".video-desc").text()` — teks gabungan SEMUA elemen yang cocok.
    let judul = teks_semua(&semua_dalam(&akar, &|el| punya_kelas(el, "video-desc")));
    let penulis = teks_semua(&semua_dalam_berlapis(
        &akar,
        &|el| punya_kelas(el, "video-author"),
        &|el| el.tag == "b",
    ));
    // `.attr("src")` pada koleksi = elemen PERTAMA; `undefined` tidak ikut di
    // `JSON.stringify`.
    let avatar = semua_dalam_berlapis(
        &akar,
        &|el| punya_kelas(el, "img-area"),
        &|el| el.tag == "img",
    )
    .first()
    .and_then(|el| atribut(el, "src"))
    .map(Value::String);

    let mut obj_author = Map::new();
    obj_author.insert("username".to_string(), Value::String(penulis));
    if let Some(a) = avatar {
        obj_author.insert("avatar".to_string(), a);
    }

    Ok(json!({
        "title": judul,
        "author": Value::Object(obj_author),
        "cover": cover,
        "downloads": downloads,
    }))
}

/// Gabungan `.text()` dari koleksi elemen, lalu `.trim()`.
fn teks_semua(elemen: &[&Elemen]) -> String {
    let gabung = elemen
        .iter()
        .map(|el| teks_dari(el))
        .collect::<Vec<_>>()
        .join("");
    gabung.trim().to_string()
}

/// `ttdown(url)` (`tiktok.js:262-302`) — orkestrator berurutan + gabungan galat.
pub fn ttdown(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    if !url.contains("tiktok.com") {
        return Err(HttpError {
            message: "Invalid url.".to_string(),
        });
    }
    let resolved = resolve_tiktok_url(http, url)?;

    // `[url, resolved].filter((u, i, arr) => u && arr.indexOf(u) === i)`
    let mut varian: Vec<String> = Vec::new();
    for u in [url.to_string(), resolved.clone()] {
        if !u.is_empty() && !varian.contains(&u) {
            varian.push(u);
        }
    }

    let mut errors: Vec<String> = Vec::new();

    match ttdown_from_tikwm(http, &resolved) {
        Ok(hasil) => return Ok(hasil),
        Err(e) => errors.push(format!("tikwm: {}", e.message)),
    }

    for v in &varian {
        match ttdown_from_save_tt(http, v) {
            Ok(hasil) => {
                if unduhan_ada(&hasil) {
                    return Ok(hasil);
                }
            }
            Err(e) => {
                let nama = if v == url { "raw" } else { "resolved" };
                errors.push(format!("savett({nama}): {}", e.message));
            }
        }
    }

    // JS menjalankan fallback satu per satu dan BERHENTI di provider pertama
    // yang sukses — jadi tidak boleh dihitung semua di awal.
    for nama in ["yuulabs", "musicaldown"] {
        let hasil = match nama {
            "yuulabs" => ttdown_from_yuu_labs(http, &resolved),
            _ => ttdown_from_musical_down(http, &resolved),
        };
        match hasil {
            Ok(hasil) => {
                if unduhan_ada(&hasil) {
                    return Ok(hasil);
                }
            }
            Err(e) => errors.push(format!("{nama}: {}", e.message)),
        }
    }

    let pesan = if errors.is_empty() {
        "Semua provider TikTok gagal".to_string()
    } else {
        errors.join(" | ")
    };
    Err(HttpError { message: pesan })
}

/// `res?.downloads?.length` — `true` hanya kalau arraynya ada dan tidak kosong.
fn unduhan_ada(hasil: &Value) -> bool {
    hasil
        .get("downloads")
        .and_then(Value::as_array)
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Pengurai HTML mini (pengganti `cheerio` untuk selector yang dipakai file ini)
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Elemen {
    tag: String,
    attrs: Vec<(String, String)>,
    anak: Vec<Simpul>,
}

#[derive(Debug)]
enum Simpul {
    Teks(String),
    Elemen(Elemen),
}

/// Tag HTML tanpa tag penutup.
const VOID: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Isi tag ini tidak diurai sebagai HTML (isi `<script>` bisa penuh `<`).
const RAW: [&str; 2] = ["script", "style"];

/// Akhir tag (`>`) dari indeks `<`, menghormati tanda kutip nilai atribut.
fn akhir_tag(html: &str, mulai: usize) -> Option<usize> {
    let b = html.as_bytes();
    let mut i = mulai + 1;
    let mut kutip: Option<u8> = None;
    while i < b.len() {
        let c = b[i];
        match kutip {
            Some(q) => {
                if c == q {
                    kutip = None;
                }
            }
            None => match c {
                b'"' | b'\'' => kutip = Some(c),
                b'>' => return Some(i),
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Nama tag dari potongan setelah `<` atau `</`, huruf kecil.
fn nama_tag(potongan: &str) -> String {
    potongan
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ':')
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Urai `<tag attr="nilai" …>` jadi nama tag + atribut (nama huruf kecil).
fn urai_tag(potongan: &str) -> (String, Vec<(String, String)>) {
    let b = potongan.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] as char).is_ascii_whitespace() {
        i += 1;
    }
    if i < b.len() && b[i] == b'<' {
        i += 1;
    }
    let mulai_nama = i;
    while i < b.len() && !(b[i] as char).is_ascii_whitespace() && b[i] != b'/' && b[i] != b'>' {
        i += 1;
    }
    let tag = potongan[mulai_nama..i].to_ascii_lowercase();

    let mut attrs = Vec::new();
    loop {
        while i < b.len() && (b[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() || b[i] == b'>' || b[i] == b'/' {
            break;
        }
        let mulai = i;
        while i < b.len()
            && !(b[i] as char).is_ascii_whitespace()
            && b[i] != b'='
            && b[i] != b'>'
            && b[i] != b'/'
        {
            i += 1;
        }
        let nama = potongan[mulai..i].to_ascii_lowercase();
        while i < b.len() && (b[i] as char).is_ascii_whitespace() {
            i += 1;
        }
        let mut nilai = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && (b[i] as char).is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let kutip = b[i];
                i += 1;
                let mulai_nilai = i;
                while i < b.len() && b[i] != kutip {
                    i += 1;
                }
                nilai = potongan[mulai_nilai..i].to_string();
                if i < b.len() {
                    i += 1;
                }
            } else {
                let mulai_nilai = i;
                while i < b.len() && !(b[i] as char).is_ascii_whitespace() && b[i] != b'>' {
                    i += 1;
                }
                nilai = potongan[mulai_nilai..i].to_string();
            }
        }
        if !nama.is_empty() {
            attrs.push((nama, nilai));
        }
    }
    (tag, attrs)
}

/// Cari tag penutup `</tag` (tanpa peka huruf besar-kecil).
fn cari_penutup(sisa: &str, tag: &str) -> Option<(usize, usize)> {
    let rendah = sisa.to_ascii_lowercase();
    let pola = format!("</{tag}");
    let mut dari = 0;
    while let Some(p) = rendah[dari..].find(&pola) {
        let mulai = dari + p;
        let sisa_teks = &rendah[mulai + pola.len()..];
        let batas = sisa_teks
            .chars()
            .next()
            .map(|c| c.is_ascii_whitespace() || c == '>' || c == '/')
            .unwrap_or(true);
        if batas {
            let pos = sisa[mulai..].find('>')?;
            return Some((mulai, mulai + pos + 1));
        }
        dari = mulai + pola.len();
    }
    None
}

/// Urai HTML jadi pohon sederhana; komentar dan `<!DOCTYPE>` dibuang.
fn urai(html: &str) -> Elemen {
    let mut tumpukan: Vec<Elemen> = vec![Elemen::default()];
    let mut i = 0;
    while i < html.len() {
        if html.as_bytes()[i] != b'<' {
            let akhir = html[i..].find('<').map(|p| i + p).unwrap_or(html.len());
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Teks(html[i..akhir].to_string()));
            }
            i = akhir;
            continue;
        }

        if html[i..].starts_with("<!--") {
            match html[i..].find("-->") {
                Some(p) => {
                    i += p + 3;
                    continue;
                }
                None => break,
            }
        }
        if html[i..].starts_with("<!") || html[i..].starts_with("<?") {
            match html[i..].find('>') {
                Some(p) => {
                    i += p + 1;
                    continue;
                }
                None => break,
            }
        }

        if html[i..].starts_with("</") {
            let Some(pos) = html[i..].find('>') else {
                break;
            };
            let nama = nama_tag(&html[i + 2..i + pos]);
            if !nama.is_empty()
                && let Some(idx) = tumpukan.iter().rposition(|el| el.tag == nama)
            {
                while tumpukan.len() > idx + 1 {
                    let anak = tumpukan.pop().expect("lebih dari satu frame");
                    if let Some(induk) = tumpukan.last_mut() {
                        induk.anak.push(Simpul::Elemen(anak));
                    }
                }
                let anak = tumpukan.pop().expect("frame yang cocok");
                if let Some(induk) = tumpukan.last_mut() {
                    induk.anak.push(Simpul::Elemen(anak));
                }
            }
            // `pos` di sini relatif terhadap `i` (dari `html[i..].find('>')`), jadi
            // majunya harus `i += pos + 1`. Kalau `i = pos + 1`, penanda
            // `</form>` mundur ke awal dokumen: parser berputar tanpa henti
            // sambil menumpuk teks sampai proses dibunuh OOM.
            i += pos + 1;
            continue;
        }

        // `<` yang tidak diikuti huruf dianggap teks biasa, seperti htmlparser2.
        if !html[i + 1..]
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
        {
            let akhir = html[i + 1..]
                .find('<')
                .map(|p| i + 1 + p)
                .unwrap_or(html.len());
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Teks(html[i..akhir].to_string()));
            }
            i = akhir;
            continue;
        }

        let Some(pos) = akhir_tag(html, i) else {
            break;
        };
        let buka = &html[i..=pos];
        let (tag, attrs) = urai_tag(buka);
        if tag.is_empty() {
            i = pos + 1;
            continue;
        }

        if RAW.contains(&tag.as_str()) {
            let sisa = &html[pos + 1..];
            let (isi, lewat) = match cari_penutup(sisa, &tag) {
                Some((mulai, lewat)) => (&sisa[..mulai], lewat),
                None => (sisa, sisa.len()),
            };
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Elemen(Elemen {
                    tag,
                    attrs,
                    anak: vec![Simpul::Teks(isi.to_string())],
                }));
            }
            i = pos + 1 + lewat;
            continue;
        }

        let tutup_sendiri = buka.trim_end().ends_with("/>") || VOID.contains(&tag.as_str());
        let el = Elemen {
            tag,
            attrs,
            anak: Vec::new(),
        };
        if tutup_sendiri {
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Elemen(el));
            }
        } else {
            tumpukan.push(el);
        }
        i = pos + 1;
    }

    while tumpukan.len() > 1 {
        let anak = tumpukan.pop().expect("lebih dari satu frame");
        if let Some(induk) = tumpukan.last_mut() {
            induk.anak.push(Simpul::Elemen(anak));
        }
    }
    tumpukan.pop().expect("akar")
}

/// `$(el).attr(nama)` — atribut pertama yang cocok, entitas di-decode.
fn atribut(el: &Elemen, nama: &str) -> Option<String> {
    el.attrs
        .iter()
        .find(|(k, _)| k == nama)
        .map(|(_, v)| decode_entitas(v))
}

/// Elemen punya kelas itu.
fn punya_kelas(el: &Elemen, kelas: &str) -> bool {
    el.attrs
        .iter()
        .filter(|(k, _)| k == "class")
        .flat_map(|(_, v)| v.split_whitespace())
        .any(|c| c == kelas)
}

/// `$(el).text()` — teks semua keturunan, entitas di-decode.
fn teks_dari(el: &Elemen) -> String {
    let mut out = String::new();
    for simpul in &el.anak {
        match simpul {
            Simpul::Teks(t) => out.push_str(&decode_entitas(t)),
            Simpul::Elemen(e) => out.push_str(&teks_dari(e)),
        }
    }
    out
}

/// Jelajah seluruh keturunan. `leluhur` berarti elemen sekarang sudah berada
/// di bawah pematch `induk` (selector `A B`), `cocok` yang dicari.
///
/// Elemen yang punya dua leluhur cocok tetap muncul SEKALI — `cheerio`
/// mengembalikan himpunan elemen unik, bukan satu entri per jalur.
fn jelajah<'a>(
    el: &'a Elemen,
    induk: &dyn Fn(&Elemen) -> bool,
    cocok: &dyn Fn(&Elemen) -> bool,
    leluhur: bool,
    keluar: &mut Vec<&'a Elemen>,
) {
    let di_bawah = leluhur || induk(el);
    for simpul in &el.anak {
        if let Simpul::Elemen(e) = simpul
            && di_bawah
            && cocok(e)
            && !keluar.iter().any(|s| std::ptr::eq(*s, e))
        {
            keluar.push(e);
        }
    }
    for simpul in &el.anak {
        if let Simpul::Elemen(e) = simpul {
            jelajah(e, induk, cocok, di_bawah, keluar);
        }
    }
}

/// `$(selector)` — semua keturunan yang cocok, urutan dokumen.
fn semua_dalam<'a>(akar: &'a Elemen, cocok: &dyn Fn(&Elemen) -> bool) -> Vec<&'a Elemen> {
    let mut keluar = Vec::new();
    jelajah(akar, &|_| true, cocok, true, &mut keluar);
    keluar
}

/// `$(induk keturunan)` — keturunan yang cocok DAN punya leluhur yang cocok.
fn semua_dalam_berlapis<'a>(
    akar: &'a Elemen,
    induk: &dyn Fn(&Elemen) -> bool,
    cocok: &dyn Fn(&Elemen) -> bool,
) -> Vec<&'a Elemen> {
    let mut keluar = Vec::new();
    jelajah(akar, induk, cocok, induk(akar), &mut keluar);
    keluar
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpMethod, HttpResponse};
    use std::sync::Mutex;

    /// `(status, body, headers)` untuk stub HTTP — alias biar tipe
    /// balancer-nya tidak memicu `clippy::type_complexity`.
    type Balasan = (u16, String, Vec<(String, String)>);

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body, headers) berurutan; setelah habis, balasan terakhir
    /// dipakai lagi.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<Balasan>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<Balasan>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }

        /// Satu respons tanpa header.
        fn tunggal(body: &str) -> Self {
            Self::baru(vec![(200, body.to_string(), Vec::new())])
        }

        fn dengan_headers(body: &str, headers: Vec<(&str, &str)>) -> Self {
            Self::baru(vec![
                (
                    200,
                    body.to_string(),
                    headers
                        .into_iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                ),
            ])
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua permintaan lewat HttpCall::call")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("semua permintaan lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua permintaan lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("tiktok tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            let mut hitung = self.hitung.lock().unwrap();
            let balasan = self.balasan.lock().unwrap();
            let i = (*hitung).min(balasan.len().saturating_sub(1));
            *hitung += 1;
            let (status, teks, headers) = balasan[i].clone();
            Ok(HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: std::sync::Arc::from(teks.into_bytes()),
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
    fn tikwm_video_menghasilkan_tiga_varian_plus_mp3() {
        let http = StubHttp::tunggal(
            r#"{"code":0,"data":{"title":"Judul","duration":30,"hdplay":"/hd.mp4",
                "play":"/sd.mp4","wmplay":"/wm.mp4","cover":"/cover.jpg",
                "music":"/music.mp3",
                "author":{"nickname":"Tok","avatar":"/a.jpg"}}}"#,
        );
        let hasil = ttdown_from_tikwm(&http, "https://vt.tiktok.com/abc").unwrap();
        assert_eq!(hasil["title"], "Judul");
        assert_eq!(hasil["author"]["username"], "Tok");
        assert_eq!(hasil["author"]["avatar"], "https://www.tikwm.com/a.jpg");
        assert_eq!(hasil["cover"], "https://www.tikwm.com/cover.jpg");
        let d = hasil["downloads"].as_array().unwrap();
        assert_eq!(d.len(), 4);
        assert_eq!(d[0]["type"], "nowatermark_hd");
        assert_eq!(d[0]["url"], "https://www.tikwm.com/hd.mp4");
        assert_eq!(d[1]["type"], "nowatermark");
        assert_eq!(d[2]["type"], "watermark");
        assert_eq!(d[3]["type"], "mp3");

        // Permintaan: POST ke URL ber-parameter, body `{}`, header khas JS.
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, HttpMethod::Post);
        assert_eq!(
            p[0].url,
            "https://www.tikwm.com/api/?url=https%3A%2F%2Fvt.tiktok.com%2Fabc&count=12&cursor=0&web=1&hd=1"
        );
        assert_eq!(
            p[0].body,
            Some(HttpBody::Bytes {
                data: std::sync::Arc::from(&b"{}"[..]),
                mimetype: Some("application/x-www-form-urlencoded; charset=UTF-8".to_string()),
            })
        );
        assert_eq!(
            cari_header(&p[0], "x-requested-with").as_deref(),
            Some("XMLHttpRequest")
        );
        assert_eq!(
            cari_header(&p[0], "origin").as_deref(),
            Some("https://www.tikwm.com")
        );
        assert_eq!(
            cari_header(&p[0], "user-agent").as_deref(),
            Some(UA),
            "header harus persis sama dengan REQUEST_HEADERS"
        );
    }

    #[test]
    fn tikwm_photo_menggunakan_images_karena_duration_nol() {
        // `duration == 0` longgar: string "0" juga kena.
        let http = StubHttp::tunggal(
            r#"{"code":0,"data":{"duration":"0","images":["https://a/1.jpg",2],
                "music_info":{"play":"https://a/m.mp3"}}}"#,
        );
        let hasil = ttdown_from_tikwm(&http, "https://vt.tiktok.com/x").unwrap();
        let d = hasil["downloads"].as_array().unwrap();
        // Cabang foto menambah 2 gambar; `res.music` kosong lalu
        // `res.music_info?.play` kena, jadi 1 mp3 lagi (tiktok.js:57-58).
        assert_eq!(d.len(), 3);
        assert_eq!(d[0]["type"], "photo");
        assert_eq!(d[0]["url"], "https://a/1.jpg");
        assert_eq!(d[1]["url"], "2", "String(2) = \"2\"");
        assert_eq!(d[2]["type"], "mp3");
        assert_eq!(d[2]["url"], "https://a/m.mp3", "tanpa prefix tikwm");
        assert_eq!(hasil["author"]["username"], "");
        assert_eq!(hasil["cover"], Value::Null);
    }

    #[test]
    fn tikwm_kode_nol_string_tetap_diperlakukan_gagal() {
        // `raw.code !== 0` — strict, jadi "0" bukan 0.
        let http = StubHttp::tunggal(r#"{"code":"0","msg":"ditolak"}"#);
        assert_eq!(
            ttdown_from_tikwm(&http, "https://x").unwrap_err().message,
            "ditolak"
        );
        let http = StubHttp::tunggal(r#"{"code":9}"#);
        assert_eq!(
            ttdown_from_tikwm(&http, "https://x").unwrap_err().message,
            "TikWM error"
        );
    }

    #[test]
    fn tikwm_tanpa_link_melempar_pesan_yang_panjang_persis() {
        let http = StubHttp::tunggal(r#"{"code":0,"data":{"title":"x"}}"#);
        assert_eq!(
            ttdown_from_tikwm(&http, "https://x").unwrap_err().message,
            "TikWM tidak mengembalikan link download"
        );
        // Body bukan JSON tidak melempar: `transformResponse` axios diamkan
        // kegagalannya kalau `responseType` bukan "json", jadi `raw` tetap
        // string dan `raw?.data || raw` jadi string itu sendiri (truthy).
        let http = StubHttp::tunggal("bukan json");
        assert_eq!(
            ttdown_from_tikwm(&http, "https://x").unwrap_err().message,
            "TikWM tidak mengembalikan link download"
        );
        // `raw` benar-benar falsy baru masuk cabang "no data" (tiktok.js:47-48).
        let http = StubHttp::tunggal("null");
        assert_eq!(
            ttdown_from_tikwm(&http, "https://x").unwrap_err().message,
            "TikWM no data"
        );
    }

    #[test]
    fn savett_foto_membaca_data_data_dan_opsi_mp3() {
        let html = r#"
            <div id="video-info"><h3>  Judul Video  </h3></div>
            <div class="carousel-item" data-data="{&quot;URL&quot;:[&quot;https://p/pendek.jpg&quot;,&quot;https://p/panjang-lebih.jpg&quot;]}"></div>
            <div class="carousel-item" data-data="{&quot;URL&quot;:[]}"></div>
            <div class="carousel-item" data-data="bukan json"></div>
            <select id="formatselect">
              <option value="{&quot;URL&quot;:[&quot;https://a/1.mp3&quot;,&quot;https://a/2.mp3&quot;]}">MP3 128kbps</option>
              <option value="rusak">MP3 rusak</option>
            </select>
        "#;
        let http = StubHttp::baru(vec![
            (
                200,
                r#"<input name="csrf_token" value="TOKENSAYA">"#.to_string(),
                vec![
                    ("set-cookie".to_string(), "a=1; Path=/".to_string()),
                    ("set-cookie".to_string(), "b=2; HttpOnly".to_string()),
                ],
            ),
            (200, html.to_string(), Vec::new()),
        ]);
        let hasil = ttdown_from_save_tt(&http, "https://vt.tiktok.com/abc").unwrap();
        assert_eq!(hasil["title"], "Judul Video");
        assert_eq!(hasil["author"]["username"], "Judul Video");
        assert_eq!(hasil["author"]["avatar"], Value::Null);
        assert_eq!(hasil["cover"], Value::Null);
        let d = hasil["downloads"].as_array().unwrap();
        assert_eq!(d.len(), 2, "URL kosong dan JSON rusak dilewati");
        // Sort by panjang string: yang paling panjang menang.
        assert_eq!(d[0]["type"], "photo");
        assert_eq!(d[0]["url"], "https://p/panjang-lebih.jpg");
        assert_eq!(d[1]["type"], "mp3");
        assert_eq!(d[1]["url"], "https://a/1.mp3");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].method, HttpMethod::Get);
        assert_eq!(p[0].url, "https://savett.cc/en1/download");
        assert_eq!(
            p[1].body,
            Some(HttpBody::Form(
                "csrf_token=TOKENSAYA&url=https%3A%2F%2Fvt.tiktok.com%2Fabc".to_string()
            ))
        );
        // `set-cookie` dipotong di `;` lalu disambung `; `.
        assert_eq!(cari_header(&p[1], "cookie").as_deref(), Some("a=1; b=2"));
    }

    #[test]
    fn savett_video_membaca_format_tanpa_carousel() {
        let html = r#"
            <select id="formatselect">
              <option value="{&quot;URL&quot;:[&quot;https://v/720.mp4&quot;,&quot;https://v/1080.mp4&quot;]}">MP4 1080p</option>
              <option value="{&quot;URL&quot;:[&quot;https://v/wm.mp4&quot;]}">MP4 Watermark</option>
              <option value="{&quot;URL&quot;:[&quot;https://v/a.mp3&quot;]}">MP3 Audio</option>
              <option value="{&quot;URL&quot;:[&quot;https://v/webm&quot;]}">WEBM lain</option>
              <option value="{&quot;URL&quot;:&quot;https://v/bukan-array&quot;}">MP4 string</option>
            </select>
        "#;
        let http = StubHttp::baru(vec![
            (
                200,
                r#"name="csrf_token" value="T""#.to_string(),
                vec![("set-cookie".to_string(), "s=1".to_string())],
            ),
            (200, html.to_string(), Vec::new()),
        ]);
        let hasil = ttdown_from_save_tt(&http, "https://x").unwrap();
        let d = hasil["downloads"].as_array().unwrap();
        // 2 (mp4) + 1 (watermark) + 1 (mp3); WEBM dan `URL` string dilewati.
        assert_eq!(d.len(), 4);
        assert_eq!(d[0]["type"], "nowatermark");
        assert_eq!(d[0]["label"], "MP4 1080p");
        assert_eq!(d[1]["url"], "https://v/1080.mp4");
        assert_eq!(d[2]["type"], "watermark");
        assert_eq!(d[3]["type"], "mp3");
        assert_eq!(d[3]["url"], "https://v/a.mp3");
    }

    #[test]
    fn savett_tanpa_token_atau_cookie_melempar_pesan_yang_sama() {
        let http = StubHttp::dengan_headers(r#"name="csrf_token" value="T""#, vec![]);
        assert_eq!(
            ttdown_from_save_tt(&http, "https://x").unwrap_err().message,
            "Gagal mendapatkan token saveTT"
        );
        let http = StubHttp::dengan_headers("<p>tanpa form</p>", vec![("set-cookie", "s=1")]);
        assert_eq!(
            ttdown_from_save_tt(&http, "https://x").unwrap_err().message,
            "Gagal mendapatkan token saveTT"
        );
    }

    #[test]
    fn savett_tanpa_unduhan_melempar_pesan_per_kasus() {
        let http = StubHttp::baru(vec![
            (
                200,
                r#"name="csrf_token" value="T""#.to_string(),
                vec![("set-cookie".to_string(), "s=1".to_string())],
            ),
            (
                200,
                "<div class=\"carousel-item\" data-data=\"{}\"></div>".to_string(),
                Vec::new(),
            ),
        ]);
        assert_eq!(
            ttdown_from_save_tt(&http, "https://x").unwrap_err().message,
            "SaveTT photo no downloads"
        );

        let http = StubHttp::baru(vec![
            (
                200,
                r#"name="csrf_token" value="T""#.to_string(),
                vec![("set-cookie".to_string(), "s=1".to_string())],
            ),
            (
                200,
                "<select id=\"formatselect\"></select>".to_string(),
                Vec::new(),
            ),
        ]);
        assert_eq!(
            ttdown_from_save_tt(&http, "https://x").unwrap_err().message,
            "SaveTT tidak mengembalikan link download"
        );
    }

    #[test]
    fn yukulabs_menolak_status_false_dengan_pesan_data() {
        let http = StubHttp::tunggal(r#"{"status":false,"message":"limit"}"#);
        assert_eq!(
            ttdown_from_yuu_labs(&http, "https://x").unwrap_err().message,
            "limit"
        );
        let http = StubHttp::tunggal(r#"{"status":true}"#);
        assert_eq!(
            ttdown_from_yuu_labs(&http, "https://x").unwrap_err().message,
            "YuuLabs response invalid"
        );
        // Body bukan JSON → `data` string → `status` undefined → teks bawaan.
        let http = StubHttp::tunggal("html");
        assert_eq!(
            ttdown_from_yuu_labs(&http, "https://x").unwrap_err().message,
            "YuuLabs response invalid"
        );
    }

    #[test]
    fn yukulabs_mengembalikan_video_hd_dan_audio() {
        let http = StubHttp::tunggal(
            r#"{"status":true,"result":{"description":"D","author":"A",
                "videoUrl":"https://v/sd.mp4","hdVideo":"https://v/hd.mp4",
                "audioUrl":"https://v/a.mp3"}}"#,
        );
        let hasil = ttdown_from_yuu_labs(&http, "https://vt.tiktok.com/x").unwrap();
        assert_eq!(hasil["title"], "D");
        assert_eq!(hasil["author"]["username"], "A");
        assert_eq!(hasil["author"]["avatar"], Value::Null);
        assert_eq!(hasil["cover"], Value::Null);
        let d = hasil["downloads"].as_array().unwrap();
        assert_eq!(d.len(), 3);
        assert_eq!(d[0]["type"], "nowatermark");
        assert_eq!(d[1]["type"], "nowatermark_hd");
        assert_eq!(d[2]["type"], "mp3");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            p[0].url,
            "https://api.yuulabs.web.id/api/downloader/tiktok?url=https%3A%2F%2Fvt.tiktok.com%2Fx"
        );
    }

    #[test]
    fn yukulabs_tanpa_link_melempar_pesannya() {
        let http = StubHttp::tunggal(r#"{"status":true,"result":{"description":"D"}}"#);
        assert_eq!(
            ttdown_from_yuu_labs(&http, "https://x").unwrap_err().message,
            "YuuLabs tidak mengembalikan link download"
        );
    }

    #[test]
    fn musicaldown_mengisi_field_kosong_dan_membaca_halaman_hasil() {
        let halaman_awal = r#"
            <form id="submit-form">
              <input name="q" value="tiktok">
              <input name="url" value="">
              <input name="lang" value="en">
            </form>
        "#;
        let halaman_hasil = r#"
            <div class="video-header" style="background:url(https://c/cover.jpg) center"></div>
            <div class="video-desc">  Deskripsi Video  </div>
            <div class="video-author"><b>  NamaAuthor  </b></div>
            <div class="img-area"><img src="https://c/av.jpg"></div>
            <a class="download" data-event="_download_click_hd" href="https://d/720.mp4"> MP4 HD </a>
            <a class="download" href="https://d/audio.mp3">MP3</a>
            <a class="download" data-event="_download_click_x">tanpa href</a>
        "#;
        let http = StubHttp::baru(vec![
            (
                200,
                halaman_awal.to_string(),
                vec![("set-cookie".to_string(), "k=v".to_string())],
            ),
            (200, halaman_hasil.to_string(), Vec::new()),
        ]);
        let hasil = ttdown_from_musical_down(&http, "https://x").unwrap();
        assert_eq!(hasil["title"], "Deskripsi Video");
        assert_eq!(hasil["author"]["username"], "NamaAuthor");
        assert_eq!(hasil["author"]["avatar"], "https://c/av.jpg");
        assert_eq!(hasil["cover"], "https://c/cover.jpg");
        let d = hasil["downloads"].as_array().unwrap();
        assert_eq!(d.len(), 2, "tanpa href dilewati");
        // `.replace("_download_click", "")` menghapus hanya kemunculan PERTAMA;
        // sisanya (termasuk `_` di depan) ikut, jadi hasilnya persis JS.
        assert_eq!(d[0]["type"], "_hd");
        assert_eq!(d[0]["label"], "MP4 HD");
        // `type` `undefined` tidak ikut di `JSON.stringify`.
        assert!(d[1].get("type").is_none());

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].url, "https://musicaldown.com/en");
        assert_eq!(
            p[1].body,
            Some(HttpBody::Form(
                // `q` terisi, `url` kosong → diisi URL tadi.
                "q=tiktok&url=https%3A%2F%2Fx&lang=en".to_string()
            ))
        );
        assert_eq!(cari_header(&p[1], "cookie").as_deref(), Some("k=v"));
        assert_eq!(
            cari_header(&p[1], "content-type").as_deref(),
            Some("application/x-www-form-urlencoded; charset=UTF-8")
        );
    }

    #[test]
    fn musicaldown_tanpa_link_melempar_pesannya() {
        let http = StubHttp::baru(vec![
            (
                200,
                "<form id=\"submit-form\"></form>".to_string(),
                Vec::new(),
            ),
            (200, "<p>kosong</p>".to_string(), Vec::new()),
        ]);
        assert_eq!(
            ttdown_from_musical_down(&http, "https://x").unwrap_err().message,
            "MusicalDown tidak mengembalikan link download"
        );
    }

    #[test]
    fn ttdown_menolak_url_bukan_tiktok_tanpa_permintaan() {
        let http = StubHttp::tunggal("{}");
        assert_eq!(
            ttdown(&http, "https://youtube.com/x").unwrap_err().message,
            "Invalid url."
        );
        assert_eq!(http.permintaan.lock().unwrap().len(), 0);
    }

    #[test]
    fn ttdown_menggabungkan_semua_galat_berurutan() {
        // Satu respons untuk semua permintaan: TikWM gagal, SaveTT gagal
        // (token), lalu fallback.
        let http = StubHttp::tunggal(r#"{"code":1,"msg":"gagal"}"#);
        let err = ttdown(&http, "https://www.tiktok.com/@a/video/1").unwrap_err();
        assert_eq!(
            err.message,
            "tikwm: gagal \
             | savett(raw): Gagal mendapatkan token saveTT \
             | yuulabs: YuuLabs response invalid \
             | musicaldown: MusicalDown tidak mengembalikan link download"
        );
        // URL panjang tidak memicu GET resolve. Urutan: TikWM, SaveTT (GET
        // token), YuuLabs, MusicalDown (GET + POST) = 5 permintaan.
        assert_eq!(http.permintaan.lock().unwrap().len(), 5);
    }

    #[test]
    fn ttdown_url_pendek_memicu_get_resolve_dulu() {
        let http = StubHttp::tunggal(
            r#"{"code":0,"data":{"duration":10,"play":"/p.mp4","music":"/m.mp3"}}"#,
        );
        let hasil = ttdown(&http, "https://vt.tiktok.com/abc").unwrap();
        assert_eq!(hasil["downloads"].as_array().unwrap().len(), 2);
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].method, HttpMethod::Get);
        assert_eq!(p[0].url, "https://vt.tiktok.com/abc");
        assert_eq!(p[1].method, HttpMethod::Post, "TikWM jalan setelah resolve");
    }

    #[test]
    fn encoder_axios_bukan_encode_uri_component() {
        // Spasi jadi `+`; `encodeURIComponent` biasa akan jadi `%20`.
        assert_eq!(
            encode_param_axios("https://vt.tiktok.com/a b?x=1"),
            "https%3A%2F%2Fvt.tiktok.com%2Fa+b%3Fx%3D1"
        );
        // `!'()~` dibiarkan axios, dan `:` TIDAK dikembalikan di axios 1.x.
        assert_eq!(encode_param_axios("a!'()~b"), "a!'()~b");
        assert_eq!(encode_param_axios("a:b"), "a%3Ab");
    }

    #[test]
    fn operator_nol_longgar_mengikuti_js() {
        assert!(sama_dengan_nol_loose(Some(&json!(0))));
        assert!(sama_dengan_nol_loose(Some(&json!("0"))));
        assert!(sama_dengan_nol_loose(Some(&json!(""))));
        assert!(sama_dengan_nol_loose(Some(&json!(false))));
        assert!(sama_dengan_nol_loose(Some(&json!([]))));
        assert!(!sama_dengan_nol_loose(Some(&Value::Null)));
        assert!(!sama_dengan_nol_loose(None));
        assert!(!sama_dengan_nol_loose(Some(&json!(1))));
        assert!(!sama_dengan_nol_loose(Some(&json!("abc"))));
    }

    #[test]
    fn indeks_nol_untuk_string_mengambil_karakter_pertama() {
        assert_eq!(
            indeks_nol_atau_nilai(&json!("https://a")),
            json!("h"),
            "`URL[0]` pada string = karakter pertama"
        );
        assert_eq!(indeks_nol_atau_nilai(&json!(["a", "b"])), json!("a"));
        assert_eq!(indeks_nol_atau_nilai(&json!([])), json!([]));
        assert_eq!(indeks_nol_atau_nilai(&json!(7)), json!(7));
    }
}