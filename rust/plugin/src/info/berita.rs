//! `plugins/info/berita.js` — berita terkini dari RSS berbagai sumber.
//!
//! `ponytail:` JS memakai `cheerio` dengan `{ xmlMode: true }` untuk parsing
//! RSS. `oktz-plugins` tidak punya parser HTML/XML dan menambah dependency
//! baru dilarang, jadi markup RSS — `<item>`, `<title>`, `<link>`,
//! `<pubDate>`, `<description>` — dipakai manual di bawah.
//!
//! `ponytail:` `formatDate` di JS pakai `new Date(...).toLocaleString('id-ID')`.
//! Tanpa crate tanggal, pubDate yang tidak bisa di-parse dikembalikan apa
//! adanya (cabang `catch` di JS). Format yang dihasilkan hanya pendekatan.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

/// `NEWS_SOURCES` — kunci command, URL RSS, nama, emoji.
#[derive(Clone)]
struct NewsSource {
    url: &'static str,
    name: &'static str,
    emoji: &'static str,
}

const SOURCES: &[NewsSource] = &[
    NewsSource {
        url: "https://www.antaranews.com/rss/terkini.xml",
        name: "Antara News",
        emoji: "📰",
    },
    NewsSource {
        url: "https://www.cnnindonesia.com/nasional/rss",
        name: "CNN Indonesia",
        emoji: "📺",
    },
    NewsSource {
        url: "https://www.cnbcindonesia.com/rss",
        name: "CNBC Indonesia",
        emoji: "💹",
    },
    NewsSource {
        url: "https://international.sindonews.com/rss",
        name: "Sindo News",
        emoji: "📰",
    },
];

/// Kunci file (slug) untuk setiap sumber, cocok dengan `m.command` JS.
fn source_slug(s: &NewsSource) -> &'static str {
    match s.url {
        u if u.contains("antaranews") => "antara",
        u if u.contains("cnnindonesia") => "cnn",
        u if u.contains("cnbcindonesia") => "cnbc",
        u if u.contains("sindonews") => "sindonews",
        _ => "berita",
    }
}

fn find_source(slug: &str) -> Option<&'static NewsSource> {
    SOURCES.iter().find(|s| source_slug(s) == slug || s.name.to_ascii_lowercase() == slug)
}

/// Daftar sumber untuk balasan `.berita` tanpa argumen.
fn daftar_sumber(prefix: &str) -> String {
    let mut txt = "📰 *ᴅᴀꜰᴛᴀʀ sᴜᴍʙᴇʀ ʙᴇʀɪᴛᴀ*\n\n".to_string();
    for s in SOURCES {
        txt.push_str(&format!(
            "> {} `{}{}` - {}\n",
            s.emoji,
            prefix,
            source_slug(s),
            s.name
        ));
    }
    txt.push_str(&format!("\n_Atau gunakan: `{}berita <sumber>`_", prefix));
    txt
}

/// `fetchRSS` — ambil RSS dan parse `<item>` (maksimal 10).
fn fetch_rss(
    http: &dyn oktz_plugin_api::HttpGet,
    url: &str,
) -> Result<Vec<Article>, String> {
    let res = http
        .get(url)
        .map_err(|e| format!("Gagal mengambil RSS: {}", e.message))?;
    let html = res.text();
    Ok(parse_items(&html))
}

#[derive(Clone, Debug)]
struct Article {
    title: String,
    link: String,
    pub_date: String,
    description: String,
}

/// `parseItems` — cari `<item>...</item>`, ekstrak judul/link/tanggal/deskripsi.
fn parse_items(html: &str) -> Vec<Article> {
    let mut items = Vec::new();
    let mut i = 0;
    while i < html.len() && items.len() < 10 {
        let Some(item_start) = html[i..].find("<item>") else { break };
        let abs_start = i + item_start;
        let Some(item_end) = html[abs_start..].find("</item>") else { break };
        let abs_end = abs_start + item_end + "</item>".len();
        let item_html = &html[abs_start..abs_end];
        let title = tag_content(item_html, "title");
        let link = tag_content(item_html, "link");
        if title.is_empty() || link.is_empty() {
            i = abs_end;
            continue;
        }
        let pub_date = format_rss_date(&tag_content(item_html, "pubDate"));
        let mut description = tag_content(item_html, "description");
        description = strip_html_tags(&description);
        if description.len() > 150 {
            description.truncate(150);
        }
        items.push(Article {
            title,
            link,
            pub_date,
            description,
        });
        i = abs_end;
    }
    items
}

/// Isi tag pertama yang bernama `nama` (case-insensitif pada nama tag).
fn tag_content(html: &str, nama: &str) -> String {
    let open = format!("<{nama}>");
    let close = format!("</{nama}>");
    let mut start = 0;
    while let Some(os) = html[start..].find(&open) {
        let abs_os = start + os;
        let after_open = abs_os + open.len();
        if let Some(cs) = html[after_open..].find(&close) {
            return html[after_open..after_open + cs].to_string();
        }
        start = abs_os + open.len();
    }
    String::new()
}

/// Buang semua tag HTML dari teks.
fn strip_html_tags(raw: &str) -> String {
    // JS memakai cheerio `$(...).text()`: entitas di-decode LEBIH DULU, baru
    // tag hasil decode ikut dibuang. Tanpa urutan ini `&lt;b&gt;` tetap
    // tampil sebagai teks mentah di Rust.
    let decoded = decode_entities(raw);
    let mut out = String::with_capacity(decoded.len());
    let mut in_tag = false;
    for c in decoded.chars() {
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

/// Decode entitas HTML yang lazim muncul di RSS.
fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut sisa = raw;
    while let Some(i) = sisa.find('&') {
        out.push_str(&sisa[..i]);
        let ekor = &sisa[i..];
        let Some(j) = ekor.find(';').filter(|j| *j <= 10) else {
            out.push('&');
            sisa = &ekor[1..];
            continue;
        };
        let entitas = &ekor[1..j];
        let ganti = match entitas {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" | "#39" => Some('\''),
            "nbsp" => Some(' '),
            _ => entitas
                .strip_prefix("#x")
                .and_then(|n| u32::from_str_radix(n, 16).ok())
                .or_else(|| {
                    entitas
                        .strip_prefix('#')
                        .and_then(|n| n.parse::<u32>().ok())
                })
                .and_then(char::from_u32),
        };
        match ganti {
            Some(c) => {
                out.push(c);
                sisa = &ekor[j + 1..];
            }
            None => {
                out.push('&');
                sisa = &ekor[1..];
            }
        }
    }
    out.push_str(sisa);
    out
}

/// `formatDate` — parse pubDate RSS (RFC 2822) dan format ringkas.
/// Gagal parse -> balik string asli (mirip `catch` di JS).
fn format_rss_date(date_str: &str) -> String {
    if date_str.is_empty() {
        return String::new();
    }
    let parsed = parse_rfc2822(date_str);
    match parsed {
        Some((day, month_idx, year, hour, minute)) => {
            let bulan = MONTH_ID_SHORT[month_idx as usize];
            format!("{day} {bulan} {year} {hour:02}:{minute:02}")
        }
        None => date_str.to_string(),
    }
}

const MONTH_ID_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "Mei", "Jun",
    "Jul", "Ags", "Sep", "Okt", "Nov", "Des",
];

/// Parse RFC 2822 tanggal, balik (hari, bulan 0-based, tahun, jam, menit).
/// Hanya bentuk umum: ", DD Mon YYYY HH:MM:SS TZ".
fn parse_rfc2822(s: &str) -> Option<(u32, u32, i32, u32, u32)> {
    let s = s.trim();
    let koma = s.find(',')?;
    let after = s[koma + 1..].trim();
    let parts: Vec<&str> = after.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }
    let day: u32 = parts[0].parse().ok()?;
    let month_idx = month_en_to_idx(parts[1])?;
    let year: i32 = parts[2].parse().ok()?;
    let time_parts: Vec<&str> = parts[3].split(':').collect();
    if time_parts.len() < 2 {
        return None;
    }
    let hour: u32 = time_parts[0].parse().ok()?;
    let minute: u32 = time_parts[1].parse().ok()?;
    Some((day, month_idx, year, hour, minute))
}

fn month_en_to_idx(m: &str) -> Option<u32> {
    match m {
        "Jan" => Some(0),
        "Feb" => Some(1),
        "Mar" => Some(2),
        "Apr" => Some(3),
        "May" => Some(4),
        "Jun" => Some(5),
        "Jul" => Some(6),
        "Aug" => Some(7),
        "Sep" => Some(8),
        "Oct" => Some(9),
        "Nov" => Some(10),
        "Dec" => Some(11),
        _ => None,
    }
}

/// Kartu pakai saat `.berita` tanpa argumen.
fn help(prefix: &str) -> String {
    daftar_sumber(prefix)
}

#[plugin(
    name = "antara",
    alias = ["cnn", "cnbc", "sindonews", "berita"],
    category = "berita",
    description = "Mendapatkan berita terkini dari berbagai sumber",
    usage = ".berita <sumber>",
    example = ".berita cnn",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn berita(ctx: Ctx) -> Reply {
    let cmd = ctx.command.to_lowercase();

    let source_slug = if cmd == "berita" {
        let arg = ctx.text.trim().to_lowercase();
        if arg.is_empty() {
            return Reply::text(help(&ctx.prefix));
        }
        match find_source(&arg) {
            Some(s) => source_slug(s),
            None => {
                return Reply::text(format!(
                    "❌ Sumber berita tidak ditemukan.\n> Gunakan: `{}`berita untuk melihat daftar.",
                    ctx.prefix
                ));
            }
        }
    } else {
        cmd.as_str()
    };

    let Some(source) = find_source(source_slug) else {
        return Reply::text("❌ Sumber berita tidak valid.");
    };

    let articles = match fetch_rss(ctx.http.as_ref(), source.url) {
        Ok(a) => a,
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command));
        }
    };

    if articles.is_empty() {
        return Reply::text("❌ Tidak ada berita ditemukan.");
    }

    let mut txt = format!("{} *{}*\n", source.emoji, source.name.to_uppercase());
    txt.push_str("━━━━━━━━━━━━━━━\n\n");

    let limit = articles.len().min(7);
    for i in 0..limit {
        let a = &articles[i];
        txt.push_str(&format!("*{}. {}*\n", i + 1, a.title));
        if !a.description.is_empty() {
            txt.push_str(&format!("{}...\n", a.description));
        }
        txt.push_str(&format!("🔗 {}\n", a.link));
        if !a.pub_date.is_empty() {
            txt.push_str(&format!("📅 _{}_\n", a.pub_date));
        }
        txt.push('\n');
    }

    txt.push_str("━━━━━━━━━━━━━━━\n");
    txt.push_str(&format!("_Total: {} artikel tersedia_", articles.len()));

    Reply::text(txt)
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

    fn run(http: Arc<StubHttp>, text: &str, command: &str) -> Reply {
        let mut args = text.split_whitespace();
        let ctx = Ctx {
            text: text.to_string(),
            args: args.by_ref().map(str::to_string).collect(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            prefix: ".".into(),
            command: command.to_string(),
            http,
            ..Ctx::default()
        };
        berita(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const RSS: &str = r#"<rss version="2.0">
<channel>
  <item>
    <title>Berita Satu</title>
    <link>https://example.com/1</link>
    <pubDate>Mon, 01 Jan 2024 12:00:00 GMT</pubDate>
    <description>Ini deskripsi berita pertama yang cukup panjang.</description>
  </item>
  <item>
    <title>Berita Dua</title>
    <link>https://example.com/2</link>
    <pubDate>Tue, 02 Jan 2024 08:30:00 +0700</pubDate>
    <description>&lt;b&gt;Deskripsi dengan HTML&lt;/b&gt; dan teks.</description>
  </item>
</channel>
</rss>"#;

    #[test]
    fn source_slug_dikenali() {
        assert_eq!(source_slug(&SOURCES[0]), "antara");
        assert_eq!(source_slug(&SOURCES[1]), "cnn");
        assert_eq!(source_slug(&SOURCES[2]), "cnbc");
        assert_eq!(source_slug(&SOURCES[3]), "sindonews");
    }

    #[test]
    fn daftar_sumber_tampilkan_semua() {
        let txt = daftar_sumber(".");
        assert!(txt.contains("Antara News"));
        assert!(txt.contains("CNN Indonesia"));
        assert!(txt.contains("CNBC Indonesia"));
        assert!(txt.contains("Sindo News"));
        assert!(txt.contains(".berita <sumber>"), "{txt}");
    }

    #[test]
    fn parse_items_ekstrak_dua_item() {
        let items = parse_items(RSS);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Berita Satu");
        assert_eq!(items[0].link, "https://example.com/1");
        assert_eq!(items[1].title, "Berita Dua");
        assert_eq!(items[1].link, "https://example.com/2");
    }

    #[test]
    fn deskripsi_di_strip_html_dan_dipotong() {
        let items = parse_items(RSS);
        assert_eq!(items[1].description, "Deskripsi dengan HTML dan teks.");
    }

    #[test]
    fn pub_date_diformat() {
        let items = parse_items(RSS);
        assert_eq!(items[0].pub_date, "1 Jan 2024 12:00");
        assert_eq!(items[1].pub_date, "2 Jan 2024 08:30");
    }

    #[test]
    fn pub_date_tidak_terdapat_dikembalikan_apa_adanya() {
        assert_eq!(format_rss_date("bukan tanggal"), "bukan tanggal");
        assert_eq!(format_rss_date(""), "");
    }

    #[test]
    fn tanpa_argumen_tampilkan_daftar() {
        let r = run(StubHttp::offline(), "", "berita");
        assert_eq!(text_of(&r), help("."));
    }

    #[test]
    fn sumber_tidak_ditemukan() {
        let r = run(StubHttp::offline(), "tidakada", "berita");
        assert!(text_of(&r).contains("❌ Sumber berita tidak ditemukan"));
    }

    #[test]
    fn command_antara_mengambil_rss_antara() {
        let http = StubHttp::body(RSS);
        let r = run(http.clone(), "", "antara");
        assert!(!text_of(&r).contains("❌"));
        let seen = http.seen.lock().unwrap();
        assert!(seen.iter().any(|u| u.contains("antaranews")), "{seen:?}");
    }

    #[test]
    fn command_cnn_mengambil_rss_cnn() {
        let http = StubHttp::body(RSS);
        let r = run(http.clone(), "", "cnn");
        assert!(!text_of(&r).contains("❌"));
        let seen = http.seen.lock().unwrap();
        assert!(seen.iter().any(|u| u.contains("cnnindonesia")), "{seen:?}");
    }

    #[test]
    fn rss_kosong_menampilkan_tidak_ada_berita() {
        let http = StubHttp::body("<rss><channel></channel></rss>");
        let r = run(http.clone(), "", "cnn");
        assert_eq!(text_of(&r), "❌ Tidak ada berita ditemukan.");
    }

    #[test]
    fn jaringan_gagal_menggunakan_error_template() {
        let r = run(StubHttp::offline(), "", "cnn");
        assert_eq!(
            text_of(&r),
            "☢ Kayaknya command `.cnn` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
    }
}