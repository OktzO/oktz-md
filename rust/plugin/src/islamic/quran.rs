//! `plugins/islamic/quran.js` — ayat Al-Quran dari quran.nu.or.id.
//!
//! `ponytail:` JS memakai `cheerio` untuk parse `h1`, `h1 + span`, dan
//! `div[id]` dengan id numerik, lalu ambil `[dir="rtl"]`, `.text-primary-500`,
//! `.text-neutral-700`. `oktz-plugins` tidak punya parser HTML, jadi semua
//! dipakai manual di bawah.
//!
//! `ponytail:` Balasan di-chunk jika lebih dari 60000 karakter (JS).
//! `Reply::sequence` digunakan untuk beberapa pesan berurutan.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Satu ayat dari halaman surah.
#[derive(Clone, Debug)]
struct Ayat {
    ayat: u32,
    arab: String,
    latin: String,
    arti: String,
}

/// Hasil parsing halaman surah.
struct SurahData {
    surah: String,
    info: String,
    ayat: Vec<Ayat>,
}

/// Ambil halaman dan parse.
fn fetch_quran(http: &dyn oktz_plugin_api::HttpGet, slug: &str) -> Result<SurahData, String> {
    let url = format!("https://quran.nu.or.id/{slug}");
    let res = http
        .get(&url)
        .map_err(|e| format!("Gagal mengambil halaman: {}", e.message))?;
    let html = res.text();
    Ok(parse_quran(&html))
}

/// Parse halaman HTML.
fn parse_quran(html: &str) -> SurahData {
    let surah = first_tag_text(html, "h1").unwrap_or_default();
    let info = h1_next_span(html).unwrap_or_default();
    let ayat = parse_ayat(html);
    SurahData { surah, info, ayat }
}

/// Teks dari tag pertama bernama `nama`.
fn first_tag_text(html: &str, nama: &str) -> Option<String> {
    let open = format!("<{nama}");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&open) {
        let abs = start + pos;
        let after = &html[abs + open.len()..];
        if after.starts_with('>') || after.starts_with(' ') || after.starts_with('/') {
            let content_start = abs + open.len();
            let close = format!("</{nama}>");
            let content_end = html[content_start..].find(&close)?;
            return Some(html[content_start..content_start + content_end].to_string());
        }
        start = abs + open.len();
    }
    None
}

/// Nilai atribut `nama` dari tag pertama.
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

/// Teks dari tag pertama dengan atribut `attr="nilai"`.
fn tag_text_by_attr(html: &str, nama: &str, attr: &str, nilai: &str) -> Option<String> {
    let pola = format!("<{nama} {attr}=\"{nilai}\"");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&pola) {
        let abs = start + pos;
        let after = &html[abs + pola.len()..];
        if after.starts_with('>') || after.starts_with(' ') {
            let content_start = abs + pola.len();
            if let Some(gt) = html[content_start..].find('>') {
                let after_gt = content_start + gt + 1;
                let close = format!("</{nama}>");
                if let Some(cs) = html[after_gt..].find(&close) {
                    return Some(html[after_gt..after_gt + cs].to_string());
                }
            }
        }
        start = abs + pola.len();
    }
    None
}

/// `h1` berikutnya langsung adalah `span`.
fn h1_next_span(html: &str) -> Option<String> {
    // Cari `</h1>` lalu tag `<span` setelahnya.
    if let Some(h1_close) = html.find("</h1>") {
        let after_h1 = &html[h1_close + 5..];
        if let Some(span_pos) = after_h1.find('<') {
            let span_tag = &after_h1[span_pos..];
            if span_tag.starts_with("<span") {
                let gt = span_tag.find('>')?;
                let inner = &span_tag[gt + 1..];
                let close = inner.find("</span>")?;
                return Some(inner[..close].to_string());
            }
        }
    }
    None
}

/// Parse semua `div[id]` dengan id numerik, ambil arab/latin/arti.
fn parse_ayat(html: &str) -> Vec<Ayat> {
    let mut hasil = Vec::new();
    let mut i = 0;
    while i < html.len() {
        let Some(div_pos) = html[i..].find("<div ") else { break };
        let abs = i + div_pos;
        // Cari atribut id
        let id_val = match attr_value(&html[abs..], "id") {
            Some(v) => v,
            None => {
                i = abs + 5;
                continue;
            }
        };
        // Hanya id yang seluruhnya angka
        if !id_val.chars().all(|c| c.is_ascii_digit()) {
            i = abs + 5;
            continue;
        }
        let ayat_num: u32 = id_val.parse().unwrap_or(0);
        // Cari penutup div ini
        let div_end_tag = format!("</div>");
        let after_div = &html[abs..];
        let gt_pos = after_div.find('>').unwrap_or(0);
        let content_start = abs + gt_pos + 1;
        let div_close = html[content_start..].find(&div_end_tag).unwrap_or(html.len() - content_start);
        let div_html = &html[content_start..content_start + div_close];

        let arab = tag_text_by_attr(div_html, "dir", "rtl", "rtl").unwrap_or_default();
        let latin = tag_text_by_class(div_html, "text-primary-500").unwrap_or_default();
        let arti = tag_text_by_class(div_html, "text-neutral-700").unwrap_or_default();

        if !arab.is_empty() && !latin.is_empty() && !arti.is_empty() {
            hasil.push(Ayat { ayat: ayat_num, arab, latin, arti });
        }
        i = content_start + div_close;
    }
    hasil
}

/// Teks dari tag pertama dengan class tertentu.
fn tag_text_by_class(html: &str, kelas: &str) -> Option<String> {
    let pola = format!("class=\"{kelas}\"");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&pola) {
        let abs = start + pos;
        let after = &html[abs + pola.len()..];
        if after.starts_with('>') {
            let content_start = abs + pola.len() + 1;
            // Cari tag penutup — cari tag pembuka yang sama sebelum konten
            let tag_start = html[..abs].rfind('<').unwrap_or(abs);
            let tag_name_end = html[tag_start + 1..]
                .find(|c: char| c == ' ' || c == '>')
                .unwrap_or(0);
            let tag_name = &html[tag_start + 1..tag_start + 1 + tag_name_end];
            let close = format!("</{tag_name}>");
            if let Some(cs) = html[content_start..].find(&close) {
                return Some(html[content_start..content_start + cs].to_string());
            }
        }
        start = abs + pola.len();
    }
    None
}

/// Format balasan surah. Jika lebih dari 60000 karakter, di-chunk.
fn format_surah(data: &SurahData) -> Vec<String> {
    let mut teks = format!("📖 *{}*\n{}\n\n", data.surah, data.info);
    for a in &data.ayat {
        teks.push_str(&format!("{}\n{}\n_{}_\n\n", a.arab, a.latin, a.arti));
    }
    let trimmed = teks.trim();
    if trimmed.len() > 60000 {
        let mut chunks = Vec::new();
        let mut current = String::new();
        for a in &data.ayat {
            let block = format!("{}\n{}\n_{}_\n\n", a.arab, a.latin, a.arti);
            if (current.len() + block.len()) > 60000 {
                chunks.push(current.trim().to_string());
                current = String::new();
            }
            current.push_str(&block);
        }
        if !current.trim().is_empty() {
            chunks.push(current.trim().to_string());
        }
        let mut out = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let prefix = if i == 0 {
                format!("📖 *{}*\n{}\n\n", data.surah, data.info)
            } else {
                String::new()
            };
            out.push(format!("{prefix}{chunk}"));
        }
        out
    } else {
        vec![trimmed.to_string()]
    }
}

fn help(prefix: &str) -> String {
    format!(
        "📖 *QURAN*\n\n> Masukkan nama surah\n\n`Contoh: {prefix}quran al fatihah`\n`Contoh: {prefix}quran al baqarah`"
    )
}

#[plugin(
    name = "quran",
    alias = ["alquran", "bacaquran"],
    category = "islamic",
    description = "Baca ayat Al-Quran berdasarkan nama surah",
    usage = ".quran <nama surah>",
    example = ".quran al fatihah",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn quran(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ").trim();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let slug = query.to_ascii_lowercase().replace(' ', "-");
    let data = match fetch_quran(ctx.http.as_ref(), &slug) {
        Ok(d) => d,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx));
        }
    };

    if data.ayat.is_empty() {
        return Reply::text(format!("❌ Surah *{query}* tidak ditemukan"));
    }

    let chunks = format_surah(&data);
    let mut replies: Vec<Reply> = chunks
        .into_iter()
        .map(Reply::text)
        .collect();
    replies.last_mut().map(|r| r.with_reaction("✅"));

    Reply::sequence(replies)
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
            command: "quran".into(),
            http,
            ..Ctx::default()
        };
        quran(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const HTML: &str = r#"<h1>Al Fatihah</h1><span>Surah ke-1</span>
<div id="1">
  <span dir="rtl">بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ</span>
  <span class="text-primary-500">Bismillahirrahmanirrahim</span>
  <span class="text-neutral-700">Dengan nama Allah</span>
</div>
<div id="2">
  <span dir="rtl">ٱلْحَمْدُ لِلَّهِ رَبِّ ٱلْعَالَمِينَ</span>
  <span class="text-primary-500">Alhamdulillahi rabbil alamin</span>
  <span class="text-neutral-700">Segala puji bagi Allah</span>
</div>"#;

    #[test]
    fn parse_quran_ekstrak_surah_dan_ayat() {
        let data = parse_quran(HTML);
        assert_eq!(data.surah, "Al Fatihah");
        assert_eq!(data.info, "Surah ke-1");
        assert_eq!(data.ayat.len(), 2);
        assert_eq!(data.ayat[0].ayat, 1);
        assert_eq!(data.ayat[0].arab, "بِسْمِ ٱللَّهِ ٱلرَّحْمَٰنِ ٱلرَّحِيمِ");
        assert_eq!(data.ayat[0].latin, "Bismillahirrahmanirrahim");
        assert_eq!(data.ayat[0].arti, "Dengan nama Allah");
    }

    #[test]
    fn slug_dibentuk_dari_query() {
        let q = "al fatihah";
        let slug = q.to_ascii_lowercase().replace(' ', "-");
        assert_eq!(slug, "al-fatihah");
    }

    #[test]
    fn ayat_kosong_menampilkan_tidak_ditemukan() {
        let http = StubHttp::body("<html></html>");
        let r = run(http.clone(), "tidakada");
        assert_eq!(text_of(&r), "❌ Surah *tidakada* tidak ditemukan");
    }

    #[test]
    fn tanpa_query_tampilkan_karcis_pakai() {
        let r = run(StubHttp::offline(), "");
        assert_eq!(text_of(&r), help("."));
    }

    #[test]
    fn jaringan_gagal_menggunakan_error_template() {
        let r = run(StubHttp::offline(), "al fatihah");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.quran` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
    }

    #[test]
    fn format_surah_chunk_melebihi_batas() {
        let mut ayat = Vec::new();
        for i in 0..2000 {
            ayat.push(Ayat {
                ayat: i as u32 + 1,
                arab: "بِسْمِ ٱللَّهِ".to_string(),
                latin: "Bismillah".to_string(),
                arti: "Dengan nama Allah".repeat(100),
            });
        }
        let data = SurahData {
            surah: "Al Fatihah".to_string(),
            info: "Surah ke-1".to_string(),
            ayat,
        };
        let chunks = format_surah(&data);
        assert!(chunks.len() > 1, "harus di-chunk: {}", chunks.len());
        for c in &chunks {
            assert!(c.len() <= 60100, "chunk terlalu panjang: {}", c.len());
        }
    }
}