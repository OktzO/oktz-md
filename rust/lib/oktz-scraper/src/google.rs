//! `src/scraper/google.js` — Google News RSS lewat host ter-resolve.
//!
//! Yang ikut disalin dari JS:
//! - `TARGET_HOST` bukan nama host: `bmV3cy5nb29nbGUuY29t` adalah base64 dari
//!   `news.google.com`, dan di-decode saat runtime, bukan ditulis langsung.
//! - Query di-encode dengan `encodeURIComponent` (spasi `%20`, bukan `+`).
//! - `compileBinaryPayload`: token diambil dari belakang `articles/`, dipisah
//!   di `?`, `urlsafe` diubah ke base64 biasa, dipad ke kelipatan 4, di-decode
//!   base64, lalu URL pertama yang cocok diambil. Kalau `articles/` tidak ada
//!   atau tidak ada URL, `rawUrl` dikembalikan apa adanya —
//!   `compileBinaryPayload` tidak pernah melempar ke luar (`try/catch` di
//!   `google.js:57-59`).
//! - Query kosong (setelah `trim`) -> `{ status: false, error }` dengan teks
//!   yang persis, tanpa menyentuh jaringan.
//! - Kesalahan apa pun di dalam pipeline, termasuk HTTP, jadi
//!   `{ status: false, error }` — bukan `Err`.
//! - `index_node` mulai dari 1 dan urutan kunci mengikuti
//!   `Object.defineProperties`: `index_node`, `resource_title`,
//!   `resolved_endpoint`, `temporal_stamp`, `origin_node`.
//!
//! Beda yang diketahui:
//! - `cheerio.load(data, { xmlMode: true })` diganti pengurai XML mini milik
//!   modul ini. Yang dijamin benar: urutan dokumen, tag yang tidak punya tag
//!   penutup, CDATA, dan dekode entitas. Yang bisa beda: namespace dan
//!   entitas di luar daftar `common::decode_entitas`.
//! - `axios` di sini tidak memakai `validateStatus` khusus, jadi status bukan
//!   2xx jadi `Err` di lapisan HTTP; teksnya milik `UreqHttp`, bukan
//!   `"Request failed with status code …"` milik axios. Pesan `EngineException`
//!   sendiri (`Network core rejected stream with status: …`) tetap disalin
//!   karena itu yang keluar ke pengguna saat status 200 tapi body-nya aneh.
//! - `timeout: 8500` tidak ditiru (batas mesin, bukan tampilan); `UreqHttp`
//!   memakai batas global 20 detik.
//! - `EngineException` hanya membawa `message` ke hasil; `coreCode` dan
//!   `timestamp` tidak pernah dibaca, jadi tidak ada padanannya di sini.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::decode_entitas;

// ---------------------------------------------------------------------------
// Konfigurasi (`ENGINE_CONFIG`, `google.js:4-11`)
// ---------------------------------------------------------------------------

/// `TARGET_HOST` — base64, bukan host. Didecode saat runtime seperti di
/// konstruktor `GoogleScraperKernel` (`google.js:24-27`).
const TARGET_HOST: &str = "bmV3cy5nb29nbGUuY29t";

const DEFAULT_LANG: &str = "id";
const DEFAULT_REGION: &str = "ID";

/// `UA_POOL` (`google.js:9-10`).
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

/// `X-Engine-Context` (`google.js:75`).
const ENGINE_CONTEXT: &str = "Production-Kernel-v3.0.4";

// ---------------------------------------------------------------------------
// Pengurai XML mini (pengganti cheerio `xmlMode`)
// ---------------------------------------------------------------------------

struct Elemen {
    tag: String,
    anak: Vec<Simpul>,
}

enum Simpul {
    Teks(String),
    /// `CDATA` tidak meng-decode entitas di dalamnya.
    Cdata(String),
    Elemen(Elemen),
}

/// Nama tag apa adanya, TANPA `to_ascii_lowercase`: `cheerio` `xmlMode`
/// menyisakan `lowerCaseTags` mati, jadi `find("pubDate")` hanya cocok dengan
/// `<pubDate>`. Menurunkan huruf di sini akan membuat `<PubDate>` ikut cocok —
/// beda dari JS.
fn nama_tag(potong: &str) -> String {
    potong
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == ':')
        .collect::<String>()
}

/// Urai XML jadi pohon sederhana. Komentar, deklarasi, dan `CDATA` dipegang
/// terpisah; tag yang tidak pernah ditutup tetap naik ke induknya (parser
/// lenient, seperti htmlparser2).
fn urai(xml: &str) -> Elemen {
    let mut tumpukan: Vec<Elemen> = vec![Elemen {
        tag: String::new(),
        anak: Vec::new(),
    }];
    let b = xml.as_bytes();
    let mut i = 0;

    while i < b.len() {
        if b[i] != b'<' {
            let akhir = xml[i..].find('<').map(|p| i + p).unwrap_or(xml.len());
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Teks(xml[i..akhir].to_string()));
            }
            i = akhir;
            continue;
        }

        if xml[i..].starts_with("<!--") {
            match xml[i..].find("-->") {
                Some(p) => {
                    i += p + 3;
                    continue;
                }
                None => break,
            }
        }
        if xml[i..].starts_with("<![CDATA[") {
            let mulai = i + 9;
            let Some(akhir) = xml[mulai..].find("]]>") else {
                // CDATA tak tertutup: ambil sisa dokumen sebagai teks.
                if let Some(induk) = tumpukan.last_mut() {
                    induk.anak.push(Simpul::Cdata(xml[mulai..].to_string()));
                }
                break;
            };
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Cdata(xml[mulai..mulai + akhir].to_string()));
            }
            i = mulai + akhir + 3;
            continue;
        }
        if xml[i..].starts_with("<?") || xml[i..].starts_with("<!") {
            match xml[i..].find('>') {
                Some(p) => {
                    i += p + 1;
                    continue;
                }
                None => break,
            }
        }

        // Tag penutup.
        if xml[i..].starts_with("</") {
            let Some(pos) = xml[i..].find('>') else {
                break;
            };
            let nama = nama_tag(&xml[i + 2..i + pos]);
            if !nama.is_empty()
                && let Some(idx) = tumpukan.iter().rposition(|el| el.tag == nama)
            {
                while tumpukan.len() > idx + 1 {
                    let anak = tumpukan.pop().expect("frame lebih dari satu");
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

        // Tag buka.
        let Some(pos) = akhir_tag(xml, i) else {
            break;
        };
        let tag = nama_tag(&xml[i + 1..pos]);
        if tag.is_empty() {
            i = pos + 1;
            continue;
        }
        // XML tidak punya tag void seperti HTML; satu-satunya cara menutup
        // diri sendiri adalah `/>`.
        let tutup_sendiri = xml[i..=pos].trim_end().ends_with("/>");
        let el = Elemen {
            tag,
            anak: Vec::new(),
        };
        if tutup_sendiri {
            // `<link … />` tetap harus bisa dibaca `find("link").text()` — di
            // sini hasilnya kosong — jadi dimasukkan sebagai anak, bukan frame.
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Elemen(el));
            }
        } else {
            tumpukan.push(el);
        }
        i = pos + 1;
    }

    while tumpukan.len() > 1 {
        let anak = tumpukan.pop().expect("frame lebih dari satu");
        if let Some(induk) = tumpukan.last_mut() {
            induk.anak.push(Simpul::Elemen(anak));
        }
    }
    tumpukan.pop().expect("akar")
}

/// Indeks `>` penutup tag mulai dari `<` di `mulai`, hormati tanda kutip nilai
/// atribut supaya `>` di dalam nilai tidak menutup tag.
fn akhir_tag(xml: &str, mulai: usize) -> Option<usize> {
    let b = xml.as_bytes();
    let mut i = mulai + 1;
    let mut kutip: Option<u8> = None;
    while i < b.len() {
        match kutip {
            Some(q) => {
                if b[i] == q {
                    kutip = None;
                }
            }
            None => match b[i] {
                b'"' | b'\'' => kutip = Some(b[i]),
                b'>' => return Some(i),
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Semua elemen dengan nama tag tertentu, urutan dokumen (pre-order).
fn dengan_tag<'a>(akar: &'a Elemen, tag: &str, kumpulan: &mut Vec<&'a Elemen>) {
    for simpul in &akar.anak {
        if let Simpul::Elemen(el) = simpul {
            if el.tag == tag {
                kumpulan.push(el);
            }
            dengan_tag(el, tag, kumpulan);
        }
    }
}

/// `$(el).text()` — teks semua keturunan, entitas di-decode; isi `CDATA`
/// taken verbatim.
fn teks_dari(el: &Elemen) -> String {
    let mut out = String::new();
    for simpul in &el.anak {
        match simpul {
            Simpul::Teks(t) => out.push_str(&decode_entitas(t)),
            Simpul::Cdata(t) => out.push_str(t),
            Simpul::Elemen(e) => out.push_str(&teks_dari(e)),
        }
    }
    out
}

/// `$(el).find(nama).text()` — gabungan teks semua keturunan bernama itu.
fn teks_find(el: &Elemen, nama: &str) -> String {
    let mut semua = Vec::new();
    dengan_tag(el, nama, &mut semua);
    let mut out = String::new();
    for e in semua {
        out.push_str(&teks_dari(e));
    }
    out
}

// ---------------------------------------------------------------------------
// Base64 dan URL
// ---------------------------------------------------------------------------

/// Decoder base64 yang meniru `Buffer.from(s, 'base64')` Node: alfabet
/// URL-safe diterima, karakter asing dilewati, padding kurang tidak apa-apa,
/// berhenti di `=` pertama.
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
        acc = (acc << 6) | u32::from(idx);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xFF) as u8);
        }
    }
    out
}

/// `/https?:\/\/[^\s"\><]+/g` (`google.js:55`) — hanya kemunculan pertama yang
/// dipakai (`urlVector[0]`).
fn pola_url() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"https?://[^\s"<>]+"#).expect("pola url"))
}

/// `String#trim` JS.
fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

// ---------------------------------------------------------------------------
// Alur
// ---------------------------------------------------------------------------

/// `GoogleSearch(query)` (`google.js:125-128`) — pembungkus tipis yang memanggil
/// `executeQueryPipeline` pada kernel baru.
pub fn google_search(http: &dyn HttpGet, query: &str) -> Result<Value, HttpError> {
    let host = String::from_utf8_lossy(&base64_node(TARGET_HOST)).to_string();
    execute_query_pipeline(http, query, &host)
}

/// `executeQueryPipeline(searchToken)` (`google.js:62-122`).
fn execute_query_pipeline(
    http: &dyn HttpGet,
    search_token: &str,
    host: &str,
) -> Result<Value, HttpError> {
    // `!searchToken || searchToken.trim().length === 0`
    if search_token.is_empty() || trim_js(search_token).is_empty() {
        return Ok(json!({ "status": false, "error": "Query tidak boleh kosong" }));
    }

    let query_hex = crate::common::encode_uri_component(search_token);
    let endpoint = format!(
        "https://{host}/rss/search?q={query_hex}&hl={DEFAULT_LANG}&gl={DEFAULT_REGION}&ceid={DEFAULT_REGION}:{DEFAULT_LANG}"
    );

    match pipeline(http, &endpoint) {
        Ok(results) => Ok(json!({ "status": true, "results": results })),
        Err(e) => Ok(json!({ "status": false, "error": e.message })),
    }
}

/// Bagian `try` di `executeQueryPipeline` (`google.js:70-118`).
fn pipeline(http: &dyn HttpGet, endpoint: &str) -> Result<Vec<Value>, HttpError> {
    let res = http.call(
        &HttpCall::get(endpoint)
            .header("user-agent", UA)
            .header("x-engine-context", ENGINE_CONTEXT),
    )?;

    // `axios` melempar lebih dulu untuk status di luar 2xx, jadi cabang ini
    // hanya terlihat untuk 2xx yang bukan 200 (mis. 204).
    if res.status != 200 {
        return Err(HttpError {
            message: format!(
                "Network core rejected stream with status: {}",
                res.status
            ),
        });
    }

    let xml = res.text();
    let dok = urai(&xml);
    let mut items = Vec::new();
    dengan_tag(&dok, "item", &mut items);

    let mut hasil = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let mut o = Map::new();
        o.insert("index_node".to_string(), json!(i + 1));
        o.insert(
            "resource_title".to_string(),
            Value::String(teks_find(item, "title")),
        );
        o.insert(
            "resolved_endpoint".to_string(),
            Value::String(compile_binary_payload(&teks_find(item, "link"))),
        );
        o.insert(
            "temporal_stamp".to_string(),
            Value::String(teks_find(item, "pubDate")),
        );
        o.insert(
            "origin_node".to_string(),
            Value::String(teks_find(item, "source")),
        );
        hasil.push(Value::Object(o));
    }
    Ok(hasil)
}

// ---------------------------------------------------------------------------
// `GoogleScraperKernel`
// ---------------------------------------------------------------------------

/// `$extractCipherPayload(sourceUrl)` (`google.js:30-35`).
fn extract_cipher_payload(source_url: &str) -> Option<String> {
    // `indexOf('articles/') === -1` -> `null`.
    let pointer = source_url.find("articles/")?;
    // `substring(pointer + 9).split('?')[0]`
    Some(
        source_url[pointer + "articles/".len()..]
            .split('?')
            .next()
            .unwrap_or("")
            .to_string(),
    )
}

/// `$transformSerialization(cipherText)` (`google.js:37-44`) — base64url ke
/// base64 biasa, lalu pad ke kelipatan 4.
fn transform_serialization(cipher: &str) -> String {
    let mut s = cipher.replace('-', "+").replace('_', "/");
    let sisa = s.len() % 4;
    if sisa > 0 {
        s.push_str(&"=".repeat(4 - sisa));
    }
    s
}

/// `compileBinaryPayload(rawUrl)` (`google.js:46-60`) — selalu mengembalikan
/// nilai; `try/catch` JS mengembalikan `rawUrl` kalau ada yang gagal.
fn compile_binary_payload(raw_url: &str) -> String {
    // `if (!sourceUrl || typeof sourceUrl !== 'string') return null` ->
    // `if (!token) return rawUrl`, dan string kosong ikut falsy.
    let Some(token) = extract_cipher_payload(raw_url).filter(|t| !t.is_empty()) else {
        return raw_url.to_string();
    };
    let matriks = transform_serialization(&token);
    let teks = String::from_utf8_lossy(&base64_node(&matriks)).to_string();
    pola_url()
        .find(&teks)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| raw_url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub `HttpGet` tanpa jaringan: mencatat `HttpCall`, membalas status dan
    /// body tetap.
    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        status: u16,
        body: String,
        gagal: Option<String>,
    }

    impl StubHttp {
        fn baru(status: u16, body: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                status,
                body: body.to_string(),
                gagal: None,
            }
        }

        fn gagal(pesan: &str) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                status: 0,
                body: String::new(),
                gagal: Some(pesan.to_string()),
            }
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("semua lewat HttpCall::call")
        }
        fn post_json(&self, _url: &str, _body: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
        }
        fn post_form(&self, _url: &str, _form: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
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
                None => Ok(HttpResponse {
                    status: self.status,
                    body: HttpBody::Bytes {
                        data: Arc::from(self.body.as_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
            }
        }
    }

    fn cari_header(p: &oktz_plugin_api::HttpCall, nama: &str) -> Option<String> {
        p.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nama))
            .map(|(_, v)| v.clone())
    }

    /// RSS Google News dengan dua `item`; `link` pertama memakai token base64
    /// url-safe yang memuat URL asli, `link` kedua URL biasa (tanpa
    /// `articles/`).
    fn rss() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/">
<channel>
  <title>Google News</title>
  <item>
    <title>Gempa di &#39;Sulawesi&#39; &amp; tsunami</title>
    <link>https://news.google.com/rss/articles/cHJlZml4IGh0dHBzOi8vanVybmFsLmV4YW1wbGUuY29tL2Jlcml0YT91dG09MSAjZnJhZw?oc=5</link>
    <guid isPermaLink="false">abc</guid>
    <pubDate>Mon, 06 Jan 2025 04:20:00 GMT</pubDate>
    <description><![CDATA[<a href="https://x.example/b">b</a>]]></description>
    <source url="https://jurnal.example.com">Jurnal Example</source>
  </item>
  <item>
    <title>Judul Kedua</title>
    <link>https://news.google.com/rss/articles/plain</link>
    <pubDate>Tue, 07 Jan 2025 00:00:00 GMT</pubDate>
    <source>Tanpa URL</source>
  </item>
</channel>
</rss>"#
        .to_string()
    }

    #[test]
    fn host_didecode_dari_base64_bukan_ditulis_langsung() {
        assert_eq!(
            String::from_utf8_lossy(&base64_node(TARGET_HOST)),
            "news.google.com"
        );
    }

    #[test]
    fn query_kosong_tidak_pernah_menyentuh_jaringan() {
        for q in ["", "   ", "\u{00A0}"] {
            let http = StubHttp::baru(200, "");
            let hasil = google_search(&http, q).unwrap();
            assert_eq!(hasil["status"], false);
            assert_eq!(hasil["error"], "Query tidak boleh kosong");
            assert!(http.permintaan.lock().unwrap().is_empty(), "q = {q:?}");
        }
    }

    #[test]
    fn hasil_sukses_membaca_semua_kolom_dan_urutan_kunci() {
        let http = StubHttp::baru(200, &rss());
        let hasil = google_search(&http, "gempa Magnitude 5,0").unwrap();
        assert_eq!(hasil["status"], true);

        let results = hasil["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["index_node"], 1);
        // Entitas di-decode seperti cheerio.
        assert_eq!(results[0]["resource_title"], "Gempa di 'Sulawesi' & tsunami");
        assert_eq!(
            results[0]["resolved_endpoint"],
            "https://jurnal.example.com/berita?utm=1"
        );
        assert_eq!(
            results[0]["temporal_stamp"],
            "Mon, 06 Jan 2025 04:20:00 GMT"
        );
        assert_eq!(results[0]["origin_node"], "Jurnal Example");

        // Tanpa `articles/` -> `rawUrl` apa adanya, dan urutan kunci tetap
        // seperti `Object.defineProperties`.
        assert_eq!(results[1]["index_node"], 2);
        assert_eq!(results[1]["resource_title"], "Judul Kedua");
        assert_eq!(
            results[1]["resolved_endpoint"],
            "https://news.google.com/rss/articles/plain"
        );
        assert_eq!(
            results[1].as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "index_node",
                "resource_title",
                "resolved_endpoint",
                "temporal_stamp",
                "origin_node"
            ]
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://news.google.com/rss/search?q=gempa%20Magnitude%205%2C0&hl=id&gl=ID&ceid=ID:id"
        );
        assert_eq!(cari_header(&p[0], "user-agent").as_deref(), Some(UA));
        assert_eq!(
            cari_header(&p[0], "x-engine-context").as_deref(),
            Some(ENGINE_CONTEXT)
        );
    }

    #[test]
    fn status_bukan_200_memberi_pesan_engine_exception() {
        // Status 204 lolos ke axios tanpa melempar, lalu ditolak cek `!== 200`.
        let http = StubHttp::baru(204, "");
        let hasil = google_search(&http, "gempa").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(
            hasil["error"],
            "Network core rejected stream with status: 204"
        );

        // Kegagalan transport dibungkus `catch` jadi objek, bukan `Err`.
        let http = StubHttp::gagal("HTTP 403");
        let hasil = google_search(&http, "gempa").unwrap();
        assert_eq!(hasil["status"], false);
        assert_eq!(hasil["error"], "HTTP 403");
    }

    #[test]
    fn rss_kosong_memberi_status_true_dengan_daftar_kosong() {
        let http = StubHttp::baru(200, r#"<?xml version="1.0"?><rss><channel><title>x</title></channel></rss>"#);
        let hasil = google_search(&http, "tidak ada").unwrap();
        assert_eq!(hasil["status"], true);
        assert_eq!(hasil["results"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn compile_binary_payload_meniru_aturan_javascript() {
        // Tidak ada `articles/` -> `rawUrl` apa adanya.
        assert_eq!(
            compile_binary_payload("https://a.example/x"),
            "https://a.example/x"
        );
        // String kosong ikut falsy di `if (!token)`.
        assert_eq!(compile_binary_payload(""), "");
        // Token yang berisi `?` dipotong sebelum `?`.
        assert_eq!(
            compile_binary_payload(
                "https://news.google.com/rss/articles/aHR0cHM6Ly9hLmV4YW1wbGUveA?oc=5"
            ),
            "https://a.example/x"
        );
        // Hanya URL pertama yang diambil kalau ada dua.
        assert_eq!(
            compile_binary_payload(
                "https://news.google.com/rss/articles/aHR0cHM6Ly9iLmV4YW1wbGUveSBodHRwczovL2MuZXhhbXBsZS96"
            ),
            "https://b.example/y"
        );
        // Base64 hasil decode tapi tanpa URL -> `rawUrl`.
        assert_eq!(
            compile_binary_payload("https://news.google.com/rss/articles/ICAg"),
            "https://news.google.com/rss/articles/ICAg"
        );
        // Karakter terlarang pada pola URL (`"`, `<`, `>`) jadi batas.
        assert_eq!(
            compile_binary_payload(
                "https://news.google.com/rss/articles/aHR0cHM6Ly9kLmV4YW1wbGUvaW5xdW90ZSI"
            ),
            "https://d.example/inquote"
        );
    }

    #[test]
    fn pengurai_xml_menghormati_cdata_dan_void() {
        let dok = urai(
            r#"<rss><channel><item><title>a &amp; b</title><link/><description><![CDATA[<b>x & y</b>]]></description><!-- <item>jahat</item> --><pubDate>now</pubDate></item></channel></rss>"#,
        );
        let mut items = Vec::new();
        dengan_tag(&dok, "item", &mut items);
        assert_eq!(items.len(), 1, "item di dalam komentar ikut terhitung");
        assert_eq!(teks_find(items[0], "title"), "a & b");
        assert_eq!(teks_find(items[0], "link"), "");
        // CDATA verbatim: entitas dan tag di dalamnya tidak diurai.
        assert_eq!(teks_find(items[0], "description"), "<b>x & y</b>");
        assert_eq!(teks_find(items[0], "pubDate"), "now");
    }
}