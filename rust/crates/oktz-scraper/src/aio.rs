//! `src/scraper/aio.js` — pengenal platform lalu deleter per platform.
//!
//! Alur `aiodl` disalin apa adanya: `detectPlatform` lebih dulu, handler
//! sesuai platform, dan kalau handler gagal maka `handleSavefbs` dicoba lagi
//! sebagai cadangan — kecuali untuk `twitter`, `threads`, dan `reddit` yang
//! memang memakai `handleSavefbs` sebagai handler utama. Kalau cadangan juga
//! gagal, error PERTAMA yang dilempar, bukan error cadangan.
//!
//! Yang ikut disalin dari JS:
//! - Semua `throw new Error(...)` jadi `Err(HttpError)` dengan teks persis.
//! - `platform`, `title`, `thumbnail`, `author`, `media` beserta urutan
//!   kuncinya mengikuti objek literal di JS; `undefined` tidak ikut.
//! - `handleTiktok` mencoba TikWM lalu SaveTT; kalau keduanya gagal, yang
//!   dilempar error TikWM.
//!
//! Beda yang diketahui:
//! - `cheerio` diganti pengurai HTML mini yang dipakai di `sfiledl.rs`: parser
//!   HTML penuh tidak ditiru. Selector yang dipakai file ini (`h3.text-sm`,
//!   `img.aio-thumbnail`, `.aio-format-btn`, `.carousel-item[data-data]`,
//!   `#formatselect option`, `#video-info h3`) semuanya ditangani; entitas
//!   HTML yang di-decode hanya yang dikenal `common::decode_entitas`.
//! - `console.warn` / `console.error` tidak ada padanannya; percobaan yang
//!   gagal diam-diam, sama seperti kalau `console` tidak di-stub.
//! - `fetch` tidak melempar untuk status 4xx/5xx, jadi kedua permintaan
//!   Savefbs memakai `.terima_semua_status()`. axios di modul lain tetap
//!   melempar seperti aslinya.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{decode_entitas, encode_uri_component, truthy};

/// `config.baseUrl` (`btch-downloader/dist/Defaults/site.js:15`).
const BTCH_BASE: &str = "https://backend1.tioo.eu.org";

/// Versi paket `btch-downloader` yang terpasang — dipakai `X-Client-Version`.
const BTCH_VERSI: &str = "6.3.6";

const UA_TIKWM: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/116.0.0.0 Mobile Safari/537.36";

const UA_SAVETT: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Mobile Safari/537.36";

const UA_SAVEFBS: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Mobile Safari/537.36";

// ---------------------------------------------------------------------------
// Utilitas kecil
// ---------------------------------------------------------------------------

/// `String(v)` — dipakai untuk `String(v)` dan `String(best)` di JS.
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

/// `v || null` / `v || bawaan` — nilai falsy JS jatuh ke bawaan.
fn nilai_atau(v: Option<&Value>, bawaan: Value) -> Value {
    match v {
        Some(x) if truthy(x) => x.clone(),
        _ => bawaan,
    }
}

/// `v || "teks"` sebagai string.
fn teks_atau(v: Option<&Value>, bawaan: &str) -> String {
    match v {
        Some(x) if truthy(x) => teks_js(x),
        _ => bawaan.to_string(),
    }
}

/// `v ? "teks" : null` — `|| null` di JS bentuknya begini.
fn teks_atau_null(v: Option<&Value>) -> Value {
    nilai_atau(v, Value::Null)
}

/// Enkode query `URLSearchParams` (dipakai axios): alfanumerik + `*-._`
/// dibiarkan, spasi `+`, sisanya persen huruf besar.
fn encode_query(s: &str) -> String {
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

/// `axios` `buildURL`: `?` kalau URL belum punya query, `&` kalau sudah punya.
fn tambah_query(base: &str, bagian: &[(&str, String)]) -> String {
    let query = bagian
        .iter()
        .map(|(k, v)| format!("{}={}", encode_query(k), encode_query(v)))
        .collect::<Vec<_>>()
        .join("&");
    if query.is_empty() {
        return base.to_string();
    }
    if base.contains('?') {
        format!("{base}&{query}")
    } else {
        format!("{base}?{query}")
    }
}

/// Pola regex yang dipakai beberapa kali dalam file ini.
struct Pola {
    /// `/name="csrf_token" value="([^"]+)"/`
    csrf: Regex,
    /// `/'([^']+)'/`
    kutip: Regex,
}

fn pola() -> &'static Pola {
    static P: OnceLock<Pola> = OnceLock::new();
    P.get_or_init(|| Pola {
        csrf: Regex::new(r#"name="csrf_token" value="([^"]+)""#).expect("pola csrf"),
        kutip: Regex::new(r"'([^']+)'").expect("pola kutip"),
    })
}

/// `json.URL[0] || json.URL` — indeks nol untuk array maupun string (string
/// diambil karakter pertamanya), lalu jatuh ke nilai asal kalau falsy.
fn indeks_nol_atau_nilai(v: &Value) -> Value {
    let pertama = match v {
        Value::Array(a) => a.first().cloned().unwrap_or(Value::Null),
        Value::String(s) => s
            .chars()
            .next()
            .map(|c| Value::String(c.to_string()))
            .unwrap_or(Value::Null),
        _ => Value::Null,
    };
    if truthy(&pertama) {
        pertama
    } else {
        v.clone()
    }
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
            i += pos + 1 + lewat;
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

/// `$(el).text()` — teks semua keturunan.
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

/// Elemen punya kelas itu (`class="aio-format-btn aktif"`).
fn punya_kelas(el: &Elemen, kelas: &str) -> bool {
    el.attrs
        .iter()
        .filter(|(k, _)| k == "class")
        .flat_map(|(_, v)| v.split_whitespace())
        .any(|c| c == kelas)
}

/// `$('#id')` — elemen pertama dengan `id` itu.
fn cari_id<'a>(akar: &'a Elemen, id: &str) -> Option<&'a Elemen> {
    for simpul in &akar.anak {
        if let Simpul::Elemen(el) = simpul {
            if atribut(el, "id").as_deref() == Some(id) {
                return Some(el);
            }
            if let Some(cocok) = cari_id(el, id) {
                return Some(cocok);
            }
        }
    }
    None
}

/// Keturunan pertama dengan nama tag itu (`$('#id').find('h3')`).
fn turunan_tag<'a>(akar: &'a Elemen, tag: &str) -> Option<&'a Elemen> {
    for simpul in &akar.anak {
        if let Simpul::Elemen(el) = simpul {
            if el.tag == tag {
                return Some(el);
            }
            if let Some(cocok) = turunan_tag(el, tag) {
                return Some(cocok);
            }
        }
    }
    None
}

/// Semua keturunan dengan nama tag, urutan dokumen.
fn dengan_tag<'a>(akar: &'a Elemen, tag: &str, keluar: &mut Vec<&'a Elemen>) {
    for simpul in &akar.anak {
        if let Simpul::Elemen(el) = simpul {
            if el.tag == tag {
                keluar.push(el);
            }
            dengan_tag(el, tag, keluar);
        }
    }
}

/// Semua keturunan dengan kelas itu, apa pun tag-nya (`$('.aio-format-btn')`).
fn dengan_kelas<'a>(akar: &'a Elemen, kelas: &str, keluar: &mut Vec<&'a Elemen>) {
    for simpul in &akar.anak {
        if let Simpul::Elemen(el) = simpul {
            if punya_kelas(el, kelas) {
                keluar.push(el);
            }
            dengan_kelas(el, kelas, keluar);
        }
    }
}

/// Elemen pertama dengan tag + kelas (`$('h3.text-sm')`).
fn cari_kelas<'a>(akar: &'a Elemen, tag: &str, kelas: &str) -> Option<&'a Elemen> {
    for simpul in &akar.anak {
        if let Simpul::Elemen(el) = simpul {
            if el.tag == tag && punya_kelas(el, kelas) {
                return Some(el);
            }
            if let Some(cocok) = cari_kelas(el, tag, kelas) {
                return Some(cocok);
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Handler per platform
// ---------------------------------------------------------------------------

/// `PLATFORM_DETECT` (`aio.js:7-17`) — urutan menentukan platform yang dipilih
/// kalau URL-nya cocok lebih dari satu.
const PLATFORM_DETECT: [(&str, &[&str]); 9] = [
    ("instagram", &["instagram.com"]),
    ("youtube", &["youtube.com", "youtu.be"]),
    ("tiktok", &["tiktok.com", "vt.tiktok.com"]),
    ("facebook", &["facebook.com", "fb.watch", "fb.com"]),
    ("pinterest", &["pinterest.com", "pin.it"]),
    ("capcut", &["capcut.com"]),
    ("twitter", &["twitter.com", "x.com"]),
    ("threads", &["threads.net"]),
    ("reddit", &["reddit.com"]),
];

/// `detectPlatform(url)` tanpa argumen HTTP.
fn deteksi_platform(url: &str) -> Option<&'static str> {
    let kecil = url.to_lowercase();
    PLATFORM_DETECT
        .iter()
        .find(|(_, pola)| pola.iter().any(|p| kecil.contains(p)))
        .map(|(kunci, _)| *kunci)
}

/// `detectPlatform(url)` (`aio.js:19-25`) — diekspor JS; tidak melakukan
/// permintaan apa pun, tapi tetap menerima `&dyn HttpGet` supaya semua scraper
/// punya bentuk argumen yang sama.
pub fn detect_platform(_http: &dyn HttpGet, url: &str) -> Option<&'static str> {
    deteksi_platform(url)
}

/// `HttpGet(endpoint, url, …)` dari SDK btch-downloader: satu GET ke
/// `{baseUrl}/{endpoint}?url=…` dengan header versi.
fn btch_get(http: &dyn HttpGet, endpoint: &str, url: &str) -> Result<Value, HttpError> {
    let alamat = format!("{BTCH_BASE}/{endpoint}?url={}", encode_query(url));
    let res = http.call(
        &HttpCall::get(&alamat)
            .header("content-type", "application/json")
            .header("user-agent", format!("btch/{BTCH_VERSI}"))
            .header("x-client-version", BTCH_VERSI),
    )?;
    res.json().ok_or_else(|| HttpError {
        message: format!("respons {endpoint} bukan JSON"),
    })
}

/// `handleInstagram(url)` (`aio.js:27-39`).
fn handle_instagram(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let result = crate::ig::instagram_download(http, url)?;
    let mut media = Vec::new();
    if let Some(daftar) = result.get("media").and_then(Value::as_array) {
        for item in daftar {
            let tipe = item.get("type").and_then(Value::as_str);
            media.push(json!({
                "type": if matches!(tipe, Some("video") | Some("mp4")) { "video" } else { "image" },
                "url": item.get("url").cloned().unwrap_or(Value::Null),
            }));
        }
    }
    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("instagram".to_string()));
    out.insert(
        "title".to_string(),
        Value::String(teks_atau(result.get("title"), &teks_atau(
            result.get("username"),
            "Instagram"
        ))),
    );
    out.insert("thumbnail".to_string(), teks_atau_null(result.get("thumbnail")));
    out.insert("media".to_string(), Value::Array(media));
    Ok(Value::Object(out))
}

/// `hasil.results.download` — jalur unduhan yang dipakai kedua konversi.
/// Fungsi biasa, bukan closure, supaya pemanggil `&video` dan `&audio` yang
/// dua nilai berbeda bisa memakai helper yang sama.
fn unduh_hasil(hasil: &Value) -> Option<&Value> {
    hasil
        .get("results")
        .and_then(|r| r.get("download"))
        .filter(|v| truthy(v))
}

/// `hasil.results.title`.
fn judul_hasil(hasil: &Value) -> Option<&Value> {
    hasil
        .get("results")
        .and_then(|r| r.get("title"))
        .filter(|v| truthy(v))
}

/// `handleYoutube(url)` (`aio.js:41-67`) — dua konversi terpisah: mp4 lalu mp3.
/// Kalau yang pertama gagal, yang kedua tidak dijalankan.
fn handle_youtube(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let video = crate::ytdl::youtube_download(http, url, Some("mp4"))?;
    let audio = crate::ytdl::youtube_download(http, url, Some("mp3"))?;
    let mut media = Vec::new();
    if let Some(url_video) = unduh_hasil(&video) {
        media.push(json!({ "type": "video", "url": url_video, "quality": "mp4" }));
    }
    if let Some(url_audio) = unduh_hasil(&audio) {
        media.push(json!({ "type": "audio", "url": url_audio, "quality": "mp3" }));
    }
    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("youtube".to_string()));
    let title = match judul_hasil(&video) {
        Some(t) => teks_js(t),
        None => match judul_hasil(&audio) {
            Some(t) => teks_js(t),
            None => "YouTube".to_string(),
        },
    };
    out.insert("title".to_string(), Value::String(title));
    out.insert("thumbnail".to_string(), Value::Null);
    out.insert("media".to_string(), Value::Array(media));
    Ok(Value::Object(out))
}

/// `handleFacebook(url)` (`aio.js:192-204`) — `fbdown` menangkap kesalahannya
/// sendiri jadi `status: false`, jadi kegagalan jaringan berakhir pada teks
/// "Facebook API returned no data", bukan pesan klien HTTP.
fn handle_facebook(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let gagal = || HttpError {
        message: "Facebook API returned no data".to_string(),
    };
    let data = btch_get(http, "fbdown", url).map_err(|_| gagal())?;
    if !data.get("status").map(truthy).unwrap_or(false) {
        return Err(gagal());
    }
    // `data.HD || data.Normal_video`
    let hd = data.get("HD").filter(|v| truthy(v));
    let video_url = hd.cloned().or_else(|| data.get("Normal_video").cloned());
    if !video_url.as_ref().map(truthy).unwrap_or(false) {
        return Err(HttpError {
            message: "No video URL".to_string(),
        });
    }
    let mut media = Map::new();
    media.insert("type".to_string(), Value::String("video".to_string()));
    media.insert("url".to_string(), video_url.unwrap_or(Value::Null));
    media.insert(
        "quality".to_string(),
        Value::String((if hd.is_some() { "HD" } else { "SD" }).to_string()),
    );

    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("facebook".to_string()));
    // `fbdown` tidak pernah mengembalikan `title`/`thumbnail`, jadi bawaan dan
    // `null` ini yang practically selalu terpakai.
    out.insert(
        "title".to_string(),
        Value::String(teks_atau(data.get("title"), "Facebook")),
    );
    out.insert(
        "thumbnail".to_string(),
        teks_atau_null(data.get("thumbnail")),
    );
    out.insert("media".to_string(), Value::Array(vec![Value::Object(media)]));
    Ok(Value::Object(out))
}

/// `handleCapcut(url)` (`aio.js:221-232`).
fn handle_capcut(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let gagal = || HttpError {
        message: "No CapCut video".to_string(),
    };
    let data = btch_get(http, "capcut", url).map_err(|_| gagal())?;
    if !data.get("status").map(truthy).unwrap_or(false)
        || !data.get("originalVideoUrl").map(truthy).unwrap_or(false)
    {
        return Err(gagal());
    }
    let mut media = Map::new();
    media.insert("type".to_string(), Value::String("video".to_string()));
    media.insert(
        "url".to_string(),
        data.get("originalVideoUrl").cloned().unwrap_or(Value::Null),
    );

    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("capcut".to_string()));
    out.insert(
        "title".to_string(),
        Value::String(teks_atau(data.get("title"), "CapCut")),
    );
    out.insert(
        "thumbnail".to_string(),
        teks_atau_null(data.get("thumbnail")),
    );
    out.insert("media".to_string(), Value::Array(vec![Value::Object(media)]));
    Ok(Value::Object(out))
}

/// `handlePinterest(url)` (`aio.js:206-219`).
fn handle_pinterest(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let result = crate::pindl::scrape_pinterest(http, url)?;
    let daftar = result.get("media").and_then(Value::as_array);
    if !daftar.is_some_and(|a| !a.is_empty()) {
        return Err(HttpError {
            message: "No media found".to_string(),
        });
    }
    let mut media = Vec::new();
    for item in daftar.map_or(&[][..], Vec::as_slice) {
        media.push(json!({
            "type": if item.get("type").and_then(Value::as_str) == Some("video") { "video" } else { "image" },
            "url": item.get("url").cloned().unwrap_or(Value::Null),
            "quality": teks_atau_null(item.get("quality")),
        }));
    }
    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("pinterest".to_string()));
    out.insert(
        "title".to_string(),
        Value::String(teks_atau(result.get("title"), "Pinterest")),
    );
    out.insert("thumbnail".to_string(), Value::Null);
    out.insert("media".to_string(), Value::Array(media));
    Ok(Value::Object(out))
}

/// `tryTikWM(u)` (`aio.js:70-119`) — bagian dalam `handleTiktok`.
fn try_tikwm(http: &dyn HttpGet, u: &str) -> Result<Value, HttpError> {
    let alamat = tambah_query(
        "https://www.tikwm.com/api/",
        &[
            ("url", u.to_string()),
            ("count", "12".to_string()),
            ("cursor", "0".to_string()),
            ("web", "1".to_string()),
            ("hd", "1".to_string()),
        ],
    );
    let res = http.call(
        &HttpCall::post(&alamat)
            .header("accept", "application/json, text/javascript, */*; q=0.01")
            .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
            .header("origin", "https://www.tikwm.com")
            .header("referer", "https://www.tikwm.com/")
            .header("user-agent", UA_TIKWM)
            .header("x-requested-with", "XMLHttpRequest")
            // `data: {}` axios — body-nya object kosong, tapi content-type
            // ditulis eksplisit jadi menimpa bawaan axios.
            .raw(b"{}".to_vec(), "application/x-www-form-urlencoded; charset=UTF-8"),
    )?;
    // axios mem-parse JSON; body lain dianggap string oleh axios, dan string
    // tidak punya `code`/`data` sehingga jatuh ke jalur "no media".
    let raw = match res.json() {
        Some(v) => v,
        None => Value::String(res.text()),
    };

    // `if (raw?.code !== undefined && raw.code !== 0)` — hanya angka nol yang
    // lolos; string "0" justru dianggap error.
    if let Some(code) = raw.get("code")
        && !(code.is_number() && code.as_f64() == Some(0.0))
    {
        return Err(HttpError {
            message: teks_atau(raw.get("msg"), "TikWM error"),
        });
    }
    // `const res = raw?.data || raw` lalu `if (!res) throw`
    let isi = nilai_atau(raw.get("data"), raw.clone());
    if !truthy(&isi) {
        return Err(HttpError {
            message: "TikWM no data".to_string(),
        });
    }

    let mut media: Vec<Value> = Vec::new();
    // `res?.duration === 0` — perbandingan ketat, string "0" tidak ikut.
    if isi
        .get("duration")
        .is_some_and(|d| d.is_number() && d.as_f64() == Some(0.0))
    {
        if let Some(gambar) = isi.get("images").filter(|v| truthy(v)) {
            for v in gambar.as_array().map_or(&[][..], Vec::as_slice) {
                media.push(json!({ "type": "image", "url": teks_js(v) }));
            }
        }
    } else {
        if let Some(hd) = isi.get("hdplay").filter(|v| truthy(v)) {
            media.push(json!({
                "type": "video",
                "url": format!("https://www.tikwm.com{}", teks_js(hd)),
                "quality": "HD",
            }));
        }
        if let Some(play) = isi.get("play").filter(|v| truthy(v)) {
            media.push(json!({
                "type": "video",
                "url": format!("https://www.tikwm.com{}", teks_js(play)),
                "quality": "NoWM",
            }));
        }
    }
    if media.is_empty() {
        return Err(HttpError {
            message: "TikWM no media".to_string(),
        });
    }

    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("tiktok".to_string()));
    out.insert(
        "title".to_string(),
        Value::String(teks_atau(isi.get("title"), "TikTok")),
    );
    out.insert(
        "thumbnail".to_string(),
        match isi.get("cover").filter(|v| truthy(v)) {
            Some(c) => Value::String(format!("https://www.tikwm.com{}", teks_js(c))),
            None => Value::Null,
        },
    );
    out.insert(
        "author".to_string(),
        teks_atau_null(isi.get("author").and_then(|a| a.get("nickname"))),
    );
    out.insert("media".to_string(), Value::Array(media));
    Ok(Value::Object(out))
}

/// `trySaveTT(u)` (`aio.js:121-177`) — bagian kedua `handleTiktok`.
fn try_savett(http: &dyn HttpGet, u: &str) -> Result<Value, HttpError> {
    let token_res = http.call(
        &HttpCall::get("https://savett.cc/en1/download").header("user-agent", UA_SAVETT),
    )?;
    let html = token_res.text();
    let csrf = pola()
        .csrf
        .captures(&html)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string());
    // `(headers["set-cookie"] || []).map(v => v.split(";")[0]).join("; ")`
    let cookie = token_res
        .headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|(_, v)| v.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    let Some(csrf) = csrf.filter(|c| !c.is_empty()) else {
        return Err(HttpError {
            message: "SaveTT token failed".to_string(),
        });
    };
    if cookie.is_empty() {
        return Err(HttpError {
            message: "SaveTT token failed".to_string(),
        });
    }

    let body = format!(
        "csrf_token={}&url={}",
        encode_uri_component(&csrf),
        encode_uri_component(u)
    );
    let html_res = http.call(
        &HttpCall::post("https://savett.cc/en1/download")
            .header("content-type", "application/x-www-form-urlencoded")
            .header(
                "accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("origin", "https://savett.cc")
            .header("referer", "https://savett.cc/en1/download")
            .header("user-agent", UA_SAVETT)
            .header("cookie", cookie.as_str())
            .raw(body.into_bytes(), "application/x-www-form-urlencoded"),
    )?;
    let dok = urai(&html_res.text());

    let mut media: Vec<Value> = Vec::new();
    let mut slides = Vec::new();
    dengan_kelas(&dok, "carousel-item", &mut slides);
    slides.retain(|el| atribut(el, "data-data").is_some());

    let judul = cari_id(&dok, "video-info")
        .and_then(|el| turunan_tag(el, "h3"))
        .map(teks_dari)
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| "TikTok".to_string());

    let opsi = cari_id(&dok, "formatselect")
        .map(|el| {
            let mut daftar = Vec::new();
            dengan_tag(el, "option", &mut daftar);
            daftar
        })
        .unwrap_or_default();

    if !slides.is_empty() {
        for el in &slides {
            let Some(raw) = atribut(el, "data-data") else {
                continue;
            };
            let Ok(nilai) = serde_json::from_str::<Value>(&raw.replace("&quot;", "\"")) else {
                continue;
            };
            // `Array.isArray(json.URL) && json.URL.length`
            let Some(daftar) = nilai.get("URL").and_then(Value::as_array) else {
                continue;
            };
            if daftar.is_empty() {
                continue;
            }
            // `[...json.URL].sort((a, b) => String(b).length - String(a).length)[0]`
            let mut urls: Vec<String> = daftar.iter().map(teks_js).collect();
            urls.sort_by_key(|s| std::cmp::Reverse(s.len()));
            media.push(json!({ "type": "image", "url": urls.first().cloned().unwrap_or_default() }));
        }
        // MP3 dari `formatselect` untuk unggahan foto.
        for el in &opsi {
            if !teks_dari(el).to_lowercase().contains("mp3") {
                continue;
            }
            let Some(raw) = atribut(el, "value") else {
                continue;
            };
            let Ok(nilai) = serde_json::from_str::<Value>(&raw.replace("&quot;", "\"")) else {
                continue;
            };
            let Some(url) = nilai.get("URL") else {
                continue;
            };
            media.push(json!({
                "type": "audio",
                "url": indeks_nol_atau_nilai(url),
                "quality": "mp3",
            }));
        }
    } else {
        for el in &opsi {
            let Some(raw) = atribut(el, "value").filter(|v| !v.is_empty()) else {
                continue;
            };
            let label = teks_dari(el).to_lowercase();
            let Ok(nilai) = serde_json::from_str::<Value>(&raw.replace("&quot;", "\"")) else {
                continue;
            };
            let Some(url) = nilai.get("URL").filter(|v| truthy(v)) else {
                continue;
            };
            // `json.URL.forEach` melemparkan untuk non-array; di situ `try`
            // di dalam perulangan yang menelan error-nya.
            let Some(daftar) = url.as_array() else {
                continue;
            };
            if label.contains("mp4") && !label.contains("watermark") {
                for v in daftar {
                    media.push(json!({ "type": "video", "url": v.clone(), "quality": "NoWM" }));
                }
            } else if label.contains("mp3") {
                for v in daftar {
                    media.push(json!({ "type": "audio", "url": v.clone(), "quality": "mp3" }));
                }
            }
        }
    }

    if media.is_empty() {
        return Err(HttpError {
            message: "SaveTT no media".to_string(),
        });
    }

    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("tiktok".to_string()));
    out.insert("title".to_string(), Value::String(judul.clone()));
    out.insert("thumbnail".to_string(), Value::Null);
    out.insert("author".to_string(), Value::String(judul));
    out.insert("media".to_string(), Value::Array(media));
    Ok(Value::Object(out))
}

/// `handleTiktok(url)` (`aio.js:69-190`) — TikWM dulu, SaveTT kedua; kalau
/// keduanya gagal, error TikWM yang dilempar.
fn handle_tiktok(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    match try_tikwm(http, url) {
        Ok(v) => Ok(v),
        Err(first) => try_savett(http, url).map_err(|_| first),
    }
}

/// `handleSavefbs(url)` (`aio.js:242-294`) — dipakai sebagai handler
/// twitter/threads/reddit sekaligus cadangan semua platform lain.
fn handle_savefbs(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let base = "https://savefbs.com/api/v1/aio";
    let info = http.call(
        &HttpCall::post(format!("{base}/html"))
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .header("referer", "https://savefbs.com/all-in-one-video-downloader/")
            .header("user-agent", UA_SAVEFBS)
            .terima_semua_status()
            .json(&json!({ "vid": url, "prefix": "savefbs.com", "ex": "", "format": "" })),
    )?;
    let dok = urai(&info.text());

    let title = cari_kelas(&dok, "h3", "text-sm")
        .map(teks_dari)
        .map(|t| t.trim().to_string())
        .unwrap_or_default();
    let thumb = cari_kelas(&dok, "img", "aio-thumbnail").and_then(|el| atribut(el, "src"));

    let mut tombol = Vec::new();
    dengan_kelas(&dok, "aio-format-btn", &mut tombol);
    let token = tombol
        .first()
        .and_then(|el| atribut(el, "data-loader-id"))
        .filter(|t| !t.is_empty());
    let mut formats: Vec<String> = Vec::new();
    for el in &tombol {
        let Some(onclick) = atribut(el, "onclick") else {
            continue;
        };
        if let Some(tangkapan) = pola().kutip.captures(&onclick).and_then(|c| c.get(1)) {
            formats.push(tangkapan.as_str().to_string());
        }
    }

    let Some(token) = token else {
        return Err(HttpError {
            message: "Savefbs returned no data".to_string(),
        });
    };
    if formats.is_empty() {
        return Err(HttpError {
            message: "Savefbs returned no data".to_string(),
        });
    }

    const VIDEO_PRIORITY: [&str; 4] = ["1080", "720", "480", "360"];
    const AUDIO_FORMATS: [&str; 2] = ["mp3", "wav"];
    let format = VIDEO_PRIORITY
        .iter()
        .find(|f| formats.iter().any(|x| x.as_str() == **f))
        .map(|f| (*f).to_string())
        .or_else(|| {
            formats
                .iter()
                .find(|f| !AUDIO_FORMATS.contains(&f.as_str()))
                .cloned()
        })
        .or_else(|| formats.first().cloned())
        .unwrap_or_default();

    let dl = http.call(
        &HttpCall::post(format!("{base}/download"))
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .header("referer", "https://savefbs.com/all-in-one-video-downloader/")
            .header("user-agent", UA_SAVEFBS)
            .terima_semua_status()
            .json(&json!({ "token": token, "format": format })),
    )?;
    let dl_data = dl.json().unwrap_or(Value::Null);
    let unduh = match dl_data.get("url").filter(|v| truthy(v)) {
        Some(v) => Some(v.clone()),
        None => match dl_data.get("download").filter(|v| truthy(v)) {
            Some(v) => Some(v.clone()),
            None => dl_data
                .get("data")
                .and_then(|d| d.get("url"))
                .filter(|v| truthy(v))
                .cloned(),
        },
    };
    let Some(unduh) = unduh else {
        return Err(HttpError {
            message: "Savefbs no download URL".to_string(),
        });
    };

    let is_audio = AUDIO_FORMATS.contains(&format.as_str());
    let mut media = Map::new();
    media.insert(
        "type".to_string(),
        Value::String((if is_audio { "audio" } else { "video" }).to_string()),
    );
    media.insert("url".to_string(), unduh);
    media.insert("quality".to_string(), Value::String(format));

    let mut out = Map::new();
    out.insert("platform".to_string(), Value::String("savefbs".to_string()));
    out.insert(
        "title".to_string(),
        Value::String(if title.is_empty() {
            "Downloaded".to_string()
        } else {
            title
        }),
    );
    out.insert(
        "thumbnail".to_string(),
        match thumb.filter(|t| !t.is_empty()) {
            Some(t) => Value::String(t),
            None => Value::Null,
        },
    );
    out.insert("media".to_string(), Value::Array(vec![Value::Object(media)]));
    Ok(Value::Object(out))
}

/// `aiodl(url)` (`aio.js:308-329`).
pub fn aiodl(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let Some(platform) = deteksi_platform(url) else {
        return Err(HttpError {
            message: "Platform tidak dikenali".to_string(),
        });
    };
    let hasil = match platform {
        "instagram" => handle_instagram(http, url),
        "youtube" => handle_youtube(http, url),
        "tiktok" => handle_tiktok(http, url),
        "facebook" => handle_facebook(http, url),
        "pinterest" => handle_pinterest(http, url),
        "capcut" => handle_capcut(http, url),
        "twitter" | "threads" | "reddit" => handle_savefbs(http, url),
        lain => Err(HttpError {
            message: format!("Handler untuk {lain} tidak tersedia"),
        }),
    };
    match hasil {
        Ok(v) => Ok(v),
        Err(err) => {
            // Cadangan Savefbs, kecuali tiga platform yang memang memakainya.
            if !matches!(platform, "twitter" | "threads" | "reddit")
                && let Ok(v) = handle_savefbs(http, url)
            {
                return Ok(v);
            }
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` yang mencatat permintaan dan membalas berurutan sesuai
    /// antrean — `aiodl` bisa memanggil banyak URL berbeda dalam satu alur.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<Vec<String>>,
        header: Mutex<Vec<(String, String)>>,
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
            unreachable!("scraper ini lewat HttpCall::call")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
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
            let mut antrean = self.balasan.lock().unwrap();
            let isi = if antrean.is_empty() {
                String::new()
            } else {
                antrean.remove(0)
            };
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(isi.into_bytes()),
                    mimetype: None,
                },
                headers: self.header.lock().unwrap().clone(),
            })
        }
    }

    fn stub(daftar: &[&str]) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(daftar.iter().map(|s| (*s).to_string()).collect()),
            header: Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn platform_dikenali_sesuai_urutan_pola_js() {
        let h = oktz_plugin_api::NoHttp;
        assert_eq!(
            detect_platform(&h, "https://WWW.Instagram.com/p/abc"),
            Some("instagram")
        );
        assert_eq!(detect_platform(&h, "https://vt.tiktok.com/x/"), Some("tiktok"));
        assert_eq!(detect_platform(&h, "https://fb.watch/abc/"), Some("facebook"));
        assert_eq!(detect_platform(&h, "https://pin.it/abc"), Some("pinterest"));
        assert_eq!(detect_platform(&h, "https://x.com/a/status/1"), Some("twitter"));
        assert_eq!(detect_platform(&h, "https://t.co/a"), None);
        // Instagram diperiksa lebih dulu, jadi URL gabungan tetap instagram.
        assert_eq!(
            detect_platform(&h, "https://instagram.com/reel/youtu.be"),
            Some("instagram")
        );
    }

    #[test]
    fn platform_tidak_dikenali_melempar_teks_js() {
        let http = stub(&[]);
        let err = aiodl(&http, "https://contoh.example/nope").unwrap_err();
        assert_eq!(err.message, "Platform tidak dikenali");
        assert!(http.permintaan.lock().unwrap().is_empty());
    }

    #[test]
    fn instagram_melewati_scraper_modul_lain() {
        let http = stub(&[r#"[{"thumbnail":"t","url":"https://cdn/a.jpg"}]"#]);
        let hasil = aiodl(&http, "https://www.instagram.com/p/abc").unwrap();
        assert_eq!(hasil["platform"], "instagram");
        assert_eq!(hasil["title"], "Instagram");
        assert_eq!(hasil["thumbnail"], "t");
        assert_eq!(hasil["media"][0]["type"], "image");
        assert_eq!(hasil["media"][0]["url"], "https://cdn/a.jpg");
    }

    #[test]
    fn facebook_gagal_jadi_teks_tetap_bukan_pesan_klien_http() {
        let http = stub(&[r#"{"status":false,"message":"404 Not Found"}"#]);
        let err = aiodl(&http, "https://www.facebook.com/watch/?v=1").unwrap_err();
        // Cadangan Savefbs juga gagal (antrean habis), jadi error asli yang
        // dilempar.
        assert_eq!(err.message, "Facebook API returned no data");
    }

    #[test]
    fn capcut_mengambil_video_asli_dan_thumbnail() {
        let http = stub(&[
            r#"{"status":true,"title":"Template","thumbnail":"https://c/cover.jpg","originalVideoUrl":"https://c/v.mp4"}"#,
        ]);
        let hasil = aiodl(&http, "https://www.capcut.com/template-detail/7299286607478181121")
            .unwrap();
        assert_eq!(hasil["platform"], "capcut");
        assert_eq!(hasil["title"], "Template");
        assert_eq!(hasil["thumbnail"], "https://c/cover.jpg");
        assert_eq!(hasil["media"][0]["url"], "https://c/v.mp4");
    }

    #[test]
    fn pinterest_mengambil_dari_modul_scraper_lain() {
        // `mediaFileSize` wajib ada: `pindl` menyortir video dengan
        // `parseFloat(a.mediaFileSize) * (…includes("MB") ? 1024 : 1)`, dan
        // field yang hilang membuat `.includes` melempar TypeError di JS.
        let http = stub(&[
            "<html>",
            r#"{"api":{"status":"OK","title":"Pin","mediaItems":[{"type":"Video","mediaQuality":"HD","mediaRes":"720p","mediaFileSize":"12.5 MB","mediaUrl":"https://s/v.mp4"}]}}"#,
        ]);
        let hasil = aiodl(&http, "https://id.pinterest.com/pin/1/").unwrap();
        assert_eq!(hasil["platform"], "pinterest");
        assert_eq!(hasil["title"], "Pin");
        assert_eq!(hasil["media"][0]["type"], "video");
        assert_eq!(hasil["media"][0]["quality"], "HD");
    }

    #[test]
    fn tikwm_dipakai_dulu_sebelum_savett() {
        let http = stub(&[r#"{"code":0,"data":{"title":"Viral","duration":12,"hdplay":"/v/hd.mp4","play":"/v/no.mp4","cover":"/c.jpg","author":{"nickname":"omag"}}}"#]);
        let hasil = aiodl(&http, "https://vt.tiktok.com/ZSkGPK9Kj/").unwrap();
        assert_eq!(hasil["platform"], "tiktok");
        assert_eq!(hasil["title"], "Viral");
        assert_eq!(hasil["thumbnail"], "https://www.tikwm.com/c.jpg");
        assert_eq!(hasil["author"], "omag");
        let media = hasil["media"].as_array().unwrap();
        assert_eq!(media.len(), 2);
        assert_eq!(media[0]["url"], "https://www.tikwm.com/v/hd.mp4");
        assert_eq!(media[0]["quality"], "HD");
        assert_eq!(media[1]["quality"], "NoWM");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert!(p[0].url.starts_with("https://www.tikwm.com/api/?url=https%3A%2F%2Fvt.tiktok.com"));
        assert!(p[0].url.contains("&count=12&cursor=0&web=1&hd=1"));
        assert_eq!(
            p[0].body,
            Some(oktz_plugin_api::HttpBody::Bytes {
                data: std::sync::Arc::from(b"{}".to_vec()),
                mimetype: Some("application/x-www-form-urlencoded; charset=UTF-8".to_string()),
            })
        );
    }

    #[test]
    fn tikwm_gagal_lalu_savett_menampilkan_media_dari_html() {
        // TikWM: satu POST, gagal karena `code: 1001`.
        let tikwm_gagal = r#"{"code":1001,"msg":"TikWM error"}"#;
        // SaveTT: GET halaman token (csrf + cookie), lalu POST hasilnya.
        let halaman_token =
            r#"<html><body><input type="hidden" name="csrf_token" value="abc"></body></html>"#;
        let html = r#"<html><body>
            <div id="video-info"><h3>  Video Savett  </h3></div>
            <div class="formatselect" id="formatselect">
              <option value="{&quot;URL&quot;:[&quot;https://sv/hd.mp4&quot;]}">MP4 1080p</option>
              <option value="{&quot;URL&quot;:[&quot;https://sv/sm.mp4&quot;]}">MP4 with watermark</option>
              <option value="{&quot;URL&quot;:[&quot;https://sv/a.mp3&quot;]}">MP3 320kbps</option>
            </div>
            </body></html>"#;
        let http = stub(&[tikwm_gagal, halaman_token, html]);
        *http.header.lock().unwrap() = vec![(
            "Set-Cookie".to_string(),
            "savett_sid=zzz; Path=/; HttpOnly".to_string(),
        )];
        let hasil = aiodl(&http, "https://www.tiktok.com/@x/video/1").unwrap();
        assert_eq!(hasil["platform"], "tiktok");
        assert_eq!(hasil["title"], "Video Savett");
        assert_eq!(hasil["author"], "Video Savett");
        let media = hasil["media"].as_array().unwrap();
        // Opsi berlabel "with watermark" dilewati JS, jadi hanya video HD
        // pertama dan audio MP3 yang masuk.
        assert_eq!(media.len(), 2);
        assert_eq!(media[0]["type"], "video");
        assert_eq!(media[0]["url"], "https://sv/hd.mp4");
        assert_eq!(media[0]["quality"], "NoWM");
        assert_eq!(media[1]["type"], "audio");
        assert_eq!(media[1]["quality"], "mp3");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 3, "tikwm, halaman token savett, hasil savett");
        assert_eq!(p[1].url, "https://savett.cc/en1/download");
        assert_eq!(p[2].url, "https://savett.cc/en1/download");
        let body = p[2].body.as_ref().unwrap().bytes().to_vec();
        assert_eq!(
            String::from_utf8_lossy(&body),
            "csrf_token=abc&url=https%3A%2F%2Fwww.tiktok.com%2F%40x%2Fvideo%2F1",
            "csrf dan url di-encode dengan encodeURIComponent"
        );
        let cari = |nama: &str| {
            p[2].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("cookie").as_deref(), Some("savett_sid=zzz"));
        assert_eq!(cari("origin").as_deref(), Some("https://savett.cc"));
    }

    #[test]
    fn savett_gagal_lalu_savefbs_menolong_sebagai_cadangan() {
        let tikwm_gagal = r#"{"code":1,"msg":"gagal"}"#;
        let halaman_token =
            r#"<html><body><input type="hidden" name="csrf_token" value="abc"></body></html>"#;
        // Halaman hasil SaveTT tanpa media yang bisa diambil.
        let tanpa_media =
            r#"<html><body><select id="formatselect"><option value="{&quot;URL&quot;:[...]}">MP4</option></select></body></html>"#;
        let tombol = r#"<html><body><h3 class="text-sm"> Judul Savefbs </h3>
             <img class="aio-thumbnail" src="https://f/t.jpg">
             <button class="aio-format-btn" data-loader-id="TOK" onclick="dl('360')"></button>
             <button class="aio-format-btn" data-loader-id="TOK" onclick="dl('1080')"></button>
             <button class="aio-format-btn" data-loader-id="TOK" onclick="dl('mp3')"></button>
             </body></html>"#;
        let http = stub(&[
            tikwm_gagal,
            halaman_token,
            tanpa_media,
            tombol,
            r#"{"url":"https://f/720.mp4"}"#,
        ]);
        // Tanpa `set-cookie` SaveTT berhenti setelah permintaan pertama dengan
        // "SaveTT token failed"; cookie di sini supaya alur penuh diuji.
        *http.header.lock().unwrap() = vec![(
            "Set-Cookie".to_string(),
            "savett_sid=zzz; Path=/".to_string(),
        )];
        let hasil = aiodl(&http, "https://vt.tiktok.com/ZSkGPK9Kj/").unwrap();
        assert_eq!(hasil["platform"], "savefbs");
        assert_eq!(hasil["title"], "Judul Savefbs");
        assert_eq!(hasil["thumbnail"], "https://f/t.jpg");
        // Prioritas 1080 menang dari 360 dan mp3.
        assert_eq!(hasil["media"][0]["type"], "video");
        assert_eq!(hasil["media"][0]["quality"], "1080");
        assert_eq!(hasil["media"][0]["url"], "https://f/720.mp4");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 5, "tikwm, 2x savett, lalu 2x savefbs");
        assert_eq!(p[3].url, "https://savefbs.com/api/v1/aio/html");
        assert_eq!(p[4].url, "https://savefbs.com/api/v1/aio/download");
        assert_eq!(
            p[4].body,
            Some(oktz_plugin_api::HttpBody::Bytes {
                data: std::sync::Arc::from(
                    serde_json::to_vec(&json!({"token": "TOK", "format": "1080"})).unwrap()
                ),
                mimetype: Some("application/json".to_string()),
            })
        );
        assert!(p[4].terima_semua_status, "fetch tidak melempar untuk 4xx");
    }

    #[test]
    fn savefbs_tanpa_token_melempar_teks_js() {
        let http = stub(&["<html><body><button class=\"aio-format-btn\" onclick=\"dl('360')\"></button></body></html>"]);
        let err = aiodl(&http, "https://x.com/satu/status/1").unwrap_err();
        assert_eq!(err.message, "Savefbs returned no data");
    }

    #[test]
    fn youtube_menjalankan_dua_konversi_dan_menggabungkan_media() {
        let http = stub(&[
            r#"{"convertURL":"https://konv/c"}"#,
            r#"{"progressURL":"https://konv/p","downloadURL":"https://konv/v.mp4","title":"Vidi"}"#,
            r#"{"progress":3}"#,
            r#"{"convertURL":"https://konv/c"}"#,
            r#"{"progressURL":"https://konv/p","downloadURL":"https://konv/a.mp3","title":"Vidi"}"#,
            r#"{"progress":3}"#,
        ]);
        let hasil = aiodl(&http, "https://youtu.be/dQw4w9WgXcQ").unwrap();
        assert_eq!(hasil["platform"], "youtube");
        assert_eq!(hasil["title"], "Vidi");
        assert_eq!(hasil["thumbnail"], json!(null));
        let media = hasil["media"].as_array().unwrap();
        assert_eq!(media.len(), 2);
        assert_eq!(media[0], json!({"type": "video", "url": "https://konv/v.mp4", "quality": "mp4"}));
        assert_eq!(media[1], json!({"type": "audio", "url": "https://konv/a.mp3", "quality": "mp3"}));
    }
}