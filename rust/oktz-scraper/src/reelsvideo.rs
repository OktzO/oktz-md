//! `src/scraper/reelsvideo.js` — POST form ke `reelsvideo.io` dengan
//! tanda tangan md5.
//!
//! Yang disalin persis:
//! - `ts = Math.floor(Date.now() / 1000)`, `tt = md5(ts + "X-Fc-Pp-Ty-eZ")`.
//! - Body `URLSearchParams` (spasi jadi `+`, encoding form-urlencoded —
//!   bukan `encodeURIComponent`).
//! - `type` ditentukan urutan carousel → video → photo → unknown.
//!
//! Cheerio tidak ada di Rust: HTML dipindai dengan regex sederhana
//! (bukan parser HTML penuh); perbedaan yang diketahui ada di laporan.

use md5::{Digest, Md5};
use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Map, Value, json};

use crate::common::decode_entitas;

/// `reelsvideo(url)` (`reelsvideo.js:23-70`).
pub fn reelsvideo(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let ts = detik_sekarang();
    let tt = md5_hex(&format!("{ts}X-Fc-Pp-Ty-eZ"));
    let body = form_urlencoded(&[
        ("id", url.to_string()),
        ("locale", "en".to_string()),
        ("cf-turnstile-response", String::new()),
        ("tt", tt),
        ("ts", ts.to_string()),
    ]);

    let res = http.call(
        &HttpCall::post("https://reelsvideo.io/reel/DUU67gXiTwU/?igsh=MTZxdm1yd3pnN3Rvdg==/")
            .header(
                "user-agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36",
            )
            .header("accept", "*/*")
            .header("hx-request", "true")
            .header("hx-current-url", "https://reelsvideo.io/")
            .header("hx-target", "target")
            .header("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
            .header("origin", "https://reelsvideo.io")
            .header("referer", "https://reelsvideo.io/")
            .form(body),
    )?;
    let html = res.text();
    let els = pindai(&html);

    let username = els
        .iter()
        .enumerate()
        .find(|(i, e)| {
            e.nama.eq_ignore_ascii_case("span")
                && e.kelas("text-400-16-18")
                && induk_kelas(&els, *i, "bg-white")
        })
        .map(|(i, _)| teks(els[i].isi()).trim().to_string())
        .filter(|s| !s.is_empty());

    let thumb = els
        .iter()
        .find(|e| e.nama.eq_ignore_ascii_case("div") && e.ada_attr("data-bg"))
        .and_then(|e| e.attr("data-bg"))
        .filter(|s| !s.is_empty());

    let mut videos: Vec<Value> = Vec::new();
    let mut images: Vec<Value> = Vec::new();
    let mut mp3: Vec<Value> = Vec::new();
    for e in els.iter().filter(|e| e.nama.eq_ignore_ascii_case("a")) {
        let href = e.attr("href").filter(|s| !s.is_empty());
        if e.kelas("type_videos") {
            videos.extend(href.map(Value::String));
        } else if e.kelas("type_images") {
            images.extend(href.map(Value::String));
        } else if e.kelas("type_audio") {
            let id = e.attr("data-id").filter(|s| !s.is_empty());
            if let (Some(href), Some(id)) = (href, id) {
                mp3.push(json!({ "id": id, "url": href }));
            }
        }
    }

    let tipe = if !videos.is_empty() && !images.is_empty() {
        "carousel"
    } else if !videos.is_empty() {
        "video"
    } else if !images.is_empty() {
        "photo"
    } else {
        "unknown"
    };

    let mut out = Map::new();
    out.insert("type".to_string(), Value::String(tipe.to_string()));
    out.insert(
        "username".to_string(),
        username.map(Value::String).unwrap_or(Value::Null),
    );
    out.insert(
        "thumb".to_string(),
        thumb.map(Value::String).unwrap_or(Value::Null),
    );
    out.insert("videos".to_string(), Value::Array(videos));
    out.insert("images".to_string(), Value::Array(images));
    out.insert("mp3".to_string(), Value::Array(mp3));
    Ok(Value::Object(out))
}

/// `Math.floor(Date.now() / 1000)`.
fn detik_sekarang() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `createHash("md5").update(teks).digest("hex")`.
fn md5_hex(teks: &str) -> String {
    let mut h = Md5::new();
    h.update(teks.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// `URLSearchParams.toString()` JS: spasi `+`, hanya alfanumerik dan
/// `* - . _` dibiarkan — beda dari `encodeURIComponent` (spasi `%20`,
/// `!'()~` dibiarkan).
fn form_urlencoded(bagian: &[(&str, String)]) -> String {
    bagian
        .iter()
        .map(|(k, v)| format!("{}={}", form_escape(k), form_escape(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b' ' => out.push('+'),
            b'*' | b'-' | b'.' | b'_' => out.push(b as char),
            _ if b.is_ascii_alphanumeric() => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
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

    /// Apakah atribut `nama` ada (termasuk bentuk boolean tanpa `=`).
    fn ada_attr(&self, nama: &str) -> bool {
        ada_attr(self.tag, nama)
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

/// Apakah atribut `nama` ada — termasuk atribut boolean tanpa `=`.
fn ada_attr(tag: &str, nama: &str) -> bool {
    let pola = format!(r#"(?is)(?:^|\s){}(?:\s*=|[\s>])"#, regex::escape(nama));
    Regex::new(&pola).unwrap().is_match(tag)
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

/// Apakah elemen `i` punya leluhur dengan kelas `kelas`.
fn induk_kelas(els: &[Elemen], i: usize, kelas: &str) -> bool {
    let mut sekarang = els[i].induk;
    while let Some(j) = sekarang {
        if els[j].kelas(kelas) {
            return true;
        }
        sekarang = els[j].induk;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct StubHttp {
        permintaan: Mutex<Vec<HttpCall>>,
        balasan: String,
    }

    impl HttpGet for StubHttp {
        fn get(&self, _url: &str) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("reelsvideo cuma POST")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("reelsvideo pakai form-urlencoded")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("reelsvideo lewat HttpCall::call")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("reelsvideo tidak multipart")
        }
        fn call(&self, req: &HttpCall) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            self.permintaan.lock().unwrap().push(req.clone());
            Ok(oktz_plugin_api::HttpResponse {
                status: 200,
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
            balasan: balasan.to_string(),
        }
    }

    #[test]
    fn menyusun_objek_dari_html_khas() {
        let http = stub(
            r#"<html><body>
                 <div class="bg-white p-4"><span class="text-400-16-18"> @user </span></div>
                 <div data-bg="https://thumb/x.jpg"></div>
                 <a class="type_videos d-block" href="https://reelsvideo.io/v/1">v</a>
                 <a class="type_videos" href="">kosong</a>
                 <a class="type_images" href="https://reelsvideo.io/i/1">i</a>
                 <a class="type_audio" href="https://reelsvideo.io/a/1" data-id="7">a</a>
                 <a class="type_audio" href="https://reelsvideo.io/a/2">tanpa id</a>
                 <a class="type_audio" data-id="8">tanpa href</a>
               </body></html>"#,
        );
        let hasil = reelsvideo(&http, "https://www.instagram.com/reel/x").unwrap();
        assert_eq!(hasil["type"], "carousel");
        assert_eq!(hasil["username"], "@user");
        assert_eq!(hasil["thumb"], "https://thumb/x.jpg");
        assert_eq!(hasil["videos"], json!(["https://reelsvideo.io/v/1"]));
        assert_eq!(hasil["images"], json!(["https://reelsvideo.io/i/1"]));
        assert_eq!(
            hasil["mp3"],
            json!([{"id": "7", "url": "https://reelsvideo.io/a/1"}])
        );

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(
            p[0].url,
            "https://reelsvideo.io/reel/DUU67gXiTwU/?igsh=MTZxdm1yd3pnN3Rvdg==/"
        );
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("hx-request").unwrap(), "true");
        assert_eq!(cari("hx-current-url").unwrap(), "https://reelsvideo.io/");
        assert_eq!(cari("hx-target").unwrap(), "target");
        assert_eq!(
            cari("content-type").unwrap(),
            "application/x-www-form-urlencoded; charset=UTF-8"
        );
        assert_eq!(cari("origin").unwrap(), "https://reelsvideo.io");
        assert_eq!(cari("referer").unwrap(), "https://reelsvideo.io/");
    }

    #[test]
    fn body_form_berisi_tt_md5_dari_ts() {
        let http = stub("<html></html>");
        reelsvideo(&http, "https://www.instagram.com/reel/x?a=b").unwrap();
        let p = http.permintaan.lock().unwrap();
        let Some(oktz_plugin_api::HttpBody::Form(body)) = &p[0].body else {
            panic!("body harus Form");
        };
        let bagian: std::collections::HashMap<&str, &str> = body
            .split('&')
            .map(|bagian| bagian.split_once('=').unwrap())
            .collect();
        assert_eq!(bagian["id"], "https%3A%2F%2Fwww.instagram.com%2Freel%2Fx%3Fa%3Db");
        assert_eq!(bagian["locale"], "en");
        assert_eq!(bagian["cf-turnstile-response"], "");
        let ts: u64 = bagian["ts"].parse().unwrap();
        assert_eq!(bagian["tt"], md5_hex(&format!("{ts}X-Fc-Pp-Ty-eZ")));
    }

    #[test]
    fn type_dan_null_saat_penanda_tidak_ada() {
        let http = stub(
            r#"<html><body>
                 <div class="bg-white"><span class="text-400-16-18"></span></div>
                 <span class="text-400-16-18">bukan di bg-white</span>
                 <div data-bg=""></div>
                 <a class="type_images" href="https://reelsvideo.io/i/9">i</a>
               </body></html>"#,
        );
        let hasil = reelsvideo(&http, "https://x").unwrap();
        assert_eq!(hasil["type"], "photo");
        assert_eq!(hasil["username"], Value::Null, "teks kosong → null");
        assert_eq!(hasil["thumb"], Value::Null, "data-bg kosong → null");
        assert_eq!(hasil["videos"], json!([]));
        assert_eq!(hasil["mp3"], json!([]));

        let http = stub("<html><body><p>tak ada apa-apa</p></body></html>");
        let hasil = reelsvideo(&http, "https://x").unwrap();
        assert_eq!(hasil["type"], "unknown");
        assert_eq!(hasil["username"], Value::Null);
        assert_eq!(hasil["thumb"], Value::Null);
    }
}
