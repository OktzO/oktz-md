//! `src/scraper/mediafire.js` — GET halaman MediaFire, ambil meta + tautan
//! unduh.
//!
//! Cheerio tidak ada di Rust: ekstraksi memakai pemindai tag regex sederhana
//! (bukan parser HTML penuh) + `decode_entitas`. Struktur yang dibaca sama
//! (`meta[property]`, `<title>`, `#downloadButton`, `a[aria-label]`) dan
//! jatuhan `||` tiap field disalin apa adanya.
//!
//! Yang disalin persis: regex ukuran `\(([^)]+)\)`, pemetaan ekstensi → MIME
//! (`getMimeTypeFromUrl`), dan throw "MediaFire download link not found".

use oktz_plugin_api::{HttpCall, HttpError, HttpGet};
use regex::Regex;
use serde_json::{Value, json};

use crate::common::decode_entitas;

/// `mediafire(url)` (`mediafire.js:41-84`).
pub fn mediafire(http: &dyn HttpGet, url: &str) -> Result<Value, HttpError> {
    let res = http.call(
        &HttpCall::get(url)
            .header("user-agent", "Mozilla/5.0")
            .header(
                "accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            ),
    )?;
    let html = res.text();
    let els = pindai(&html);

    let title = meta_konten(&els, "og:title")
        .filter(|s| !s.is_empty())
        .or_else(|| {
            els.iter()
                .find(|e| e.nama.eq_ignore_ascii_case("title"))
                .map(|e| teks(e.isi()).trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "MediaFire File".to_string());

    let images = meta_konten(&els, "og:image").unwrap_or_default();
    let description = meta_konten(&els, "og:description")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "not found description.".to_string());

    let link_download = els
        .iter()
        .find(|e| e.attr("id").as_deref() == Some("downloadButton"))
        .and_then(|e| e.attr("href"))
        .filter(|s| !s.is_empty())
        .or_else(|| {
            els.iter()
                .find(|e| {
                    e.nama.eq_ignore_ascii_case("a")
                        && e.attr("aria-label").as_deref() == Some("Download file")
                })
                .and_then(|e| e.attr("href"))
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_default();

    let sizes = els
        .iter()
        .find(|e| e.attr("id").as_deref() == Some("downloadButton"))
        .map(|e| teks(e.isi()).trim().to_string())
        .unwrap_or_default();
    let size = Regex::new(r"\(([^)]+)\)")
        .unwrap()
        .captures(&sizes)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default();

    let mimetype = mime_dari_url(&link_download);
    if link_download.is_empty() {
        return Err(HttpError {
            message: "MediaFire download link not found".to_string(),
        });
    }

    Ok(json!({
        "meta": {
            "title": title,
            "images": images,
            "description": description,
        },
        "download": {
            "link_download": link_download,
            "size": size,
            "mimetype": mimetype,
        },
    }))
}

/// `$('meta[property="…"]').attr("content")`.
fn meta_konten(els: &[Elemen], properti: &str) -> Option<String> {
    els.iter()
        .find(|e| {
            e.nama.eq_ignore_ascii_case("meta")
                && e.attr("property").as_deref() == Some(properti)
        })
        .and_then(|e| e.attr("content"))
}

/// `getMimeTypeFromUrl(url)` (`mediafire.js:30-39`).
fn mime_dari_url(url: &str) -> &'static str {
    if url.is_empty() {
        return "application/octet-stream";
    }
    let nama_berkas = url
        .rsplit('/')
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("");
    let ekstensi = if nama_berkas.contains('.') {
        nama_berkas
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase()
    } else {
        String::new()
    };
    match ekstensi.as_str() {
        "7z" => "application/x-7z-compressed",
        "zip" => "application/zip",
        "rar" => "application/x-rar-compressed",
        "apk" => "application/vnd.android.package-archive",
        "exe" => "application/x-msdownload",
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "txt" => "text/plain",
        "json" => "application/json",
        "js" => "application/javascript",
        "html" => "text/html",
        "css" => "text/css",
        _ => "application/octet-stream",
    }
}

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
}

/// Tag HTML yang tidak perlu penutup.
const TAG_VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

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
        let tutup_sendiri =
            tag.ends_with("/>") || TAG_VOID.iter().any(|v| v.eq_ignore_ascii_case(nama));
        hasil.push(Elemen {
            nama,
            tag,
            isi_mulai: m.end(),
            akhir: if tutup_sendiri { m.end() } else { html.len() },
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

/// `$(...).text()` — tag dibuang, entitas HTML didekode.
fn teks(isi: &str) -> String {
    let re = Regex::new(r"(?is)<[^>]*>").unwrap();
    decode_entitas(&re.replace_all(isi, ""))
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
            unreachable!("mediafire lewat HttpCall::call")
        }
        fn post_json(
            &self,
            _url: &str,
            _body: &Value,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("mediafire cuma GET")
        }
        fn post_form(
            &self,
            _url: &str,
            _form: &str,
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("mediafire cuma GET")
        }
        fn post_multipart(
            &self,
            _url: &str,
            _parts: &[(&str, oktz_plugin_api::HttpBody)],
        ) -> Result<oktz_plugin_api::HttpResponse, HttpError> {
            unreachable!("mediafire cuma GET")
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
    fn mengekstrak_meta_tombol_dan_mime() {
        let http = stub(
            r#"<html><head>
                 <meta property="og:title" content="Nama Berkas">
                 <meta property="og:image" content="https://static.mediafire.com/thumb.png">
                 <meta property="og:description" content="Tautan &amp; deskripsi">
               </head><body>
                 <a id="downloadButton" class="input popsok"
                    href="https://download.mediafire.com/file.zip?a=1&amp;b=2">Download (12.5 MB)</a>
               </body></html>"#,
        );
        let hasil = mediafire(&http, "https://www.mediafire.com/file/abc").unwrap();
        assert_eq!(hasil["meta"]["title"], "Nama Berkas");
        assert_eq!(
            hasil["meta"]["images"],
            "https://static.mediafire.com/thumb.png"
        );
        assert_eq!(hasil["meta"]["description"], "Tautan & deskripsi");
        assert_eq!(
            hasil["download"]["link_download"],
            "https://download.mediafire.com/file.zip?a=1&b=2"
        );
        assert_eq!(hasil["download"]["size"], "12.5 MB");
        assert_eq!(hasil["download"]["mimetype"], "application/zip");

        let p = http.permintaan.lock().unwrap();
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].method, oktz_plugin_api::HttpMethod::Get);
        assert_eq!(p[0].url, "https://www.mediafire.com/file/abc");
        let cari = |nama: &str| {
            p[0].headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(nama))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(cari("user-agent").unwrap(), "Mozilla/5.0");
        assert_eq!(
            cari("accept").unwrap(),
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8"
        );
    }

    #[test]
    fn jatuhan_title_dan_aria_label_dipakai() {
        let http = stub(
            r#"<html><head><title>Judul Halaman</title></head><body>
                 <a aria-label="Download file" href="https://download.mediafire.com/laporan.PDF">unduh</a>
               </body></html>"#,
        );
        let hasil = mediafire(&http, "https://x").unwrap();
        assert_eq!(hasil["meta"]["title"], "Judul Halaman");
        assert_eq!(hasil["meta"]["images"], "");
        assert_eq!(hasil["meta"]["description"], "not found description.");
        assert_eq!(
            hasil["download"]["link_download"],
            "https://download.mediafire.com/laporan.PDF"
        );
        assert_eq!(hasil["download"]["size"], "");
        assert_eq!(hasil["download"]["mimetype"], "application/pdf");
    }

    #[test]
    fn title_bawaan_dan_href_kosong_jatuh_ke_aria_label() {
        let http = stub(
            r#"<html><body>
                 <a id="downloadButton" href="">x</a>
                 <a aria-label="Download file" href="https://d/f.txt">unduh</a>
               </body></html>"#,
        );
        let hasil = mediafire(&http, "https://x").unwrap();
        assert_eq!(hasil["meta"]["title"], "MediaFire File");
        assert_eq!(hasil["download"]["link_download"], "https://d/f.txt");
        assert_eq!(hasil["download"]["mimetype"], "text/plain");
        assert_eq!(hasil["download"]["size"], "");
    }

    #[test]
    fn tanpa_link_melempar_pesan_js() {
        let http = stub("<html><body>kosong</body></html>");
        let salah = mediafire(&http, "https://x").unwrap_err();
        assert_eq!(salah.message, "MediaFire download link not found");
    }

    #[test]
    fn mime_dari_url_mengikuti_pemetaan_js() {
        assert_eq!(mime_dari_url(""), "application/octet-stream");
        assert_eq!(mime_dari_url("https://a/b"), "application/octet-stream");
        assert_eq!(mime_dari_url("https://a/b.7z"), "application/x-7z-compressed");
        assert_eq!(mime_dari_url("https://a/b.MP3?x=1"), "audio/mpeg");
        assert_eq!(
            mime_dari_url("https://a/nama.dengan.titik.png"),
            "image/png"
        );
    }
}
