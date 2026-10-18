//! `src/scraper/youtube.js` — SaveTube (link langsung YouTube) dan pencarian
//! InnerTube.
//!
//! Yang ikut disalin dari JS:
//! - `getYoutubeDirectUrl` mencoba `[format, '480', '720', '240', '144']` dengan
//!   buang duplikat `indexOf` (bukan `Set`, jadi urutan tetap), lalu melempar
//!   error percobaan TERAKHIR kalau semua gagal.
//! - `SaveTube.download` memeriksa format terhadap
//!   `['144','240','360','480','720','1080','mp3']` SEBELUM percobaan pertama,
//!   lalu mengulang sampai 4 kali dengan jeda `800 * attempt` ms.
//! - `decode()` memecrypt AES-128-CBC: 16 byte pertama payload base64 adalah
//!   IV, sisanya ciphertext, kunci dari `this.ky` (hex). Padding PKCS#7 dilepas
//!   OpenJS — padding rusak melempar, bukan dipotong diam-diam.
//! - Kunci hasil `JSON.stringify` yang tidak ada TIDAK ikut: `rawTitle` dan
//!   `duration` hilang kalau `info` tidak punya field itu.
//! - `searchYoutube` mengembalikan string, bukan objek, jadi tipe balikannya
//!   `Result<String, _>`; `toBuffer` mengembalikan byte mentah, jadi
//!   `Result<Vec<u8>, _>`.
//!
//! Beda yang diketahui:
//! - `timeout: 10000/15000/30000` milik axios TIDAK ditiru (batas mesin, bukan
//!   tampilan); `UreqHttp` memakai batas global 20 detik.
//! - `await this.sleep(...)` jadi `std::thread::sleep`, jadi percobaan ulang
//!   memblokir thread — sama seperti yang sudah dilakukan `ytdl.rs`.
//! - `fetch` + `AbortSignal.timeout(15000)` di `searchYoutube` tidak punya
//!   padanan; yang dipakai `HttpCall`.
//! - Teks error dari `crypto` Node (`error:1C800064:Provider routines::bad
//!   decrypt`) disalin karena muncul di `SaveTube gagal: …`, tapi teks
//!   `JSON.parse` milik V8 tidak direkonstruksi — pesannya ditulis ulang dan
//!   karena itu beda dari JS.
//!
//! Bagian yang TIDAK di-port: `remuxVideoBuffer` (`youtube.js:315-353`) dan
//! `downloadWithFallback` (`youtube.js:369-377`). Keduanya memanggil
//! `ffprobe`/`ffmpeg` lewat `child_process` dan menulis berkas sementara di
//! `os.tmpdir()`, dan `downloadWithFallback` mengembalikan `_buffer` berupa
//! `Buffer` yang tidak bisa masuk `serde_json::Value`. Stub yang mengembalikan
//! hasil palsu lebih berbahaya daripada tidak ada.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::{Captures, Regex};
use serde_json::{Map, Value, json};

use crate::common::{field, truthy};

// ---------------------------------------------------------------------------
// Konstanta
// ---------------------------------------------------------------------------

/// `this.ky` (`youtube.js:6`) — kunci AES-128 SaveTube, 16 byte dalam hex.
const KY: &str = "C5D58EF67A7584E4A29F6C35BBC4EB12";

/// `this.fmt` (`youtube.js:7`) — format yang diterima `SaveTube.download`.
const FMT: [&str; 7] = ["144", "240", "360", "480", "720", "1080", "mp3"];

/// `this.ua` SaveTube (`youtube.js:10-11`).
const UA_SAVETUBE: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Mobile Safari/537.36";

/// `this.maxTry` (`youtube.js:12`).
const MAX_COB: u32 = 4;

const REFERER_SAVETUBE: &str = "https://save-tube.com/";

/// `innertubeKey` (`youtube.js:264`).
const INNER_KEY: &str = "AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";

/// `innertubeUa` (`youtube.js:265-266`).
const INNER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// Pola regex (dikompilasi sekali)
// ---------------------------------------------------------------------------

/// `lp` (`youtube.js:36`). Sumber yang sama juga disisipkan ke
/// `new RegExp(...)` di baris 43, jadi dua pola di bawah harus berasal dari
/// satu teks ini.
const LP: &str = r"lyrics?|official\s*(music\s*)?video|audio|visualizer|mv|m/v|hd|4k|8k|full\s*song|original\s*mix|remaster(ed)?|live|cover|clean|explicit|extended|radio\s*edit|slowed|reverb|sped\s*up|color\s*coded|sub\s*indo|terjemahan|lyric\s*video";

struct Pola {
    /// `this.re` (`youtube.js:8-9`) — grup 3 = ID video.
    id_video: Regex,
    /// Uji "sudah URL YouTube" di `searchYoutube` (`youtube.js:278`).
    url_youtube: Regex,
    /// `lp` polos untuk `lp.test(m)` di baris 38.
    lp: Regex,
    /// Kata `lp` dengan batas kata: `new RegExp('\\b(' + lp.source + ')\\b', 'gi')`.
    lp_kata: Regex,
    /// Emoji dan simbol yang dibuang `cleanTitle` (`youtube.js:37`). Rentang
    /// `A-B` ditulis dengan `\x{...}`, bukan `\u{...}`.
    simbol: Regex,
    /// `[\(\[\{][^\)\]\}]*[\)\]\}]` (`youtube.js:38`).
    kurung: Regex,
    /// `//+` (`youtube.js:40`).
    garis_miring: Regex,
    /// `[|:]{1,2}` (`youtube.js:41`).
    pisah: Regex,
    /// `[#@]\S+` (`youtube.js:42`).
    tag: Regex,
    /// `\s{2,}` (`youtube.js:45`).
    spasi_panjang: Regex,
    /// `\s+-\s*$` (`youtube.js:46`).
    ekor: Regex,
    /// `^\s*-\s+` (`youtube.js:47`).
    kepala: Regex,
}

fn pola() -> &'static Pola {
    static P: std::sync::OnceLock<Pola> = std::sync::OnceLock::new();
    P.get_or_init(|| Pola {
        id_video: Regex::new(
            r"^((?:https?:)?//)?((?:www|m|music)\.)?(?:youtube\.com|youtu\.be)/(?:watch\?v=)?(?:embed/)?(?:v/)?(?:shorts/)?([a-zA-Z0-9_-]{11})",
        )
        .expect("pola id video"),
        url_youtube: Regex::new(r"^((?:https?:)?//)?((?:www|m|music)\.)?(?:youtube\.com|youtu\.be)/")
            .expect("pola url youtube"),
        lp: Regex::new(&format!("(?i)(?:{LP})")).expect("pola lp"),
        lp_kata: Regex::new(&format!(r"(?i)\b(?:{LP})\b")).expect("pola lp kata"),
        simbol: Regex::new(
            r"[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}\x{2190}-\x{21FF}\x{2B00}-\x{2BFF}\x{FE0F}\x{200D}]",
        )
        .expect("pola simbol"),
        kurung: Regex::new(r"[\(\[\{][^\)\]\}]*[\)\]\}]").expect("pola kurung"),
        garis_miring: Regex::new(r"//+").expect("pola garis miring"),
        pisah: Regex::new(r"[|:]{1,2}").expect("pola pisah"),
        tag: Regex::new(r"[#@]\S+").expect("pola tag"),
        spasi_panjang: Regex::new(r"\s{2,}").expect("pola spasi panjang"),
        ekor: Regex::new(r"\s+-\s*$").expect("pola ekor"),
        kepala: Regex::new(r"^\s*-\s+").expect("pola kepala"),
    })
}

// ---------------------------------------------------------------------------
// Fungsi publik
// ---------------------------------------------------------------------------

/// `searchYoutube(q)` (`youtube.js:277-299`) — mengembalikan URL video.
pub fn search_youtube(http: &dyn HttpGet, q: &str) -> Result<String, HttpError> {
    if pola().url_youtube.is_match(q) {
        return Ok(q.to_string());
    }
    let res = http.call(
        &HttpCall::post(format!(
            "https://www.youtube.com/youtubei/v1/search?key={INNER_KEY}"
        ))
        .header("content-type", "application/json")
        .header("user-agent", INNER_UA)
        .header("x-youtube-client-name", "1")
        .header("x-youtube-client-version", "2.20240101.00.00")
        .json(&json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": "2.20240101.00.00",
                    "hl": "en",
                    "gl": "US"
                }
            },
            "query": q
        })),
    )?;
    // `fetch` tidak melempar untuk status 4xx; `!res.ok` yang melempar, jadi
    // statusnya harus terbaca di sini dan tidak boleh jadi `Err` lebih dulu.
    if !(200..300).contains(&res.status) {
        return Err(HttpError {
            message: format!("Pencarian gagal HTTP {}", res.status),
        });
    }
    let data = res.json().unwrap_or(Value::Null);

    let mut ketemu: Vec<&Value> = Vec::new();
    kumpulkan_video_renderer(data.get("contents"), &mut ketemu);
    if ketemu.is_empty() {
        return Err(HttpError {
            message: "video tidak ditemukan".to_string(),
        });
    }
    Ok(format!(
        "https://www.youtube.com/watch?v={}",
        teks_js(ketemu[0].get("videoId").unwrap_or(&Value::Null))
    ))
}

/// `toBuffer(url, retries = 3)` (`youtube.js:301-313`).
pub fn to_buffer(http: &dyn HttpGet, url: &str, retries: u32) -> Result<Vec<u8>, HttpError> {
    let mut terakhir: Option<HttpError> = None;
    for i in 0..retries {
        match http.get(url) {
            Ok(res) => return Ok(res.body.bytes().to_vec()),
            Err(e) => {
                terakhir = Some(e);
                if i + 1 < retries {
                    std::thread::sleep(std::time::Duration::from_millis(800 * u64::from(i) + 800));
                }
            }
        }
    }
    // `throw lastErr` dengan `retries = 0` melempar `null`; tidak ada teks pesan
    // yang bisa disalin dari sana.
    Err(terakhir.unwrap_or(HttpError {
        message: String::new(),
    }))
}

/// `getYoutubeDirectUrl(url, format = '360')` (`youtube.js:355-367`).
pub fn get_youtube_direct_url(
    http: &dyn HttpGet,
    url: &str,
    format: Option<&str>,
) -> Result<Value, HttpError> {
    let format = format.unwrap_or("360");
    // `filter((v, i, a) => a.indexOf(v) === i)` membuang kemunculan kedua,
    // bukan kiramah yang berbeda.
    let mut formats: Vec<&str> = Vec::new();
    for f in [format, "480", "720", "240", "144"] {
        if !formats.contains(&f) {
            formats.push(f);
        }
    }

    let mut terakhir: Option<HttpError> = None;
    for fmt in formats {
        match savetube_download(http, url, fmt) {
            Ok(v) if truthy(v.get("url").unwrap_or(&Value::Null)) => return Ok(v),
            Ok(_) => {}
            Err(e) => terakhir = Some(e),
        }
    }
    Err(terakhir.unwrap_or(HttpError {
        message: "Gagal mendapatkan link download video".to_string(),
    }))
}

// ---------------------------------------------------------------------------
// SaveTube
// ---------------------------------------------------------------------------

/// `getCdn()` (`youtube.js:24-27`) — `res.data.cdn`, tanpa header khusus.
fn ambil_cdn(http: &dyn HttpGet) -> Result<String, HttpError> {
    let res = http.get("https://media.savetube.vip/api/random-cdn")?;
    let data = res.json().unwrap_or(Value::Null);
    Ok(data
        .get("cdn")
        .map(teks_js)
        .unwrap_or_else(|| "undefined".to_string()))
}

/// `SaveTube.download(url, format = 'mp3')` (`youtube.js:52-100`).
fn savetube_download(http: &dyn HttpGet, url: &str, format: &str) -> Result<Value, HttpError> {
    let Some(id) = pola()
        .id_video
        .captures(url)
        .and_then(|c| c.get(3))
        .map(|m| m.as_str().to_string())
    else {
        return Err(HttpError {
            message: "ID tidak bisa diambil dari URL".to_string(),
        });
    };
    if !FMT.contains(&format) {
        return Err(HttpError {
            message: format!("Format tidak valid. Pilihan: {}", FMT.join(", ")),
        });
    }

    let mut terakhir: Option<HttpError> = None;
    for attempt in 1..=MAX_COB {
        match coba_savetube(http, &id, format) {
            Ok(v) => return Ok(v),
            Err(e) => {
                terakhir = Some(e);
                if attempt < MAX_COB {
                    std::thread::sleep(std::time::Duration::from_millis(800 * u64::from(attempt)));
                }
            }
        }
    }
    Err(HttpError {
        message: format!(
            "SaveTube gagal: {}",
            terakhir.map(|e| e.message).unwrap_or_default()
        ),
    })
}

/// Satu percobaan di dalam `SaveTube.download` (`youtube.js:58-93`).
fn coba_savetube(http: &dyn HttpGet, id: &str, format: &str) -> Result<Value, HttpError> {
    let cdn = ambil_cdn(http)?;

    let info_res = http.call(
        &HttpCall::post(format!("https://{cdn}/v2/info"))
            .header("content-type", "application/json")
            .header("user-agent", UA_SAVETUBE)
            .header("referer", REFERER_SAVETUBE)
            .json(&json!({ "url": format!("https://www.youtube.com/watch?v={id}") })),
    )?;
    let info = dekrip(
        info_res
            .json()
            .unwrap_or(Value::Null)
            .get("data")
            .unwrap_or(&Value::Null),
    )?;

    // `key: info.key` — `JSON.stringify` membuang `undefined`, jadi `key`
    // tidak ikut di body kalau `info` tidak punya field itu.
    let mut body = Map::new();
    body.insert(
        "downloadType".to_string(),
        json!(if format == "mp3" { "audio" } else { "video" }),
    );
    body.insert(
        "quality".to_string(),
        json!(if format == "mp3" { "128" } else { format }),
    );
    if let Some(k) = info.get("key") {
        body.insert("key".to_string(), k.clone());
    }

    let dl_res = http.call(
        &HttpCall::post(format!("https://{cdn}/download"))
            .header("content-type", "application/json")
            .header("user-agent", UA_SAVETUBE)
            .header("referer", REFERER_SAVETUBE)
            .json(&Value::Object(body)),
    )?;
    let dl = dl_res.json().unwrap_or(Value::Null);
    let dl_url = field(&dl, "data.downloadUrl")
        .cloned()
        .unwrap_or(Value::Null);
    if !truthy(&dl_url) {
        return Err(HttpError {
            message: "downloadUrl tidak ditemukan".to_string(),
        });
    }

    let mut out = Map::new();
    out.insert(
        "title".to_string(),
        Value::String(clean_title(info.get("title"))),
    );
    // `rawTitle: info.title` dan `duration: info.duration` hilang kalau
    // fieldnya tidak ada di `info`, seperti `JSON.stringify`.
    if let Some(v) = info.get("title") {
        out.insert("rawTitle".to_string(), v.clone());
    }
    let thumb = info
        .get("thumbnail")
        .filter(|v| truthy(v))
        .map(teks_js)
        .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg"));
    out.insert("thumb".to_string(), Value::String(thumb));
    if let Some(v) = info.get("duration") {
        out.insert("duration".to_string(), v.clone());
    }
    out.insert("format".to_string(), Value::String(format.to_string()));
    out.insert("url".to_string(), dl_url);
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------------------
// Pencarian InnerTube
// ---------------------------------------------------------------------------

/// `findVideoRenderer(node, out)` (`youtube.js:268-275`).
///
/// Kalau `node.videoRenderer.videoId` truthy, node itu langsung dimasukkan dan
/// penelusuran berhenti di sana (tidak turun ke dalam `videoRenderer`), sama
/// seperti `return` di JS.
fn kumpulkan_video_renderer<'a>(node: Option<&'a Value>, out: &mut Vec<&'a Value>) {
    let Some(node) = node.filter(|v| v.is_object() || v.is_array()) else {
        return;
    };
    if let Some(r) = node.get("videoRenderer")
        && truthy(r.get("videoId").unwrap_or(&Value::Null))
    {
        out.push(r);
        return;
    }
    match node {
        Value::Array(a) => {
            for v in a {
                kumpulkan_video_renderer(Some(v), out);
            }
        }
        Value::Object(m) => {
            for v in m.values() {
                kumpulkan_video_renderer(Some(v), out);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// `cleanTitle`
// ---------------------------------------------------------------------------

/// `String#trim` JS — `char::is_whitespace` Rust tidak memotong `U+FEFF`.
fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// `cleanTitle(raw = '')` (`youtube.js:33-50`). Argumen `None` berarti
/// `undefined`, yang kena nilai bawaan `''`.
fn clean_title(raw: Option<&Value>) -> String {
    let sumber = match raw {
        None => String::new(),
        Some(v) => teks_js(v),
    };
    let p = pola();
    let mut t: String = sumber.clone();

    t = p.simbol.replace_all(&t, "").into_owned();
    t = p
        .kurung
        // `lp.test(m)` tanpa flag `g`, jadi selalu posisi nol. Balikan
            // closure harus `String`: `&str` yang dipinjam dari `Captures`
            // hidup lebih singkat dari `Cow` yang sedang dibangun.
            .replace_all(&t, |m: &Captures| {
                let isi = m.get(0).map(|x| x.as_str()).unwrap_or("");
                if p.lp.is_match(isi) {
                    String::new()
                } else {
                    isi.to_string()
                }
            })
            .into_owned();
    t = p.garis_miring.replace_all(&t, " - ").into_owned();
    t = p.pisah.replace_all(&t, " ").into_owned();
    t = p.tag.replace_all(&t, "").into_owned();
    t = p.lp_kata.replace_all(&t, "").into_owned();
    t = p.spasi_panjang.replace_all(&t, " ").into_owned();
    t = p.ekor.replace_all(&t, "").into_owned();
    t = p.kepala.replace_all(&t, "").into_owned();
    let t = trim_js(&t);

    if t.is_empty() {
        trim_js(&sumber).to_string()
    } else {
        t.to_string()
    }
}

// ---------------------------------------------------------------------------
// `String(x)` JS untuk nilai JSON
// ---------------------------------------------------------------------------

fn teks_js(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Array(a) => a.iter().map(teks_js).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Base64 dan hex — aturan `Buffer.from(x, …)` Node
// ---------------------------------------------------------------------------

/// Decoder base64 yang meniru `Buffer.from(s, 'base64')` Node, bukan decoder
/// ketat: alfabet URL-safe (`-` ke `+`, `_` ke `/`) diterima, karakter di luar
/// alfabet dilewati bukan dianggap error, padding kurang tidak apa-apa, sisa
/// bit dibuang, dan hasil berhenti begitu `=` pertama ditemukan.
fn base64_node(s: &str) -> Vec<u8> {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for c in s.bytes() {
        if c == b'=' {
            break;
        }
        let c = match c {
            b'-' => b'+',
            b'_' => b'/',
            lain => lain,
        };
        let Some(idx) = A.iter().position(|x| *x == c) else {
            continue;
        };
        acc = (acc << 6) | idx as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    out
}

/// `Buffer.from(hex, 'hex')` Node: pasangan digit, digit ganjil diabaikan.
fn dari_hex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i + 1 < b.len() {
        if let (Some(hi), Some(lo)) = (
            (b[i] as char).to_digit(16),
            (b[i + 1] as char).to_digit(16),
        ) {
            out.push((hi * 16 + lo) as u8);
        }
        i += 2;
    }
    out
}

// ---------------------------------------------------------------------------
// AES-128-CBC (dekripsi saja)
// ---------------------------------------------------------------------------

/// Perkalian Galois `GF(2^8)` dengan polinom reduksi `0x11B`.
fn gmul(a: u8, b: u8) -> u8 {
    let mut a = a;
    let mut b = b;
    let mut r = 0u8;
    while b != 0 {
        if b & 1 != 0 {
            r ^= a;
        }
        let bawa = a & 0x80;
        a <<= 1;
        if bawa != 0 {
            a ^= 0x1B;
        }
        b >>= 1;
    }
    r
}

/// Tabel S-box dan inversnya. Dihitung dari invers multiplikatif supaya tidak
/// ada tabel 256 angka yang bisa salah ketik; `test` membandingkannya dengan
/// dua titik tetap (S-box[0x53] = 0xED, S-box[0] = 0x63).
struct Kotak {
    sbox: [u8; 256],
    inv_sbox: [u8; 256],
}

fn kotak() -> &'static Kotak {
    static K: std::sync::OnceLock<Kotak> = std::sync::OnceLock::new();
    K.get_or_init(|| {
        let mut inv = [0u8; 256];
        for b in 1..=255u8 {
            for a in 1..=255u8 {
                if gmul(a, b) == 1 {
                    inv[usize::from(b)] = a;
                    break;
                }
            }
        }
        let mut sbox = [0u8; 256];
        for i in 0..256usize {
            let b = inv[i];
            sbox[i] = b
                ^ b.rotate_left(1)
                ^ b.rotate_left(2)
                ^ b.rotate_left(3)
                ^ b.rotate_left(4)
                ^ 0x63;
        }
        let mut inv_sbox = [0u8; 256];
        for i in 0..256usize {
            inv_sbox[usize::from(sbox[i])] = i as u8;
        }
        Kotak { sbox, inv_sbox }
    })
}

const RCON: [u8; 11] = [
    0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1b, 0x36,
];

/// Ekspansi kunci AES-128 — 11 kunci ronde, 16 byte masing-masing.
fn kunci_ronde(key: &[u8]) -> [u8; 176] {
    let k = kotak();
    let mut rk = [0u8; 176];
    rk[..16].copy_from_slice(&key[..16]);
    let mut i = 16;
    while i < 176 {
        let mut t = [rk[i - 4], rk[i - 3], rk[i - 2], rk[i - 1]];
        if i % 16 == 0 {
            t = [
                k.sbox[usize::from(t[1])] ^ RCON[i / 16],
                k.sbox[usize::from(t[2])],
                k.sbox[usize::from(t[3])],
                k.sbox[usize::from(t[0])],
            ];
        }
        rk[i] = rk[i - 16] ^ t[0];
        rk[i + 1] = rk[i - 15] ^ t[1];
        rk[i + 2] = rk[i - 14] ^ t[2];
        rk[i + 3] = rk[i - 13] ^ t[3];
        i += 4;
    }
    rk
}

fn xor16(a: &[u8; 16], b: &[u8]) -> [u8; 16] {
    let mut o = [0u8; 16];
    for i in 0..16 {
        o[i] = a[i] ^ b[i];
    }
    o
}

fn ke_blok16(src: &[u8]) -> [u8; 16] {
    let mut o = [0u8; 16];
    o.copy_from_slice(&src[..16]);
    o
}

/// Dekripsi satu blok 16 byte (inverse cipher, 10 ronde).
fn dekrip_blok(rk: &[u8; 176], blok: &[u8]) -> [u8; 16] {
    let k = kotak();
    let mut st = xor16(&ke_blok16(blok), &rk[160..176]);
    for r in (1..=9usize).rev() {
        // InvShiftRows: baris r digeser ke kanan r kolom.
        let mut geser = [0u8; 16];
        for c in 0..4usize {
            for baris in 0..4usize {
                geser[baris + 4 * c] = st[baris + 4 * ((c + 4 - baris) % 4)];
            }
        }
        // InvSubBytes.
        let mut sub = [0u8; 16];
        for i in 0..16 {
            sub[i] = k.inv_sbox[usize::from(geser[i])];
        }
        st = xor16(&sub, &rk[r * 16..r * 16 + 16]);
        // InvMixColumns.
        let mut campur = [0u8; 16];
        for c in 0..4usize {
            let p = 4 * c;
            let (a0, a1, a2, a3) = (st[p], st[p + 1], st[p + 2], st[p + 3]);
            campur[p] = gmul(0x0E, a0) ^ gmul(0x0B, a1) ^ gmul(0x0D, a2) ^ gmul(0x09, a3);
            campur[p + 1] = gmul(0x09, a0) ^ gmul(0x0E, a1) ^ gmul(0x0B, a2) ^ gmul(0x0D, a3);
            campur[p + 2] = gmul(0x0D, a0) ^ gmul(0x09, a1) ^ gmul(0x0E, a2) ^ gmul(0x0B, a3);
            campur[p + 3] = gmul(0x0B, a0) ^ gmul(0x0D, a1) ^ gmul(0x09, a2) ^ gmul(0x0E, a3);
        }
        st = campur;
    }
    // Ronde terakhir tanpa InvMixColumns.
    let mut geser = [0u8; 16];
    for c in 0..4usize {
        for baris in 0..4usize {
            geser[baris + 4 * c] = st[baris + 4 * ((c + 4 - baris) % 4)];
        }
    }
    let mut sub = [0u8; 16];
    for i in 0..16 {
        sub[i] = k.inv_sbox[usize::from(geser[i])];
    }
    xor16(&sub, &rk[..16])
}

/// `crypto.createDecipheriv('aes-128-cbc', key, iv)` lalu `update` dan `final`.
///
/// `final` melempar kalau panjang ciphertext bukan kelipatan 16 (`wrong final
/// block length`) atau padding PKCS#7 rusak (`bad decrypt`); teks keduanya ikut
/// disalin karena muncul di pesan `SaveTube gagal: …`.
fn aes128_cbc_dekrip(key: &[u8], iv: &[u8], ct: &[u8]) -> Result<Vec<u8>, HttpError> {
    if iv.len() != 16 {
        return Err(HttpError {
            message: "Invalid initialization vector".to_string(),
        });
    }
    if ct.is_empty() || !ct.len().is_multiple_of(16) {
        return Err(HttpError {
            message: "error:1C80006B:Provider routines::wrong final block length".to_string(),
        });
    }
    let rk = kunci_ronde(key);
    let mut sebelum = ke_blok16(iv);
    let mut out: Vec<u8> = Vec::with_capacity(ct.len());
    for blok in ct.chunks(16) {
        let b = ke_blok16(blok);
        let d = dekrip_blok(&rk, &b);
        let mut p = [0u8; 16];
        for i in 0..16 {
            p[i] = d[i] ^ sebelum[i];
        }
        out.extend_from_slice(&p);
        sebelum = b;
    }
    // PKCS#7: OpenJS juga menolak byte terakhir 0, lebih dari 16, atau
    // isi padding yang tidak semuanya sama.
    let n = usize::from(*out.last().unwrap_or(&0));
    if n == 0
        || n > 16
        || n > out.len()
        || out[out.len() - n..].iter().any(|b| usize::from(*b) != n)
    {
        return Err(HttpError {
            message: "error:1C800064:Provider routines::bad decrypt".to_string(),
        });
    }
    out.truncate(out.len() - n);
    Ok(out)
}

/// `this.decode(enc)` (`youtube.js:15-22`) — base64, lalu AES-128-CBC, lalu
/// `JSON.parse`.
fn dekrip(enc: &Value) -> Result<Value, HttpError> {
    let Value::String(s) = enc else {
        return Err(HttpError {
            message: format!(
                "The first argument must be of type string or an instance of Buffer, \
                 ArrayBuffer, or Array or an Array-like Object. Received {}",
                deskripsi_node(enc)
            ),
        });
    };
    let data = base64_node(s);
    // `data.slice(0, 16)` dan `data.slice(16)` tidak pernah melempar; IV yang
    // bukan 16 byte baru ditolak `createDecipheriv`.
    let batas = data.len().min(16);
    let polos = aes128_cbc_dekrip(&dari_hex(KY), &data[..batas], &data[batas..])?;
    // `JSON.parse` di JS melempar dengan pesan V8; teks itu tidak
    // direkonstruksi (lihat catatan beda di kepala modul).
    serde_json::from_slice::<Value>(&polos).map_err(|e| HttpError {
        message: format!("JSON tidak valid dari SaveTube: {e}"),
    })
}

/// Bagian `Received …` dari pesan `Buffer.from`, mengikuti `util.inspect`.
fn deskripsi_node(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "type boolean (true)".to_string(),
        Value::Bool(false) => "type boolean (false)".to_string(),
        Value::Number(n) => format!("type number ({n})"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
        Value::String(_) => "a string".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpMethod, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet` tanpa jaringan: mencatat `HttpCall` dan membalas
    /// (status, body) berurutan; setelah habis, balasan terakhir diulang.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<(u16, String)>>,
        hitung: Mutex<usize>,
        gagal: Option<String>,
    }

    impl StubHttp {
        fn baru(balasan: Vec<(u16, String)>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
                gagal: None,
            }
        }

        fn gagal_selalu(pesan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(Vec::new()),
                hitung: Mutex::new(0),
                gagal: Some(pesan.to_string()),
            }
        }

        /// Balasan berikutnya sesuai urutan pemanggilan; `get` dan `call`
        /// memakai jalur yang sama supaya `getCdn` ikut terhitung.
        fn balasan_berikut(&self) -> HttpResponse {
            let mut h = self.hitung.lock().unwrap();
            let b = self.balasan.lock().unwrap();
            let (status, body) = if b.is_empty() {
                (200, "{}".to_string())
            } else {
                b[(*h).min(b.len() - 1)].clone()
            };
            *h += 1;
            HttpResponse {
                status,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            // Sengaja lewat `call` supaya catatan permintaan tetap lengkap.
            let req = HttpCall::get(_url);
            self.permintaan.lock().unwrap().push(req);
            match &self.gagal {
                Some(p) => Err(HttpError {
                    message: p.clone(),
                }),
                None => Ok(self.balasan_berikut()),
            }
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _p: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            match &self.gagal {
                Some(p) => Err(HttpError {
                    message: p.clone(),
                }),
                None => Ok(self.balasan_berikut()),
            }
        }
    }

    fn cari_header(p: &HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    /// Payload SaveTube hasil `AES-128-CBC(ky, iv=00112233…eeff)` dari
    /// `{"key":"K3Y9x","title":"Lagu Uji ‹Official Video› [HD]", …}`.
    const INFO_B64: &str = "ABEiM0RVZneImaq7zN3u/ySRS+noY6Ay7XJ1nsZrbPP+2RcuGwwaev1qt4NlUA72hdBgn3XNiSl7QfGCo62X8B9A/hxp+1z3KaAFgGgptf1W9l6yzPe64HY5MMw0+o0UIV9n44ophBe+83JkcPivUaDXq2VdkgkvfkNkCj8KBaSrAXnP+WsnSmzRTPJUDJp/Ho+m4WSj/2N97wh2sdHfkQ==";

    #[test]
    fn aes128_cbc_sama_dengan_vektor_standar() {
        let k = kotak();
        assert_eq!(k.sbox[0], 0x63);
        assert_eq!(k.sbox[0x53], 0xED);
        assert_eq!(k.inv_sbox[0xED], 0x53);

        // FIPS-197 C.1 (AES-128): dekripsi satu blok tanpa padding.
        let rk = kunci_ronde(&dari_hex("000102030405060708090a0b0c0d0e0f"));
        let p = dekrip_blok(&rk, &dari_hex("69c4e0d86a7b0430d8cdb78070b4c55a"));
        let hex: String = p.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "00112233445566778899aabbccddeeff");

        // Ciphertext blok 1 SP 800-38A F.2.1 di-dekripsi ECB (tanpa XOR IV),
        // jadi hasilnya BUKAN plaintext tabel yang hanya berlaku kalau blok
        // itu langsung di-XOR IV-nya. Nilai di bawah dihitung dengan
        // `crypto.createDecipheriv('aes-128-ecb', …)` Node.
        let rk = kunci_ronde(&dari_hex("2b7e151628aed2a6abf7158809cf4f3c"));
        let d = dekrip_blok(&rk, &dari_hex("7649abac8119b246cee98e9b12e9197d"));
        let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "6bc0bce12a459991e134741a7f9e1925");

        // Jalur CBC + PKCS#7 penuh, dua blok, diacak Node dengan kunci dan IV
        // yang sama supaya rantai XOR dan pemotongan padding ikut terukur.
        let polos = aes128_cbc_dekrip(
            &dari_hex("2b7e151628aed2a6abf7158809cf4f3c"),
            &dari_hex("000102030405060708090a0b0c0d0e0f"),
            &dari_hex("7649abac8119b246cee98e9b12e9197d2e013f890472d82217b17f45f6e7f539"),
        )
        .unwrap();
        assert_eq!(polos, dari_hex("6bc1bee22e409f96e93d7e117393172aae2d8a57"));
        assert_eq!(
            aes128_cbc_dekrip(
                &dari_hex("2b7e151628aed2a6abf7158809cf4f3c"),
                &dari_hex("000102030405060708090a0b0c0d0e0f"),
                &dari_hex("deadbeef"),
            )
            .unwrap_err()
            .message,
            "error:1C80006B:Provider routines::wrong final block length"
        );

        // Syarat yang OpenJS tolak.
        assert_eq!(
            aes128_cbc_dekrip(&dari_hex(KY), &[0u8; 16], &[0u8; 20])
                .unwrap_err()
                .message,
            "error:1C80006B:Provider routines::wrong final block length"
        );
        assert_eq!(
            aes128_cbc_dekrip(&dari_hex(KY), &[0u8; 8], &[0u8; 16])
                .unwrap_err()
                .message,
            "Invalid initialization vector"
        );
    }

    #[test]
    fn dekrip_membaca_payload_savetube() {
        let data = base64_node(INFO_B64);
        // 16 byte IV + 144 byte ciphertext (9 blok) + padding PKCS#7.
        assert_eq!(data.len(), 160);
        assert_eq!(data.len() - 16, 9 * 16);
        let info = dekrip(&Value::String(INFO_B64.to_string())).unwrap();
        assert_eq!(info["key"], "K3Y9x");
        assert_eq!(info["title"], "Lagu Uji ‹Official Video› [HD]");
        assert_eq!(info["duration"], 213);
        // `data` bukan string -> pesan `Buffer.from` yang disalin.
        assert!(
            dekrip(&Value::Null)
                .unwrap_err()
                .message
                .ends_with("Received null")
        );
    }

    #[test]
    fn base64_node_meniru_buffer_dari_base64() {
        // Padding kurang, karakter asing, dan URL-safe semuanya diterima Node.
        assert_eq!(base64_node("YWJj"), b"abc");
        assert_eq!(base64_node("YWJj="), b"abc");
        assert_eq!(base64_node("YWJ"), b"ab");
        assert_eq!(base64_node("YW"), b"a");
        assert_eq!(base64_node("Y"), b"");
        assert_eq!(base64_node("YWJj\nZGVm"), b"abcdef");
        assert_eq!(base64_node("YW-j_jg=="), [0x61, 0x6F, 0xA3, 0xFE, 0x38]);
        assert_eq!(base64_node("YW!@#j"), [0x61, 0x68]);
        assert_eq!(base64_node("===="), b"");
        assert_eq!(base64_node(""), b"");
    }

    #[test]
    fn clean_title_membuang_simbol_dan_kata_lp() {
        // Setiap masukan dibungkus `Value` yang hidup selama assertion,
        // karena `clean_title` hanya-pinjam.
        fn s(v: &str) -> String {
            clean_title(Some(&Value::String(v.to_string())))
        }
        assert_eq!(s("Lagu Uji ‹Official Video› [HD]"), "Lagu Uji ‹›");
        // Kurung dibuang hanya kalau isinya cocok `lp`; kurung lain tetap.
        assert_eq!(
            s("  -  Artist   -   Title (Official Music Video) [4K] -  "),
            "Artist - Title"
        );
        assert_eq!(s("SONG//NAME :: LIVE @GORENG  #tag"), "SONG - NAME");
        assert_eq!(s("Dj - Judul (Lyrics)"), "Dj - Judul");
        assert_eq!(s("Halo Dunia"), "Halo Dunia");
        assert_eq!(s("M/V - Judul (Audio)"), "Judul");
        assert_eq!(s("Judul | Outro"), "Judul Outro");
        assert_eq!(s("\u{1F600} Emoji Lagu"), "Emoji Lagu");
        // Judul kosong: `t || String(raw).trim()` jatuh ke teks asal.
        assert_eq!(s("   "), "");
        // `raw = ''` hanya berlaku untuk `undefined`; `null` jadi teks "null".
        assert_eq!(clean_title(None), "");
        assert_eq!(clean_title(Some(&Value::Null)), "null");
    }

    #[test]
    fn alur_savetube_mengirim_permintaan_yang_benar() {
        let http = StubHttp::baru(vec![
            (200, r#"{"cdn":"cdn.savetube.vip"}"#.to_string()),
            (200, format!(r#"{{"data":"{INFO_B64}"}}"#)),
            (
                200,
                r#"{"data":{"downloadUrl":"https://cdn.savetube.vip/file.mp4"}}"#.to_string(),
            ),
        ]);
        let hasil = get_youtube_direct_url(
            &http,
            "https://www.youtube.com/watch?v=abcdefghijk",
            Some("360"),
        )
        .unwrap();
        assert_eq!(hasil["title"], "Lagu Uji ‹›");
        assert_eq!(hasil["rawTitle"], "Lagu Uji ‹Official Video› [HD]");
        assert_eq!(
            hasil["thumb"],
            "https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg"
        );
        assert_eq!(hasil["duration"], 213);
        assert_eq!(hasil["format"], "360");
        assert_eq!(hasil["url"], "https://cdn.savetube.vip/file.mp4");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].url, "https://media.savetube.vip/api/random-cdn");
        assert_eq!(p[1].url, "https://cdn.savetube.vip/v2/info");
        assert_eq!(p[1].method, HttpMethod::Post);
        assert_eq!(
            p[1].body.as_ref().unwrap().bytes(),
            br#"{"url":"https://www.youtube.com/watch?v=abcdefghijk"}"#
        );
        assert_eq!(
            cari_header(&p[1], "referer").as_deref(),
            Some("https://save-tube.com/")
        );
        assert_eq!(cari_header(&p[1], "user-agent").as_deref(), Some(UA_SAVETUBE));
        assert_eq!(p[2].url, "https://cdn.savetube.vip/download");
        assert_eq!(
            p[2].body.as_ref().unwrap().bytes(),
            br#"{"downloadType":"video","quality":"360","key":"K3Y9x"}"#
        );
    }

    #[test]
    fn format_mp3_memetakan_download_type_audio_dan_kualitas_128() {
        let http = StubHttp::baru(vec![
            (200, r#"{"cdn":"c"}"#.to_string()),
            (200, format!(r#"{{"data":"{INFO_B64}"}}"#)),
            (
                200,
                r#"{"data":{"downloadUrl":"https://cdn.savetube.vip/a.m4a"}}"#.to_string(),
            ),
        ]);
        let hasil =
            get_youtube_direct_url(&http, "https://youtu.be/abcdefghijk", Some("mp3")).unwrap();
        assert_eq!(hasil["format"], "mp3");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(
            p[2].body.as_ref().unwrap().bytes(),
            br#"{"downloadType":"audio","quality":"128","key":"K3Y9x"}"#
        );
    }

    #[test]
    fn id_kosong_memberi_pesan_yang_pasti_tanpa_jaringan() {
        // Cek ID ada di depan loop percobaan, jadi `get_youtube_direct_url`
        // gagal untuk kelima format tanpa satu pun permintaan atau jeda.
        let http = StubHttp::baru(vec![]);
        let err = get_youtube_direct_url(&http, "https://vimeo.com/1", Some("360")).unwrap_err();
        assert_eq!(err.message, "ID tidak bisa diambil dari URL");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn format_tidak_valid_memberi_pesan_yang_pasti() {
        // `savetube_download` dipanggil langsung: `get_youtube_direct_url`
        // akan mencoba lima format dan setiap format yang valid mengulang
        // empat kali dengan jeda, jadi test-nya jadi beberapa detik.
        let http = StubHttp::baru(vec![]);
        let err = savetube_download(&http, "https://youtu.be/abcdefghijk", "mp4").unwrap_err();
        assert_eq!(
            err.message,
            "Format tidak valid. Pilihan: 144, 240, 360, 480, 720, 1080, mp3"
        );
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn semua_percobaan_gagal_membungkus_pesan_save_tube() {
        // `downloadUrl` tidak ada -> empat percobaan, lalu `SaveTube gagal:`.
        // Balasannya diulang empat kali: stub mengulang balasan terakhir saat
        // daftar habis, jadi satu blok saja membuat tiga permintaan berikutnya
        // dapat `{"data":{}}` untuk `random-cdn` juga dan pesan errornya berubah.
        let info = format!(r#"{{"data":"{INFO_B64}"}}"#);
        let mut balasan = Vec::new();
        for _ in 0..4 {
            balasan.push((200, r#"{"cdn":"c"}"#.to_string()));
            balasan.push((200, info.clone()));
            balasan.push((200, r#"{"data":{}}"#.to_string()));
        }
        let http = StubHttp::baru(balasan);
        let err = savetube_download(&http, "https://youtu.be/abcdefghijk", "720").unwrap_err();
        assert_eq!(err.message, "SaveTube gagal: downloadUrl tidak ditemukan");
        // Empat percobaan, tiga permintaan tiap percobaan.
        assert_eq!(http.permintaan.lock().unwrap().len(), 4 * 3);
    }

    #[test]
    fn kegagalan_transport_dilewatkan_seperti_catch_js() {
        let http = StubHttp::gagal_selalu("HTTP 403");
        let err = savetube_download(&http, "https://youtu.be/abcdefghijk", "480").unwrap_err();
        assert_eq!(err.message, "SaveTube gagal: HTTP 403");
    }

    #[test]
    fn search_youtube_mengembalikan_url_dan_mencari_video_renderer() {
        let http = StubHttp::baru(vec![(
            200,
            r#"{"contents":{"tabs":[{"tabRenderer":{"content":{"sectionListRenderer":{"contents":[
                 {"itemSectionRenderer":{"contents":[
                   {"videoRenderer":{"videoId":"aaaaaaaaaaa","title":{"runs":[{"text":"x"}]}}},
                   {"videoRenderer":{"videoId":""}}
                 ]}}
               ]}}}}]}}"#
                .to_string(),
        )]);
        let hasil = search_youtube(&http, "lagu uji").unwrap();
        assert_eq!(hasil, "https://www.youtube.com/watch?v=aaaaaaaaaaa");
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, HttpMethod::Post);
        assert_eq!(
            p[0].url,
            "https://www.youtube.com/youtubei/v1/search?key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8"
        );
        assert_eq!(
            cari_header(&p[0], "x-youtube-client-name").as_deref(),
            Some("1")
        );
        assert_eq!(
            cari_header(&p[0], "x-youtube-client-version").as_deref(),
            Some("2.20240101.00.00")
        );
        let badan = String::from_utf8(p[0].body.as_ref().unwrap().bytes().to_vec()).unwrap();
        assert!(badan.contains(r#""query":"lagu uji""#), "{badan}");
        assert!(badan.contains(r#""clientName":"WEB""#), "{badan}");

        // Query yang sudah URL YouTube dikembalikan apa adanya, tanpa jaringan.
        let http = StubHttp::baru(vec![]);
        assert_eq!(
            search_youtube(&http, "https://youtu.be/abcdefghijk").unwrap(),
            "https://youtu.be/abcdefghijk"
        );
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn search_youtube_status_gagal_dan_tanpa_hasil() {
        let http = StubHttp::baru(vec![(503, r#"{"error":"busy"}"#.to_string())]);
        assert_eq!(
            search_youtube(&http, "apa saja").unwrap_err().message,
            "Pencarian gagal HTTP 503"
        );

        let http = StubHttp::baru(vec![(200, r#"{"contents":{"tabs":[]}}"#.to_string())]);
        assert_eq!(
            search_youtube(&http, "apa saja").unwrap_err().message,
            "video tidak ditemukan"
        );
    }

    #[test]
    fn to_buffer_mengembalikan_byte_mentah_dan_mengulang_saat_gagal() {
        let http = StubHttp::baru(vec![(200, "abc".to_string())]);
        assert_eq!(to_buffer(&http, "https://x/y", 3).unwrap(), b"abc");
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);

        let http = StubHttp::gagal_selalu("HTTP 500");
        assert_eq!(
            to_buffer(&http, "https://x/y", 2).unwrap_err().message,
            "HTTP 500"
        );
        assert_eq!(http.permintaan.lock().unwrap().len(), 2);
    }
}