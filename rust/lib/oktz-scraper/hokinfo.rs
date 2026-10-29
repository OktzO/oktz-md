//! `src/scraper/hokinfo.js` — scraper karakter Honor of Kings dari fandom.
//!
//! Yang disalin persis:
//! - `if (!name)` → "Nama karakter kosong"; `fetch` benar-benar dicek
//!   `res.ok` (status di luar 2xx) → "Gagal mengambil halaman karakter".
//! - `clean` = `replace(/\s+/g, " ").trim()`; `.pi-item.pi-data` hanya masuk
//!   kalau label DAN nilai tidak kosong; `profile.Skills` dipecah `[,•]`.
//! - Lore: untuk tiap `h2`/`h3` yang teksnya cocok `/Background|Lore/i`,
//!   telusuri saudara-saudara berikutnya sampai `h2`/`h3` berikutnya, ambil
//!   hanya paragraf; gabung dengan spasi, `null` kalau kosong.
//!
//! Cheerio tidak ada di Rust: HTML dipindai regex sederhana (tag seimbang,
//! seperti keluaran MediaWiki). Perbedaan yang diketahui ada di laporan.

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value};

use crate::common::{decode_entitas, encode_uri_component};

/// `scrapeHokCharacter(name)` (`hokinfo.js:2-77`).
pub fn scrape_hok_character(http: &dyn HttpGet, name: &str) -> Result<Value, HttpError> {
    if name.is_empty() {
        return Err(HttpError {
            message: "Nama karakter kosong".to_string(),
        });
    }

    let url = format!(
        "https://honor-of-kings.fandom.com/wiki/{}",
        encode_uri_component(name)
    );
    let res = http.call(
        &HttpCall::get(&url)
            .header("user-agent", "Mozilla/5.0")
            .terima_semua_status(),
    )?;
    if !(200..=299).contains(&res.status) {
        return Err(HttpError {
            message: "Gagal mengambil halaman karakter".to_string(),
        });
    }

    let html = res.text();
    let els = pindai(&html);

    let title = cari(&els, |e| e.attr("id").as_deref() == Some("firstHeading"))
        .map(|i| bersih(&teks(els[i].isi())))
        .unwrap_or_default();
    if title.is_empty() {
        return Err(HttpError {
            message: "Karakter tidak ditemukan".to_string(),
        });
    }

    let mut image = cari(&els, |e| e.kelas("pi-image"))
        .and_then(|k| {
            els.iter()
                .position(|e| e.nama.eq_ignore_ascii_case("img") && dalam_isi(e, &els[k]))
        })
        .and_then(|i| els[i].attr("src"))
        .filter(|s| !s.is_empty());
    if image.is_none() {
        image = els
            .iter()
            .find(|e| {
                e.nama.eq_ignore_ascii_case("meta")
                    && e.attr("property").as_deref() == Some("og:image")
            })
            .and_then(|e| e.attr("content"))
            .filter(|s| !s.is_empty());
    }
    if let Some(gambar) = &image
        && gambar.starts_with("//") {
            image = Some(format!("https:{gambar}"));
        }

    let mut profil: Vec<(String, String)> = Vec::new();
    for e in els.iter().filter(|e| e.kelas("pi-item") && e.kelas("pi-data")) {
        let label = els
            .iter()
            .find(|x| dalam_isi(x, e) && x.kelas("pi-data-label"))
            .map(|x| bersih(&teks(x.isi())))
            .unwrap_or_default();
        let nilai = els
            .iter()
            .find(|x| dalam_isi(x, e) && x.kelas("pi-data-value"))
            .map(|x| bersih(&teks(x.isi())))
            .unwrap_or_default();
        if label.is_empty() || nilai.is_empty() {
            continue;
        }
        // Label kembar menimpa nilai, posisi kunci pertama dipertahankan
        // seperti objek JS.
        match profil.iter().position(|(l, _)| l == &label) {
            Some(pos) => profil[pos].1 = nilai,
            None => profil.push((label, nilai)),
        }
    }

    let bio = cari(&els, |e| e.kelas("mw-parser-output"))
        .and_then(|k| {
            els.iter()
                .position(|e| e.induk == Some(k) && e.nama.eq_ignore_ascii_case("p"))
        })
        .map(|i| bersih(&teks(els[i].isi())))
        .unwrap_or_default();

    let mut skills: Vec<Value> = Vec::new();
    if let Some((_, nilai)) = profil.iter().find(|(l, _)| l == "Skills") {
        for bagian in nilai.split([',', '\u{2022}']) {
            let v = bersih(bagian);
            if !v.is_empty() {
                skills.push(Value::String(v));
            }
        }
    }

    let mut lore_parts: Vec<String> = Vec::new();
    for (i, e) in els.iter().enumerate() {
        if !(e.nama.eq_ignore_ascii_case("h2") || e.nama.eq_ignore_ascii_case("h3")) {
            continue;
        }
        let heading = bersih(&teks(e.isi()));
        let lower = heading.to_lowercase();
        if !lower.contains("background") && !lower.contains("lore") {
            continue;
        }
        let mut j = i;
        while let Some(k) = (j + 1..els.len()).find(|&k| els[k].induk == e.induk) {
            if els[k].nama.eq_ignore_ascii_case("h2") || els[k].nama.eq_ignore_ascii_case("h3") {
                break;
            }
            if els[k].nama.eq_ignore_ascii_case("p") {
                let t = bersih(&teks(els[k].isi()));
                if !t.is_empty() {
                    lore_parts.push(t);
                }
            }
            j = k;
        }
    }

    let mut profile = Map::new();
    for (label, nilai) in profil {
        profile.insert(label, Value::String(nilai));
    }

    let mut out = Map::new();
    out.insert("title".to_string(), Value::String(title));
    out.insert(
        "image".to_string(),
        image.map(Value::String).unwrap_or(Value::Null),
    );
    out.insert("profile".to_string(), Value::Object(profile));
    out.insert("bio".to_string(), Value::String(bio));
    out.insert("skills".to_string(), Value::Array(skills));
    out.insert(
        "lore".to_string(),
        if lore_parts.is_empty() {
            Value::Null
        } else {
            Value::String(lore_parts.join(" "))
        },
    );
    out.insert("url".to_string(), Value::String(url));
    Ok(Value::Object(out))
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

/// `clean` hokinfo: `str?.replace(/\s+/g, " ").trim()`.
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
            unreachable!("hokinfo lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("hokinfo cuma GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("hokinfo cuma GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("hokinfo cuma GET")
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
    fn mengekstrak_title_profil_bio_skills_dan_lore() {
        let http = stub(
            r#"<html><head><meta property="og:image" content="https://og/cadangan.png"></head><body>
                 <h1 id="firstHeading" class="page-header__title"><span class="mw-page-title-main">Angela</span></h1>
                 <aside class="portable-infobox pi-background pi-image">
                   <figure><img src="//static.wikia.nocookie.net/hok/images/a/Ange.png/revision/latest?cb=1" alt="Angela"></figure>
                 </aside>
                 <div class="pi-item pi-data pi-item-spacing" data-source="role">
                   <h3 class="pi-data-label pi-secondary-font">Role</h3>
                   <div class="pi-data-value pi-font"> Support </div>
                 </div>
                 <div class="pi-item pi-data" data-source="skills">
                   <h3 class="pi-data-label">Skills</h3>
                   <div class="pi-data-value">Heal, Shield • Dash</div>
                 </div>
                 <div class="mw-parser-output">
                   <p>Angela adalah <b>hero</b> support.</p>
                   <h2>Background</h2>
                   <p>Lahir di lab.</p>
                   <p>   </p>
                   <div><p>Paragraf di dalam kotak.</p></div>
                   <h3>Lore Resmi</h3>
                   <p>Cerita lainnya.</p>
                 </div>
               </body></html>"#,
        );
        let hasil = scrape_hok_character(&http, "Angela Sun").unwrap();
        assert_eq!(hasil["title"], "Angela");
        assert_eq!(
            hasil["image"],
            "https://static.wikia.nocookie.net/hok/images/a/Ange.png/revision/latest?cb=1"
        );
        assert_eq!(hasil["profile"]["Role"], "Support");
        assert_eq!(hasil["profile"]["Skills"], "Heal, Shield • Dash");
        assert_eq!(hasil["bio"], "Angela adalah hero support.");
        assert_eq!(hasil["skills"], json!(["Heal", "Shield", "Dash"]));
        assert_eq!(hasil["lore"], "Lahir di lab. Cerita lainnya.");
        assert_eq!(
            hasil["url"],
            "https://honor-of-kings.fandom.com/wiki/Angela%20Sun"
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(
            p[0].url,
            "https://honor-of-kings.fandom.com/wiki/Angela%20Sun"
        );
        assert!(p[0].terima_semua_status, "res.ok dicek manual seperti fetch");
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("user-agent").unwrap(), "Mozilla/5.0");
    }

    #[test]
    fn image_jatuh_ke_og_image_lalu_null() {
        let http = stub(
            r#"<html><head><meta property="og:image" content="https://og/g.png"></head>
               <body><h1 id="firstHeading">Bob</h1></body></html>"#,
        );
        let hasil = scrape_hok_character(&http, "Bob").unwrap();
        assert_eq!(hasil["image"], "https://og/g.png");
        assert_eq!(hasil["skills"], json!([]));
        assert_eq!(hasil["lore"], Value::Null);
        assert_eq!(hasil["bio"], "");
        assert_eq!(hasil["title"], "Bob");

        let http = stub(r#"<html><body><h1 id="firstHeading">Bob</h1></body></html>"#);
        let hasil = scrape_hok_character(&http, "Bob").unwrap();
        assert_eq!(hasil["image"], Value::Null);
    }

    #[test]
    fn error_pesan_js() {
        let http = stub("<html></html>");
        assert_eq!(
            scrape_hok_character(&http, "").unwrap_err().message,
            "Nama karakter kosong"
        );
        assert_eq!(
            scrape_hok_character(&http, "X").unwrap_err().message,
            "Karakter tidak ditemukan"
        );

        let mut http = stub("<html></html>");
        http.status = 404;
        assert_eq!(
            scrape_hok_character(&http, "X").unwrap_err().message,
            "Gagal mengambil halaman karakter"
        );
    }
}
