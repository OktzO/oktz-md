//! `src/scraper/gsmarena.js` — katalog HP GSMArena.
//!
//! JS membungkus paket npm `gsmarena-api`
//! (`node_modules/gsmarena-api/src/services/{search,catalog,utils}.js`); di sini
//! permintaan HTTP dan penguraian HTML-nya ditulis ulang karena cheerio tidak
//! tersedia di crate ini. Empat fungsi yang diekspor JS dipetakan satu-satu.
//!
//! Yang ikut disalin dari paket itu:
//! - `axios({ method: 'get', url })` tanpa header khusus → `HttpCall::get` polos.
//! - `getBrands`/`getDevices` memakai `$(el).find('a').attr('href').replace('.php','')`;
//!   `attr`/`html` yang `undefined`/`null` membuat JS melempar `TypeError`, dan itu
//!   jadi `Err` dengan pesan V8 yang sama.
//! - `.replace(' devices','')` hanya kemunculan PERTAMA; `.replace(/[0-9]/g,'')`
//!   membuang semua angka; `parseInt(...)` yang `NaN` menjadi `null` di
//!   `JSON.stringify`.
//! - `getBrand` mengikuti halaman berikutnya selama ada
//!   `a.prevnextbutton[title="Next page"]`.
//!
//! Beda yang diketahui: `while (getNextPage($))` di JS tidak punya batas halaman,
//! jadi tautan "Next page" yang tidak pernah maju membuat JS berputar tanpa henti.
//! Di sini halaman yang sudah diambil dilewati supaya scraper tidak mengunci
//! worker.
#![allow(clippy::too_many_lines)]

use std::sync::{LazyLock, Mutex};

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use serde_json::{Map, Value, json};

use crate::common::parse_int_js;

/// `utils.js:6` — prefix semua permintaan.
const BASE: &str = "https://www.gsmarena.com";

/// `search(query)` (`gsmarena.js:3-5` → `search.js:4`).
pub fn search(http: &dyn HttpGet, query: &str) -> Result<Value, HttpError> {
    // `searchValue` disisipkan apa adanya — JS tidak melakukan encode.
    let html = ambil(http, &format!("/results.php3?sQuickSearch=yes&sName={query}"))?;
    let li = di_dalam(
        &html,
        &semua_elemen(&html, 0, html.len(), &|_, isi| punya_kelas(isi, "makers")),
        &|nama, _| nama.eq_ignore_ascii_case("li"),
    );

    let mut hasil: Vec<Value> = Vec::new();
    for e in li {
        let img = pertama_elemen(&html, e.isi_awal, e.isi_akhir, &|nama, _| {
            nama.eq_ignore_ascii_case("img")
        });
        let id = id_dari_a(&html, &e)?;
        let src = img.as_ref().and_then(|i| atribut(&i.isi_tag, "src"));
        let title = img.as_ref().and_then(|i| atribut(&i.isi_tag, "title"));
        // `.find('span').html()` menghasilkan `null` untuk pilihan kosong, jadi
        // `.split` di JS melempar TypeError.
        let Some(span) = span_pertama(&html, &e) else {
            return Err(tipe_error("split", true));
        };
        // `.html().split('<br>').join(' ')` — Unlike `getDevices` yang memakai
        // `.text()`, `<br>` di sini diganti spasi, bukan dibuang.
        let name = html_dalam(&html, &span).split("<br>").collect::<Vec<_>>().join(" ");
        let mut o = Map::new();
        o.insert("id".to_string(), json!(id));
        o.insert("name".to_string(), json!(name));
        sisipkan_atribut(&mut o, "img", src);
        sisipkan_atribut(&mut o, "description", title);
        hasil.push(Value::Object(o));
    }
    Ok(Value::Array(hasil))
}

/// `detail(deviceId)` (`gsmarena.js:7-9` → `catalog.js:65`).
pub fn detail(http: &dyn HttpGet, device_id: &str) -> Result<Value, HttpError> {
    let html = ambil(http, &format!("/{device_id}.php"))?;

    // `quickSpec` (`catalog.js:78-86`) — urutan pushes ikut, termasuk yang kosong.
    let quick: Vec<Value> = [
        (
            "Display size",
            teks_selector(&html, &|nama, isi| {
                nama.eq_ignore_ascii_case("span")
                    && atribut_bernilai(isi, "data-spec", "displaysize-hl")
            }),
        ),
        (
            "Display resolution",
            teks_selector(&html, &|nama, isi| {
                nama.eq_ignore_ascii_case("div")
                    && atribut_bernilai(isi, "data-spec", "displayres-hl")
            }),
        ),
        (
            "Camera pixels",
            teks_selector(&html, &|_, isi| punya_kelas(isi, "accent-camera")),
        ),
        (
            "Video pixels",
            teks_selector(&html, &|nama, isi| {
                nama.eq_ignore_ascii_case("div")
                    && atribut_bernilai(isi, "data-spec", "videopixels-hl")
            }),
        ),
        (
            "RAM size",
            teks_selector(&html, &|_, isi| punya_kelas(isi, "accent-expansion")),
        ),
        (
            "Chipset",
            teks_selector(&html, &|nama, isi| {
                nama.eq_ignore_ascii_case("div")
                    && atribut_bernilai(isi, "data-spec", "chipset-hl")
            }),
        ),
        (
            "Battery size",
            teks_selector(&html, &|_, isi| punya_kelas(isi, "accent-battery")),
        ),
        (
            "Battery type",
            teks_selector(&html, &|nama, isi| {
                nama.eq_ignore_ascii_case("div")
                    && atribut_bernilai(isi, "data-spec", "battype-hl")
            }),
        ),
    ]
    .into_iter()
    .map(|(name, value)| json!({ "name": name, "value": value }))
    .collect();

    // `detailSpec` (`catalog.js:91-111`).
    let mut detail_spec: Vec<Value> = Vec::new();
    for tabel in semua_di(&html, &|nama, _| nama.eq_ignore_ascii_case("table")) {
        let category = teks_semua(
            &html,
            &semua_elemen(&html, tabel.isi_awal, tabel.isi_akhir, &|nama, _| {
                nama.eq_ignore_ascii_case("th")
            }),
        );
        // `if (category)` — tabel tanpa `th` tidak jadi kategori.
        if category.is_empty() {
            continue;
        }
        let mut spesifikasi: Vec<Value> = Vec::new();
        for tr in semua_elemen(&html, tabel.isi_awal, tabel.isi_akhir, &|nama, _| {
            nama.eq_ignore_ascii_case("tr")
        }) {
            // `$('td.ttl', ele)` dan `$('td.nfo', ele)` — cakupannya `tr` itu,
            // dan `.text()` digabung dari semua `td` yang cocok di dalamnya.
            spesifikasi.push(json!({
                "name": teks_td(&html, tr.isi_awal, tr.isi_akhir, "ttl"),
                "value": teks_td(&html, tr.isi_awal, tr.isi_akhir, "nfo"),
            }));
        }
        detail_spec.push(json!({ "category": category, "specifications": spesifikasi }));
    }

    // `.specs-photo-main a img` — img di dalam `a`, bukan img langsung di div.
    let foto = semua_di(&html, &|_, isi| punya_kelas(isi, "specs-photo-main"))
        .iter()
        .flat_map(|d| {
            semua_elemen(&html, d.isi_awal, d.isi_akhir, &|nama, _| {
                nama.eq_ignore_ascii_case("a")
            })
        })
        .flat_map(|a| {
            semua_elemen(&html, a.isi_awal, a.isi_akhir, &|nama, _| {
                nama.eq_ignore_ascii_case("img")
            })
        })
        .next()
        .and_then(|i| atribut(&i.isi_tag, "src"));

    let mut o = Map::new();
    o.insert(
        "name".to_string(),
        json!(teks_selector(
            &html,
            &|_, isi| punya_kelas(isi, "specs-phone-name-title")
        )),
    );
    sisipkan_atribut(&mut o, "img", foto);
    o.insert("detailSpec".to_string(), Value::Array(detail_spec));
    o.insert("quickSpec".to_string(), Value::Array(quick));
    Ok(Value::Object(o))
}

/// `brands()` (`gsmarena.js:11-13` → `catalog.js:4`).
pub fn brands(http: &dyn HttpGet) -> Result<Value, HttpError> {
    let html = ambil(http, "/makers.php3")?;
    // `$('table').find('td')` — td di dalam tabel mana pun yang cocok.
    let td = di_dalam(
        &html,
        &semua_elemen(&html, 0, html.len(), &|nama, _| nama.eq_ignore_ascii_case("table")),
        &|nama, _| nama.eq_ignore_ascii_case("td"),
    );

    let mut hasil: Vec<Value> = Vec::new();
    for e in td {
        let anchors = semua_elemen(&html, e.isi_awal, e.isi_akhir, &|nama, _| {
            nama.eq_ignore_ascii_case("a")
        });
        let Some(href) = anchors
            .first()
            .and_then(|a| atribut(&a.isi_tag, "href"))
        else {
            return Err(tipe_error("replace", false));
        };
        let mut nama_brand: String = anchors
            .iter()
            .map(|a| teks_elemen(&html, a))
            .collect::<Vec<_>>()
            .concat();
        // `.replace(' devices','')` kemunculan pertama, lalu semua angka.
        nama_brand = nama_brand.replacen(" devices", "", 1);
        let jumlah = span_pertama(&html, &e).map(|s| {
            let t = teks_elemen(&html, &s).replacen(" devices", "", 1);
            parse_int_js(&Value::String(t))
        });
        hasil.push(json!({
            "id": href.replacen(".php", "", 1),
            "name": nama_brand.chars().filter(|c| !c.is_ascii_digit()).collect::<String>(),
            // `parseInt` yang `NaN` diserialisasi `null` oleh `JSON.stringify`.
            "devices": jumlah.flatten().map_or(Value::Null, angka_js),
        }));
    }
    Ok(Value::Array(hasil))
}

/// `brandDevices(brandId)` (`gsmarena.js:15-17` → `catalog.js:46`).
pub fn brand_devices(http: &dyn HttpGet, brand_id: &str) -> Result<Value, HttpError> {
    let mut halaman = brand_id.to_string();
    let mut sudah: Vec<String> = Vec::new();
    let mut hasil: Vec<Value> = Vec::new();
    loop {
        // Tanpa ini `while (getNextPage($))` di JS tidak pernah berhenti kalau
        // situs mengembalikan tautan halaman berikutnya yang sama terus-menerus.
        if sudah.contains(&halaman) {
            break;
        }
        sudah.push(halaman.clone());
        let html = ambil(http, &format!("/{halaman}.php"))?;
        hasil.extend(perangkat(&html)?);
        match berikutnya(&html) {
            Some(next) => halaman = next,
            None => break,
        }
    }
    Ok(Value::Array(hasil))
}

/// `getDevices($, $('.makers').find('li'))` (`catalog.js:31-44`).
fn perangkat(html: &str) -> Result<Vec<Value>, HttpError> {
    let li = di_dalam(
        html,
        &semua_elemen(html, 0, html.len(), &|_, isi| punya_kelas(isi, "makers")),
        &|nama, _| nama.eq_ignore_ascii_case("li"),
    );
    let mut hasil: Vec<Value> = Vec::new();
    for e in li {
        let img = pertama_elemen(html, e.isi_awal, e.isi_akhir, &|nama, _| {
            nama.eq_ignore_ascii_case("img")
        });
        let id = id_dari_a(html, &e)?;
        let src = img.as_ref().and_then(|i| atribut(&i.isi_tag, "src"));
        let title = img.as_ref().and_then(|i| atribut(&i.isi_tag, "title"));
        let mut o = Map::new();
        o.insert("id".to_string(), json!(id));
        o.insert(
            "name".to_string(),
            json!(span_pertama(html, &e)
                .map(|s| teks_elemen(html, &s))
                .unwrap_or_default()),
        );
        sisipkan_atribut(&mut o, "img", src);
        sisipkan_atribut(&mut o, "description", title);
        hasil.push(Value::Object(o));
    }
    Ok(hasil)
}

/// `getNextPage($)` (`catalog.js:23`) — `href` tanpa `.php`; `None` =
/// `false` di JS.
fn berikutnya(html: &str) -> Option<String> {
    let e = pertama_elemen(html, 0, html.len(), &|nama, isi| {
        nama.eq_ignore_ascii_case("a")
            && punya_kelas(isi, "prevnextbutton")
            && atribut(isi, "title").as_deref() == Some("Next page")
    })?;
    let href = atribut(&e.isi_tag, "href")?;
    if href.is_empty() {
        return None;
    }
    Some(href.replacen(".php", "", 1))
}

/// `$(el).find('a').attr('href').replace('.php','')` — `attr` yang `undefined`
/// melempar `TypeError` di JS.
fn id_dari_a(html: &str, e: &Elemen) -> Result<String, HttpError> {
    let href =
        pertama_elemen(html, e.isi_awal, e.isi_akhir, &|nama, _| nama.eq_ignore_ascii_case("a"))
            .and_then(|a| atribut(&a.isi_tag, "href"))
            .ok_or_else(|| tipe_error("replace", false))?;
    Ok(href.replacen(".php", "", 1))
}

/// Hasil `parseInt` sebagai JSON.
///
/// `parseInt` di JS selalu mengembalikan bilangan bulat, jadi `1234` harus
/// ditulis `1234`, bukan `1234.0`. `serde_json` menyimpan `f64` sebagai float
/// dan mencetaknya dengan titik desimal, jadi nilainya harus dicasting ke `i64`.
fn angka_js(n: f64) -> Value {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

/// `$(selector)` di seluruh dokumen.
fn semua_di(html: &str, cocok: &dyn Fn(&str, &str) -> bool) -> Vec<Elemen> {
    semua_elemen(html, 0, html.len(), cocok)
}

/// `$(selector).text()` di seluruh dokumen.
fn teks_selector(html: &str, cocok: &dyn Fn(&str, &str) -> bool) -> String {
    teks_semua(html, &semua_di(html, cocok))
}

/// Teks `$('td.ttl', ele)` / `$('td.nfo', ele)` — gabungan seluruh `td` yang
/// cocok di dalam `tr`.
fn teks_td(html: &str, dari: usize, sampai: usize, kelas: &str) -> String {
    teks_semua(
        html,
        &semua_elemen(html, dari, sampai, &|nama, isi| {
            nama.eq_ignore_ascii_case("td") && punya_kelas(isi, kelas)
        }),
    )
}

/// `getDataFromUrl(url)` (`utils.js:3-10`) — axios GET polos.
fn ambil(http: &dyn HttpGet, jalur: &str) -> Result<String, HttpError> {
    Ok(http.call(&HttpCall::get(format!("{BASE}{jalur}")))?.text())
}

/// `TypeError` V8 untuk membaca properti dari `undefined`/`null`.
fn tipe_error(properti: &str, dari_null: bool) -> HttpError {
    HttpError {
        message: format!(
            "Cannot read properties of {} (reading '{properti}')",
            if dari_null { "null" } else { "undefined" }
        ),
    }
}

/// Kunci dengan `attr` yang `undefined` tidak ikut, seperti `JSON.stringify`.
fn sisipkan_atribut(o: &mut Map<String, Value>, kunci: &str, nilai: Option<String>) {
    if let Some(v) = nilai {
        o.insert(kunci.to_string(), json!(v));
    }
}

// ── bedah HTML kecil (pengganti cheerio) ────────────────────────────────────

struct Elemen {
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
        .take_while(char::is_ascii_alphanumeric)
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

fn buat_elemen(html: &str, awal: usize, isi_awal: usize, nama: &str, isi_tag: String) -> Elemen {
    let kosong = isi_tag.ends_with('/') || TAG_KOSONG.iter().any(|t| nama.eq_ignore_ascii_case(t));
    let isi_akhir = if kosong {
        isi_awal
    } else {
        let a = akhir_elemen(html, awal, nama);
        html[isi_awal..a]
            .rfind('<')
            .map(|p| isi_awal + p)
            .unwrap_or(a)
    };
    Elemen {
        isi_awal,
        isi_akhir,
        isi_tag,
    }
}

/// Nilai atribut dari teks tag, entitas HTML didekode seperti cheerio.
fn atribut(tag: &str, nama: &str) -> Option<String> {
    static CACHE: LazyLock<Mutex<std::collections::HashMap<String, regex::Regex>>> =
        LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
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

fn punya_kelas(tag: &str, kelas: &str) -> bool {
    atribut(tag, "class")
        .map(|c| c.split_whitespace().any(|k| k == kelas))
        .unwrap_or(false)
}

/// `[data-spec=displaysize-hl]` — nilai atribut tanpa tanda kutip, persis
/// seperti penulisan selector di `catalog.js`.
fn atribut_bernilai(tag: &str, nama: &str, nilai: &str) -> bool {
    atribut(tag, nama).as_deref() == Some(nilai)
}

/// Semua elemen di `[dari, sampai)` yang cocok, termasuk yang di dalam elemen
/// lain — padanan `$(...)` untuk selector yang dipakai file ini.
fn semua_elemen(
    html: &str,
    dari: usize,
    sampai: usize,
    cocok: &dyn Fn(&str, &str) -> bool,
) -> Vec<Elemen> {
    let mut out = Vec::new();
    let mut i = dari;
    while let Some((awal, isi_awal, nama, isi_tag)) = tag_berikut(html, i) {
        if awal >= sampai {
            break;
        }
        if !nama.is_empty()
            && !isi_tag.starts_with('/')
            && !isi_tag.starts_with('!')
            && cocok(&nama, &isi_tag)
        {
            out.push(buat_elemen(html, awal, isi_awal, &nama, isi_tag));
        }
        i = isi_awal;
    }
    out
}

fn pertama_elemen(
    html: &str,
    dari: usize,
    sampai: usize,
    cocok: &dyn Fn(&str, &str) -> bool,
) -> Option<Elemen> {
    semua_elemen(html, dari, sampai, cocok).into_iter().next()
}

/// Gabungan hasil pencarian di dalam tiap elemen induk — `$(induk).find(...)`.
fn di_dalam(html: &str, induk: &[Elemen], cocok: &dyn Fn(&str, &str) -> bool) -> Vec<Elemen> {
    induk
        .iter()
        .flat_map(|p| semua_elemen(html, p.isi_awal, p.isi_akhir, cocok))
        .collect()
}

fn span_pertama(html: &str, e: &Elemen) -> Option<Elemen> {
    pertama_elemen(html, e.isi_awal, e.isi_akhir, &|nama, _| {
        nama.eq_ignore_ascii_case("span")
    })
}

/// `$(el).html()` — HTML dalam elemen apa adanya.
fn html_dalam(html: &str, e: &Elemen) -> String {
    html.get(e.isi_awal..e.isi_akhir).unwrap_or_default().to_string()
}

/// `$(...).text()` — buang tag, dekode entitas, gabungkan semua yang cocok.
fn teks_semua(html: &str, daftar: &[Elemen]) -> String {
    daftar.iter().map(|e| teks_elemen(html, e)).collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;

    /// Stub `HttpGet` berantrean: balasan diberikan berurutan, satu per
    /// permintaan; habisnya antrean dibalas dengan body kosong.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<VecDeque<String>>,
        gagal: Option<String>,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            if let Some(pesan) = &self.gagal {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
            let body = self.balasan.lock().unwrap().pop_front().unwrap_or_default();
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
                body: oktz_plugin_api::HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn stub(daftar: &[&str]) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            // Urutan respons harus sama dengan urutan permintaan: `pop_front`
            // mengambil yang paling depan, jadi JANGAN dibalik.
            balasan: Mutex::new(daftar.iter().map(|s| s.to_string()).collect()),
            gagal: None,
        }
    }

    fn gagal(pesan: &str) -> StubHttp {
        StubHttp {
            permintaan: Mutex::new(Vec::new()),
            balasan: Mutex::new(VecDeque::new()),
            gagal: Some(pesan.to_string()),
        }
    }

    const HALAMAN_CARI: &str = r#"<div class="makers"><ul>
  <li>
    <a href="samsung_galaxy_s24-12345.php"><img src="//f/1.jpg" title="Flagship 2024"></a>
    <span>Samsung Galaxy S24<br>Ultra 2024</span>
  </li>
  <li>
    <a href="samsung_galaxy_a55-12399.php"><img src="//f/2.jpg" title="Mid range"></a>
    <span>Samsung Galaxy A55<br>5G</span>
  </li>
</ul></div>"#;

    #[test]
    fn search_membaca_id_nama_gambar_dan_deskripsi() {
        let http = stub(&[HALAMAN_CARI]);
        let hasil = search(&http, "galaxy s24").unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "samsung_galaxy_s24-12345");
        // `.html().split('<br>').join(' ')` — `<br>` jadi spasi, bukan dibuang.
        assert_eq!(arr[0]["name"], "Samsung Galaxy S24 Ultra 2024");
        assert_eq!(arr[0]["img"], "//f/1.jpg");
        assert_eq!(arr[0]["description"], "Flagship 2024");
        assert_eq!(arr[1]["name"], "Samsung Galaxy A55 5G");
        assert_eq!(arr[1]["id"], "samsung_galaxy_a55-12399");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://www.gsmarena.com/results.php3?sQuickSearch=yes&sName=galaxy s24"
        );
        assert!(p[0].headers.is_empty(), "axios tanpa header khusus");
        assert!(p[0].body.is_none());
    }

    #[test]
    fn search_halaman_kosong_memberi_array_kosong() {
        let http = stub(&["<html><body>Tidak ada</body></html>"]);
        assert_eq!(search(&http, "zzz").unwrap(), json!([]));
    }

    #[test]
    fn search_tanpa_anchor_melempar_type_error_js() {
        let http = stub(&[r#"<div class="makers"><ul><li><span>x</span></li></ul></div>"#]);
        let err = search(&http, "x").unwrap_err();
        assert_eq!(
            err.message,
            "Cannot read properties of undefined (reading 'replace')"
        );
    }

    #[test]
    fn search_tanpa_span_melempar_type_error_dari_null() {
        let http = stub(&[r#"<div class="makers"><ul><li><a href="x.php">x</a></li></ul></div>"#]);
        let err = search(&http, "x").unwrap_err();
        assert_eq!(
            err.message,
            "Cannot read properties of null (reading 'split')"
        );
    }

    #[test]
    fn brands_membaca_tabel_dan_membuang_angka_dari_nama() {
        let http = stub(&[
            r#"<table><tr>
  <td><a href="samsung.php">Samsung devices</a><span>1234 devices</span></td>
  <td><a href="xiaomi.php">Xiaomi  devices</a><span>abc devices</span></td>
</tr></table>"#,
        ]);
        let hasil = brands(&http).unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "samsung");
        // `.replace(' devices','')` hanya kemunculan PERTAMA, dan
        // `.replace(/[0-9]/g,'')` membuang semua angka.
        assert_eq!(arr[0]["name"], "Samsung");
        assert_eq!(arr[0]["devices"], 1234);
        assert_eq!(arr[1]["name"], "Xiaomi ");
        // `parseInt("abc")` = NaN -> `null` di JSON.stringify.
        assert_eq!(arr[1]["devices"], Value::Null);

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, "https://www.gsmarena.com/makers.php3");
    }

    #[test]
    fn brand_devices_mengikuti_halaman_berikutnya() {
        let halaman1 = r#"<div class="makers"><ul>
  <li><a href="samsung_galaxy_s24-1.php"><img src="//f/1.jpg" title="S24"></a><span>S24</span></li>
</ul></div>
<a class="prevnextbutton" title="Next page" href="samsung-phones-2.php">Next</a>"#;
        let halaman2 = r#"<div class="makers"><ul>
  <li><a href="samsung_galaxy_a55-2.php"><img src="//f/2.jpg" title="A55"></a><span>A55</span></li>
</ul></div>"#;
        let http = stub(&[halaman1, halaman2]);
        let hasil = brand_devices(&http, "samsung").unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["id"], "samsung_galaxy_s24-1");
        assert_eq!(arr[1]["id"], "samsung_galaxy_a55-2");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].url, "https://www.gsmarena.com/samsung.php");
        assert_eq!(p[1].url, "https://www.gsmarena.com/samsung-phones-2.php");
    }

    #[test]
    fn brand_devices_berhenti_kalau_tautan_halaman_berputar() {
        // Tanpa pengaman ini scraper akan looping tanpa akhir seperti JS.
        let halaman = r#"<div class="makers"><ul></ul></div>
<a class="prevnextbutton" title="Next page" href="samsung.php">Next</a>"#;
        let http = stub(&[halaman, halaman]);
        assert_eq!(brand_devices(&http, "samsung").unwrap(), json!([]));
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }

    const HALAMAN_DETAIL: &str = r#"<div class="specs-phone-name-title">Samsung Galaxy S24</div>
<div class="specs-photo-main"><a href="samsung_galaxy_s24-1.php"><img src="//f/d.jpg"></a></div>
<span data-spec="displaysize-hl">6.2 inches</span>
<div data-spec="displayres-hl">1080 x 2340</div>
<div class="accent-camera">200 MP</div>
<div data-spec="videopixels-hl">8K</div>
<div class="accent-expansion">8GB/12GB</div>
<div data-spec="chipset-hl">Exynos 2400</div>
<div class="accent-battery">4000 mAh</div>
<div data-spec="battype-hl">Li-Ion</div>
<table>
  <tr><th>Body</th></tr>
  <tr><td class="ttl">Dimensions</td><td class="nfo">147 x 70 x 7.9 mm</td></tr>
</table>
<table>
  <tr><td class="ttl">Tampa kamera</td><td class="nfo">Tidak ada</td></tr>
</table>"#;

    #[test]
    fn detail_membaca_nama_foto_ringkas_dan_spesifikasi() {
        let http = stub(&[HALAMAN_DETAIL]);
        let hasil = detail(&http, "samsung_galaxy_s24-1").unwrap();
        assert_eq!(hasil["name"], "Samsung Galaxy S24");
        assert_eq!(hasil["img"], "//f/d.jpg");

        let quick = hasil["quickSpec"].as_array().unwrap();
        assert_eq!(quick.len(), 8);
        assert_eq!(quick[0]["name"], "Display size");
        assert_eq!(quick[0]["value"], "6.2 inches");
        assert_eq!(quick[1]["value"], "1080 x 2340");
        assert_eq!(quick[2]["value"], "200 MP");
        assert_eq!(quick[3]["value"], "8K");
        assert_eq!(quick[4]["value"], "8GB/12GB");
        assert_eq!(quick[5]["value"], "Exynos 2400");
        assert_eq!(quick[6]["value"], "4000 mAh");
        assert_eq!(quick[7]["value"], "Li-Ion");

        // Tabel tanpa `th` tidak jadi kategori (`if (category)` di `catalog.js:105`).
        let spec = hasil["detailSpec"].as_array().unwrap();
        assert_eq!(spec.len(), 1);
        assert_eq!(spec[0]["category"], "Body");
        // Setiap `tr` dihitung, termasuk baris `th` yang tidak punya td.
        let rows = spec[0]["specifications"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "");
        assert_eq!(rows[1]["name"], "Dimensions");
        assert_eq!(rows[1]["value"], "147 x 70 x 7.9 mm");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p[0].url, "https://www.gsmarena.com/samsung_galaxy_s24-1.php");
    }

    #[test]
    fn detail_tanpa_foto_tidak_menghasilkan_kunci_img() {
        let http = stub(&[r#"<div class="specs-phone-name-title">X</div>"#]);
        let hasil = detail(&http, "x").unwrap();
        assert!(hasil.get("img").is_none(), "attr undefined tidak ikut JSON");
        assert_eq!(hasil["quickSpec"].as_array().unwrap().len(), 8);
        assert_eq!(hasil["detailSpec"], json!([]));
    }

    #[test]
    fn kegagalan_transport_diteruskan_persis() {
        let err = search(&gagal("HTTP 503"), "x").unwrap_err();
        assert_eq!(err.message, "HTTP 503");
    }
}