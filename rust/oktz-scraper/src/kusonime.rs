//! `src/scraper/kusonime.js` — daftar anime dari `kusonime.com`: halaman
//! depan, pencarian, genre, dan detail lengkap dengan tautan unduhan.
//!
//! Cheerio tidak tersedia di crate ini, jadi selector di bawah dibedah dari
//! pohon elemen buatan sendiri; preseden `konachan.rs`. Batasnya jujur:
//! selector `:has()` yang dipakai JS hanya bisa ditangani untuk pola yang
//! benar-benar ada di halaman kusonime (`p` yang memuat `i.fa-clock-o` atau
//! `i.fa-tag`), dan isi `<script>`/`<style>` diperlakukan sebagai teks mentah
//! seperti parser5.
//!
//! Semantik yang disalin dari JS:
//! - `url` dan `thumb` bisa `undefined` (atribut tidak ada) — `JSON.stringify`
//!   membuang kuncinya, jadi kuncinya memang tidak ikut di hasil.
//! - `attr('src') || attr('data-src')` — `src` kosong ikut jatuh ke `data-src`,
//!   karena string kosong itu falsy.
//! - `text().replace('Released on','').trim()` — hanya kemunculan PERTAMA.
//! - `find('.content p:has(i.fa-tag) a')` lalu `.map(..).get()` menghasilkan
//!   array teks; urutannya urutan dokumen.
//! - `detail` memanggil halaman yang sama DUA kali: sekali untuk blok unduhan,
//!   sekali lagi di dalam fungsi `metadata`.
//! - `catch` di dalam `metadata` mengembalikan objek `Error` itu sendiri, dan
//!   `JSON.stringify(new Error(..))` adalah `{}` — jadi metadata kosong, bukan
//!   pesan error.
//! - `.info p` dipecah pada titik dua PERTAMA; sisanya rejoined, jadi nilai
//!   `12:30` tetap utuh.

use std::sync::LazyLock;

use oktz_plugin_api::{HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::{decode_entitas, encode_uri_component};

/// `kusonime.latest()` (`kusonime.js:9-52`) — halaman depan.
pub fn kusonime_latest(http: &dyn HttpGet) -> Result<Value, HttpError> {
    let html = http.get("https://kusonime.com/")?.text();
    Ok(daftar_kover(&urai(&html)))
}

/// `kusonime.search(q, page = 1)` (`kusonime.js:53-96`).
///
/// `page` punya bawaan `1` di JS; di Rust pemanggil yang menetakkannya.
pub fn kusonime_search(http: &dyn HttpGet, q: &str, page: u64) -> Result<Value, HttpError> {
    let html = http
        .get(&format!(
            "https://kusonime.com/page/{page}/?s={}",
            encode_uri_component(q)
        ))?
        .text();
    Ok(daftar_kover(&urai(&html)))
}

/// `kusonime.genre(genres, page = 1)` (`kusonime.js:97-140`).
pub fn kusonime_genre(http: &dyn HttpGet, genres: &str, page: u64) -> Result<Value, HttpError> {
    let html = http
        .get(&format!(
            "https://kusonime.com/genres/{}/page/{page}",
            encode_uri_component(genres)
        ))?
        .text();
    Ok(daftar_kover(&urai(&html)))
}

/// `kusonime.detail(url)` (`kusonime.js:141-253`) — metadata + daftar unduhan.
pub fn kusonime_detail(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let doc = urai(&http.get(url)?.text());
    let download = kumpulan_batch(&doc);
    // `catch (error) { return error }` di `metadata`: objek `Error` diserialisasi
    // jadi `{}`, jadi metadata kosong — bukan pesan error.
    let metadata = match metadata(http, url) {
        Ok(m) => m,
        Err(_) => json!({}),
    };
    Ok(json!({ "metadata": metadata, "download": download }))
}

/// `metadata(url)` (`kusonime.js:146-197`) — pemanggilan kedua ke halaman yang
/// sama, di dalam `try` sendiri.
fn metadata(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let doc = urai(&http.get(url)?.text());
    let mut met = Map::new();

    // `h1.jdlz` — satu elemen dengan dua syarat, BUKAN `h1` di dalam `.jdlz`.
    let judul = turun(&doc, &tag_kelas("h1", "jdlz"));
    met.insert("title".to_string(), json!(teks_trim(&judul)));

    // `.post-thumb img.wp-post-image` — `wp-post-image` menempel pada `img`
    // yang sama, bukan elemen di bawahnya; `attr('src') || ''`.
    let poster = rantai(&doc, &[&kelas("post-thumb"), &tag_kelas("img", "wp-post-image")])
        .first()
        .and_then(|n| n.atribut("src"))
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    met.insert("poster_url".to_string(), json!(poster));

    // `$('.info p').each(...)` — urutan penyisipan kunci mengikuti dokumen.
    let mut info = Map::new();
    for p in rantai(&doc, &[&kelas("info"), &tag_el("p")]) {
        let baris_mentah = p.teks_elemen();
        let baris = trim_js(&baris_mentah);
        let Some((kunci_mentah, nilai_mentah)) = baris.split_once(':') else {
            continue;
        };
        let kunci = trim_js(kunci_mentah).to_lowercase();
        let kunci = spasi_jadi_underscore(&kunci);
        let nilai = trim_js(nilai_mentah).to_string();

        match kunci.as_str() {
            "genre" => {
                let daftar: Vec<Value> = turun(p, &tag_el("a"))
                    .iter()
                    .map(|a| json!(trim_js(&a.teks_elemen())))
                    .collect();
                info.insert("genres".to_string(), Value::Array(daftar));
            }
            "seasons" => {
                let dari_tautan = teks_trim(&turun(p, &tag_el("a")));
                // `seasonLink.text().trim() || value`
                let isi = if dari_tautan.is_empty() {
                    nilai
                } else {
                    dari_tautan.to_string()
                };
                info.insert("season".to_string(), json!(isi));
            }
            "producers" => {
                info.insert("producers".to_string(), json!(nilai));
            }
            _ => {
                info.insert(kunci.clone(), json!(nilai));
            }
        }
    }
    met.insert("info".to_string(), Value::Object(info));

    // `$('.venutama > p')` — anak LANGSUNG saja, dan `parent().attr('class')`
    // dibandingkan utuh dengan `'info'` (bukan "mengandung info").
    let mut bagian = Vec::new();
    for induk in turun(&doc, &kelas("venutama")) {
        let kelas_induk = induk.atribut("class");
        for p in &induk.anak {
            if p.tag != "p" {
                continue;
            }
            let mentah = p.teks_elemen();
            let teks = trim_js(&mentah).to_string();
            let kecil = teks.to_lowercase();
            if teks.is_empty() || kecil.contains("download") || kecil.contains("credit") {
                continue;
            }
            if kelas_induk.as_deref() == Some("info") {
                continue;
            }
            bagian.push(teks);
        }
    }
    met.insert("sinopsis".to_string(), json!(bagian.join("\n\n")));
    met.insert(
        "posted_info".to_string(),
        json!(trim_js(&teks_gabungan(&turun(&doc, &kelas("kategoz"))))),
    );
    Ok(Value::Object(met))
}

/// `.venz > ul > .kover` lalu isi tiap kartu — dipakai tiga fungsi pertama,
/// yang badannya identik (`kusonime.js:16-43`).
fn daftar_kover(doc: &Node) -> Value {
    let mut hasil = Vec::new();
    let kover = rantai_anak(
        doc,
        &[&kelas("venz"), &tag_el("ul"), &kelas("kover")],
    );
    for kartu in kover {
        let tautan_judul = rantai(kartu, &[&kelas("content"), &tag_el("h2"), &tag_el("a")]);
        let gambar = rantai(kartu, &[&kelas("thumb"), &tag_el("img")]);

        let mut o = Map::new();
        o.insert(
            "title".to_string(),
            json!(trim_js(&teks_gabungan(&tautan_judul))),
        );
        // `attr('href')` undefined -> kunci hilang di `JSON.stringify`.
        if let Some(url) = tautan_judul.first().and_then(|n| n.atribut("href")) {
            o.insert("url".to_string(), json!(url));
        }
        // `attr('src') || attr('data-src')` — `src` kosong itu falsy.
        let thumb = gambar
            .first()
            .and_then(|n| n.atribut("src"))
            .filter(|s| !s.is_empty())
            .or_else(|| gambar.first().and_then(|n| n.atribut("data-src")));
        if let Some(thumb) = thumb {
            o.insert("thumb".to_string(), json!(thumb));
        }

        let paragraf_waktu: Vec<&Node> = rantai(kartu, &[&kelas("content"), &tag_el("p")])
            .into_iter()
            .filter(|p| punya_i_kelas(p, "fa-clock-o"))
            .collect();
        // `text().replace('Released on','')` — hanya kemunculan pertama, lalu trim.
        let released_mentah = teks_gabungan(&paragraf_waktu).replacen("Released on", "", 1);
        o.insert(
            "released".to_string(),
            json!(trim_js(&released_mentah).to_string()),
        );

        let mut genre = Vec::new();
        for p in rantai(kartu, &[&kelas("content"), &tag_el("p")]) {
            if !punya_i_kelas(p, "fa-tag") {
                continue;
            }
            for a in turun(p, &tag_el("a")) {
                genre.push(json!(trim_js(&a.teks_elemen())));
            }
        }
        o.insert("genre".to_string(), Value::Array(genre));

        hasil.push(Value::Object(o));
    }
    Value::Array(hasil)
}

/// `$('.smokeddlrh')` — blok unduhan per episode/versi (`kusonime.js:201-239`).
fn kumpulan_batch(doc: &Node) -> Value {
    static POLA_RESOLUSI: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\d{3,4}P").expect("pola resolusi"));
    let mut hasil = Vec::new();
    for blok in turun(doc, &kelas("smokeddlrh")) {
        let title = teks_trim(&turun(blok, &kelas("smokettlrh")));
        if title.is_empty() {
            continue;
        }
        let mut resolutions = Map::new();
        for baris in turun(blok, &kelas("smokeurlrh")) {
            let resolusi = teks_trim(&turun(baris, &tag_el("strong")));
            if resolusi.is_empty() || !POLA_RESOLUSI.is_match(&resolusi) {
                continue;
            }
            let mut links = Vec::new();
            for a in turun(baris, &tag_el("a")) {
                let provider = trim_js(&a.teks_elemen()).to_string();
                let Some(url) = a.atribut("href") else {
                    continue;
                };
                if url.starts_with("http") && !provider.is_empty() {
                    links.push(json!({ "provider": provider, "url": url }));
                }
            }
            if !links.is_empty() {
                resolutions.insert(resolusi, Value::Array(links));
            }
        }
        if !resolutions.is_empty() {
            hasil.push(json!({ "title": title, "resolutions": Value::Object(resolutions) }));
        }
    }
    Value::Array(hasil)
}

fn punya_i_kelas(n: &Node, kelas: &str) -> bool {
    turun(n, &tag_el("i")).iter().any(|i| i.punya_kelas(kelas))
}

fn teks_gabungan(nodes: &[&Node]) -> String {
    let mut out = String::new();
    for n in nodes {
        out.push_str(&n.teks_elemen());
    }
    out
}

/// `$(...).text().trim()` — balik `String`, bukan `&str`: teks hasil
/// `teks_elemen` milik sementara yang tidak boleh dipinjam keluar.
fn teks_trim(nodes: &[&Node]) -> String {
    let mentah = teks_gabungan(nodes);
    trim_js(&mentah).to_string()
}

fn spasi_jadi_underscore(s: &str) -> String {
    static POLA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("pola spasi"));
    POLA.replace_all(s, "_").into_owned()
}

/// `String#trim` JS (termasuk NBSP dan spasi Unicode lain) — salinan privat
/// `konachan.rs`, belum ada di `common.rs`.
fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(c,
            '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{00A0}' | '\u{1680}'
            | '\u{2000}'..='\u{200A}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}'
            | '\u{3000}' | '\u{FEFF}')
    })
}

// ── pohon elemen kecil (pengganti cheerio) ───────────────────────────────────

#[derive(Debug)]
struct Node {
    /// Hurus kecil; `#text` untuk simpul teks, `#document` untuk akar.
    tag: String,
    atribut: Vec<(String, String)>,
    anak: Vec<Node>,
    /// Isi simpul teks (dipakai hanya kalau `tag == "#text"`).
    teks: String,
}

impl Node {
    fn baru(tag: &str, atribut: Vec<(String, String)>) -> Self {
        Self {
            tag: tag.to_string(),
            atribut,
            anak: Vec::new(),
            teks: String::new(),
        }
    }

    fn atribut(&self, nama: &str) -> Option<String> {
        self.atribut
            .iter()
            .find(|(k, _)| k == nama)
            .map(|(_, v)| v.clone())
    }

    fn punya_kelas(&self, kelas: &str) -> bool {
        self.atribut("class")
            .map(|c| c.split_whitespace().any(|k| k == kelas))
            .unwrap_or(false)
    }

    /// `.text()` cheerio: gabungan seluruh teks turunan.
    fn teks_elemen(&self) -> String {
        if self.tag == "#text" {
            return self.teks.clone();
        }
        let mut out = String::new();
        for a in &self.anak {
            out.push_str(&a.teks_elemen());
        }
        out
    }
}

const TAG_KOSONG: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// Isi tag di antara `<` dan `>`: nama tag, atribut, dan penanda self-closing.
fn urai_tag(isi: &str) -> (String, Vec<(String, String)>, bool) {
    let b = isi.as_bytes();
    let mut i = 0usize;
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    let mulai = i;
    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'/' && b[i] != b'=' {
        i += 1;
    }
    let nama = isi[mulai..i].to_lowercase();
    let mut atribut = Vec::new();
    let mut self_close = false;
    while i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        if b[i] == b'/' {
            self_close = true;
            i += 1;
            continue;
        }
        let mulai_nama = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' {
            i += 1;
        }
        if i == mulai_nama {
            i += 1;
            continue;
        }
        let kunci = isi[mulai_nama..i].to_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut nilai = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let kutip = b[i];
                i += 1;
                let mulai_val = i;
                while i < b.len() && b[i] != kutip {
                    i += 1;
                }
                nilai = decode_entitas(&isi[mulai_val..i.min(isi.len())]);
                i += 1;
            } else {
                let mulai_val = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                    i += 1;
                }
                nilai = decode_entitas(&isi[mulai_val..i.min(isi.len())]);
            }
        }
        atribut.push((kunci, nilai));
    }
    (nama, atribut, self_close)
}

/// Bangun pohon dari HTML. `<script>`/`<style>` dilewati sebagai teks mentah,
/// sama seperti parser5 — isinya tidak pernah jadi selector.
fn urai(html: &str) -> Node {
    let mut tumpukan: Vec<Node> = vec![Node::baru("#document", Vec::new())];
    let mut i = 0usize;
    let mut teks_akhir = 0usize;
    while i < html.len() {
        let Some(rel) = html[i..].find('<') else {
            dorong_teks(&mut tumpukan, &html[teks_akhir..]);
            break;
        };
        let mulai = i + rel;
        dorong_teks(&mut tumpukan, &html[teks_akhir..mulai]);

        if html[mulai..].starts_with("<!--") {
            i = html[mulai..]
                .find("-->")
                .map(|p| mulai + p + 3)
                .unwrap_or(html.len());
            teks_akhir = i;
            continue;
        }
        if html[mulai..].starts_with("<!") {
            i = html[mulai..].find('>').map(|p| mulai + p + 1).unwrap_or(html.len());
            teks_akhir = i;
            continue;
        }
        let Some(gt) = html[mulai..].find('>') else {
            dorong_teks(&mut tumpukan, &html[mulai..]);
            break;
        };
        let isi = &html[mulai + 1..mulai + gt];
        if let Some(nama) = isi.strip_prefix('/') {
            let nama = nama.trim().to_lowercase();
            // Stray close tag diabaikan, bukan membongkar seluruh tumpukan.
            if let Some(pos) = tumpukan.iter().rposition(|n| n.tag == nama)
                && pos > 0
            {
                let node = tumpukan.remove(pos);
                tumpukan.last_mut().unwrap().anak.push(node);
            }
            i = mulai + gt + 1;
            teks_akhir = i;
            continue;
        }
        let (nama, atribut, self_close) = urai_tag(isi);
        if nama.is_empty() {
            i = mulai + gt + 1;
            teks_akhir = i;
            continue;
        }
        let kosong = self_close || TAG_KOSONG.iter().any(|k| *k == nama);
        let node = Node::baru(&nama, atribut);
        if kosong {
            tumpukan.last_mut().unwrap().anak.push(node);
        } else {
            tumpukan.push(node);
        }
        i = mulai + gt + 1;
        teks_akhir = i;
        // Isi script/style bukan HTML: lompat ke tag penutupnya.
        if nama == "script" || nama == "style" {
            let penutup = format!("</{nama}");
            let Some(pos) = html[i..].to_lowercase().find(&penutup) else {
                let _ = tumpukan.pop();
                break;
            };
            let dari = i + pos;
            let akhir = html[dari..]
                .find('>')
                .map(|p| dari + p + 1)
                .unwrap_or(html.len());
            tumpukan.pop();
            i = akhir;
            teks_akhir = akhir;
        }
    }
    while tumpukan.len() > 1 {
        let node = tumpukan.pop().unwrap();
        tumpukan.last_mut().unwrap().anak.push(node);
    }
    tumpukan.pop().unwrap()
}

fn dorong_teks(tumpukan: &mut [Node], teks: &str) {
    if teks.is_empty() {
        return;
    }
    let node = Node {
        tag: "#text".to_string(),
        atribut: Vec::new(),
        anak: Vec::new(),
        teks: decode_entitas(teks),
    };
    tumpukan.last_mut().unwrap().anak.push(node);
}

/// Turunan yang cocok, urutan dokumen.
fn turun<'a>(n: &'a Node, cocok: &(dyn Fn(&Node) -> bool + 'static)) -> Vec<&'a Node> {
    let mut out = Vec::new();
    for a in &n.anak {
        if cocok(a) {
            out.push(a);
        }
        out.extend(turun(a, cocok));
    }
    out
}

/// Rantai selektor descendant — `a b c` berarti `c` di dalam `b` di dalam `a`.
fn rantai<'a>(
    mulai: &'a Node,
    langkah: &[&(dyn Fn(&Node) -> bool + 'static)],
) -> Vec<&'a Node> {
    let mut sekarang: Vec<&'a Node> = vec![mulai];
    for l in langkah {
        let mut berikut: Vec<&'a Node> = Vec::new();
        for n in &sekarang {
            berikut.extend(turun(n, *l));
        }
        sekarang = berikut;
    }
    sekarang
}

/// Rantai selektor anak langsung — `a > b > c`.
fn rantai_anak<'a>(
    mulai: &'a Node,
    langkah: &[&(dyn Fn(&Node) -> bool + 'static)],
) -> Vec<&'a Node> {
    let mut sekarang: Vec<&'a Node> = vec![mulai];
    for l in langkah {
        let mut berikut: Vec<&'a Node> = Vec::new();
        for n in &sekarang {
            berikut.extend(n.anak.iter().filter(|a| (*l)(a)));
        }
        sekarang = berikut;
    }
    sekarang
}

fn kelas(k: &'static str) -> impl Fn(&Node) -> bool + 'static {
    move |n: &Node| n.punya_kelas(k)
}

fn tag_el(t: &'static str) -> impl Fn(&Node) -> bool + 'static {
    move |n: &Node| n.tag == t
}

/// Selektor majemuk `tag.kelas` — syaratnya menempel pada elemen yang SAMA.
fn tag_kelas(t: &'static str, k: &'static str) -> impl Fn(&Node) -> bool + 'static {
    move |n: &Node| n.tag == t && n.punya_kelas(k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpResponse};
    use std::sync::Mutex;

    #[derive(Debug)]
    struct Stub {
        urls: Mutex<Vec<String>>,
        halaman: String,
        /// Kalau Some, GET paling awal yang sampai Amount gagal.
        gagal: Option<(usize, String)>,
    }

    impl Stub {
        fn baru(halaman: &str) -> Self {
            Self {
                urls: Mutex::new(Vec::new()),
                halaman: halaman.to_string(),
                gagal: None,
            }
        }

        /// GET ke-`n` (basis 0) dan seterusnya gagal — untuk menguji `catch`
        /// di dalam `metadata` tanpa membuat dua stub.
        fn gagal_mulai(n: usize, halaman: &str) -> Self {
            Self {
                gagal: Some((n, "jaringan putus".to_string())),
                ..Self::baru(halaman)
            }
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            let ke = self.urls.lock().unwrap().len();
            self.urls.lock().unwrap().push(url.to_string());
            if let Some((dari, pesan)) = &self.gagal
                && ke >= *dari
            {
                return Err(HttpError {
                    message: pesan.clone(),
                });
            }
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: std::sync::Arc::from(self.halaman.as_bytes()),
                    mimetype: None,
                },
                headers: Vec::new(),
            })
        }
        fn post_json(&self, _u: &str, _b: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn post_form(&self, _u: &str, _f: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
        fn post_multipart(
            &self,
            _u: &str,
            _p: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("scraper ini hanya GET")
        }
    }

    const DAFTAR: &str = r##"<div class="venz">
  <ul>
    <li class="kover">
      <div class="thumb"><img data-src="https://img/lazy.jpg" src="https://img/cover.jpg"></div>
      <div class="content">
        <h2><a href="https://kusonime.com/anime-a/"> Anime &amp; A </a></h2>
        <p><i class="fa fa-clock-o"></i> Released on 12 Jan 2024 </p>
        <p><i class="fa fa-tag"></i><a href="/genres/action/">Action</a><a href="/genres/isekai/">Isekai</a></p>
      </div>
    </li>
    <li class="kover">
      <div class="thumb"><img data-src="https://img/lazy2.jpg"></div>
      <div class="content">
        <h2><a> Tanpa Tautan </a></h2>
        <p><i class="fa fa-clock-o"></i>Released on</p>
      </div>
    </li>
  </ul>
</div>"##;

    #[test]
    fn latest_membaca_kartu_lengkap_dan_kunci_yang_hilang_dihilangkan() {
        let http = Stub::baru(DAFTAR);
        let hasil = kusonime_latest(&http).unwrap();
        let arr = hasil.as_array().unwrap();
        assert_eq!(arr.len(), 2);

        assert_eq!(arr[0]["title"], "Anime & A");
        assert_eq!(arr[0]["url"], "https://kusonime.com/anime-a/");
        // `src` menang kalau ada dan tidak kosong.
        assert_eq!(arr[0]["thumb"], "https://img/cover.jpg");
        // Hanya kemunculan pertama "Released on" yang dihapus.
        assert_eq!(arr[0]["released"], "12 Jan 2024");
        assert_eq!(arr[0]["genre"], json!(["Action", "Isekai"]));

        // Tanpa `href` -> kunci `url` hilang, bukan `null`.
        assert_eq!(arr[1]["title"], "Tanpa Tautan");
        assert_eq!(arr[1].get("url"), None);
        // `src` kosong/absen -> jatuh ke `data-src`.
        assert_eq!(arr[1]["thumb"], "https://img/lazy2.jpg");
        assert_eq!(arr[1]["released"], "");
        assert_eq!(arr[1]["genre"], json!([]));

        assert_eq!(http.urls.lock().unwrap()[0], "https://kusonime.com/");
    }

    #[test]
    fn search_dan_genre_menggunakan_encode_uri_component() {
        let http = Stub::baru(DAFTAR);
        kusonime_search(&http, "one piece", 2).unwrap();
        kusonime_genre(&http, "action & isekai", 3).unwrap();
        let urls = http.urls.lock().unwrap();
        // Spasi jadi `%20`, bukan `+`.
        assert_eq!(urls[0], "https://kusonime.com/page/2/?s=one%20piece");
        assert_eq!(urls[1], "https://kusonime.com/genres/action%20%26%20isekai/page/3");
    }

    #[test]
    fn halaman_tanpa_daftar_memberi_array_kosong() {
        let http = Stub::baru("<html><body>Kosong</body></html>");
        assert_eq!(kusonime_latest(&http).unwrap(), json!([]));
    }

    const DETAIL: &str = r##"<div class="post-thumb"><img class="wp-post-image" src="https://img/poster.jpg"></div>
<h1 class="jdlz">  Judul Anime  </h1>
<div class="info">
  <p> Judul: Judul Anime </p>
  <p> Status: Ongoing </p>
  <p> Type: TV </p>
  <p> Genre: <a href="/g/1/">Action</a>, <a href="/g/2/">Isekai</a> </p>
  <p> Seasons: <a href="/s/1/">Season 2</a> </p>
  <p> Producers: Studio A, Studio B </p>
  <p> Score: 8.5 </p>
  <p> Durasi: 12:30 per episode </p>
  <p> tanpa titik dua </p>
</div>
<div class="kategoz">  Anime, Action  </div>
<div class="venutama info">
  <p> Paragraf di induk yang juga punya kelas info. </p>
</div>
<div class="venutama">
  <p> Sinopsis pertama. </p>
  <p> Contains DOWNLOAD link here. </p>
  <p> Credit to studio. </p>
  <p> Sinopsis kedua. </p>
</div>
<div class="smokeddlrh">
  <span class="smokettlrh">Episode 1</span>
  <div class="smokeurlrh"><strong>360P</strong><a href="https://a/360">Pixeldrain</a><a href="/lokal">Bukan</a></div>
  <div class="smokeurlrh"><strong>720p HD</strong><a href="https://a/720">Mega</a></div>
  <div class="smokeurlrh"><strong>HD</strong><a href="https://a/hd">Kualitas tanpa angka</a></div>
  <div class="smokeurlrh"><strong>1080P</strong><a href="https://a/1080b">  </a></div>
</div>
<div class="smokeddlrh">
  <span class="smokettlrh">  </span>
  <div class="smokeurlrh"><strong>480P</strong><a href="https://a/480">Nutup</a></div>
</div>
<div class="smokeddlrh">
  <span class="smokettlrh">Tanpa Resolusi</span>
  <div class="smokeurlrh"><strong>480P</strong><span>bukan tautan</span></div>
</div>"##;

    #[test]
    fn detail_membaca_metadata_info_sinopsis_dan_unduhan() {
        let http = Stub::baru(DETAIL);
        let hasil = kusonime_detail(&http, "https://kusonime.com/anime-a/").unwrap();
        let meta = &hasil["metadata"];
        assert_eq!(meta["title"], "Judul Anime");
        assert_eq!(meta["poster_url"], "https://img/poster.jpg");
        assert_eq!(meta["posted_info"], "Anime, Action");

        let info = &meta["info"];
        assert_eq!(info["judul"], "Judul Anime");
        assert_eq!(info["status"], "Ongoing");
        assert_eq!(info["type"], "TV");
        assert_eq!(info["genres"], json!(["Action", "Isekai"]));
        assert_eq!(info["season"], "Season 2");
        assert_eq!(info["producers"], "Studio A, Studio B");
        assert_eq!(info["score"], "8.5");
        // Titik dua di dalam nilai tidak memecah: hanya yang pertama dipecah.
        assert_eq!(info["durasi"], "12:30 per episode");
        // `Genre:`/`Seasons:`/`Producers:` jadi kunci khusus, bukan nama aslinya.
        assert_eq!(info.get("genre"), None);
        assert_eq!(info.get("seasons"), None);

        // Penyaring kata jalan: paragraf ber-"download"/"credit" dibuang.
        // `parent().attr('class') !== 'info'` dibandingkan UTUH, jadi induk
        // ber-class `venutama info` tidak ikut tersaring — paragrafnya tetap
        // masuk, persis seperti JS.
        assert_eq!(
            meta["sinopsis"],
            "Paragraf di induk yang juga punya kelas info.\n\nSinopsis pertama.\n\nSinopsis kedua."
        );

        let unduhan = hasil["download"].as_array().unwrap();
        // Blok tanpa judul dan blok tanpa resolusi tidak ikut.
        assert_eq!(unduhan.len(), 1);
        assert_eq!(unduhan[0]["title"], "Episode 1");
        let res = &unduhan[0]["resolutions"];
        assert_eq!(res["360P"], json!([{ "provider": "Pixeldrain", "url": "https://a/360" }]));
        assert_eq!(res["720p HD"], json!([{ "provider": "Mega", "url": "https://a/720" }]));
        assert_eq!(res.get("HD"), None, "resolusi tanpa angka 3-4 digit");
        assert_eq!(res.get("1080P"), None, "provider kosong membuat link dibuang");
        // `href` non-http juga dibuang walau provider ada.
        let res_polos = &unduhan[0]["resolutions"];
        assert_eq!(res_polos["360P"].as_array().unwrap().len(), 1);

        // Dua GET ke halaman yang sama, persis seperti JS.
        let urls = http.urls.lock().unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().all(|u| u == "https://kusonime.com/anime-a/"));
    }

    #[test]
    fn metadata_gagal_memberi_objek_kosong_bukan_pesan_error() {
        // GET pertama (blok unduhan) berhasil, GET kedua di dalam `metadata`
        // gagal — `catch (error) { return error }` mengembalikan objek `Error`,
        // dan `JSON.stringify(new Error(..))` = `{}`.
        let http = Stub::gagal_mulai(1, DETAIL);
        let hasil = kusonime_detail(&http, "https://kusonime.com/anime-a/").unwrap();
        assert_eq!(hasil["metadata"], json!({}));
        // Blok unduhan dari GET pertama tetap ada.
        assert_eq!(hasil["download"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn get_pertama_gagal_lempar_keluar_seperti_js() {
        // `catch` luar melempar ulang, jadi ini jadi `Err`, bukan objek error.
        let http = Stub::gagal_mulai(0, DETAIL);
        let galat = kusonime_detail(&http, "https://kusonime.com/anime-a/").unwrap_err();
        assert_eq!(galat.message, "jaringan putus");
    }
}