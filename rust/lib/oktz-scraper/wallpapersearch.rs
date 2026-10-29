//! `src/scraper/wallpapersearch.js` — ambil `<li itemprop="associatedMedia">`
//! dari halaman pencarian wallpaperflare.com.
//!
//! Cheerio tidak ada di crate ini, jadi selector yang dipakai file ini
//! dijawab dengan pengindeks HTML kecil di bawah (`elemen_mulai` +
//! `atribut` + `teks_elemen`), disalin dari `reddit.rs` supaya gayanya sama.
//! Batasnya jujur: tag yang punya `>` di dalam nilai atribut tidak diparse
//! persis seperti parse5, dan penutup tag yang hilang tidak ditebak
//! (cheerio/htmlparser2 menutupnya otomatis).
//!
//! Semantik yang disalin dari JS:
//! - `$(el).text()` pada himpunan kosong menghasilkan `""`, jadi `title`,
//!   `resolution`, dan `license`-miripnya selalu string (bisa kosong), bukan
//!   `undefined`.
//! - `if (image && page)` — `data-src`/`href` kosong dipakai sebagai falsy,
//!   item lalu dibuang seluruhnya (tidak ada kunci dengan nilai kosong).
//! - Href relatif dikembalikan setelah digabung dengan host tetap,
//!   `https://www.wallpaperflare.com${page}`.
//! - `!results.length` → `{ success: false, message }` dengan teks persis;
//!   `catch` → `{ success: false, error: error.message }`.
//!
//! Beda yang diketahui:
//! - `axios` melempar untuk di luar 2xx; agar `error.message` tetap sama
//!   dengan axios, permintaan memakai `.terima_semua_status()` dan status
//!   diperiksa sendiri (`Request failed with status code NNN`).
//! - `err.message` untuk kegagalan jaringan milik mesin Rust (`HttpError`).

use std::sync::LazyLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::{decode_entitas, encode_uri_component};

const HOST: &str = "https://www.wallpaperflare.com";

/// `wallpaperScraper(query)` (`wallpapersearch.js:3-56`).
pub fn wallpaper_scraper(http: &dyn HttpGet, query: &str) -> Result<Value, HttpError> {
    let url = format!("{HOST}/search?wallpaper={}", encode_uri_component(query));
    let res = http.call(
        &HttpCall::get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            )
            .terima_semua_status(),
    )?;
    if !(200..300).contains(&res.status) {
        return Ok(json!({
            "success": false,
            "error": format!("Request failed with status code {}", res.status),
        }));
    }

    let html = res.text();
    let mut results: Vec<Value> = Vec::new();
    let mut pos = 0usize;
    while let Some(el) = elemen_mulai(&html, pos, html.len(), &|nama, isi| {
        nama.eq_ignore_ascii_case("li") && atribut(isi, "itemprop").as_deref() == Some("associatedMedia")
    }) {
        let title = elemen_mulai(&html, el.isi_awal, el.akhir, &|nama, isi| {
            nama.eq_ignore_ascii_case("figcaption")
                && atribut(isi, "itemprop").as_deref() == Some("caption description")
        })
        .map(|e| trim_js(&teks_elemen(&html, &e)).to_string())
        .unwrap_or_default();

        let image = elemen_mulai(&html, el.isi_awal, el.akhir, &|nama, _| {
            nama.eq_ignore_ascii_case("img")
        })
        .and_then(|e| atribut(&e.isi_tag, "data-src"));

        let page = elemen_mulai(&html, el.isi_awal, el.akhir, &|nama, isi| {
            nama.eq_ignore_ascii_case("a") && atribut(isi, "itemprop").as_deref() == Some("url")
        })
        .and_then(|e| atribut(&e.isi_tag, "href"));

        let resolution = elemen_mulai(&html, el.isi_awal, el.akhir, &|_, isi| {
            punya_kelas(isi, "res")
        })
        .map(|e| trim_js(&teks_elemen(&html, &e)).to_string())
        .unwrap_or_default();

        if let (Some(image), Some(page)) = (image, page)
            && !image.is_empty()
            && !page.is_empty()
        {
            let mut o = Map::new();
            o.insert("title".to_string(), Value::String(title));
            o.insert("resolution".to_string(), Value::String(resolution));
            o.insert("image".to_string(), Value::String(image));
            o.insert("page".to_string(), Value::String(format!("{HOST}{page}")));
            results.push(Value::Object(o));
        }
        // Maju satu karakter supaya `li` di dalam `li` juga ikut, seperti
        // cheerio yang mencocokkan semua elemen yang cocok.
        pos = el.awal + 1;
    }

    if results.is_empty() {
        return Ok(json!({
            "success": false,
            "message": "Tidak ditemukan wallpaper",
        }));
    }
    Ok(json!({
        "success": true,
        "total": results.len(),
        "results": results,
    }))
}

// ── bedah HTML kecil (pengganti cheerio; lihat reddit.rs) ───────────────────

struct Elemen {
    /// Indeks `<` pembuka.
    awal: usize,
    /// Indeks setelah `>` pembuka.
    isi_awal: usize,
    /// Indeks `<` penutup (kalau tidak ada, panjang html).
    akhir: usize,
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
/// `nama_tag` untuk tag penutup tetap nama tagnya (`</h1>` → `h1`). Komentar
/// dan doctype punya `nama` kosong.
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

/// Indeks `<` penutup yang berpasangan dengan pembuka di `awal`.
fn akhir_elemen(html: &str, awal: usize, nama: &str) -> usize {
    let mut depth = 0usize;
    let mut i = awal;
    while let Some((pos, isi_awal, n, isi)) = tag_berikut(html, i) {
        if n.eq_ignore_ascii_case(nama) {
            if isi.starts_with('/') {
                if depth <= 1 {
                    // `pos` = indeks `<` penutup yang berpasangan.
                    return pos;
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
    Some(decode_entitas(nilai.as_str()))
}

/// `.kelas` ala CSS: token di atribut `class` dipisah spasi.
fn punya_kelas(tag: &str, kelas: &str) -> bool {
    atribut(tag, "class")
        .map(|c| c.split_whitespace().any(|k| k == kelas))
        .unwrap_or(false)
}

/// Elemen pertama di `[dari, sampai)` yang cocok — padanan
/// `$(...).first()` untuk pemakaian di file ini.
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
            let akhir = if kosong {
                isi_awal
            } else {
                akhir_elemen(html, awal, &nama)
            };
            return Some(Elemen {
                awal,
                isi_awal,
                akhir: akhir.min(sampai),
                isi_tag,
            });
        }
        i = isi_awal;
    }
    None
}

/// `$(el).text()` — buang tag, dekode entitas.
fn teks_elemen(html: &str, e: &Elemen) -> String {
    let dalam = html.get(e.isi_awal..e.akhir).unwrap_or("");
    decode_entitas(&buang_tag(dalam))
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

/// Bantu test: balas `HttpResponse` tetap tanpa jaringan.
#[cfg(test)]
fn dummy_res(status: u16, body: &str) -> oktz_plugin_api::HttpResponse {
    oktz_plugin_api::HttpResponse {
        status,
        body: oktz_plugin_api::HttpBody::Bytes {
            data: std::sync::Arc::from(body.as_bytes()),
            mimetype: None,
        },
        headers: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan, membalas status +
    /// body tetap.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        status: u16,
        balasan: String,
    }

    impl StubHttp {
        fn baru(status: u16, balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                status,
                balasan: balasan.to_string(),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn call(
            &self,
            req: &HttpCall,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            Ok(dummy_res(self.status, &self.balasan))
        }
    }

    const HALAMAN: &str = r#"<html><body><ul>
      <li itemprop="associatedMedia">
        <a itemprop="url" href="/wallpaper/gunung-1.html"><img class="wthumb" data-src="https://img/1.jpg"></a>
        <figcaption itemprop="caption description">  Sunset di gunung  </figcaption>
        <span class="res"> 1920 x 1080 </span>
      </li>
      <li itemprop="associatedMedia">
        <a itemprop="url" href="/wallpaper/laut-2.html"><img class="wthumb"></a>
        <figcaption itemprop="caption description">Laut</figcaption>
        <span class="res"> 3840 x 2160 </span>
      </li>
      <li itemprop="associatedMedia">
        <a itemprop="url" href="/wallpaper/tanpa-gambar.html"><img data-src=""></a>
      </li>
      <li><span>bukan media</span></li>
    </ul></body></html>"#;

    #[test]
    fn mengambil_item_yang_lengkap_saja() {
        let http = StubHttp::baru(200, HALAMAN);
        let hasil = wallpaper_scraper(&http, "gunung").unwrap();

        assert_eq!(hasil["success"], true);
        assert_eq!(hasil["total"], 1);
        let results = hasil["results"].as_array().unwrap();
        assert_eq!(results[0]["title"], "Sunset di gunung");
        assert_eq!(results[0]["resolution"], "1920 x 1080");
        assert_eq!(results[0]["image"], "https://img/1.jpg");
        assert_eq!(
            results[0]["page"],
            "https://www.wallpaperflare.com/wallpaper/gunung-1.html"
        );
        assert_eq!(
            results[0]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["title", "resolution", "image", "page"]
        );

        // Query di-encode dengan `encodeURIComponent` (spasi `%20`).
        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(
            p[0].url,
            "https://www.wallpaperflare.com/search?wallpaper=gunung"
        );
        assert!(p[0]
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("user-agent") && v.contains("Chrome/120.0.0.0")));
    }

    #[test]
    fn query_dengan_spasi_menghasilkan_persen20() {
        let http = StubHttp::baru(200, HALAMAN);
        wallpaper_scraper(&http, "gunung laut").unwrap();
        let p = http.permintaan.lock().unwrap();
        assert!(p[0].url.ends_with("?wallpaper=gunung%20laut"), "{}", p[0].url);
    }

    #[test]
    fn tanpa_item_mengembalikan_pesan_yang_persis() {
        for balasan in [
            "<html><body><ul><li><span>kosong</span></li></ul></body></html>",
            r#"<html><body><ul><li itemprop="associatedMedia"><figcaption itemprop="caption description">x</figcaption><span class="res">1</span></li></ul></body></html>"#,
            "",
        ] {
            let http = StubHttp::baru(200, balasan);
            let hasil = wallpaper_scraper(&http, "x").unwrap();
            assert_eq!(hasil["success"], false);
            assert_eq!(hasil["message"], "Tidak ditemukan wallpaper");
            assert!(hasil.get("results").is_none());
        }
    }

    #[test]
    fn status_bukan_2xx_mengikuti_pesan_axios() {
        let http = StubHttp::baru(503, "");
        let hasil = wallpaper_scraper(&http, "x").unwrap();
        assert_eq!(hasil["success"], false);
        assert_eq!(hasil["error"], "Request failed with status code 503");
        assert!(hasil.get("message").is_none());
    }

    #[test]
    fn elemen_tanpa_isi_tidak_menelan_isi_sesudahnya() {
        // `<br>` dan `<img>` tidak punya penutup; teks setelahnya harus tetap
        // terbaca oleh `$(el).text()`.
        let dok = "<div class=\"res\">abc<br>def</div><p>luar</p>";
        let el = elemen_mulai(dok, 0, dok.len(), &|_, isi| punya_kelas(isi, "res")).unwrap();
        assert_eq!(teks_elemen(dok, &el), "abcdef");
        let img = elemen_mulai(dok, 0, dok.len(), &|nama, _| nama.eq_ignore_ascii_case("img"));
        assert!(img.is_none(), "dokumen uji tidak punya img");
    }
}