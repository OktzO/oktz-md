//! `plugins/islamic/murrotal.js` — audio murottal Al-Quran dari islamipedia.id.
//!
//! `ponytail:` JS memakai `cheerio` untuk parse `.surah-item` dan atribut
//! `data-title`/`data-audio`. `oktz-plugins` tidak punya parser HTML, jadi
//! markup — `.surah-item`, `h5`, `p`, atribut — dipakai manual di bawah.
//!
//! `ponytail:` `sock.sendMedia(..., { type: 'audio' })` menjadi
//! `Reply::audio(MediaSource::Url)`; URL di-fetch oleh pipeline pada saat
//! kirim, tidak di-download plugin terlebih dahulu.
//!
//! `ponytail:` `m.react` dua kali (`🔍` lalu `✅`/`❌`) — `Reply::with_reaction`
//! hanya membawa satu emoji, jadi reaksi akhir saja yang terkirim.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

/// Satu entri surah dari halaman murottal.
#[derive(Clone, Debug)]
struct SurahItem {
    no: u32,
    surah: String,
    arti: String,
    audio: String,
}

/// Ambil HTML dan parse semua `.surah-item`.
fn fetch_surah_list(http: &dyn oktz_plugin_api::HttpGet) -> Result<Vec<SurahItem>, String> {
    let res = http
        .get("https://islamipedia.id/murottal/")
        .map_err(|e| format!("Gagal mengambil halaman: {}", e.message))?;
    let html = res.text();
    Ok(parse_surah_items(&html))
}

/// Parse `.surah-item` elemen dari HTML.
fn parse_surah_items(html: &str) -> Vec<SurahItem> {
    let mut daftar = Vec::new();
    let mut i = 0;
    let bytes = html.as_bytes();
    while i < bytes.len() {
        let Some(pos) = html[i..].find("class=\"surah-item\"") else { break };
        let abs = i + pos;
        // Cari elemen pembuka lengkap (tag yang berisi class="surah-item")
        let tag_start = html[..abs].rfind('<').unwrap_or(abs);
        let tag_end = match html[abs..].find('>') {
            Some(e) => abs + e + 1,
            None => break,
        };
        let tag_html = &html[tag_start..tag_end];
        // Cari penutup elemen ini (asumsi div)
        let Some(el_end) = html[tag_end..].find("</div>") else { break };
        let abs_el_end = tag_end + el_end + "</div>".len();
        let item_html = &html[tag_start..abs_el_end];

        let no = parse_no(item_html);
        let surah = attr_value(item_html, "data-title").unwrap_or_default();
        let arti = tag_text(item_html, "p").unwrap_or_default();
        let audio = attr_value(item_html, "data-audio").unwrap_or_default();

        if !audio.is_empty() {
            daftar.push(SurahItem { no, surah, arti, audio });
        }
        i = abs_el_end;
    }
    daftar
}

/// Ekstrak nomor dari teks `h5` (format "1. Al Fatihah").
fn parse_no(html: &str) -> u32 {
    let h5 = tag_text(html, "h5").unwrap_or_default();
    let titik = h5.find('.').map_or(h5.len(), |p| p);
    let nomor = h5[..titik].trim();
    nomor.parse::<u32>().unwrap_or(0)
}

/// Nilai atribut `nama` dari tag pertama dalam `html`.
fn attr_value(html: &str, nama: &str) -> Option<String> {
    let pola = format!("{nama}=\"");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&pola) {
        let abs = start + pos + pola.len();
        let end = html[abs..].find('"')?;
        return Some(html[abs..abs + end].to_string());
    }
    None
}

/// Teks dari tag pertama bernama `nama` dalam `html`.
fn tag_text(html: &str, nama: &str) -> Option<String> {
    let open = format!("<{nama}");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&open) {
        let abs = start + pos;
        // Pastikan itu tag pembuka (tidak penutup)
        let after = &html[abs + open.len()..];
        if after.starts_with('>') || after.starts_with(' ') || after.starts_with('/') {
            let close = format!("</{nama}>");
            let content_start = abs + open.len();
            let content_end = html[content_start..].find(&close)?;
            return Some(html[content_start..content_start + content_end].to_string());
        }
        start = abs + open.len();
    }
    None
}

/// Normalisasi query seperti JS: lowercase, hapus non-alphanumeric.
fn normalisasi(q: &str) -> String {
    q.to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn help(prefix: &str) -> String {
    format!(
        "🎧 *MURROTTAL*\n\n> Masukkan nama surah\n\n`Contoh: {prefix}murrotal al fatihah`\n`Contoh: {prefix}murrotal ar rahman`"
    )
}

#[plugin(
    name = "murrotal",
    alias = ["audioquran", "quraudio"],
    category = "islamic",
    description = "Dengarkan audio murottal Al-Quran berdasarkan surah",
    usage = ".murrotal <nama surah>",
    example = ".murrotal al fatihah",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn murrotal(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let daftar = match fetch_surah_list(ctx.http.as_ref()) {
        Ok(d) => d,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command));
        }
    };

    let q = normalisasi(&query);
    let found = daftar.iter().find(|v| {
        normalisasi(&v.surah).contains(&q)
    });

    let Some(s) = found.filter(|v| !v.audio.is_empty()) else {
        return Reply::text(format!("❌ Surah *{query}* tidak ditemukan"));
    };

    Reply::audio(MediaSource::url(s.audio.clone())).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const SENDER: &str = "628111@s.whatsapp.net";

    #[derive(Debug, Default)]
    struct StubHttp {
        body: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl StubHttp {
        fn body(b: &str) -> Arc<StubHttp> {
            Arc::new(StubHttp {
                body: Some(b.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<StubHttp> {
            Arc::new(StubHttp {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for StubHttp {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            match &self.body {
                Some(b) => Ok(HttpResponse {
                    status: 200,
                    body: HttpBody::Bytes {
                        data: Arc::from(b.clone().into_bytes()),
                        mimetype: None,
                    },
                    headers: Vec::new(),
                }),
                None => Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                }),
            }
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<StubHttp>, text: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            prefix: ".".into(),
            command: "murrotal".into(),
            http,
            ..Ctx::default()
        };
        murrotal(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const HTML: &str = r#"<div class="surah-item" data-title="Al Fatihah" data-audio="https://cdn/audio/1.mp3">
  <h5>1. Al Fatihah</h5>
  <p>Pembuka</p>
</div>
<div class="surah-item" data-title="Ar Rahman" data-audio="https://cdn/audio/55.mp3">
  <h5>55. Ar Rahman</h5>
  <p>Rahman</p>
</div>"#;

    #[test]
    fn parse_surah_items_ekstrak_dua() {
        let daftar = parse_surah_items(HTML);
        assert_eq!(daftar.len(), 2);
        assert_eq!(daftar[0].no, 1);
        assert_eq!(daftar[0].surah, "Al Fatihah");
        assert_eq!(daftar[0].arti, "Pembuka");
        assert_eq!(daftar[0].audio, "https://cdn/audio/1.mp3");
        assert_eq!(daftar[1].no, 55);
        assert_eq!(daftar[1].surah, "Ar Rahman");
        assert_eq!(daftar[1].audio, "https://cdn/audio/55.mp3");
    }

    #[test]
    fn normalisasi_hapus_non_alphanumeric() {
        assert_eq!(normalisasi("Al Fatihah"), "alfatihah");
        assert_eq!(normalisasi("ar-rahman"), "arrahman");
    }

    #[test]
    fn tanpa_query_tampilkan_karcis_pakai() {
        let r = run(StubHttp::offline(), "");
        assert_eq!(text_of(&r), help("."));
    }

    #[test]
    fn surah_ditemukan_mengembalikan_audio() {
        let http = StubHttp::body(HTML);
        let r = run(http.clone(), "al fatihah");
        match r.without_mutations() {
            Reply::Audio { media, .. } => {
                let src = media.as_ref().expect("harus ada media");
                assert_eq!(src, &MediaSource::url("https://cdn/audio/1.mp3"));
            }
            other => panic!("harus Audio, dapat {other:?}"),
        }
    }

    #[test]
    fn surah_tidak_ditemukan() {
        let http = StubHttp::body(HTML);
        let r = run(http.clone(), "tidakada");
        assert_eq!(text_of(&r), "❌ Surah *tidakada* tidak ditemukan");
    }

    #[test]
    fn pencarian_tidak_peka_huruf_dan_tanpa_spasi() {
        let http = StubHttp::body(HTML);
        let r = run(http.clone(), "AL FAtiHAH");
        match r.without_mutations() {
            Reply::Audio { media, .. } => {
                let src = media.as_ref().expect("harus ada media");
                assert_eq!(src, &MediaSource::url("https://cdn/audio/1.mp3"));
            }
            other => panic!("harus Audio, dapat {other:?}"),
        }
    }

    #[test]
    fn jaringan_gagal_menggunakan_error_template() {
        let r = run(StubHttp::offline(), "al fatihah");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.murrotal` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
    }
}