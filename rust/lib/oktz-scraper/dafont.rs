//! `src/scraper/dafont.js` — cari font di dafont.com lalu bedah HTML-nya.
//!
//! JS memakai `cheerio`, di sini diganti pengurai pohon HTML kecil yang
//! disalin dari `sfiledl.rs` (yang butuh `nextAll`, jadi pohon, bukan
//! pengindeks `reddit.rs`). Batasnya jujur: parser HTML penuh tidak ditiru —
//! tag yang tidak ditutup tidak ditebak, dan entitas yang di-decode hanya yang
//! dikenal `common::decode_entitas`.
//!
//! Semantik yang disalin dari JS:
//! - `base.text().replace(/\s+/g, " ").trim()` — semua rentang spasi jadi satu
//!   spasi, lalu trim. `"by ..."` di ujung dibuang dari `name` dengan pola
//!   `/\s*by\s*.+$/i` (huruf besar-kecil tidak berpengaruh).
//! - `$(...).text()` pada himpunan kosong = `""` (bukan `undefined`), jadi
//!   `name`, `author`, dan `license` selalu string.
//! - `.match(...)?.[0] || null` — `downloads`, `yesterday` bisa `null`;
//!   string kosong ikut jadi `null`.
//! - `dl ? "https:" + dl : null` dan `style ? ... : null` — `href` atau `style`
//!   kosong itu falsy, jadi `null`.
//! - `timeout: 15000` milik axios tidak ditiru (batas mesin); `UreqHttp` punya
//!   timeout global 20 detik.
//!
//! Beda yang diketahui:
//! - `axios` melempar untuk di luar 2xx; agar `e.message` sama dengan axios,
//!   permintaan memakai `.terima_semua_status()` dan status diperiksa sendiri
//!   (`Request failed with status code NNN`).
//! - `err.message` untuk kegagalan jaringan milik mesin Rust (`HttpError`).

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{decode_entitas, encode_uri_component};

const HOST: &str = "https://www.dafont.com";

// ---------------------------------------------------------------------------
// Pengurai HTML mini (pengganti `cheerio`; disalin dari `sfiledl.rs`)
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

/// Elemen tanpa tag penutup di HTML.
const VOID: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Elemen yang isinya diperlakukan mentah (tanpa penguraian tag di dalamnya).
const RAW: [&str; 2] = ["script", "style"];

/// Akhir tag (`>`) mulai dari indeks `<` di `mulai`, menghormati tanda kutip
/// nilai atribut supaya `>` di dalam nilai tidak menutup tag.
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

/// Urai `<tag attr="nilai" ...>` jadi nama tag + daftar atribut (nama huruf
/// kecil, nilai apa adanya — decode entitas dilakukan saat dibaca).
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

/// Cari tag penutup `</tag` (tanpa peka huruf besar-kecil) di `sisa`.
/// Balikan `(indeks '<' penutup, indeks setelah '>' penutup)`.
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

/// Urai HTML jadi pohon sederhana. Komentar dan `<!DOCTYPE>` dibuang; `<` yang
/// tidak diikuti huruf dianggap teks biasa seperti htmlparser2.
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

        // Tag penutup: tutup frame sampai nama yang cocok (parser lenient).
        if html[i..].starts_with("</") {
            let Some(pos) = html[i..].find('>') else {
                break;
            };
            let nama = html[i + 2..i + pos]
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ':')
                .collect::<String>()
                .to_ascii_lowercase();
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

        // `<` yang tidak diikuti huruf bukan awal tag (mis. "1 < 2").
        if !html[i + 1..]
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic())
            .unwrap_or(false)
        {
            let akhir = html[i + 1..].find('<').map(|p| i + 1 + p).unwrap_or(html.len());
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Teks(html[i..akhir].to_string()));
            }
            i = akhir;
            continue;
        }

        // Tag buka.
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

    // Elemen yang tidak pernah ditutup tetap naik ke induknya.
    while tumpukan.len() > 1 {
        let anak = tumpukan.pop().expect("lebih dari satu frame");
        if let Some(induk) = tumpukan.last_mut() {
            induk.anak.push(Simpul::Elemen(anak));
        }
    }
    tumpukan.pop().expect("akar")
}

/// `$(el).text()` — teks semua keturunan, entitas HTML di-decode.
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

/// `$(el).attr(nama)`.
fn atribut(el: &Elemen, nama: &str) -> Option<String> {
    el.attrs
        .iter()
        .find(|(k, _)| k == nama)
        .map(|(_, v)| decode_entitas(v))
}

/// `.kelas` — token di atribut `class` dipisah spasi.
fn punya_kelas(el: &Elemen, kelas: &str) -> bool {
    atribut(el, "class")
        .map(|c| c.split_whitespace().any(|k| k == kelas))
        .unwrap_or(false)
}

/// `$(induk).find(...)` — keturunan pertama (urutan dokumen) yang cocok.
fn turunan<'a>(el: &'a Elemen, cocok: &dyn Fn(&Elemen) -> bool) -> Option<&'a Elemen> {
    for simpul in &el.anak {
        if let Simpul::Elemen(e) = simpul {
            if cocok(e) {
                return Some(e);
            }
            if let Some(ketemu) = turunan(e, cocok) {
                return Some(ketemu);
            }
        }
    }
    None
}

/// Posisi elemen di dalam daftar anak induknya.
struct Posisi<'a> {
    induk: &'a Elemen,
    indeks: usize,
}

/// Semua elemen yang punya SEMUA kelas di `daftar` (selector `.a.b`).
fn semua_kelas<'a>(akar: &'a Elemen, daftar: &[&str], keluar: &mut Vec<Posisi<'a>>) {
    for (indeks, simpul) in akar.anak.iter().enumerate() {
        if let Simpul::Elemen(el) = simpul {
            if daftar.iter().all(|k| punya_kelas(el, k)) {
                keluar.push(Posisi { induk: akar, indeks });
            }
            semua_kelas(el, daftar, keluar);
        }
    }
}

/// `$(pos).nextAll(".kelas").first()` — saudara setelah elemen yang cocok.
fn saudara_berikut<'a>(pos: &Posisi<'a>, kelas: &str) -> Option<&'a Elemen> {
    pos.induk
        .anak
        .get(pos.indeks + 1..)?
        .iter()
        .find_map(|s| match s {
            Simpul::Elemen(el) if punya_kelas(el, kelas) => Some(el),
            _ => None,
        })
}

// ---------------------------------------------------------------------------
// Pola regex JS (dikompilasi sekali)
// ---------------------------------------------------------------------------

struct Pola {
    /// `/\s+by\s*.+$/i` — bagian yang dibuang dari nama font.
    buang_by: Regex,
    /// `/[\d,]+ downloads/`
    unduhan: Regex,
    /// `/\((.*?)\)/`
    kurung: Regex,
    /// `/url\((.*?)\)/`
    url_gambar: Regex,
}

fn pola() -> &'static Pola {
    static P: OnceLock<Pola> = OnceLock::new();
    P.get_or_init(|| Pola {
        buang_by: Regex::new(r"(?i)\s*by\s*.+$").expect("pola buang by"),
        unduhan: Regex::new(r"[\d,]+ downloads").expect("pola unduhan"),
        kurung: Regex::new(r"\((.*?)\)").expect("pola kurung"),
        url_gambar: Regex::new(r"url\((.*?)\)").expect("pola url gambar"),
    })
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

/// `s.replace(/\s+/g, " ").trim()` — ditulis tangan, bukan regex, supaya
/// karakter `$` di teks font tidak dibaca sebagai GroupRef.
fn rapatkan_spasi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut ada_spasi = false;
    for c in s.chars() {
        if spasi_js(c) {
            ada_spasi = true;
        } else {
            if ada_spasi {
                out.push(' ');
                ada_spasi = false;
            }
            out.push(c);
        }
    }
    if ada_spasi {
        out.push(' ');
    }
    out
}

// ---------------------------------------------------------------------------
// Alur
// ---------------------------------------------------------------------------

/// `DaFont(query)` (`dafont.js:4-52`).
pub fn dafont(http: &dyn HttpGet, query: &str) -> Result<Value, HttpError> {
    let url = format!("{HOST}/search.php?q={}", encode_uri_component(query));
    let res = http.call(
        &HttpCall::get(url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36",
            )
            .terima_semua_status(),
    )?;
    if !(200..300).contains(&res.status) {
        return Ok(json!({
            "status": false,
            "error": format!("Request failed with status code {}", res.status),
        }));
    }

    let dok = urai(&res.text());
    let mut isi: Vec<Posisi> = Vec::new();
    semua_kelas(&dok, &["lv1left", "dfbg"], &mut isi);

    let mut results: Vec<Value> = Vec::with_capacity(isi.len());
    for pos in &isi {
        let base = match &pos.induk.anak[pos.indeks] {
            Simpul::Elemen(el) => el,
            _ => continue,
        };

        let raw = trim_js(&rapatkan_spasi(&teks_dari(base))).to_string();
        let author = turunan(base, &|el| el.tag == "a")
            .map(|el| trim_js(&teks_dari(el)).to_string())
            .unwrap_or_default();
        // `raw.replace(/\s*by\s*.+$/i, "").trim()`
        let name = trim_js(&pola().buang_by.replace(&raw, "")).to_string();

        let lv2 = saudara_berikut(pos, "lv2right");
        let info = lv2
            .and_then(|lv2| turunan(lv2, &|el| punya_kelas(el, "light")))
            .map(|el| trim_js(&teks_dari(el)).to_string())
            .unwrap_or_default();

        let downloads = pola()
            .unduhan
            .find(&info)
            .map(|m| m.as_str().to_string())
            .filter(|s| !s.is_empty());
        let yesterday = pola()
            .kurung
            .captures(&info)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .filter(|s| !s.is_empty());

        let license = lv2
            .and_then(|lv2| turunan(lv2, &|el| el.tag == "a" && punya_kelas(el, "help")))
            .map(|el| trim_js(&teks_dari(el)).to_string())
            .unwrap_or_default();

        let dl = saudara_berikut(pos, "dlbox")
            .and_then(|dlbox| turunan(dlbox, &|el| el.tag == "a" && punya_kelas(el, "dl")))
            .and_then(|el| atribut(el, "href"))
            // `dl ? ... : null` — `href` kosong itu falsy, jadi `null`.
            .filter(|h| !h.is_empty());
        let download = dl.map(|h| format!("https:{h}"));

        let preview = saudara_berikut(pos, "preview")
            .and_then(|el| atribut(el, "style"))
            // `style ? ... : null` — `style` kosong juga falsy.
            .filter(|s| !s.is_empty())
            .map(|style| {
                let dalam = pola()
                    .url_gambar
                    .captures(&style)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                format!("{HOST}{dalam}")
            });

        let mut o = Map::new();
        o.insert("name".to_string(), Value::String(name));
        o.insert("author".to_string(), Value::String(author));
        o.insert(
            "downloads".to_string(),
            downloads.map(Value::String).unwrap_or(Value::Null),
        );
        o.insert(
            "yesterday".to_string(),
            yesterday.map(Value::String).unwrap_or(Value::Null),
        );
        o.insert("license".to_string(), Value::String(license));
        o.insert(
            "download".to_string(),
            download.map(Value::String).unwrap_or(Value::Null),
        );
        o.insert(
            "preview".to_string(),
            preview.map(Value::String).unwrap_or(Value::Null),
        );
        results.push(Value::Object(o));
    }

    if results.is_empty() {
        return Ok(json!({ "status": false, "error": "Font tidak ditemukan" }));
    }
    Ok(json!({
        "status": true,
        "query": query,
        "count": results.len(),
        "results": results,
    }))
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
    use oktz_plugin_api::HttpBody;
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
            _parts: &[(&str, HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            Ok(dummy_res(self.status, &self.balasan))
        }
    }

    const HALAMAN: &str = r#"<html><body><div class="fontbox">
      <div class="lv1left dfbg">
        <div class="fontname">Bebas Neue</div>
        <div class="author">by <a href="/a/nunito">Nunito</a></div>
        <span class="fontlabel">preview</span>
      </div>
      <div class="lv2right">
        <span class="light"> 12,345 downloads (Yesterday: 12:00 PM)</span>
        <div class="lv2font"><a class="help" href="/licenses/free">Free for personal use</a></div>
      </div>
      <div class="dlbox"><a class="dl" href="//dl.dafont.com/dl.php?f=nunito" title="Download"><b>Download</b></a></div>
      <div class="preview" style="background:url(/img/nunito.png) no-repeat"></div>
      <div class="lv1left dfbg">
        <div class="fontname">Tanpa Detail</div>
      </div>
    </div></body></html>"#;

    #[test]
    fn membaca_semua_kolom_font() {
        let http = StubHttp::baru(200, HALAMAN);
        let hasil = dafont(&http, "bebas neue").unwrap();

        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["query"], "bebas neue");
        assert_eq!(hasil["count"], 2);
        let results = hasil["results"].as_array().unwrap();

        assert_eq!(results[0]["name"], "Bebas Neue");
        assert_eq!(results[0]["author"], "Nunito");
        assert_eq!(results[0]["downloads"], "12,345 downloads");
        assert_eq!(results[0]["yesterday"], "Yesterday: 12:00 PM");
        assert_eq!(results[0]["license"], "Free for personal use");
        assert_eq!(results[0]["download"], "https://dl.dafont.com/dl.php?f=nunito");
        assert_eq!(results[0]["preview"], "https://www.dafont.com/img/nunito.png");
        assert_eq!(
            results[0]
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            [
                "name",
                "author",
                "downloads",
                "yesterday",
                "license",
                "download",
                "preview"
            ]
        );

        // Font tanpa `.lv2right`/`.dlbox`/`.preview` → semua kosong, bukan null
        // untuk `author`/`license`.
        assert_eq!(results[1]["name"], "Tanpa Detail");
        assert_eq!(results[1]["author"], "");
        assert_eq!(results[1]["license"], "");
        assert_eq!(results[1]["downloads"], Value::Null);
        assert_eq!(results[1]["yesterday"], Value::Null);
        assert_eq!(results[1]["download"], Value::Null);
        assert_eq!(results[1]["preview"], Value::Null);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].url, "https://www.dafont.com/search.php?q=bebas%20neue");
        assert!(p[0]
            .headers
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("user-agent") && v.contains("Chrome/147.0.0.0")));
    }

    #[test]
    fn nama_memotong_setiap_kemunculan_by_dari_ujung() {
        let http = StubHttp::baru(
            200,
            r#"<div class="lv1left dfbg">Font  X   by odi  by eye</div>"#,
        );
        let hasil = dafont(&http, "x").unwrap();
        assert_eq!(hasil["results"][0]["name"], "Font X");
    }

    #[test]
    fn style_tanpa_url_masih_menghasilkan_host_kosong() {
        let http = StubHttp::baru(
            200,
            r#"<div class="lv1left dfbg">A</div><div class="preview" style="width:10px"></div>"#,
        );
        let hasil = dafont(&http, "x").unwrap();
        assert_eq!(hasil["results"][0]["preview"], "https://www.dafont.com");
    }

    #[test]
    fn href_dan_style_kosong_jadi_null_bukan_string_kosong() {
        let http = StubHttp::baru(
            200,
            r#"<div class="lv1left dfbg">A</div><div class="dlbox"><a class="dl" href=""></a></div><div class="preview" style=""></div>"#,
        );
        let hasil = dafont(&http, "x").unwrap();
        assert_eq!(hasil["results"][0]["download"], Value::Null);
        assert_eq!(hasil["results"][0]["preview"], Value::Null);
    }

    #[test]
    fn tanpa_font_mengembalikan_pesan_yang_persis() {
        let http = StubHttp::baru(200, "<html><body><p>Tidak ada</p></body></html>");
        let hasil = dafont(&http, "zzz").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Font tidak ditemukan");
        assert!(hasil.get("results").is_none());
        assert!(hasil.get("query").is_none());
    }

    #[test]
    fn status_bukan_2xx_mengikuti_pesan_axios() {
        let http = StubHttp::baru(403, "");
        let hasil = dafont(&http, "x").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "Request failed with status code 403");
    }

    #[test]
    fn parser_mengabaikan_komentar_dan_void_serta_merawat_saudara() {
        let dok = urai(
            r#"<ul><li class="a">satu<!-- <li class="b">palsu</li> --><br><img src="x"></li><li class="a">dua</li></ul>"#,
        );
        let mut isi = Vec::new();
        semua_kelas(&dok, &["a"], &mut isi);
        assert_eq!(isi.len(), 2, "li di dalam komentar ikut terhitung");
        let (_, indeks) = (isi[0].induk, isi[0].indeks);
        let el = match &isi[0].induk.anak[indeks] {
            Simpul::Elemen(el) => el,
            _ => panic!("harus elemen"),
        };
        assert_eq!(teks_dari(el), "satu");
        assert_eq!(
            turunan(el, &|e| e.tag == "img")
                .and_then(|e| atribut(e, "src"))
                .as_deref(),
            Some("x")
        );
    }
}