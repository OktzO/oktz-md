//! `src/scraper/sfiledl.js` — sfile.co: tiga GET berantai (halaman masukan →
//! halaman gerbang → halaman unduhan) lalu URL akhir dicari di dalam
//! `<script>`.
//!
//! Yang ikut disalin dari JS:
//! - `set-cookie` dari respons pertama (segmen sebelum `;`) dipakai sebagai
//!   header `cookie` di permintaan kedua dan ketiga.
//! - `referer` berubah: permintaan kedua = url masukan, ketiga = `og:url`.
//! - Kalau `og:url` atau `#download` tidak ada, hasilnya tetap objek lengkap
//!   dengan `download_url: null` dan tidak ada permintaan lanjutan.
//! - Pola URL akhir menuntut bentuk JSON ter-escape (`https:\/\/...`) di dalam
//!   `<script>`; `\/` diganti jadi `/` lewat `replace(/\\\//g, "/")`.
//!
//! Beda yang diketahui:
//! - JS lewat `cheerio` (htmlparser2 penuh) + `he`. Di sini ada pengurai HTML
//!   mini privat yang cukup untuk `h1`, `meta[property=...]`, `span`,
//!   `#download`, dan isi mentah `script`. Entitas yang di-decode hanya yang
//!   dikenal `common::decode_entitas`; di luar itu dibiarkan apa adanya, dan
//!   serialisasi ulang `.html()` untuk `script` memakai tag pembuka apa adanya
//!   dari sumber, bukan hasil normalisasi cheerio.
//! - `axios` men-parse body JSON otomatis; port ini selalu memperlakukan body
//!   sebagai teks HTML.
//! - Argumen `{ signal }` (pembatalan) tidak punya padanan di `HttpGet`.
//!   `httpAxios` juga memasang timeout 15 detik dan `maxRedirects: 3`;
//!   di Rust timeout global 20 detik milik `UreqHttp`.

use std::sync::OnceLock;

use oktz_plugin_api::{HttpCall, HttpError, HttpGet, HttpResponse};
use regex::Regex;
use serde_json::{Map, Value};

use crate::common::decode_entitas;

const UA: &str = "Mozilla/5.0 (Linux; Android 10; K)";
const ACCEPT: &str = "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8";
const BAHASA: &str = "id-ID,id;q=0.9,en;q=0.8";

// ---------------------------------------------------------------------------
// Pengurai HTML mini (pengganti `cheerio` untuk selector yang dipakai file ini)
// ---------------------------------------------------------------------------

/// Satu elemen hasil penguraian.
#[derive(Debug, Default)]
struct Elemen {
    tag: String,
    attrs: Vec<(String, String)>,
    anak: Vec<Simpul>,
    /// Tag pembuka apa adanya untuk elemen raw-text (`script`/`style`) —
    /// dipakai saat meniru `cheerio.load(el).html()`.
    buka: String,
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

/// Nama tag dari potongan setelah `<` atau `</`, huruf kecil.
fn nama_tag(potongan: &str) -> String {
    potongan
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ':')
        .collect::<String>()
        .to_ascii_lowercase()
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

/// Urai HTML jadi pohon sederhana. Parser HTML penuh tidak ditiru — ini cukup
/// untuk selector yang dipakai `sfiledl.js`; komentar dan `<!DOCTYPE>` dibuang.
fn urai(html: &str) -> Elemen {
    let mut tumpukan: Vec<Elemen> = vec![Elemen::default()];
    let mut i = 0;
    while i < html.len() {
        if html.as_bytes()[i] != b'<' {
            let akhir = html[i..].find('<').map(|p| i + p).unwrap_or(html.len());
            let teks = &html[i..akhir];
            if let Some(induk) = tumpukan.last_mut() {
                induk.anak.push(Simpul::Teks(teks.to_string()));
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
            let nama = nama_tag(&html[i + 2..i + pos]);
            // Akar (tag "") tidak pernah ikut ditutup, dan `</>` diabaikan.
            if !nama.is_empty()
                && let Some(idx) = tumpukan.iter().rposition(|el| el.tag == nama) {
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

        // `<` yang tidak diikuti huruf bukan awal tag (mis. "1 < 2"):
        // dianggap teks biasa seperti htmlparser2, bukan tag kosong.
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
                    buka: buka.to_string(),
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
            buka: String::new(),
        };
        if tutup_sendiri {
            // `<br>`, `<meta ...>`, `<img ...>`: langsung jadi anak, bukan frame.
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

/// `$(el).attr(nama)` — atribut pertama yang cocok (nama atribut huruf kecil).
fn atribut(el: &Elemen, nama: &str) -> Option<String> {
    el.attrs
        .iter()
        .find(|(k, _)| k == nama)
        .map(|(_, v)| decode_entitas(v))
}

/// `.html()` untuk elemen raw-text: tag pembuka apa adanya + isi + penutup.
fn html_dari(el: &Elemen) -> String {
    let mut out = String::new();
    out.push_str(&el.buka);
    for simpul in &el.anak {
        match simpul {
            Simpul::Teks(t) => out.push_str(t),
            Simpul::Elemen(e) => out.push_str(&html_dari(e)),
        }
    }
    out.push_str("</");
    out.push_str(&el.tag);
    out.push('>');
    out
}

/// `$('#id')` — elemen pertama (urutan dokumen) dengan `id` itu.
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

/// `$('meta[property="x"]').attr('content')` — elemen pertama yang cocok.
fn meta_konten(dok: &Elemen, property: &str) -> Option<String> {
    let mut semua = Vec::new();
    dengan_tag(dok, "meta", &mut semua);
    semua
        .into_iter()
        .find(|el| atribut(el, "property").as_deref() == Some(property))
        .and_then(|el| atribut(el, "content"))
}

// ---------------------------------------------------------------------------
// Pola regex JS (dikompilasi sekali)
// ---------------------------------------------------------------------------

struct Pola {
    ukuran: Regex,
    penulis: Regex,
    tanggal: Regex,
    angka: Regex,
    unduhan: Regex,
}

fn pola() -> &'static Pola {
    static P: OnceLock<Pola> = OnceLock::new();
    P.get_or_init(|| Pola {
        // `/(\d+(?:\.\d+)?\s?(?:KB|MB|GB))/i`
        ukuran: Regex::new(r"(?i)(\d+(?:\.\d+)?\s?(?:KB|MB|GB))").expect("pola ukuran"),
        // `/uploaded by\s([^ ]+)/i`
        penulis: Regex::new(r"(?i)uploaded by\s([^ ]+)").expect("pola penulis"),
        // `/on\s(\d+\s[A-Za-z]+\s\d{4})/i`
        tanggal: Regex::new(r"(?i)on\s(\d+\s[A-Za-z]+\s\d{4})").expect("pola tanggal"),
        angka: Regex::new(r"\d+").expect("pola angka"),
        // JS: /https:\\\/\\\/download\d+\.sfile\.(?:co|mobi)\\\/downloadfile
        //      \\\/\d+\\\/\d+\\\/[a-z0-9]+\\\/[^"'\\\s]+(\?[^"']+)?/i
        unduhan: Regex::new(
            r#"(?i)https:\\/\\/download\d+\.sfile\.(?:co|mobi)\\/downloadfile\\/\d+\\/\d+\\/[a-z0-9]+\\/[^"'\\\s]+(\?[^"']+)?"#,
        )
        .expect("pola unduhan"),
    })
}

/// `teks.match(re)?.[grup]`.
fn tangkapan(re: &Regex, teks: &str, grup: usize) -> Option<String> {
    re.captures(teks)
        .and_then(|c| c.get(grup))
        .map(|m| m.as_str().to_string())
}

// ---------------------------------------------------------------------------
// Alur
// ---------------------------------------------------------------------------

/// `(r1.headers['set-cookie'] || []).map(v => v.split(';')[0]).join('; ')`.
fn cookie_dari(headers: &[(String, String)]) -> String {
    headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|(_, v)| v.split(';').next())
        .collect::<Vec<_>>()
        .join("; ")
}

/// GET dengan tiga header tetap JS, plus `cookie` dan `referer` kalau ada.
fn ambil(
    http: &dyn HttpGet,
    url: &str,
    cookie: &str,
    referer: Option<&str>,
) -> Result<HttpResponse, HttpError> {
    let mut req = HttpCall::get(url)
        .header("user-agent", UA)
        .header("accept", ACCEPT)
        .header("accept-language", BAHASA);
    if !cookie.is_empty() {
        req = req.header("cookie", cookie);
    }
    if let Some(r) = referer {
        req = req.header("referer", r);
    }
    http.call(&req)
}

/// `$('span').filter(teks memuat "download").first().text().match(/\d+/)`.
fn hitung_unduhan(dok: &Elemen) -> Option<String> {
    let mut semua = Vec::new();
    dengan_tag(dok, "span", &mut semua);
    let teks = semua
        .into_iter()
        .map(teks_dari)
        .find(|t| t.to_lowercase().contains("download"))?;
    tangkapan(&pola().angka, &teks, 0)
}

/// Balikan JS: kunci selalu ada, `null` kalau tidak ketemu.
fn hasil(
    file_name: Option<String>,
    size_from_text: Option<String>,
    author_name: Option<String>,
    upload_date: Option<String>,
    download_count: Option<String>,
    download_url: Option<String>,
) -> Value {
    let mut out = Map::new();
    for (kunci, nilai) in [
        ("file_name", file_name),
        ("size_from_text", size_from_text),
        ("author_name", author_name),
        ("upload_date", upload_date),
        ("download_count", download_count),
        ("download_url", download_url),
    ] {
        out.insert(
            kunci.to_string(),
            nilai.map(Value::String).unwrap_or(Value::Null),
        );
    }
    Value::Object(out)
}

/// `sfile(url, { signal } = {})` (`sfiledl.js:7-84`).
pub fn sfile(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let res1 = ambil(http, url, "", None)?;
    let cookie = cookie_dari(&res1.headers);
    let html1 = res1.text();
    let dok1 = urai(&html1);

    let file_name = {
        let mut kandidat = Vec::new();
        dengan_tag(&dok1, "h1", &mut kandidat);
        kandidat
            .first()
            .map(|el| teks_dari(el).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let size_from_text = tangkapan(&pola().ukuran, &html1, 1);
    let info = meta_konten(&dok1, "og:description").unwrap_or_default();
    let author_name = tangkapan(&pola().penulis, &info, 1);
    let upload_date = tangkapan(&pola().tanggal, &info, 1);
    let download_count = hitung_unduhan(&dok1);
    let pageurl = meta_konten(&dok1, "og:url").filter(|s| !s.is_empty());

    let Some(pageurl) = pageurl else {
        return Ok(hasil(
            file_name,
            size_from_text,
            author_name,
            upload_date,
            download_count,
            None,
        ));
    };

    let res2 = ambil(http, &pageurl, &cookie, Some(url))?;
    let dok2 = urai(&res2.text());
    let gate_url = cari_id(&dok2, "download")
        .and_then(|el| atribut(el, "href"))
        .filter(|s| !s.is_empty());

    let Some(gate_url) = gate_url else {
        return Ok(hasil(
            file_name,
            size_from_text,
            author_name,
            upload_date,
            download_count,
            None,
        ));
    };

    let res3 = ambil(http, &gate_url, &cookie, Some(pageurl.as_str()))?;
    let dok3 = urai(&res3.text());

    // `cheerio.load(r3.data)('script').map(el => cheerio.load(el).html())
    //  .get().join('\n')`
    let mut skrip = Vec::new();
    dengan_tag(&dok3, "script", &mut skrip);
    let gabungan = skrip
        .iter()
        .copied()
        .map(html_dari)
        .collect::<Vec<_>>()
        .join("\n");

    let download_url = pola()
        .unduhan
        .captures(&gabungan)
        .and_then(|c| c.get(0))
        .map(|m| m.as_str().replace("\\/", "/"));

    Ok(hasil(
        file_name,
        size_from_text,
        author_name,
        upload_date,
        download_count,
        download_url,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::HttpBody;
    use std::sync::Mutex;

    /// Stub `HttpGet` tanpa jaringan: mencatat permintaan dan membalas
    /// (status, body, header) berurutan; setelah habis, balasan terakhir
    /// dipakai lagi.
    /// Satu balasan stub: (status, body, header).
    type BalasanStub = (u16, String, Vec<(String, String)>);

    #[derive(Debug)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: Mutex<Vec<BalasanStub>>,
        hitung: Mutex<usize>,
    }

    impl StubHttp {
        fn new(balasan: Vec<BalasanStub>) -> Self {
            Self {
                permintaan: Mutex::new(Vec::new()),
                balasan: Mutex::new(balasan),
                hitung: Mutex::new(0),
            }
        }
    }

    impl oktz_plugin_api::HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini lewat HttpCall::call")
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
            _parts: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini cuma GET")
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

    fn halaman_masukan() -> String {
        r#"<html><head>
           <meta property="og:url" content="https://sfile.co/abc">
           <meta property="og:description" content="Spotify PREMIUM_9.1.14.864 uploaded by Planetvdn on 22 January 2026">
           </head><body>
           <h1> Spotify PREMIUM_9.1.14.864 </h1>
           <p>Ukuran: 100.42 MB</p>
           <span>Downloads 1</span>
           </body></html>"#
            .to_string()
    }

    #[test]
    fn alur_tiga_permintaan_mengambil_semua_kolom() {
        let http = StubHttp::new(vec![
            (
                200,
                halaman_masukan(),
                vec![("set-cookie".to_string(), "sf=1; Path=/; HttpOnly".to_string())],
            ),
            (
                200,
                r#"<html><body><a id="download" href="https://sfile.co/gate/2200881">Unduh</a></body></html>"#
                    .to_string(),
                Vec::new(),
            ),
            (
                200,
                r#"<html><body><script>var x = {"url":"https:\/\/download0426.sfile.co\/downloadfile\/2200881\/724157\/99f1f1d69f8fd931e1447af03be9cec0\/spotify-premium_9.1.14.864.apk?k=8a782a007c623b62889412b2f5a49424"};</script></body></html>"#
                    .to_string(),
                Vec::new(),
            ),
        ]);

        let hasil = sfile(&http, "https://sfile.co/abc").unwrap();
        assert_eq!(hasil["file_name"], "Spotify PREMIUM_9.1.14.864");
        assert_eq!(hasil["size_from_text"], "100.42 MB");
        assert_eq!(hasil["author_name"], "Planetvdn");
        assert_eq!(hasil["upload_date"], "22 January 2026");
        assert_eq!(hasil["download_count"], "1");
        assert_eq!(
            hasil["download_url"],
            "https://download0426.sfile.co/downloadfile/2200881/724157/99f1f1d69f8fd931e1447af03be9cec0/spotify-premium_9.1.14.864.apk?k=8a782a007c623b62889412b2f5a49424"
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[0].url, "https://sfile.co/abc");
        assert_eq!(p[1].url, "https://sfile.co/abc");
        assert_eq!(p[2].url, "https://sfile.co/gate/2200881");
        for permintaan in p.iter() {
            assert_eq!(cari_header(permintaan, "user-agent"), Some(UA.to_string()));
            assert_eq!(
                cari_header(permintaan, "accept-language"),
                Some(BAHASA.to_string())
            );
        }
        // Permintaan pertama belum kirim cookie/referer.
        assert_eq!(cari_header(&p[0], "cookie"), None);
        assert_eq!(cari_header(&p[0], "referer"), None);
        // Dua permintaan berikutnya membawa cookie dari respons pertama.
        assert_eq!(cari_header(&p[1], "cookie"), Some("sf=1".to_string()));
        assert_eq!(cari_header(&p[2], "cookie"), Some("sf=1".to_string()));
        // Referer berganti: masukan → og:url.
        assert_eq!(
            cari_header(&p[1], "referer"),
            Some("https://sfile.co/abc".to_string())
        );
        assert_eq!(
            cari_header(&p[2], "referer"),
            Some("https://sfile.co/abc".to_string())
        );
    }

    #[test]
    fn tanpa_og_url_langsung_selesai_dengan_download_url_null() {
        let http = StubHttp::new(vec![(
            200,
            r"<html><body><h1>Berkas</h1><span>Downloads 2</span></body></html>".to_string(),
            Vec::new(),
        )]);
        let hasil = sfile(&http, "https://sfile.co/x").unwrap();
        assert_eq!(hasil["file_name"], "Berkas");
        assert_eq!(hasil["download_count"], "2");
        assert_eq!(hasil["download_url"], Value::Null);
        assert_eq!(hasil["size_from_text"], Value::Null);
        assert_eq!(hasil["author_name"], Value::Null);
        assert_eq!(hasil["upload_date"], Value::Null);
        // Tanpa `og:url` JS berhenti sebelum permintaan kedua (`sfiledl.js:32-43`).
        assert_eq!(http.permintaan.lock().unwrap().len(), 1);
    }


    /// Dua cabang sekaligus supaya tidak ada test yatim: tanpa `#download` berhenti
    /// di permintaan kedua, dan URL polos di `<script>` (tanpa `https:\/\/`)
    /// tetap menghasilkan `download_url: null` setelah tiga permintaan.
    #[test]
    fn gerbang_tanpa_tombol_dan_url_polos_keduanya_null() {
        let http = StubHttp::new(vec![
            (200, halaman_masukan(), Vec::new()),
            (200, "<html><body><p>tidak ada tombol</p></body></html>".to_string(), Vec::new()),
        ]);
        let hasil = sfile(&http, "https://sfile.co/abc").unwrap();
        assert_eq!(hasil["file_name"], "Spotify PREMIUM_9.1.14.864");
        assert_eq!(hasil["download_url"], Value::Null);
        assert_eq!(http.permintaan.lock().unwrap().len(), 2);

        let http = StubHttp::new(vec![
            (200, halaman_masukan(), Vec::new()),
            (
                200,
                r#"<div id="download" href="https://sfile.co/gate"></div>"#.to_string(),
                Vec::new(),
            ),
            (
                200,
                r#"<script>var x = "https://download0426.sfile.co/downloadfile/1/2/abc/x.apk";</script>"#
                    .to_string(),
                Vec::new(),
            ),
        ]);
        let hasil = sfile(&http, "https://sfile.co/abc").unwrap();
        assert_eq!(hasil["download_url"], Value::Null);
        assert_eq!(http.permintaan.lock().unwrap().len(), 3);
    }

    #[test]
    fn h1_kosong_dan_span_tanpa_teks_download_jadi_null() {
        let http = StubHttp::new(vec![(
            200,
            r#"<html><body><h1>   </h1><span>3 berkas</span><meta property="og:url" content="https://sfile.co/z"></body></html>"#
                .to_string(),
            Vec::new(),
        ), (
            200,
            "<html></html>".to_string(),
            Vec::new(),
        )]);
        let hasil = sfile(&http, "https://sfile.co/z").unwrap();
        assert_eq!(hasil["file_name"], Value::Null);
        assert_eq!(hasil["download_count"], Value::Null);
        assert_eq!(hasil["download_url"], Value::Null);
    }

    #[test]
    fn pengurai_mengabaikan_komentar_dan_menghormati_void_dan_raw_text() {
        let dok = urai(
            r#"<body><!-- <h1>palsu</h1> --><br><img src="a>b"><script>var a = 1 < 2;</script><h1>halo <b>dunia</b></h1></body>"#,
        );
        let mut h1 = Vec::new();
        dengan_tag(&dok, "h1", &mut h1);
        assert_eq!(h1.len(), 1, "h1 di dalam komentar ikut terhitung");
        assert_eq!(teks_dari(h1[0]), "halo dunia");

        let mut skrip = Vec::new();
        dengan_tag(&dok, "script", &mut skrip);
        assert_eq!(skrip.len(), 1);
        assert_eq!(
            html_dari(skrip[0]),
            "<script>var a = 1 < 2;</script>",
            "isi raw-text tidak boleh diurai jadi tag"
        );
        assert_eq!(atribut(skrip[0], "src"), None);
    }

    #[test]
    fn atribut_dianggap_ada_walau_nilai_kosong_lalu_disaring_pemanggil() {
        let dok = urai(r#"<a id="download" href="">x</a><a id="download" href="https://b">y</a>"#);
        // Elemen pertama yang cocok menang, walau href-nya kosong.
        assert_eq!(atribut(cari_id(&dok, "download").unwrap(), "href"), Some(String::new()));
    }
}
