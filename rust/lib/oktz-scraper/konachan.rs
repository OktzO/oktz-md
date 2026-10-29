//! `src/scraper/konachan.js` — cari post di `konachan.net` lewat halaman HTML.
//!
//! Cheerio tidak tersedia di crate ini, jadi potongan HTML dibedah dengan
//! pengurai tulisan tangan di bawah; preseden `reddit.rs`. Batasnya jujur: tag
//! yang punya `>` di dalam nilai atribut (mis. `<a title="a>b">`) tidak
//! diparse persis seperti parse5.
//!
//! Semantik yang disalin dari JS:
//! - `$(el).attr('id')?.replace('p','')` — mengganti kemunculan PERTAMA huruf
//!   `p` di mana saja, bukan hanya di depan; hasil kosong lalu `|| null`.
//! - `find('.thumb img')` — img di dalam elemen berekelas `thumb`, bukan img
//!   pertama di `li` (yang itu buat `preview`).
//! - `titleAttr.includes('Tags:')` memilih sumber tag: substring setelah
//!   `Tags:`, dipotong sebelum `User:`, di-`trim`; kalau kosong, tag tetap
//!   array kosong. Selain itu tag diambil dari `alt` dengan `split(/\s+/)`, jadi
//!   `alt` kosong memberi `[""]`, bukan `[]`.
//! - `previewUrl.startsWith('http') ? url : "https:" + url` — thumbnail
//!   konachan ditulis relatif protokol (`//…`).
//! - `parseInt(id)` yang `NaN` menjadi `null` di `JSON.stringify`.
//! - Seluruh badan dalam `try`; error apa pun → `{ error: true, message }`,
//!   yang bertipe OBJEK, bukan array posts.

use std::sync::LazyLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{decode_entitas, encode_uri_component, parse_int_js};

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// `searchKonachan(tags)` (`konachan.js:4-51`).
///
/// Balikannya `Value`: array `posts` pada sukses, objek
/// `{ error: true, message }` pada kegagalan — bentuk yang dikembalikan JS.
pub fn search_konachan(http: &dyn HttpGet, tags: &str) -> Result<Value, HttpError> {
    match alur(http, tags) {
        Ok(v) => Ok(v),
        Err(e) => Ok(json!({ "error": true, "message": e.message })),
    }
}

fn alur(http: &dyn HttpGet, tags: &str) -> Result<Value, HttpError> {
    let url = format!(
        "https://konachan.net/post?tags={}",
        encode_uri_component(tags)
    );
    let res = http.call(&HttpCall::get(url).header("User-Agent", UA))?;
    let html = res.text();

    // `$('#post-list-posts li')`.
    let container = semua_elemen(&html, 0, html.len(), &|_, isi| {
        atribut(isi, "id").as_deref() == Some("post-list-posts")
    });
    let li = di_dalam(&html, &container, &|nama, _| nama.eq_ignore_ascii_case("li"));

    let mut posts: Vec<Value> = Vec::new();
    for e in li {
        let img = pertama_elemen(&html, e.isi_awal, e.isi_akhir, &|nama, _| {
            nama.eq_ignore_ascii_case("img")
        });
        let thumb = pertama_elemen(&html, e.isi_awal, e.isi_akhir, &|_, isi| {
            punya_kelas(isi, "thumb")
        });
        // `find('.thumb img')` — img DI DALAM elemen berekelas `thumb`.
        let thumb_img = thumb.as_ref().and_then(|t| {
            pertama_elemen(&html, t.isi_awal, t.isi_akhir, &|nama, _| {
                nama.eq_ignore_ascii_case("img")
            })
        });
        let title_attr = thumb_img
            .as_ref()
            .and_then(|i| atribut(&i.isi_tag, "title"))
            .filter(|t| !t.is_empty())
            .unwrap_or_default();
        let alt = img.as_ref().and_then(|i| atribut(&i.isi_tag, "alt"));

        let clean_tags = if title_attr.contains("Tags:") {
            // `split('Tags:')[1]?.split('User:')[0]?.trim()`; bagian `[1]`
            // selalu ada karena cabang ini hanya masuk kalau ada `Tags:`.
            let setelah_tags = title_attr.split_once("Tags:").map_or("", |(_, s)| s);
            let raw = setelah_tags
                .split_once("User:")
                .map_or(setelah_tags, |(s, _)| s);
            let raw = trim_js(raw);
            if raw.is_empty() {
                Vec::new()
            } else {
                split_spasi(raw)
            }
        } else {
            // `attr('alt')?.trim().split(/\s+/) || []` — `alt` yang hilang
            // jadi array kosong, `alt` yang kosong jadi `[""]`.
            match alt {
                Some(a) => split_spasi(trim_js(&a)),
                None => Vec::new(),
            }
        };

        // `attr('id')?.replace('p','')` — kemunculan pertama `p`, lalu `|| null`.
        let id = elemen_id(&e.isi_tag)
            .map(|v| v.replacen('p', "", 1))
            .filter(|s| !s.is_empty());
        let Some(id) = id else { continue };

        let preview_url = img
            .as_ref()
            .and_then(|i| atribut(&i.isi_tag, "src"))
            .filter(|s| !s.is_empty());
        let source_url = thumb
            .as_ref()
            .and_then(|t| atribut(&t.isi_tag, "href"))
            .filter(|s| !s.is_empty());

        let preview = preview_url.map(|p| {
            if p.starts_with("http") {
                p
            } else {
                format!("https:{p}")
            }
        });
        let details_page = source_url.map(|s| format!("https://konachan.net{s}"));

        let mut o = Map::new();
        // `parseInt(id)` yang `NaN` diserialisasi `null`.
        o.insert(
            "id".to_string(),
            parse_int_js(&Value::String(id)).map_or(Value::Null, angka_js),
        );
        o.insert("tags".to_string(), json!(clean_tags));
        o.insert(
            "details_page".to_string(),
            details_page.map_or(Value::Null, Value::String),
        );
        o.insert(
            "images".to_string(),
            json!({ "preview": preview.map_or(Value::Null, Value::String) }),
        );
        posts.push(Value::Object(o));
    }
    Ok(Value::Array(posts))
}

/// Hasil `parseInt` sebagai JSON.
///
/// `parseInt` di JS selalu mengembalikan bilangan bulat, jadi `4123456` harus
/// ditulis `4123456`, bukan `4123456.0`. `serde_json` menyimpan `f64` sebagai
/// float dan mencetaknya dengan titik desimal, jadi nilainya harus dicasting ke
/// `i64` dulu.
fn angka_js(n: f64) -> Value {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

/// `split(/\s+/)` — TANPA membuang bagian kosong: `"".split(/\s+/)` di JS
/// menghasilkan `[""]`, dan `"  a".split(/\s+/)` menghasilkan `["", "a"]`.
fn split_spasi(s: &str) -> Vec<String> {
    static P: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("pola spasi"));
    P.split(s).map(str::to_string).collect()
}

/// `String#trim` JS (termasuk NBSP dan spasi Unicode lain).
fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(c,
            '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}'
            | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}'
            | '\u{3000}' | '\u{FEFF}')
    })
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

fn punya_kelas(tag: &str, kelas: &str) -> bool {
    atribut(tag, "class")
        .map(|c| c.split_whitespace().any(|k| k == kelas))
        .unwrap_or(false)
}

fn elemen_id(tag: &str) -> Option<String> {
    atribut(tag, "id")
}

/// Semua elemen di `[dari, sampai)` yang cocok, termasuk yang ada di dalam
/// elemen lain — padanan `$(...)`.
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

/// Gabungan pencarian di dalam tiap induk — `$(induk).find(...)`.
fn di_dalam(html: &str, induk: &[Elemen], cocok: &dyn Fn(&str, &str) -> bool) -> Vec<Elemen> {
    induk
        .iter()
        .flat_map(|p| semua_elemen(html, p.isi_awal, p.isi_akhir, cocok))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: String,
        galat: Option<String>,
    }

    impl Stub {
        fn baru(balasan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: balasan.to_string(),
                galat: None,
            }
        }
        fn gagal(pesan: &str) -> Self {
            Self {
                galat: Some(pesan.to_string()),
                ..Self::baru("")
            }
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call supaya header/url tercatat")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            if let Some(pesan) = &self.galat {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.balasan.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
    }

    fn cari_header<'a>(req: &'a HttpCall, nama: &str) -> Option<&'a str> {
        req.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.as_str())
    }

    const HALAMAN: &str = r#"<ul id="post-list-posts">
  <li id="p4123456">
    <a class="thumb" href="/post/show/4123456">
      <img src="//konachan.net/image/cache/1.jpg" alt="blue_eyes long_hair solo"
           title="Tags: blue_eyes long_hair 1girl User: updater">
    </a>
    <span class="score">12 Comments</span>
  </li>
  <li id="p7654321">
    <a class="thumb" href="/post/show/7654321">
      <img src="https://konachan.net/image/2.jpg" alt="cat" title="Rating: safe">
    </a>
  </li>
</ul>"#;

    #[test]
    fn membaca_post_dan_mengirim_url_yang_benar() {
        let http = Stub::baru(HALAMAN);
        let hasil = search_konachan(&http, "blue_eyes girl").unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        assert_eq!(arr[0]["id"], 4123456);
        // `title` berisi `Tags:` -> tag dari sana, dipotong sebelum `User:`.
        assert_eq!(arr[0]["tags"], json!(["blue_eyes", "long_hair", "1girl"]));
        assert_eq!(
            arr[0]["details_page"],
            "https://konachan.net/post/show/4123456"
        );
        // Thumbnail relatif protokol -> prefix `https:`.
        assert_eq!(
            arr[0]["images"]["preview"],
            "https://konachan.net/image/cache/1.jpg"
        );

        // `title` tanpa `Tags:` -> tag dari `alt`.
        assert_eq!(arr[1]["tags"], json!(["cat"]));
        // Sudah `http`, tidak diubah.
        assert_eq!(arr[1]["images"]["preview"], "https://konachan.net/image/2.jpg");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        // `encodeURIComponent` -> spasi `%20`, bukan `+`.
        assert_eq!(
            p[0].url,
            "https://konachan.net/post?tags=blue_eyes%20girl"
        );
        assert!(cari_header(&p[0], "user-agent").unwrap().contains("Chrome/120.0.0.0"));
    }

    #[test]
    fn li_tanpa_id_dilewati() {
        let http = Stub::baru(r#"<ul id="post-list-posts"><li><a class="thumb" href="/x"><img src="//y"></a></li></ul>"#);
        assert_eq!(search_konachan(&http, "x").unwrap(), json!([]));
    }

    #[test]
    fn id_hanya_huruf_p_dan_kolom_kosong_tidak_membuat_post() {
        // `id="p"` -> `""` -> `|| null` -> post dilewati.
        let http = Stub::baru(r#"<ul id="post-list-posts"><li id="p"><a class="thumb" href="/x"><img src="//y"></a></li></ul>"#);
        assert_eq!(search_konachan(&http, "x").unwrap(), json!([]));
    }

    #[test]
    fn thumbnail_kosong_memberi_null_bukan_kunci_hilang() {
        let http = Stub::baru(
            r#"<ul id="post-list-posts"><li id="p99"><a class="thumb"><img alt="a b"></a></li></ul>"#,
        );
        let hasil = search_konachan(&http, "x").unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr[0]["images"]["preview"], Value::Null);
        // Tanpa `href` -> `details_page` tetap ada dengan `null`.
        assert_eq!(arr[0]["details_page"], Value::Null);
        assert_eq!(arr[0]["tags"], json!(["a", "b"]));
    }

    #[test]
    fn alt_kosong_memberi_satu_kosongan_bukan_array_kosong() {
        // `''.split(/\s+/)` -> `['']` di JS.
        let http = Stub::baru(
            r#"<ul id="post-list-posts"><li id="p7"><a class="thumb" href="/p/7"><img src="//i" alt=""></a></li></ul>"#,
        );
        let hasil = search_konachan(&http, "x").unwrap();
        assert_eq!(hasil[0]["tags"], json!([""]));
    }

    #[test]
    fn halaman_tanpa_daftar_post_memberi_array_kosong() {
        let http = Stub::baru("<html><body>Tidak ada tag</body></html>");
        assert_eq!(search_konachan(&http, "zzz").unwrap(), json!([]));
    }

    #[test]
    fn galat_http_memberi_objek_error_bukan_array() {
        let hasil = search_konachan(&Stub::gagal("HTTP 429"), "x").unwrap();
        assert_eq!(hasil["error"], true);
        assert_eq!(hasil["message"], "HTTP 429");
    }
}