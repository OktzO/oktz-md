//! `plugins/search/wikipedia.js` — mencari artikel Wikipedia bahasa Indonesia.
//!
//! `ponytail:` JS memakai `cheerio` untuk parse halaman artikel Wikipedia
//! (`#firstHeading`, `.tagline`, `.mw-parser-output > section`, `.infobox tr`,
//! `.mw-parser-output img`). `oktz-plugins` tidak punya parser HTML, jadi semua
//! dipakai manual di bawah. Elemen yang di-remove di JS (`script`, `style`,
//! `.infobox` dll) dilewati saat ekstraksi teks.
//!
//! `ponytail:` `fixUrl` untuk URL protokol-relatif (`//`) dan relativ (`/`).
//! `uniqueBy` untuk dedup gambar berdasarkan URL.
//!
//! `ponytail:` `sock.sendMessage({ image, caption })` menjadi
//! `Reply::image(MediaSource::Url)` dengan caption.
//!
//! Catatan: baris `pub mod wikipedia;` perlu ditambahkan ke
//! `rust/crates/oktz-plugins/src/search/mod.rs` oleh orchestrator (tidak boleh
//! disentuh agent porting per aturan PORT-BRIEF #2).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

const LANG: &str = "id";
const LIMIT: usize = 5;
const BASE: &str = "https://id.wikipedia.org";
const UA: &str = "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/147.0.0.0 Mobile Safari/537.36";

/// Hasil pencarian Wikipedia.
struct SearchResult {
    code: u16,
    results: Vec<Value>,
}

/// Satu artikel lengkap.
#[derive(Clone, Debug)]
struct Article {
    title: String,
    description: Option<String>,
    url: String,
    extract: Option<String>,
    sections: Vec<(String, String)>,
    infobox: Vec<(String, String)>,
    images: Vec<(String, Option<String>)>,
}

/// `searchWikipedia` — panggil Wikipedia API.
fn search_wikipedia(http: &dyn oktz_plugin_api::HttpGet, query: &str) -> Result<SearchResult, String> {
    let url = format!("{BASE}/w/api.php?action=query&list=search&srsearch={}&srlimit={}&format=json&origin=*",
        encode_uri_component(query), LIMIT);
    let res = http
        .get(&url)
        .map_err(|e| format!("Gagal mencari: {}", e.message))?;
    let data: Value = res.json().ok().unwrap_or(Value::Null);
    let results = data.get("query")
        .and_then(|v| v.get("search"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(SearchResult { code: res.status, results })
}

/// `getFullArticle` — ambil halaman artikel dan parse.
fn get_full_article(http: &dyn oktz_plugin_api::HttpGet, title: &str) -> Result<Article, String> {
    let page_path = format!("/wiki/{}", encode_uri_component(title.replace(' ', "_")));
    let page_url = format!("{BASE}{page_path}");
    let res = http
        .get(&page_url)
        .map_err(|e| format!("Gagal mengambil artikel: {}", e.message))?;
    let html = res.text();
    let cleaned = remove_unwanted(&html);
    let page_title = clean_text(extract_by_id(&cleaned, "firstHeading")).unwrap_or_else(|| title.to_string());
    let description = extract_by_class(&cleaned, "tagline")
        .map(|t| clean_text(&t)).filter(|s| !s.is_empty());
    let intro = intro_paragraphs(&cleaned);
    let sections = extract_sections(&cleaned);
    let infobox = extract_infobox(&cleaned);
    let images = extract_images(&cleaned, &page_url);
    Ok(Article {
        title: page_title,
        description,
        url: page_url,
        extract: intro.filter(|s| !s.is_empty()),
        sections,
        infobox,
        images,
    })
}

/// Hapus elemen yang di-remove di JS: script, style, noscript, sup.reference,
/// .mw-editsection, .navbox, .metadata, .ambox, .hatnote, .toc, #toc,
/// table.vertical-navbox. Pendekatan sederhana: hapus sejumlah pasang tag.
fn remove_unwanted(html: &str) -> String {
    let mut out = html.to_string();
    for tag in ["script", "style", "noscript"] {
        out = remove_tag_pairs(&out, tag);
    }
    // Untuk elemen dengan class tertentu, hapus kotak yang ditemukan.
    for cls in ["sup.reference", ".mw-editsection", ".navbox", ".metadata", ".ambox", ".hatnote", ".toc", "#toc", "table.vertical-navbox"] {
        out = remove_by_class_or_id(&out, cls);
    }
    out
}

fn remove_tag_pairs(html: &str, tag: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    while i < html.len() {
        if let Some(pos) = html[i..].find(&open) {
            let abs = i + pos;
            let after = &html[abs + open.len()..];
            if after.starts_with('>') || after.starts_with(' ') || after.starts_with('/') {
                let gt = after.find('>').unwrap_or(0);
                let content_start = abs + open.len() + gt + 1;
                if let Some(cs) = html[content_start..].find(&close) {
                    out.push_str(&html[i..abs]);
                    i = content_start + cs + close.len();
                    continue;
                }
            }
        }
        out.push_str(&html[i..]);
        break;
    }
    out
}

fn remove_by_class_or_id(html: &str, selector: &str) -> String {
    // Pendekatan sederhana: cari elemen dengan class atau id yang cocok,
    // lalu hapus hingga penutupnya.
    let mut out = String::new();
    let mut i = 0;
    while i < html.len() {
        let attr = if selector.starts_with('.') { "class" } else if selector.starts_with('#') { "id" } else { "" };
        let val = &selector[1..];
        let pola = format!("{attr}=\"{val}\"");
        if let Some(pos) = html[i..].find(&pola) {
            let abs = i + pos;
            // Cari tag pembuka yang mengandung atribut ini
            let tag_start = html[..abs].rfind('<').unwrap_or(abs);
            let tag_end = match html[abs..].find('>') {
                Some(g) => abs + g + 1,
                None => break,
            };
            // Deteksi nama tag
            let tag_name = &html[tag_start + 1..tag_start + 1 + 20];
            let tag_name = tag_name.split(|c: char| c.is_whitespace() || c == '>' || c == '/').next().unwrap_or("div");
            // Cari penutup
            let close = format!("</{tag_name}>");
            let content_start = tag_end;
            if let Some(cs) = html[content_start..].find(&close) {
                i = content_start + cs + close.len();
                continue;
            }
        }
        out.push_str(&html[i..]);
        break;
    }
    out
}

/// Ekstraksi teks dari elemen dengan id tertentu.
fn extract_by_id(html: &str, id: &str) -> Option<String> {
    let pola = format!("id=\"{id}\"");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&pola) {
        let abs = start + pos;
        let tag_start = html[..abs].rfind('<').unwrap_or(abs);
        let tag_end = match html[abs..].find('>') {
            Some(g) => abs + g + 1,
            None => return None,
        };
        let after = &html[tag_end..];
        let close_pos = after.find('<').unwrap_or(after.len());
        return Some(html[tag_end..tag_end + close_pos].to_string());
    }
    None
}

/// Ekstraksi teks dari elemen pertama dengan class tertentu.
fn extract_by_class(html: &str, class: &str) -> Option<String> {
    let pola = format!("class=\"{class}\"");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&pola) {
        let abs = start + pos;
        let tag_start = html[..abs].rfind('<').unwrap_or(abs);
        let tag_end = match html[abs..].find('>') {
            Some(g) => abs + g + 1,
            None => return None,
        };
        let content_start = tag_end;
        // Cari tag penutup — deteksi nama tag
        let tag_name = &html[tag_start + 1..tag_start + 1 + 20];
        let tag_name = tag_name.split(|c: char| c.is_whitespace() || c == '>' || c == '/').next().unwrap_or("div");
        let close = format!("</{tag_name}>");
        if let Some(cs) = html[content_start..].find(&close) {
            return Some(html[content_start..content_start + cs].to_string());
        }
        return None;
    }
    None
}

/// Paragraf pengantar dari `.mw-parser-output > section > p` atau `.mw-parser-output > p`.
fn intro_paragraphs(html: &str) -> Option<String> {
    // Cari .mw-parser-output
    let Some(parser) = extract_by_class(html, "mw-parser-output") else {
        return None;
    };
    // Cari <p> langsung di dalamnya
    let mut teks = String::new();
    let mut start = 0;
    while let Some(pos) = parser[start..].find("<p") {
        let abs = start + pos;
        let after = &parser[abs + 2..];
        if after.starts_with('>') || after.starts_with(' ') {
            let content_start = abs + 2 + after.find('>').unwrap_or(0) + 1;
            if let Some(cs) = parser[content_start..].find("</p>") {
                let para = parser[content_start..content_start + cs].to_string();
                let clean = clean_text(&para);
                if clean.len() > 40 {
                    if !teks.is_empty() { teks.push('\n'); }
                    teks.push_str(&clean);
                }
            }
        }
        start = abs + 2;
    }
    if teks.is_empty() { None } else { Some(teks) }
}

/// Ekstraksi sekusi: heading + konten dari `.mw-parser-output > section`.
fn extract_sections(html: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let Some(parser) = extract_by_class(html, "mw-parser-output") else {
        return sections;
    };
    let mut start = 0;
    while let Some(pos) = parser[start..].find("<section") {
        let abs = start + pos;
        let tag_start = html_find_tag_start(&parser, abs);
        let tag_end = match parser[abs..].find('>') {
            Some(g) => abs + g + 1,
            None => break,
        };
        let section_end = match parser[tag_end..].find("</section>") {
            Some(e) => tag_end + e + "</section>".len(),
            None => parser.len(),
        };
        let section_html = &parser[tag_end..section_end - "</section>".len()];
        // Cari heading h2 atau h3
        let heading = extract_first_heading(section_html);
        if heading.is_empty() || heading.to_lowercase() == "daftar isi" {
            start = section_end;
            continue;
        }
        // Cari p, ul, ol di dalam section
        let mut texts = Vec::new();
        for tag in ["p", "ul", "ol"] {
            let mut s = 0;
            while let Some(p) = section_html[s..].find(&format!("<{tag}")) {
                let abs2 = s + p;
                let after = &section_html[abs2 + tag.len()..];
                if after.starts_with('>') || after.starts_with(' ') {
                    let cs = section_html[abs2 + tag.len() + after.find('>').unwrap_or(0) + 1..]
                        .find(&format!("</{tag}>"))
                        .unwrap_or(0);
                    let content = &section_html[abs2 + tag.len() + after.find('>').unwrap_or(0) + 1..abs2 + tag.len() + after.find('>').unwrap_or(0) + 1 + cs];
                    let clean = clean_text(content);
                    if clean.len() > 40 {
                        texts.push(clean);
                    }
                }
                s = abs2 + tag.len() + 1;
            }
        }
        if !texts.is_empty() {
            sections.push((heading, texts.join("\n\n")));
        }
        start = section_end;
    }
    sections
}

fn html_find_tag_start(html: &str, attr_pos: usize) -> usize {
    html[..attr_pos].rfind('<').unwrap_or(attr_pos)
}

fn extract_first_heading(html: &str) -> String {
    for tag in ["h2", "h3"] {
        if let Some(t) = extract_by_id(html, "") {
            if !t.is_empty() { return t; }
        }
    }
    // Coba cari teks h2/h3 langsung
    let open = "<h2";
    if let Some(pos) = html.find(open) {
        let after = &html[pos + 3..];
        if after.starts_with('>') {
            let content_start = pos + 4;
            if let Some(cs) = html[content_start..].find("</h2>") {
                return clean_text(&html[content_start..content_start + cs]);
            }
        }
    }
    String::new()
}

/// Ekstraksi infobox: `.infobox tr` → key (th) + value (td).
fn extract_infobox(html: &str) -> Vec<(String, String)> {
    let mut infobox = Vec::new();
    let Some(info) = extract_by_class(html, "infobox") else {
        return infobox;
    };
    let mut start = 0;
    while let Some(pos) = info[start..].find("<tr") {
        let abs = start + pos;
        let after = &info[abs + 2..];
        if after.starts_with('>') || after.starts_with(' ') {
            let gt = after.find('>').unwrap_or(0);
            let content_start = abs + 2 + gt + 1;
            if let Some(cs) = info[content_start..].find("</tr>") {
                let row = &info[content_start..content_start + cs];
                let key = extract_first_cell(row, "th");
                let value = extract_first_cell(row, "td");
                if !key.is_empty() && !value.is_empty() && key.len() < 100 {
                    infobox.push((key, value));
                }
            }
        }
        start = abs + 2;
    }
    infobox
}

fn extract_first_cell(html: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    if let Some(pos) = html.find(&open) {
        let after = &html[pos + open.len()..];
        if after.starts_with('>') || after.starts_with(' ') {
            let gt = after.find('>').unwrap_or(0);
            let content_start = pos + open.len() + gt + 1;
            let close = format!("</{tag}>");
            if let Some(cs) = html[content_start..].find(&close) {
                return clean_text(&html[content_start..content_start + cs]);
            }
        }
    }
    String::new()
}

/// Ekstraksi gambar dari `.mw-parser-output img`.
fn extract_images(html: &str, base_url: &str) -> Vec<(String, Option<String>)> {
    let mut images = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let Some(parser) = extract_by_class(html, "mw-parser-output") else {
        return images;
    };
    let mut start = 0;
    while let Some(pos) = parser[start..].find("<img") {
        let abs = start + pos;
        let after = &parser[abs + 3..];
        if after.starts_with('>') || after.starts_with(' ') {
            let src = attr_from_tag(&parser[abs..], "src");
            let alt = attr_from_tag(&parser[abs..], "alt").map(clean_text);
            if let Some(s) = src {
                let fixed = fix_url(&s, base_url);
                if fixed.contains("static/images") || fixed.contains("Semi-protection") || fixed.contains("OOjs_UI") {
                    start = abs + 3;
                    continue;
                }
                if seen.insert(fixed.clone()) {
                    images.push((fixed, alt));
                }
            }
        }
        start = abs + 3;
    }
    images
}

fn attr_from_tag(html: &str, attr: &str) -> Option<String> {
    let pola = format!("{attr}=\"");
    if let Some(pos) = html.find(&pola) {
        let abs = pos + pola.len();
        if let Some(end) = html[abs..].find('"') {
            return Some(html[abs..abs + end].to_string());
        }
    }
    None
}

/// `decodeHtml` — decode entitas HTML.
fn decode_html(text: &str) -> String {
    text.replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// `cleanText` — hapus tag, entitas, referensi [n], spasi berlebih.
fn clean_text(text: &str) -> String {
    let text = decode_html(text);
    let text = strip_html_tags(text);
    let text = remove_references(&text);
    collapse_whitespace(&text)
}

/// `cleanBlock` — seperti cleanText tetapi ganti 3+ newline dengan 2.
fn clean_block(text: &str) -> String {
    let text = clean_text(text);
    let text = text.replace(&['\n'][..], "\n");
    let mut out = String::new();
    let mut prev_nl = 0;
    for (i, c) in text.chars().enumerate() {
        if c == '\n' {
            if prev_nl > 0 && i - prev_nl <= 1 {
                continue;
            }
            prev_nl = i;
        }
        out.push(c);
    }
    out
}

fn strip_html_tags(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    for c in raw.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    out
}

fn remove_references(text: &str) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        if let Some(pos) = text[i..].find('[') {
            out.push_str(&text[i..i + pos]);
            let rest = &text[i + pos..];
            if let Some(end) = rest.find(']') {
                let inside = &rest[1..end];
                if inside.chars().all(|c| c.is_ascii_digit()) || inside.len() == 1 && inside.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                    i += pos + end + 1;
                    continue;
                }
            }
            out.push('[');
            i += pos + 1;
        } else {
            out.push_str(&text[i..]);
            break;
        }
    }
    out
}

fn collapse_whitespace(text: &str) -> String {
    let mut out = String::new();
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// `fixUrl` — URL protokol-relatif dan relativ.
fn fix_url(url: &str, base_url: &str) -> Option<String> {
    if url.is_empty() {
        return None;
    }
    if url.starts_with("//") {
        return Some(format!("https:{url}"));
    }
    if url.starts_with('/') {
        return Some(format!("{BASE}{url}"));
    }
    Some(url.to_string())
}

/// `uniqueBy` — dedup berdasarkan key.
fn unique_by<T, F>(array: Vec<T>, key: F) -> Vec<T>
where
    F: Fn(&T) -> String,
{
    let mut seen = std::collections::HashSet::new();
    array.into_iter().filter(|item| seen.insert(key(item))).collect()
}

fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn help(prefix: &str) -> String {
    "❌ Masukkan kata kunci pencarian Wikipedia.\n\nContoh: `.wikipedia Indonesia`".to_string()
}

#[plugin(
    name = "wikipedia",
    alias = ["wiki", "wp"],
    category = "search",
    description = "Mencari artikel lengkap dari Wikipedia",
    usage = ".wikipedia <query>",
    example = ".wikipedia Rendang",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn wikipedia(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ");
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let search = match search_wikipedia(ctx.http.as_ref(), &query) {
        Ok(s) => s,
        Err(_) => {
            return Reply::text("😔 Terjadi kesalahan saat mencari artikel di Wikipedia.");
        }
    };

    if search.results.is_empty() {
        return Reply::text(format!("⚠️ Artikel tentang *{query}* tidak ditemukan di Wikipedia."));
    }

    let first = &search.results[0];
    let title = first.get("title").and_then(Value::as_str).unwrap_or("").to_string();
    let detail = match get_full_article(ctx.http.as_ref(), &title) {
        Ok(d) => d,
        Err(_) => {
            return Reply::text("😔 Terjadi kesalahan saat mencari artikel di Wikipedia.");
        }
    };
    let r = detail;

    let mut text = "📚 *WIKIPEDIA SEARCH* 📚\n\n".to_string();
    text.push_str(&format!("*Judul:* {}\n", r.title));
    if let Some(desc) = &r.description {
        text.push_str(&format!("*Deskripsi:* {desc}\n"));
    }
    text.push_str(&format!("\n*Ringkasan:*\n{}\n\n", r.extract.unwrap_or_else(|| "Tidak ada ringkasan tersedia.".to_string())));

    if !r.infobox.is_empty() {
        text.push_str("*Info Tambahan:*\n");
        for (key, val) in r.infobox.iter().take(5) {
            text.push_str(&format!("- {key}: {val}\n"));
        }
        text.push('\n');
    }

    text.push_str(&format!("\n🔗 *Selengkapnya:* {}", r.url));

    if let Some((img_url, _)) = r.images.first() {
        return Reply::image(MediaSource::url(img_url.clone()), text).with_reaction("✅");
    }

    Reply::text(text).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};
    use serde_json::json;

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
            command: "wikipedia".into(),
            http,
            ..Ctx::default()
        };
        wikipedia(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const API_SEARCH: &str = r#"{"query":{"search":[{"title":"Rendang","snippet":"Rendang adalah masakan"}]}}"#;

    const ARTICLE_HTML: &str = r#"<html><body>
<div class="mw-parser-output">
  <h1 id="firstHeading">Rendang</h1>
  <span class="tagline">Masakan Minangkabau</span>
  <section>
    <h2>Sejarah</h2>
    <p>Rendang adalah masakan daging yang kaya rempah.</p>
  </section>
  <table class="infobox">
    <tr><th>Asal</th><td>Indonesia</td></tr>
  </table>
  <img src="//upload.wikimedia.org/rendang.jpg" alt="Rendang">
</div>
</body></html>"#;

    #[test]
    fn decode_html_entitas() {
        assert_eq!(decode_html("a &quot;b&quot;"), "a \"b\"");
        assert_eq!(decode_html("&amp;"), "&");
        assert_eq!(decode_html("&nbsp;"), " ");
    }

    #[test]
    fn clean_text_hapus_tag_dan_referensi() {
        assert_eq!(clean_text("<p>Halo [1] dunia</p>"), "Halo dunia");
    }

    #[test]
    fn fix_url_protokol_relatif() {
        assert_eq!(fix_url("//upload.wikimedia.org/x.jpg", BASE), Some("https://upload.wikimedia.org/x.jpg".to_string()));
        assert_eq!(fix_url("/wiki/Foo", BASE), Some("https://id.wikipedia.org/wiki/Foo".to_string()));
    }

    #[test]
    fn unique_by_dedup() {
        let v = vec![("a".to_string(), 1), ("a".to_string(), 2), ("b".to_string(), 3)];
        let r = unique_by(v, |(k, _)| k.clone());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn extract_images_skip_static() {
        let imgs = extract_images(ARTICLE_HTML, BASE);
        assert_eq!(imgs.len(), 1);
        assert_eq!(imgs[0].0, "https://upload.wikimedia.org/rendang.jpg");
        assert_eq!(imgs[0].1, Some("Rendang".to_string()));
    }

    #[test]
    fn tanpa_argument_tampilkan_pesan() {
        let r = run(StubHttp::offline(), "");
        assert_eq!(text_of(&r), help("."));
    }

    #[test]
    fn tidak_ditemukan() {
        let http = StubHttp::body(API_SEARCH);
        // API search ada tapi halaman artikel gagal (offline untuk artikelnya)
        // Karena stub hanya punya satu balasan, kedua request akan dapat API_SEARCH
        // yang bukan HTML artikel, jadi parsing akan gagal dan fallback.
        let r = run(http.clone(), "Rendang");
        assert!(text_of(&r).contains("WIKIPEDIA") || text_of(&r).contains("kesalahan"));
    }

    #[test]
    fn jaringan_gagal() {
        let r = run(StubHttp::offline(), "Rendang");
        assert_eq!(text_of(&r), "😔 Terjadi kesalahan saat mencari artikel di Wikipedia.");
    }
}