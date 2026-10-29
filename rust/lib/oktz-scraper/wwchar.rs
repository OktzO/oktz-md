//! `src/scraper/wwchar.js` — scraper karakter Wuthering Waves dari fandom.
//!
//! Yang disalin persis:
//! - `slug = name.trim().replace(/\s+/g, "_")`; `clean` mengembalikan `null`
//!   kalau kosong (beda dari hokinfo yang mengembalikan string kosong).
//! - Kunci profil: `label.toLowerCase().replace(/[^a-z0-9]/g, "_")` lalu
//!   `_` di ujung dibuang; label/nilai kosong dilewati.
//! - Gambar: `img, noscript img`; `data-src || src`, jatuhan
//!   `data-srcset || srcset` potongan pertama; hanya URL
//!   `static.wikia.nocookie.net` yang memuat `pageSlug`; `/revision/`…
//!   dinormalkan ke `/revision/latest`; duplikat dibuang (`Set`).
//!
//! Cheerio tidak ada di Rust: HTML dipindai regex sederhana (tag seimbang).
//! Perbedaan yang diketahui ada di laporan.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value};

use crate::common::{decode_entitas, encode_uri_component};

/// `scrapeWutheringWavesCharacter(name)` (`wwchar.js:2-75`).
pub fn scrape_wuthering_waves_character(
    http: &dyn HttpGet,
    name: &str,
) -> Result<Value, HttpError> {
    if name.is_empty() {
        return Err(HttpError {
            message: "Nama karakter kosong".to_string(),
        });
    }

    let slug = name.split_whitespace().collect::<Vec<_>>().join("_");
    let url = format!(
        "https://wutheringwaves.fandom.com/wiki/{}",
        encode_uri_component(&slug)
    );
    let res = http.call(
        &HttpCall::get(&url)
            .header("user-agent", "Mozilla/5.0")
            .terima_semua_status(),
    )?;
    if !(200..=299).contains(&res.status) {
        return Err(HttpError {
            message: "Data tidak ditemukan".to_string(),
        });
    }
    let html = res.text();
    let els = pindai(&html);

    let title = cari(&els, |e| e.attr("id").as_deref() == Some("firstHeading"))
        .map(|i| teks(els[i].isi()))
        .and_then(|t| clean(&t));
    let Some(title) = title else {
        return Err(HttpError {
            message: "Halaman tidak valid".to_string(),
        });
    };

    let bio = cari(&els, |e| e.kelas("mw-parser-output"))
        .and_then(|k| {
            els.iter()
                .position(|e| e.induk == Some(k) && e.nama.eq_ignore_ascii_case("p"))
        })
        .map(|i| teks(els[i].isi()))
        .and_then(|t| clean(&t));

    let mut profile = Map::new();
    for e in els.iter().filter(|e| e.kelas("pi-item") && e.kelas("pi-data")) {
        let label = els
            .iter()
            .find(|x| dalam_isi(x, e) && x.kelas("pi-data-label"))
            .map(|x| teks(x.isi()))
            .and_then(|t| clean(&t));
        let nilai = els
            .iter()
            .find(|x| dalam_isi(x, e) && x.kelas("pi-data-value"))
            .map(|x| teks(x.isi()))
            .and_then(|t| clean(&t));
        let (Some(label), Some(nilai)) = (label, nilai) else {
            continue;
        };
        let kunci: String = label
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_ascii_lowercase() || c.is_ascii_digit() {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let kunci = kunci.trim_matches('_').to_string();
        profile.insert(kunci, Value::String(nilai));
    }

    let page_slug = slug.to_lowercase();
    let mut gambar: Vec<String> = Vec::new();
    for e in els.iter().filter(|e| e.nama.eq_ignore_ascii_case("img")) {
        let mut src = e
            .attr("data-src")
            .filter(|s| !s.is_empty())
            .or_else(|| e.attr("src").filter(|s| !s.is_empty()));
        let srcset = e
            .attr("data-srcset")
            .filter(|s| !s.is_empty())
            .or_else(|| e.attr("srcset").filter(|s| !s.is_empty()));
        if src.is_none()
            && let Some(ss) = &srcset {
                src = ss
                    .split(',')
                    .next()
                    .and_then(|p| p.split(' ').next())
                    .map(|s| s.to_string());
            }
        let Some(src) = src else { continue };
        if !src.contains("static.wikia.nocookie.net") {
            continue;
        }
        if !src.to_lowercase().contains(&page_slug) {
            continue;
        }
        let kanon = format!(
            "{}/revision/latest",
            src.split("/revision/").next().unwrap_or(&src)
        );
        if !gambar.contains(&kanon) {
            gambar.push(kanon);
        }
    }

    let mut out = Map::new();
    out.insert("title".to_string(), Value::String(title));
    out.insert("slug".to_string(), Value::String(slug));
    out.insert("url".to_string(), Value::String(url));
    out.insert(
        "bio".to_string(),
        bio.map(Value::String).unwrap_or(Value::Null),
    );
    out.insert("profile".to_string(), Value::Object(profile));
    out.insert(
        "images".to_string(),
        Value::Array(gambar.into_iter().map(Value::String).collect()),
    );
    Ok(Value::Object(out))
}

/// `clean` wwchar: `v?.replace(/\s+/g, " ").trim() || null`.
fn clean(v: &str) -> Option<String> {
    let s = bersih(v);
    if s.is_empty() { None } else { Some(s) }
}

/// Tag HTML yang tidak perlu penutup.
const TAG_VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Satu elemen hasil pemindaian HTML sederhana.
struct Elemen<'a> {
    /// Nama tag seperti tertulis di dokumen.
    nama: &'a str,
    /// Tag pembuka mentah, untuk membaca atribut.
    tag: &'a str,
    /// Offset awal isi elemen (setelah tag pembuka).
    isi_mulai: usize,
    /// Offset akhir elemen (setelah tag penutup; akhir dokumen kalau tak
    /// ditutup).
    akhir: usize,
    /// Indeks induk di daftar hasil; `None` untuk elemen paling luar.
    induk: Option<usize>,
    /// Dokumen asal, untuk memotong isi elemen.
    dokumen: &'a str,
}

impl Elemen<'_> {
    /// Isi elemen tanpa tag pembuka/penutup.
    fn isi(&self) -> &str {
        &self.dokumen[self.isi_mulai..self.akhir]
    }

    /// `$(...).attr("nama")` — nilai atribut (entitas HTML didekode).
    fn attr(&self, nama: &str) -> Option<String> {
        attr(self.tag, nama)
    }

    /// `$(...).hasClass("kelas")`.
    fn kelas(&self, kelas: &str) -> bool {
        punya_kelas(self.tag, kelas)
    }
}

/// Pemindai tag sederhana, bukan parser HTML penuh: komentar dan isi
/// `<script>`/`<style>` ikut dipindai, tag yang tidak ditutup menelan sisa
/// dokumen. Cukup untuk halaman target (tag seimbang).
fn pindai(html: &str) -> Vec<Elemen<'_>> {
    let re = Regex::new(r"(?is)</?([a-z][a-z0-9-]*)\b[^>]*>").unwrap();
    let mut hasil: Vec<Elemen> = Vec::new();
    let mut buka: Vec<usize> = Vec::new();
    for cap in re.captures_iter(html) {
        let m = cap.get(0).unwrap();
        let tag = m.as_str();
        let nama = cap.get(1).map(|c| c.as_str()).unwrap_or("");
        if tag.starts_with("</") {
            if let Some(pos) = buka
                .iter()
                .rposition(|&i| hasil[i].nama.eq_ignore_ascii_case(nama))
            {
                for &i in &buka[pos..] {
                    hasil[i].akhir = m.start();
                }
                buka.truncate(pos);
            }
            continue;
        }
        let induk = buka.last().copied();
        let tutup_sendiri =
            tag.ends_with("/>") || TAG_VOID.iter().any(|v| v.eq_ignore_ascii_case(nama));
        hasil.push(Elemen {
            nama,
            tag,
            isi_mulai: m.end(),
            akhir: if tutup_sendiri { m.end() } else { html.len() },
            induk,
            dokumen: html,
        });
        if !tutup_sendiri {
            buka.push(hasil.len() - 1);
        }
    }
    hasil
}

/// Nilai atribut `nama` dari tag mentah; `None` kalau atribut tidak ada.
fn attr(tag: &str, nama: &str) -> Option<String> {
    let pola = format!(
        r#"(?is)(?:^|\s){}\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+))"#,
        regex::escape(nama)
    );
    let re = Regex::new(&pola).unwrap();
    let cap = re.captures(tag)?;
    let nilai = cap
        .get(1)
        .or_else(|| cap.get(2))
        .or_else(|| cap.get(3))?
        .as_str();
    Some(decode_entitas(nilai))
}

/// `$(...).hasClass(...)` — token kelas dipisah spasi.
fn punya_kelas(tag: &str, kelas: &str) -> bool {
    attr(tag, "class")
        .map(|c| c.split_whitespace().any(|k| k == kelas))
        .unwrap_or(false)
}

/// `$(...).text()` — tag dibuang, entitas HTML didekode.
fn teks(isi: &str) -> String {
    let re = Regex::new(r"(?is)<[^>]*>").unwrap();
    decode_entitas(&re.replace_all(isi, ""))
}

/// `clean` hokinfo/wwchar: `replace(/\s+/g, " ").trim()`.
fn bersih(s: &str) -> String {
    let re = Regex::new(r"\s+").unwrap();
    re.replace_all(s, " ").trim().to_string()
}

/// Indeks elemen pertama yang cocok.
fn cari(els: &[Elemen], pred: impl Fn(&Elemen) -> bool) -> Option<usize> {
    els.iter().position(pred)
}

/// `e` berada di dalam elemen `wadah` (bukan wadah itu sendiri).
fn dalam_isi(e: &Elemen, wadah: &Elemen) -> bool {
    e.isi_mulai > wadah.isi_mulai && e.akhir <= wadah.akhir
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        status: u16,
        balasan: String,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("wwchar lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("wwchar cuma GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("wwchar cuma GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("wwchar cuma GET")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            Ok(oktz_plugin_api::HttpResponse {
                status: self.status,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: std::sync::Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(balasan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            status: 200,
            balasan: balasan.to_string(),
        }
    }

    #[test]
    fn profil_gambar_dan_jatuhan_srcset() {
        let http = stub(
            r#"<html><body>
                 <h1 id="firstHeading">Wuthering Waves</h1>
                 <div class="mw-parser-output"><p>Bio singkat.</p></div>
                 <div class="pi-item pi-data">
                   <div class="pi-data-label">Base HP</div>
                   <div class="pi-data-value">1,200</div>
                 </div>
                 <noscript><img data-src="https://static.wikia.nocookie.net/wutheringwaves/images/a/Wuthering_Waves.png/revision/latest?cb=1"></noscript>
                 <img data-srcset="https://static.wikia.nocookie.net/wutheringwaves/images/b/Wuthering_Waves_b.png/revision/latest?cb=2 1x, https://lain/x.png 2x">
                 <img src="https://example.com/wuthering_waves.png">
                 <img src="https://static.wikia.nocookie.net/lain/Karakter_Lain.png/revision/latest?cb=3">
                 <img data-src="https://static.wikia.nocookie.net/wutheringwaves/images/a/Wuthering_Waves.png/revision/latest?cb=9">
               </body></html>"#,
        );
        let hasil = scrape_wuthering_waves_character(&http, "Wuthering Waves").unwrap();
        assert_eq!(hasil["title"], "Wuthering Waves");
        assert_eq!(hasil["slug"], "Wuthering_Waves");
        assert_eq!(
            hasil["url"],
            "https://wutheringwaves.fandom.com/wiki/Wuthering_Waves"
        );
        assert_eq!(hasil["bio"], "Bio singkat.");
        assert_eq!(hasil["profile"]["base_hp"], "1,200");
        assert_eq!(
            hasil["images"],
            json!([
                "https://static.wikia.nocookie.net/wutheringwaves/images/a/Wuthering_Waves.png/revision/latest",
                "https://static.wikia.nocookie.net/wutheringwaves/images/b/Wuthering_Waves_b.png/revision/latest",
            ])
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(
            p[0].url,
            "https://wutheringwaves.fandom.com/wiki/Wuthering_Waves"
        );
        assert!(p[0].terima_semua_status);
    }

    #[test]
    fn kunci_profil_disaring_dan_bio_kosong_null() {
        let http = stub(
            r#"<html><body>
                 <h1 id="firstHeading">X</h1>
                 <div class="pi-item pi-data">
                   <div class="pi-data-label">Element!!!</div>
                   <div class="pi-data-value">Anemo</div>
                 </div>
                 <div class="pi-item pi-data">
                   <div class="pi-data-label">   </div>
                   <div class="pi-data-value">y</div>
                 </div>
               </body></html>"#,
        );
        let hasil = scrape_wuthering_waves_character(&http, "X").unwrap();
        assert_eq!(hasil["bio"], Value::Null);
        assert_eq!(hasil["profile"]["element"], "Anemo");
        assert_eq!(hasil["profile"].as_object().unwrap().len(), 1);
        assert_eq!(hasil["images"], json!([]));
    }

    #[test]
    fn error_pesan_js() {
        let http = stub("<html></html>");
        assert_eq!(
            scrape_wuthering_waves_character(&http, "")
                .unwrap_err()
                .message,
            "Nama karakter kosong"
        );
        assert_eq!(
            scrape_wuthering_waves_character(&http, "X")
                .unwrap_err()
                .message,
            "Halaman tidak valid"
        );

        let mut http = stub("<html></html>");
        http.status = 404;
        assert_eq!(
            scrape_wuthering_waves_character(&http, "X")
                .unwrap_err()
                .message,
            "Data tidak ditemukan"
        );
    }
}
