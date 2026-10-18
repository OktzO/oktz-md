//! `src/scraper/reddit.js` — POST ke `redvid.io`, lalu bedah HTML `data.view`
//! dengan cheerio.
//!
//! Cheerio tidak tersedia di crate ini, jadi potongan HTML di bedah dengan
//! regex kecil di bawah; batasnya jujur: tag yang punya `>` di dalam nilai
//! atribut (mis. `<a title="a>b">`) tidak diparse persis seperti parse5.
//! Semantik yang disalin:
//! - `if (data.success && data.view)` — truthy, bukan `=== true`.
//! - `thumbnail: thumb` — kalau atribut `src` tidak ada, kuncinya tidak ikut
//!   (`undefined` dibuang `JSON.stringify`); kalau `src=""`, kuncinya ikut.
//! - `typeText.toLowerCase().includes("video")`.
//! - Href relatif dikembalikan apa adanya, sama seperti JS (tanpa host).
//! - Seluruh badan dalam `try`; error apa pun → `{ status: false, error }`.

use std::sync::LazyLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::truthy;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

/// `RedditDL(redditUrl)` (`reddit.js:4-56`).
pub fn reddit_dl(http: &dyn HttpGet, reddit_url: &str) -> Result<Value, HttpError> {
    match alur(http, reddit_url) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "status": false, "error": e.message })),
    }
}

fn alur(http: &dyn HttpGet, reddit_url: &str) -> Result<Value, HttpError> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let res = http.call(
        &HttpCall::post(format!("https://redvid.io/fetch?_={ts}"))
            .header("authority", "redvid.io")
            .header("accept", "application/json, text/plain, */*")
            .header("content-type", "application/json")
            .header("origin", "https://redvid.io")
            .header("referer", "https://redvid.io/")
            .header("user-agent", UA)
            .header("x-requested-with", "XMLHttpRequest")
            .json(&json!({ "url": reddit_url, "lang": "en" })),
    )?;

    let data = res.json().unwrap_or(Value::Null);
    let sukses = data.get("success").map(truthy).unwrap_or(false);
    let view = data.get("view");

    if sukses && view.map(truthy).unwrap_or(false) {
        let Some(html) = view.and_then(Value::as_str) else {
            // `cheerio.load()` melempar untuk non-string; catch JS membungkus
            // pesan itu ke objek hasil.
            return Err(HttpError {
                message: "cheerio.load() expects a string".to_string(),
            });
        };

        let judul = elemen_mulai(html, 0, html.len(), &|_, isi| {
            punya_kelas(isi, "response-cinema-title")
        })
        .map(|e| trim_js(&teks_elemen(html, &e)).to_string())
        .unwrap_or_default();

        let mut hasil: Vec<Value> = Vec::new();
        let mut pos = 0usize;
        let mut nomor = 0usize;
        while let Some(item) = elemen_mulai(html, pos, html.len(), &|_, isi| {
            punya_kelas(isi, "response-cinema-gallery-item")
        }) {
            nomor += 1;
            let thumb = elemen_mulai(html, item.isi_awal, item.isi_akhir, &|nama, isi| {
                nama.eq_ignore_ascii_case("img") && punya_kelas(isi, "thumbnail-image")
            })
            .and_then(|e| atribut(&e.isi_tag, "src"));

            let tombol = elemen_mulai(html, item.isi_awal, item.isi_akhir, &|nama, isi| {
                nama.eq_ignore_ascii_case("a")
                    && atribut(isi, "href")
                        .map(|h| h.contains("/download?token="))
                        .unwrap_or(false)
            });
            if let Some(t) = tombol
                && let Some(url) = atribut(&t.isi_tag, "href")
                && !url.is_empty()
            {
                let teks = trim_js(&teks_elemen(html, &t)).to_string();
                let tipe = if teks.to_lowercase().contains("video") {
                    "video"
                } else {
                    "image"
                };
                let mut obj = Map::new();
                obj.insert("item".to_string(), json!(nomor));
                obj.insert("type".to_string(), json!(tipe));
                if let Some(thumb) = thumb {
                    obj.insert("thumbnail".to_string(), Value::String(thumb));
                }
                obj.insert("download_url".to_string(), Value::String(url));
                hasil.push(Value::Object(obj));
            }
            pos = item.awal + 1;
        }

        return Ok(json!({
            "status": true,
            "title": judul,
            "results": hasil,
        }));
    }

    Ok(json!({ "status": false, "error": "Gagal mengambil data" }))
}

// ── bedah HTML kecil (pengganti cheerio) ────────────────────────────────────

struct Elemen {
    /// Indeks `<` pembuka.
    awal: usize,
    /// Indeks setelah `>` pembuka.
    isi_awal: usize,
    /// Awal tag penutup (untuk tag kosong: sama dengan `isi_awal`).
    isi_akhir: usize,
    /// Teks di dalam `<` dan `>` pembuka.
    isi_tag: String,
}

/// Tag HTML yang tidak punya tag penutup.
const TAG_KOSONG: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Tag berikutnya mulai `dari`: `(awal, isi_awal, nama_tag, isi_tag)`.
///
/// `nama_tag` untuk tag penutup tetap nama tagnya (`</h1>` → `h1`), karena itu
/// yang dipakai pencocokan nesting. Pemanggil yang hanya mau tag PEMBUKA
/// menyaring lewat `isi_tag.starts_with('/')`; komentar dan doctype punya
/// `nama` kosong.
fn tag_berikut(html: &str, dari: usize) -> Option<(usize, usize, String, String)> {
    if dari >= html.len() {
        return None;
    }
    let lt = html[dari..].find('<')?;
    let awal = dari + lt;
    let gt = html[awal..].find('>')?;
    let isi_awal = awal + gt + 1;
    let isi_tag = html[awal + 1..isi_awal - 1].to_string();
    let nama = isi_tag
        .trim_start_matches('/')
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect::<String>();
    Some((awal, isi_awal, nama, isi_tag))
}

/// Indeks setelah tag penutup yang berpasangan dengan pembuka di `awal`.
fn akhir_elemen(html: &str, awal: usize, nama: &str) -> usize {
    let mut depth = 0usize;
    let mut i = awal;
    while let Some((_, isi_awal, n, isi)) = tag_berikut(html, i) {
        if n.eq_ignore_ascii_case(nama) {
            if isi.starts_with('/') {
                if depth <= 1 {
                    return isi_awal;
                }
                depth -= 1;
            } else if !isi.ends_with('/') {
                depth += 1;
            }
        }
        i = isi_awal;
    }
    html.len()
}

/// Nilai atribut dari teks tag, entitas HTML didekode seperti cheerio.
fn atribut(tag: &str, nama: &str) -> Option<String> {
    static CACHE: LazyLock<std::sync::Mutex<std::collections::HashMap<String, regex::Regex>>> =
        LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let pola = format!(
        r#"(?i)(?:^|[\s"']){}\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s/>]+))"#,
        regex::escape(nama)
    );
    let re = {
        let mut cache = CACHE.lock().unwrap();
        cache
            .entry(pola.clone())
            .or_insert_with(|| regex::Regex::new(&pola).unwrap())
            .clone()
    };
    let c = re.captures(tag)?;
    let nilai = c.get(1).or_else(|| c.get(2)).or_else(|| c.get(3))?;
    Some(crate::common::decode_entitas(nilai.as_str()))
}

/// `.kelas` ala CSS: token di atribut `class` dipisah spasi.
fn punya_kelas(tag: &str, kelas: &str) -> bool {
    atribut(tag, "class")
        .map(|c| c.split_whitespace().any(|k| k == kelas))
        .unwrap_or(false)
}

/// Elemen pertama di `[dari, sampai)` yang cocok — padanan `$(sel).first()`
/// untuk pemakaian di file ini.
fn elemen_mulai(
    html: &str,
    dari: usize,
    sampai: usize,
    cocok: &dyn Fn(&str, &str) -> bool,
) -> Option<Elemen> {
    let mut i = dari;
    while let Some((awal, isi_awal, nama, isi_tag)) = tag_berikut(html, i) {
        if awal >= sampai {
            break;
        }
        if !nama.is_empty() && !isi_tag.starts_with('/') && cocok(&nama, &isi_tag) {
            let kosong = isi_tag.ends_with('/')
                || TAG_KOSONG.iter().any(|t| nama.eq_ignore_ascii_case(t));
            let (akhir, isi_akhir) = if kosong {
                (isi_awal.min(sampai), isi_awal)
            } else {
                let a = akhir_elemen(html, awal, &nama).min(sampai);
                let ia = html[isi_awal..a]
                    .rfind('<')
                    .map(|p| isi_awal + p)
                    .unwrap_or(a);
                (a, ia)
            };
            // `akhir` tidak disimpan: semua yang butuh ujung elemen memakai
            // `isi_akhir` (indeks `<` penutup), sama seperti pembacaan di atas.
            let _ = akhir;
            return Some(Elemen {
                awal,
                isi_awal,
                isi_akhir,
                isi_tag,
            });
        }
        i = isi_awal;
    }
    None
}

/// `$(el).text()` — buang tag, dekode entitas.
fn teks_elemen(html: &str, e: &Elemen) -> String {
    let dalam = html.get(e.isi_awal..e.isi_akhir).unwrap_or("");
    crate::common::decode_entitas(&buang_tag(dalam))
}

fn buang_tag(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut dalam = false;
    for c in s.chars() {
        match c {
            '<' => dalam = true,
            '>' => dalam = false,
            _ if !dalam => out.push(c),
            _ => {}
        }
    }
    out
}

/// `String#trim` JS (termasuk NBSP dan spasi Unicode lain).
fn spasi_js(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}'
        | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}'
        | '\u{3000}' | '\u{FEFF}')
}

fn trim_js(s: &str) -> &str {
    s.trim_matches(spasi_js)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<oktz_plugin_api::HttpCall>>,
        balasan: Mutex<VecDeque<String>>,
        gagal: Option<String>,
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
            unreachable!("scraper ini pakai JSON")
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
            if let Some(pesan) = &self.gagal {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
            let mut q = self.balasan.lock().unwrap();
            let body = if q.len() > 1 {
                q.pop_front().unwrap()
            } else {
                q.front().cloned().unwrap_or_default()
            };
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: Arc::from(body.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(balasan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(VecDeque::from([balasan.to_string()])),
            gagal: None,
        }
    }

    const VIEW: &str = r#"<div class="response-cinema-content">
  <h1 class="response-cinema-title">  Judul Video  </h1>
  <div class="response-cinema-gallery-item">
    <img class="thumbnail-image" src="https://t/1.jpg">
    <a href="/download?token=abc&amp;x=1">Download Video</a>
  </div>
  <div class="response-cinema-gallery-item">
    <a href="/download?token=def">Unduh Gambar</a>
  </div>
</div>"#;

    fn badan(view: &str) -> String {
        format!(
            r#"{{"success":true,"view":{}}}"#,
            serde_json::to_string(view).unwrap()
        )
    }

    #[test]
    fn sukses_membaca_galeri_dan_title() {
        let http = stub(&badan(VIEW));
        let hasil = reddit_dl(&http, "https://reddit.com/r/x/comments/1").unwrap();

        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["title"], "Judul Video");
        let results = hasil["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["item"], 1);
        assert_eq!(results[0]["type"], "video");
        // Entitas HTML `&amp;` di href didekode seperti cheerio.
        assert_eq!(results[0]["thumbnail"], "https://t/1.jpg");
        assert_eq!(results[0]["download_url"], "/download?token=abc&x=1");
        assert_eq!(
            results[0]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["item", "type", "thumbnail", "download_url"]
        );
        // Item kedua tanpa `img`: kunci `thumbnail` tidak ikut.
        assert_eq!(results[1]["item"], 2);
        assert_eq!(results[1]["type"], "image");
        assert!(results[1].get("thumbnail").is_none());

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert!(p[0].url.starts_with("https://redvid.io/fetch?_="));
        let ts = p[0].url.trim_start_matches("https://redvid.io/fetch?_=");
        assert!(ts.parse::<u128>().is_ok(), "timestamp bukan angka: {ts}");
        assert_eq!(
            p[0].body.as_ref().unwrap().bytes(),
            br#"{"url":"https://reddit.com/r/x/comments/1","lang":"en"}"#
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("authority"), Some("redvid.io".to_string()));
        assert_eq!(cari("x-requested-with"), Some("XMLHttpRequest".to_string()));
    }

    #[test]
    fn tanpa_success_mengembalikan_pesan_js() {
        let http = stub(r#"{"success":false}"#);
        let hasil = reddit_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Gagal mengambil data");
    }

    #[test]
    fn view_bukan_string_dibungkus_catch() {
        let http = stub(r#"{"success":true,"view":123}"#);
        let hasil = reddit_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "cheerio.load() expects a string");
    }

    #[test]
    fn error_http_dibungkus_catch_js() {
        let http = StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(VecDeque::new()),
            gagal: Some("HTTP 500".to_string()),
        };
        let hasil = reddit_dl(&http, "https://x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "HTTP 500");
    }
}
