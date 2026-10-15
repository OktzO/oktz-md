//! `plugins/search/gsmarena.js` — cari spesifikasi HP di GSMArena.
//!
//! `ponytail:` JS memakai paket npm `gsmarena-api` yang mem-parse HTML dengan
//! cheerio. Yang dipindah di sini bentuk respons yang dibaca plugin (hasil
//! pencarian + detail perangkat) dengan selector yang sama persis, ditulis
//! manual karena crate ini tidak punya parser HTML dan menambah dependency baru
//! dilarang.
//!
//! `ponytail:` `www.gsmarena.com` menyajikan halaman "Turnstile check" untuk
//! klien tanpa browser. Kalau itu yang masuk, `parse_search` mengembalikan
//! daftar kosong dan plugin menjawab "HP tidak ditemukan" — persis apa yang
//! terjadi di JS ketika cheerio tidak menemukan satu pun `<li>`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value, json};

use crate::download::douyindl::query_encode;

/// `getDataFromUrl` (`gsmarena-api/src/services/utils.js`) — base host fix.
pub(crate) const BASE: &str = "https://www.gsmarena.com";

/// `gsmarena.js:34` — kartu penggunaan.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📱 *ɢsᴍᴀʀᴇɴᴀ*\n\n\
         > Cari spesifikasi HP lengkap\n\n\
         `Contoh: {prefix}gsmarena samsung galaxy s25`"
    )
}

/// `gsmarena.js:46` — tidak ada hasil.
pub(crate) fn not_found(query: &str) -> String {
    format!("📱 HP tidak ditemukan untuk *{query}*")
}

/// `te(prefix, command, pushName)` (`src/lib/error.js:3-9`).
/// `Ctx` tidak membawa `pushName`, jadi `{pushName}` jadi `User` seperti di JS.
pub(crate) fn error_reply(prefix: &str, command: &str) -> String {
    crate::common::error_template(prefix, command)
}

// ── SELECTOR MINIMAL ────────────────────────────────────────────────────────
//
// cheerio tidak ada. Yang dipakai di bawah hanya operasi yang sama dengan
// selector yang dibaca `gsmarena-api`: `.makers li`, `a`, `span`, `img`,
// `td.ttl`, `td.nfo`, `th`, dan delapan selector `quickSpec`.

/// `<tag` yang benar-benar membuka tag — batas setelah nama tag wajib `>`,
/// spasi, atau `/`, supaya `<link` tidak tertangkap sebagai `<li`.
fn find_open(html: &str, from: usize, tag: &str) -> Option<usize> {
    let needle = format!("<{tag}");
    let mut at = from;
    loop {
        let i = html[at..].find(&needle)? + at + needle.len();
        match html[i..].chars().next() {
            Some('>') | Some(' ') | Some('\n') | Some('\t') | Some('\r') | Some('/') => {
                return Some(i - needle.len());
            }
            _ => at = i,
        }
    }
}

/// Isi tag pertama setelah `from` — `$(…).text()` untuk teks biasa, dan
/// `$(…).html()` untuk `<span>` yang memang dibaca sebagai HTML mentah
/// (`search.js:16-17`).
fn inner_after(html: &str, from: usize, tag: &str) -> Option<String> {
    let open_end = html[from..].find('>')? + from + 1;
    let close = format!("</{tag}>");
    let len = html[open_end..].find(&close)?;
    Some(decode_entities(html[open_end..open_end + len].trim()))
}

/// Nilai atribut pertama `name` pada tag yang dibuka di `from`.
fn attr_after(html: &str, from: usize, name: &str) -> Option<String> {
    let open_end = html[from..].find('>')? + from + 1;
    let tag_end = html[open_end..].find('<')? + open_end;
    let tag = &html[from..tag_end];
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

/// Isi tag pertama yang memuat `needle` di dalam markup pembukaanya.
fn text_at(html: &str, needle: &str, tag: &str) -> String {
    html.find(needle)
        .and_then(|i| inner_after(html, i, tag))
        .unwrap_or_default()
}

/// Hanya entity yang dipakai GSMArena; sisanya dibiarkan apa adanya.
fn decode_entities(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        let end = tail.find(';').map_or(tail.len(), |e| e + 1);
        out.push_str(match &tail[..end] {
            "&amp;" => "&",
            "&lt;" => "<",
            "&gt;" => ">",
            "&quot;" => "\"",
            "&#039;" => "'",
            "&nbsp;" => " ",
            other => other,
        });
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// `search.search()` (`gsmarena-api/src/services/search.js`): `.makers li`,
/// lalu `a[href]`, `span` dengan `<br>` diganti spasi, dan `img[title]`.
pub(crate) fn parse_search(html: &str) -> Vec<Device> {
    let mut devices = Vec::new();
    let mut cursor = 0;
    while let Some(li) = find_open(html, cursor, "li") {
        let Some(li_end) = html[li..].find("</li>").map(|i| li + i) else {
            break;
        };
        let block = &html[li..li_end];
        cursor = li_end + "</li>".len();
        // `$(el).find('a')` mengambil yang pertama.
        let Some(a) = find_open(block, 0, "a") else {
            continue;
        };
        let Some(href) = attr_after(block, a, "href") else {
            continue;
        };
        devices.push(Device {
            // `search.js:14` — hanya satu kemunculan pertama yang dihapus.
            id: href.replacen(".php", "", 1),
            name: text_at(block, "<span", "span").replacen("<br>", " ", 1),
            description: attr_after(block, find_open(block, 0, "img").unwrap_or(0), "title")
                .unwrap_or_default(),
        });
    }
    devices
}

/// Satu hasil pencarian.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Device {
    pub id: String,
    pub name: String,
    pub description: String,
}

/// `catalog.getDevice()` → `name` (`catalog.js:47`).
pub(crate) fn device_name(html: &str) -> String {
    text_at(html, "class=\"specs-phone-name-title\"", "h1")
}

/// `catalog.getDevice()` → `quickSpec` (`catalog.js:28-38`). Delapan selector
/// tetap; yang kosong dibuang di `formatDetail`, bukan di sini.
pub(crate) fn quick_spec(html: &str) -> Vec<Spec> {
    const SELECTORS: [(&str, &str, &str); 8] = [
        ("data-spec=\"displaysize-hl\"", "Display size", "span"),
        ("data-spec=\"displayres-hl\"", "Display resolution", "div"),
        ("class=\"accent-camera\"", "Camera pixels", "div"),
        ("data-spec=\"videopixels-hl\"", "Video pixels", "div"),
        ("class=\"accent-expansion\"", "RAM size", "div"),
        ("data-spec=\"chipset-hl\"", "Chipset", "div"),
        ("class=\"accent-battery\"", "Battery size", "div"),
        ("data-spec=\"battype-hl\"", "Battery type", "div"),
    ];
    SELECTORS
        .iter()
        .map(|(needle, name, tag)| Spec {
            name: (*name).to_string(),
            value: text_at(html, needle, tag),
        })
        .collect()
}

/// Satu baris spesifikasi.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Spec {
    pub name: String,
    pub value: String,
}

/// `catalog.getDevice()` → `detailSpec` (`catalog.js:49-69`): tiap `<table>`
/// jadi satu kategori, `th` adalah namanya, tiap `<tr>` menyimpan
/// `td.ttl` + `td.nfo`.
pub(crate) fn parse_detail_spec(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(table) = find_open(html, cursor, "table") {
        let Some(table_end) = html[table..].find("</table>").map(|i| table + i) else {
            break;
        };
        let block = &html[table..table_end];
        cursor = table_end + "</table>".len();

        let Some(th) = find_open(block, 0, "th") else {
            continue;
        };
        let category = inner_after(block, th, "th").unwrap_or_default();
        if category.is_empty() {
            continue;
        }
        let mut specs = Vec::new();
        // `$(el).find('tr')` (catalog.js:97) mengambil semua baris termasuk yang
        // memuat `<th>`, jadi pemindaian mulai dari awal `<table>`.
        let mut row = 0;
        while let Some(tr) = find_open(block, row, "tr") {
            let Some(tr_end) = block[tr..].find("</tr>").map(|i| tr + i) else {
                break;
            };
            let cells = &block[tr..tr_end];
            row = tr_end + "</tr>".len();
            // `$('td.ttl', ele)` / `$('td.nfo', ele)` (catalog.js:101-102) —
            // dua selector BERBEDA, jadi tiap `<td>` dicari dari awal baris.
            let cell = |cls: &str| -> Option<String> {
                let mut at = 0;
                while let Some(td) = find_open(cells, at, "td") {
                    if attr_after(cells, td, "class").as_deref() == Some(cls) {
                        return inner_after(cells, td, "td");
                    }
                    at = td + "<td".len();
                }
                None
            };
            specs.push(json!({
                "name": cell("ttl").unwrap_or_default(),
                "value": cell("nfo").unwrap_or_default(),
            }));
        }
        // `if (category)` (catalog.js:105) — kategori kosong dibuang, daftar
        // spesifikasi yang kosong TIDAK.
        if !category.is_empty() {
            out.push(json!({ "category": category, "specifications": specs }));
        }
    }
    out
}

/// `gsmarena.js:64-81` — `formatList`. Deskripsi dipotong pada 80 karakter
/// (`desc.slice(0, 80) + "..."`), bukan 83.
pub(crate) fn format_list(devices: &[Device], query: &str, prefix: &str) -> String {
    let mut txt = format!("📱 *ʜᴀsɪʟ ᴘᴇɴᴄᴀʀɪᴀɴ*\n> *{query}*\n\n");
    for (i, d) in devices.iter().take(10).enumerate() {
        txt.push_str(&format!("{}. 📱 *{}*\n", i + 1, d.name));
        if !d.description.is_empty() {
            let desc = if d.description.chars().count() > 80 {
                format!("{}...", d.description.chars().take(80).collect::<String>())
            } else {
                d.description.clone()
            };
            txt.push_str(&format!("> {desc}\n"));
        }
    }
    txt.push_str(&format!(
        "\n> Ketik `{prefix}gsmarena <nama lengkap>` untuk detail"
    ));
    txt
}

/// `gsmarena.js:83-103` — `formatDetail`: seluruh `quickSpec` yang tidak
/// kosong, lalu maksimal 8 kategori dan 5 spesifikasi per kategori.
pub(crate) fn format_detail(name: &str, quick: &[Spec], detail: &[Value]) -> String {
    let mut txt = format!("📱 *{name}*\n\n");
    if !quick.is_empty() {
        txt.push_str("📋 *ʀɪɴɢᴋᴀsᴀɴ:*\n");
        for s in quick {
            txt.push_str(&format!("> 🔹 *{}:* {}\n", s.name, s.value));
        }
        txt.push('\n');
    }
    for cat in detail.iter().take(8) {
        txt.push_str(&format!(
            "📌 *{}:*\n",
            cat.get("category").and_then(Value::as_str).unwrap_or("")
        ));
        let specs = cat
            .get("specifications")
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        for s in specs.iter().take(5) {
            txt.push_str(&format!(
                "> • *{}:* {}\n",
                s.get("name").and_then(Value::as_str).unwrap_or(""),
                s.get("value").and_then(Value::as_str).unwrap_or("")
            ));
        }
        txt.push('\n');
    }
    txt
}

#[plugin(
    name = "gsmarena",
    alias = ["gsm", "phonespec", "spesifikasi"],
    category = "search",
    description = "Cari spesifikasi HP di GSMArena",
    usage = ".gsmarena <nama hp>",
    example = ".gsmarena infinix hot 50",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn gsmarena(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ");
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let error = error_reply(&ctx.prefix, &ctx.command);

    // `results.php3?sQuickSearch=yes&sName=<query>` (`search.js:6`).
    let search_url =
        format!("{BASE}/results.php3?sQuickSearch=yes&sName={}", query_encode(&query));
    let Some(html) = ctx.http.get(&search_url).ok().map(|r| r.text()) else {
        return Reply::text(error);
    };

    let results = parse_search(&html);
    if results.is_empty() {
        return Reply::text(not_found(&query));
    }

    // `gsmarena.js:49-53` — satu hasil langsung dibuka halamannya.
    if results.len() > 1 {
        return Reply::text(format_list(&results, &query, &ctx.prefix));
    }

    let detail_url = format!("{BASE}/{}.php", results[0].id);
    let Some(detail_html) = ctx.http.get(&detail_url).ok().map(|r| r.text()) else {
        return Reply::text(error);
    };
    Reply::text(format_detail(
        &device_name(&detail_html),
        &quick_spec(&detail_html),
        &parse_detail_spec(&detail_html),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        pages: Vec<(String, String)>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(pages: &[(&str, &str)]) -> Arc<Stub> {
            Arc::new(Stub {
                pages: pages
                    .iter()
                    .map(|(u, b)| ((*u).to_string(), (*b).to_string()))
                    .collect(),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                pages: vec![],
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = self
                .pages
                .iter()
                // Key = prefix URL halaman, value = HTML; request dicocokkan dari awal URL.
                .find(|(u, _)| url.starts_with(u.as_str()))
                .map(|(_, b)| b.clone())
                .ok_or_else(|| HttpError {
                    message: "offline".into(),
                })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: Some("text/html".into()),
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
            command: "gsmarena".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("gsmarena").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const TWO_RESULTS: &str = r#"<link rel="stylesheet" href="/x.css">
    <ul class="makers">
      <li><a href="infinix_hot_50-11800.php"><img src="a.jpg" title="Infinix Hot 50" /><span>Infinix Hot 50<br>6.78 inch</span></a></li>
      <li><a href="infinix_hot_60-12000.php"><img src="b.jpg" title="Infinix Hot 60" /><span>Infinix Hot 60</span></a></li>
    </ul>"#;

    const ONE_RESULT: &str = r#"<ul class="makers">
      <li><a href="infinix_hot_50-11800.php"><img src="a.jpg" title="Infinix Hot 50" /><span>Infinix Hot 50</span></a></li>
    </ul>"#;

    const DETAIL: &str = r#"<h1 class="specs-phone-name-title">Infinix Hot 50</h1>
    <span data-spec="displaysize-hl" class="specs-hl">6.78 inch</span>
    <div data-spec="chipset-hl" class="specs-item">Unisoc T606</div>
    <div data-spec="displayres-hl" class="specs-item"></div>
    <table><tr><th>Launch</th><td class="ttl">Announced</td><td class="nfo">2024, February</td></tr></table>
    <table><tr><th>Body</th><td class="ttl">Dimensions</td><td class="nfo">164 x 75 x 8 mm</td></tr></table>"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gsmarena").unwrap();
        assert_eq!(p.meta.name, "gsmarena");
        assert_eq!(p.meta.alias, ["gsm", "phonespec", "spesifikasi"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari spesifikasi HP di GSMArena");
        assert_eq!(p.meta.usage, ".gsmarena <nama hp>");
        assert_eq!(p.meta.example, ".gsmarena infinix hot 50");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["gsm", "phonespec", "spesifikasi"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "gsmarena"
            );
        }
    }

    #[test]
    fn an_empty_query_shows_the_usage_card_without_a_request() {
        let http = Stub::empty();
        for text in ["", "   "] {
            assert_eq!(text_of(&run(http.clone(), text)), help("."), "input {text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_search_row_becomes_one_id_name_and_description() {
        let devices = parse_search(TWO_RESULTS);
        assert_eq!(devices.len(), 2, "<link> tidak boleh ikut terhitung");
        assert_eq!(devices[0].id, "infinix_hot_50-11800");
        assert_eq!(devices[0].name, "Infinix Hot 50 6.78 inch");
        assert_eq!(devices[0].description, "Infinix Hot 50");
        assert_eq!(devices[1].name, "Infinix Hot 60");
    }

    #[test]
    fn two_hits_produce_a_numbered_list_and_no_detail_request() {
        let http = Stub::new(&[("https://www.gsmarena.com/results.php3", TWO_RESULTS)]);
        let __balasan = run(http.clone(), "infinix hot");
        let t = text_of(&__balasan);
        assert!(
            t.starts_with("📱 *ʜᴀsɪʟ ᴘᴇɴᴄᴀʀɪᴀɴ*\n> *infinix hot*"),
            "{t}"
        );
        assert!(t.contains("1. 📱 *Infinix Hot 50 6.78 inch*"), "{t}");
        assert!(t.contains("2. 📱 *Infinix Hot 60*"), "{t}");
        assert!(t.contains("> Infinix Hot 50\n"), "deskripsi = title img: {t}");
        assert!(t.ends_with("> Ketik `.gsmarena <nama lengkap>` untuk detail"), "{t}");
        assert_eq!(
            http.seen.lock().unwrap().len(),
            1,
            "daftar tidak perlu membuka halaman detail"
        );
    }

    #[test]
    fn only_ten_results_are_listed() {
        let rows: String = (0..14)
            .map(|i| format!("<li><a href=\"d{i}.php\"><span>HP {i}</span></a></li>"))
            .collect();
        let html = format!("<ul class=\"makers\">{rows}</ul>");
        let t = text_of(&run(
            Stub::new(&[("https://www.gsmarena.com/results.php3", &html)]),
            "hp",
        ));
        assert!(t.contains("10. 📱 *HP 9*"), "{t}");
        assert!(!t.contains("11. 📱"), "hasil ke-11 terpotong: {t}");
    }

    #[test]
    fn a_description_longer_than_eighty_characters_is_truncated() {
        let devices = [Device {
            id: "a".into(),
            name: "HP".into(),
            description: "x".repeat(81),
        }];
        let t = format_list(&devices, "q", ".");
        assert!(t.contains(&format!("> {}...\n", "x".repeat(80))), "{t}");
        assert_eq!(t.matches('x').count(), 80, "tepat 80 karakter: {t}");

        let devices = [Device {
            id: "a".into(),
            name: "HP".into(),
            description: "y".repeat(80),
        }];
        let t = format_list(&devices, "q", ".");
        assert!(!t.contains("..."), "tepat 80 karakter tidak dipotong: {t}");
    }

    #[test]
    fn a_single_hit_opens_the_detail_page_and_renders_both_sections() {
        let http = Stub::new(&[
            ("https://www.gsmarena.com/results.php3", ONE_RESULT),
            (
                "https://www.gsmarena.com/infinix_hot_50-11800.php",
                DETAIL,
            ),
        ]);
        let __balasan = run(http.clone(), "infinix hot 50");
        let t = text_of(&__balasan);
        assert!(t.starts_with("📱 *Infinix Hot 50*\n\n"), "{t}");
        assert!(t.contains("📋 *ʀɪɴɢᴋᴀsᴀɴ:*"), "{t}");
        assert!(t.contains("> 🔹 *Display size:* 6.78 inch"), "{t}");
        assert!(t.contains("> 🔹 *Chipset:* Unisoc T606"), "{t}");
        // `formatDetail` (gsmarena.js:88) mem-loop SELURUH `quickSpec` tanpa
        // menyaring nilai kosong, jadi entri yang kosong tetap jadi baris.
        assert!(
            t.contains("> 🔹 *Display resolution:* \n"),
            "spec kosong tetap dirender: {t}"
        );
        assert!(t.contains("📌 *Launch:*"), "{t}");
        assert!(t.contains("> • *Announced:* 2024, February"), "{t}");
        assert!(t.contains("📌 *Body:*"), "{t}");

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1], "https://www.gsmarena.com/infinix_hot_50-11800.php");
    }

    #[test]
    fn detail_categories_stop_at_eight_and_specs_at_five() {
        let cats: Vec<Value> = (0..10)
            .map(|i| {
                json!({
                    "category": format!("Kat{i}"),
                    "specifications": (0..7).map(|s| json!({"name": format!("S{s}"), "value": s.to_string()})).collect::<Vec<_>>(),
                })
            })
            .collect();
        let t = format_detail("HP", &[], &cats);
        assert!(t.contains("📌 *Kat7:*"), "{t}");
        assert!(!t.contains("Kat8"), "kategori ke-9 terpotong: {t}");
        assert!(t.contains("> • *S4:* 4"), "{t}");
        assert!(!t.contains("*S5:*"), "spec ke-6 terpotong: {t}");
    }

    #[test]
    fn a_turnstile_or_empty_page_reads_as_not_found() {
        let html = "<title>GSMArena Turnstile check</title>";
        let r = run(
            Stub::new(&[("https://www.gsmarena.com/results.php3", html)]),
            "hp nova",
        );
        assert_eq!(text_of(&r), not_found("hp nova"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::empty(), "hp nova");
        assert_eq!(text_of(&r), error_reply(".", "gsmarena"));
        assert!(text_of(&r).contains("command `.gsmarena`"), "{}", text_of(&r));
    }

    #[test]
    fn the_query_is_sent_to_the_endpoint_the_library_uses() {
        let http = Stub::new(&[("https://www.gsmarena.com/results.php3", TWO_RESULTS)]);
        let _ = run(http.clone(), "infinix hot 50");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://www.gsmarena.com/results.php3?sQuickSearch=yes&sName=infinix+hot+50"
        );
    }
}