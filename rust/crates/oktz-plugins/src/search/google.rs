//! `plugins/search/google.js` — cari berita di Google News.
//!
//! `ponytail:` JS memakai cheerio untuk mem-parse RSS. Di sini dokumen XML
//! dibaca dengan pencarian tag sederhana — RSS Google News punya bentuk yang
//! tetap (`<item>` dengan `<title>`, `<link>`, `<pubDate>`, `<source>`), jadi
//! parser HTML tidak perlu dan menambah dependency baru dilarang.
//!
//! Dua header JS (`User-Agent` dan `X-Engine-Context`) belum bisa dikirim:
//! `HttpGet` tidak punya setter header per-request. Keduanya dicatat di sini
//! supaya tidak hilang saat mekanismenya ditambahkan.
//!
//! Host tidak ditulis langsung: `ENGINE_CONFIG.TARGET_HOST` di
//! `src/scraper/google.js` disimpan sebagai base64 lalu di-decode sekali di
//! konstruktor. Hasilnya sama, dan QString base64 ikut dipakai untuk
//! `compileBinaryPayload`, jadi tidak ada kode base64 yang bisa insulting.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

/// `ENGINE_CONFIG` (`src/scraper/google.js:4-12`).
const TARGET_HOST_B64: &str = "bmV3cy5nb29nbGUuY29t";
const DEFAULT_LANG: &str = "id";
const DEFAULT_REGION: &str = "ID";

/// `ITEM_COUNT` = `result.results.slice(0, 10)` (`google.js:45`).
const MAX_ITEM: usize = 10;

fn resolver_host() -> String {
    decode_base64(TARGET_HOST_B64)
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| "news.google.com".to_string())
}

/// Satu `<item>` — urutan field mengikuti `Object.defineProperties` di
/// `google.js:74-95`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    /// `index_node` = `index + 1`, jadi mulai dari 1.
    pub(crate) index_node: usize,
    pub(crate) resource_title: String,
    pub(crate) resolved_endpoint: String,
    pub(crate) temporal_stamp: String,
    pub(crate) origin_node: String,
}

/// `google.js:64-65` — teks daftar hasil.
pub(crate) fn format_items(items: &[Item], query: &str) -> String {
    let mut txt = String::from("🔍 *Google News*\n\n");
    txt.push_str(&format!("> Pencarian: *{query}*\n\n"));
    for item in items {
        txt.push_str(&format!(
            "*{}.* {}\n   ├ 📰 {}\n   ├ 🕐 {}\n   └ 🔗 {}\n\n",
            item.index_node,
            item.resource_title,
            item.origin_node,
            item.temporal_stamp,
            item.resolved_endpoint
        ));
    }
    txt.trim().to_string()
}

/// `google.js:22-32`.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🔍 *Google News*\n\n\
         Cari berita terbaru dari Google News.\n\n\
         *PENGGUNAAN:*\n> *{prefix}google <topik>*\n\n\
         *CONTOH:*\n> *{prefix}google gempa hari ini*\n> *{prefix}google teknologi terbaru*"
    )
}

/// `executeQueryPipeline` — URL RSS persis seperti di JS.
pub(crate) fn search_url(query: &str) -> String {
    format!(
        "https://{}/rss/search?q={}&hl={}&gl={}&ceid={}:{}",
        resolver_host(),
        encode_uri_component(query),
        DEFAULT_LANG,
        DEFAULT_REGION,
        DEFAULT_REGION,
        DEFAULT_LANG
    )
}

/// `$extractCipherPayload` (`google.js:33-37`) — bagian setelah `articles/`,
/// tanpa query string.
fn extract_cipher_payload(source_url: &str) -> Option<&str> {
    let pointer = source_url.find("articles/")?;
    let tail = &source_url[pointer + "articles/".len()..];
    let end = tail.find('?').unwrap_or(tail.len());
    Some(&tail[..end])
}

/// `$transformSerialization` (`google.js:39-46`) — base64url -> base64.
fn transform_serialization(cipher: &str) -> String {
    let mut s: String = cipher.replace('-', "+").replace('_', "/");
    let pad = s.len() % 4;
    if pad > 0 {
        s.push_str(&"=".repeat(4 - pad));
    }
    s
}

/// `compileBinaryPayload` (`google.js:48-62`): token base64 di URL artikel
/// concealing sebenarnya URL asli. Kalau decode atau pencocokan gagal, URL
/// mentah dikembalikan — persis `catch` di JS.
pub(crate) fn compile_binary_payload(raw_url: &str) -> String {
    let Some(token) = extract_cipher_payload(raw_url) else {
        return raw_url.to_string();
    };
    let Some(bytes) = decode_base64(&transform_serialization(token)) else {
        return raw_url.to_string();
    };
    let Ok(decoded) = std::str::from_utf8(&bytes) else {
        return raw_url.to_string();
    };
    first_url(decoded).unwrap_or_else(|| raw_url.to_string())
}

/// Regex `/https?:\/\/[^\s"\><]+/g` mengambil kemunculan pertama.
fn first_url(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        let rest = &text[i..];
        let hit = rest.starts_with("http://") || rest.starts_with("https://");
        if !hit {
            continue;
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '"' || c == '>' || c == '<')
            .unwrap_or(rest.len());
        return Some(rest[..end].to_string());
    }
    None
}

/// Isi tag pertama di dalam `inner` — `$(node).find("link").text()`.
fn tag_text(inner: &str, tag: &str) -> String {
    let open = format!("<{tag}");
    let Some(after_open) = inner.find(&open) else {
        return String::new();
    };
    let content_start = after_open + open.len();
    let Some(gt) = inner[content_start..].find('>') else {
        return String::new();
    };
    let start = content_start + gt + 1;
    let close = format!("</{tag}>");
    match inner[start..].find(&close) {
        Some(end) => inner[start..start + end].to_string(),
        None => String::new(),
    }
}

/// `documentContext("item").each(...)` (`google.js:70-95`).
pub(crate) fn parse_items(xml: &str) -> Vec<Item> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(open) = xml[from..].find("<item>").map(|i| i + from) {
        let after = open + "<item>".len();
        let Some(close) = xml[after..].find("</item>").map(|i| i + after) else {
            break;
        };
        let inner = &xml[after..close];
        out.push(Item {
            index_node: out.len() + 1,
            resource_title: tag_text(inner, "title"),
            resolved_endpoint: compile_binary_payload(&tag_text(inner, "link")),
            temporal_stamp: tag_text(inner, "pubDate"),
            origin_node: tag_text(inner, "source"),
        });
        from = close + "</item>".len();
    }
    out
}

/// Base64 standar dengan padding. Sengaja tanpa dependency: yang dipakai hanya
/// ASCII, dan `base64` crate tidak ada di workspace ini.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const TABEL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut vals = Vec::with_capacity(input.len());
    for c in input.bytes() {
        if c == b'=' {
            break;
        }
        if c.is_ascii_whitespace() {
            continue;
        }
        let v = TABEL.iter().position(|t| *t == c)? as u32;
        vals.push(v);
    }
    let mut out = Vec::with_capacity(vals.len() * 3 / 4);
    for chunk in vals.chunks(4) {
        let b = |i: usize| -> u32 { chunk.get(i).copied().unwrap_or(0) };
        let n = (b(0) << 18) | (b(1) << 12) | (b(2) << 6) | b(3);
        let produced = match chunk.len() {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => 0,
        };
        for i in 0..produced {
            out.push(((n >> (16 - i * 8)) & 0xFF) as u8);
        }
    }
    Some(out)
}

#[plugin(
    name = "google",
    alias = ["gsearch", "googlenews"],
    category = "search",
    description = "Cari berita di Google News",
    usage = ".google <query>",
    example = ".google gempa hari ini",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn google(ctx: Ctx) -> Reply {
    let query = ctx.text.trim().to_string();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix)).with_reaction("❌");
    }

    let Some(body) = ctx.http.get(&search_url(&query)).ok().map(|r| r.text()) else {
        // `catch` di `google.js:64-68` — teksnya bukan `result.error`, karena
        // error axios tidak pernah sampai ke plugin.
        return Reply::text("❌ Gagal mencari di Google, coba lagi nanti").with_reaction("☢");
    };

    let mut items = parse_items(&body);
    items.truncate(MAX_ITEM);

    if items.is_empty() {
        return Reply::text(format!("❌ Nggak nemu hasil buat: *{query}*")).with_reaction("☢");
    }

    Reply::text(format_items(&items, &query)).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        body: String,
        seen: Mutex<Vec<String>>,
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.body.clone().into_bytes()),
                    mimetype: Some("application/rss+xml".into()),
                },
                headers: vec![],
            })
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

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "google".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("google").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn stub(body: &str) -> Arc<Stub> {
        Arc::new(Stub {
            body: body.to_string(),
            seen: Mutex::new(vec![]),
        })
    }

    // RSS asli: link-nya `articles/<base64url>` yang isinya URL tujuan.
    // `aHR0cHM6Ly9leGFtcGxlLmNvbS9iZXJpdGE` = `https://example.com/berita`.
    const FEED: &str = r#"<?xml version="1.0"?>
<rss><channel>
<item>
  <title>Gempa 5,9 SR Magnitude</title>
  <link>https://news.google.com/rss/articles/aHR0cHM6Ly9leGFtcGxlLmNvbS9iZXJpdGE?oc=5</link>
  <pubDate>Tue, 14 Oct 2026 09:00:00 GMT</pubDate>
  <source url="https://example.com">Example News</source>
</item>
<item>
  <title>Teknologi Terbaru</title>
  <link>https://example.com/langsung</link>
  <pubDate>Tue, 14 Oct 2026 08:00:00 GMT</pubDate>
  <source url="https://lain.com">Lain</source>
</item>
</channel></rss>"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let meta = oktz_plugin_api::find("google").unwrap().meta.clone();
        assert_eq!(meta.name, "google");
        assert_eq!(meta.alias, ["gsearch", "googlenews"]);
        assert_eq!(meta.category, "search");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 0);
        assert_eq!(meta.description, "Cari berita di Google News");
    }

    #[test]
    fn empty_query_reacts_and_shows_the_help_text() {
        let s = stub(FEED);
        let r = run(s.clone(), "   ");
        assert_eq!(text_of(&r), help("."));
        assert!(
            s.seen.lock().unwrap().is_empty(),
            "query kosong tidak boleh sampai ke jaringan"
        );
    }

    #[test]
    fn the_request_url_matches_the_one_axios_would_build() {
        let u = search_url("gempa hari ini");
        assert_eq!(
            u,
            "https://news.google.com/rss/search?q=gempa%20hari%20ini&hl=id&gl=ID&ceid=ID:id"
        );
    }

    #[test]
    fn items_render_in_the_same_shape_as_the_js_loop() {
        let s = stub(FEED);
        let r = run(s.clone(), "gempa");
        let t = text_of(&r);
        assert!(t.starts_with("🔍 *Google News*\n\n> Pencarian: *gempa*\n\n"), "{t}");
        assert!(t.contains("*1.* Gempa 5,9 SR Magnitude"), "{t}");
        assert!(t.contains("   ├ 📰 Example News"), "{t}");
        assert!(t.contains("   ├ 🕐 Tue, 14 Oct 2026 09:00:00 GMT"), "{t}");
        assert!(
            t.contains("   └ 🔗 https://example.com/berita"),
            "token base64 harus jadi URL asli: {t}"
        );
        assert!(t.contains("*2.* Teknologi Terbaru"), "{t}");
        assert!(
            t.contains("   └ 🔗 https://example.com/langsung"),
            "link tanpa `articles/` dipakai apa adanya: {t}"
        );
    }

    #[test]
    fn a_plain_link_is_never_run_through_base64() {
        // Tidak ada `articles/` -> URL mentah, bukan decode yang gagal.
        assert_eq!(
            compile_binary_payload("https://example.com/x"),
            "https://example.com/x"
        );
        assert_eq!(compile_binary_payload(""), "");
    }

    #[test]
    fn a_broken_token_falls_back_to_the_raw_url() {
        let raw = "https://news.google.com/rss/articles/@@@@?oc=5";
        assert_eq!(compile_binary_payload(raw), raw);
    }

    #[test]
    fn an_empty_feed_says_nothing_was_found() {
        let s = stub("<rss><channel></channel></rss>");
        let r = run(s.clone(), "tidak ada");
        assert_eq!(text_of(&r), "❌ Nggak nemu hasil buat: *tidak ada*");
    }

    #[test]
    fn at_most_ten_items_are_rendered() {
        let mut xml = String::from("<rss><channel>");
        for i in 1..=15 {
            xml.push_str(&format!("<item><title>Item {i}</title></item>"));
        }
        xml.push_str("</channel></rss>");
        let r = run(stub(&xml), "banyak");
        let t = text_of(&r);
        assert!(t.contains("*10.* Item 10"), "{t}");
        assert!(!t.contains("Item 11"), "slice(0, 10) harus dipatuhi: {t}");
    }

    #[test]
    fn a_feed_without_closing_item_does_not_panic() {
        assert!(
            parse_items("<rss><item><title>sayap").is_empty(),
            "tanpa </item> tidak ada item"
        );
        assert_eq!(parse_items("<rss><item><title>sayap</item>").len(), 1);
        assert!(parse_items("bukan xml sama sekali").is_empty());
    }

    #[test]
    fn malformed_tags_yield_empty_strings_not_panics() {
        let xml = "<item><title>Tanpa penutup</item>";
        let items = parse_items(xml);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].resource_title, "");
    }

    #[test]
    fn decode_base64_handles_the_shapes_the_kernel_produces() {
        assert_eq!(
            decode_base64("aHR0cHM6Ly9leGFtcGxlLmNvbS9iZXJpdGE").unwrap(),
            b"https://example.com/berita".to_vec()
        );
        // Panjang bukan kelipatan 4 -> `=` diisi oleh `transform_serialization`.
        assert_eq!(decode_base64("YQ==").unwrap(), b"a".to_vec());
        assert_eq!(decode_base64("YWI=").unwrap(), b"ab".to_vec());
        assert_eq!(decode_base64("YWJj").unwrap(), b"abc".to_vec());
        assert!(decode_base64("@@@@").is_none(), "karakter di luar tabel");
    }
}