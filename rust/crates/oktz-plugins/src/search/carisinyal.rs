//! `plugins/search/carisinyal.js` (`spekhp`) — cari spesifikasi HP di
//! carisinyal.com.
//!
//! `ponytail:` JS memakai `cheerio` (paket npm). `oktz-plugins` tidak punya
//! parser HTML dan menambah dependency baru dilarang, jadi markup yang dibaca
//! plugin — `.oxy-post`, `.oxy-post-title`, `.oxy-post-meta`,
//! `table.box-info tr.box-baris`, `td.kolom-satu`, `td.kolom-dua`, `h1`, dan
//! dua `<meta>` — diparse manual, sama seperti `search/gsmarena.rs`.
//!
//! `ponytail:` `.text()` di cheerio membaca teks turunan dan membuang markup;
//! `text_content` di bawah melakukan hal yang sama. Entitas HTML yang lazim
//! (`&amp;`, `&nbsp;`, dst) diterjemahkan, sisanya dibiarkan apa adanya.
//!
//! `ponytail:` `sock.sendMessage({ image: { url }, caption })` tidak punya
//! masalah apa pun di sini — `Reply::image` membawa URL sebagai
//! `MediaSource::Url`, sama seperti wire-nya.
#![allow(clippy::needless_lifetimes)]

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

/// `carisinyal.js:22` — `User-Agent` yang dikirim `request()`.
pub(crate) const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";

/// `carisinyal.js:28` — endpoint pencarian.
pub(crate) fn search_url(keyword: &str) -> String {
    format!(
        "https://carisinyal.com/?s={}",
        encode_uri_component(keyword)
    )
}

/// `carisinyal.js:133-140` — kartu saat `text` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📱 *FITUR PENCARIAN SPESIFIKASI HP*\n\n\
         Fitur ini membantumu mencari spesifikasi lengkap dari sebuah *smartphone* menggunakan *database* terpercaya dari Carisinyal.\n\n\
         *CARA PENGGUNAAN:*\n\
         - Ketik `{prefix}spekhp <nama hp>`\n\
         - Contoh: `{prefix}spekhp oppo a3s`\n\n\
         _Sistem akan secara otomatis mencari ponsel yang paling sesuai dengan kata kuncimu dan menampilkan rincian spesifikasinya lengkap dengan gambarnya!_"
    )
}

/// `carisinyal.js:149` — tidak ada hasil.
pub(crate) fn not_found(text: &str) -> String {
    format!(
        "❌ *TIDAK DITEMUKAN*\n\nMaaf, sistem tidak dapat menemukan ponsel dengan kata kunci *{text}*. Cobalah menggunakan nama merk atau seri yang lebih spesifik."
    )
}

/// `carisinyal.js:205` — semua error jaringan/parsing berakhir di sini.
pub(crate) const FETCH_FAILED: &str = "❌ *GAGAL MENGAMBIL DATA*\n\nMaaf, sistem mengalami gangguan saat mencoba mengambil data dari *Carisinyal*. Silakan coba lagi nanti.";

// ── SCANNER HTML MINIMAL ─────────────────────────────────────────────────────

/// Satu tag dari pemindaian linear HTML.
struct Tag {
    name: String,
    /// Indeks `<` tag pembuka.
    start: usize,
    /// Indeks setelah `>` tag pembuka — awal isi elemen.
    open_end: usize,
    /// Atribut apa adanya, tanpa nama tag.
    attrs: String,
    closing: bool,
}

/// Tag-tag dokumen dalam urutan kemunculannya.
fn scan_tags(html: &str) -> Vec<Tag> {
    let bytes = html.as_bytes();
    let mut tags = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &html[i..];
        if rest.starts_with("<!--") {
            i += rest.find("-->").map_or(html.len(), |e| i + e + 3);
            continue;
        }
        let mut j = i + 1;
        let closing = j < bytes.len() && bytes[j] == b'/';
        if closing {
            j += 1;
        }
        let name_start = j;
        while j < bytes.len()
            && !matches!(bytes[j], b'>' | b'/' | b' ' | b'\t' | b'\n' | b'\r')
        {
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }
        let Some(gt) = html[j..].find('>') else { break };
        tags.push(Tag {
            name: html[name_start..j].to_ascii_lowercase(),
            start: i,
            open_end: j + gt + 1,
            attrs: html[j..j + gt].to_string(),
            closing,
        });
        i = j + gt + 1;
    }
    tags
}

/// Nilai atribut `name`. Nama harus preceded whitespace lalu `=`, supaya
/// `data-class=` tidak terbaca sebagai `class`.
fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut at = 0;
    while let Some(pos) = attrs[at..].find(name) {
        let s = at + pos;
        let before_ok = attrs[..s].chars().next_back().is_some_and(char::is_whitespace);
        let after = attrs[s + name.len()..].chars().next();
        if before_ok && after == Some('=') {
            let rest = &attrs[s + name.len() + 1..];
            let quote = rest.chars().next()?;
            if quote == '"' || quote == '\'' {
                let end = rest[1..].find(quote)? + 1;
                return Some(rest[1..end].to_string());
            }
            return None;
        }
        at = s + name.len();
    }
    None
}

/// Token class persis — `class="oxy-post-title"` bukan `oxy-post`.
fn has_class(attrs: &str, class: &str) -> bool {
    attr(attrs, "class")
        .is_some_and(|v| v.split_whitespace().any(|c| c == class))
}

/// Rentang isi elemen: dari ujung tag pembuka sampai `<` tag penutup yang
/// sepadan. Tanpa tag penupun -> sampai akhir dokumen, sama seperti
/// `$(el).html()` untuk elemen yang tidak tertutup.
fn element_span(html: &str, tags: &[Tag], idx: usize) -> (usize, usize) {
    let name = tags[idx].name.as_str();
    let mut depth = 0usize;
    for t in &tags[idx..] {
        if t.name != name {
            continue;
        }
        if t.closing {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return (tags[idx].open_end, t.start);
            }
        } else {
            depth += 1;
        }
    }
    (tags[idx].open_end, html.len())
}

/// `$(el).text()` — teks turunan tanpa markup, lalu entitas HTML diterjemahkan.
fn text_content(html: &str, tags: &[Tag], idx: usize) -> String {
    let (start, end) = element_span(html, tags, idx);
    let mut out = String::new();
    let mut rest = &html[start..end];
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find('>') {
            Some(e) => &rest[i + e + 1..],
            None => "",
        };
    }
    out.push_str(rest);
    decode_entities(out.trim())
}

/// Elemen pertama di `html` dengan class `class` — `(href, teks)`.
fn first_class(html: &str, class: &str) -> Option<(Option<String>, String)> {
    let tags = scan_tags(html);
    let idx = tags
        .iter()
        .position(|t| !t.closing && has_class(&t.attrs, class))?;
    Some((attr(&tags[idx].attrs, "href"), text_content(html, &tags, idx)))
}

/// `$("h1").first().text()`.
pub(crate) fn first_h1(html: &str) -> String {
    let tags = scan_tags(html);
    tags
        .iter()
        .position(|t| !t.closing && t.name == "h1")
        .map_or(String::new(), |i| text_content(html, &tags, i))
}

/// `$('meta[property="og:image"]').attr("content")` — `None` kalau tidak ada.
pub(crate) fn meta_content(html: &str, attr_name: &str, wanted: &str) -> Option<String> {
    let tags = scan_tags(html);
    tags
        .iter()
        .find(|t| !t.closing && t.name == "meta" && attr(&t.attrs, attr_name) == Some(wanted.to_string()))
        .and_then(|t| attr(&t.attrs, "content"))
}

/// Entitas yang lazim di halaman spesifikasi; sisanya dibiarkan apa adanya
/// (persis `decode_entities` di `search/gsmarena.rs`).
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

// ── DATA HASIL ───────────────────────────────────────────────────────────────

/// Satu baris hasil pencarian.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Hit {
    pub title: String,
    /// Isi `.oxy-post-meta` — dipakai untuk memilih hasil bertipe ponsel.
    pub kind: String,
    pub url: Option<String>,
}

/// `search(keyword)` (`carisinyal.js:27-44`): tiap `.oxy-post` jadi satu
/// `Hit`; judul kosong dilewati, jadi `url` boleh `None`.
pub(crate) fn parse_search(html: &str) -> Vec<Hit> {
    let tags = scan_tags(html);
    let mut hits = Vec::new();
    for (i, t) in tags.iter().enumerate() {
        if t.closing || !has_class(&t.attrs, "oxy-post") {
            continue;
        }
        let (start, end) = element_span(html, &tags, i);
        let block = &html[start..end];
        let Some((url, title)) = first_class(block, "oxy-post-title") else {
            continue;
        };
        let title = title.trim().to_string();
        if title.is_empty() {
            continue;
        }
        let kind = first_class(block, "oxy-post-meta").map_or(String::new(), |(_, t)| t);
        hits.push(Hit { title, kind, url });
    }
    hits
}

/// `specs` hasil `detail(url)` — urutan kemunculan baris, nilai duplikat menimpa
/// baris sebelumnya seperti penugasan objek di JS.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Specs {
    pairs: Vec<(String, String)>,
}

impl Specs {
    pub(crate) fn insert(&mut self, key: String, value: String) {
        match self.pairs.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.pairs.push((key, value)),
        }
    }

    /// `get(...keys)` (`carisinyal.js:57-60`) — kunci pertama yang punya nilai
    /// tidak kosong; `None` kalau tidak ada.
    pub(crate) fn get(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|k| {
            self.pairs
                .iter()
                .find(|(pk, pv)| pk == k && !pv.is_empty())
                .map(|(_, v)| v.as_str())
        })
    }
}

/// `detail(url)` (`carisinyal.js:46-129`) — `table.box-info tr.box-baris`.
pub(crate) fn parse_specs(html: &str) -> Specs {
    let tags = scan_tags(html);
    let mut specs = Specs::default();
    for (i, t) in tags.iter().enumerate() {
        if t.closing || t.name != "tr" || !has_class(&t.attrs, "box-baris") {
            continue;
        }
        let (start, end) = element_span(html, &tags, i);
        let row = &html[start..end];
        let key = first_class(row, "kolom-satu").map(|(_, t)| t.trim().to_string());
        let value = first_class(row, "kolom-dua").map(|(_, t)| t.trim().to_string());
        if let (Some(k), Some(v)) = (key, value)
            && !k.is_empty()
            && !v.is_empty()
        {
            specs.insert(k, v);
        }
    }
    specs
}

/// `results.find(r => (r.type || "").toLowerCase().includes("ponsel")) ||
/// results[0]` (`carisinyal.js:152`).
pub(crate) fn pick_phone<'a>(hits: &'a [Hit]) -> &'a Hit {
    hits.iter()
        .find(|h| h.kind.to_lowercase().contains("ponsel"))
        .unwrap_or(&hits[0])
}

/// `caption += `- *${label}:* ${data[k]}`` untuk satu baris opsional.
fn spec_line(specs: &Specs, label: &str, keys: &[&str], out: &mut String) {
    if let Some(v) = specs.get(keys) {
        out.push_str(&format!("- *{label}:* {v}\n"));
    }
}

/// `carisinyal.js:156-193` — caption spesifikasi. Baris yang tidak punya nilai
/// dilewati persis seperti `if (data.x) caption += …`.
pub(crate) fn format_caption(title: &str, specs: &Specs) -> String {
    let mut c = format!("📱 *SPESIFIKASI {}*\n\n", title.to_uppercase());

    if let Some(v) = specs.get(&["Rilis"]) {
        c.push_str(&format!("📅 *Rilis:* {v}\n"));
    }
    if let Some(v) = specs.get(&["Jaringan"]) {
        c.push_str(&format!("📶 *Jaringan:* {v}\n"));
    }
    c.push('\n');

    c.push_str("🖥️ *LAYAR*\n");
    spec_line(specs, "Tipe", &["Jenis"], &mut c);
    spec_line(specs, "Ukuran", &["Ukuran"], &mut c);
    spec_line(specs, "Resolusi", &["Resolusi"], &mut c);
    spec_line(specs, "Refresh Rate", &["Refresh Rate"], &mut c);
    c.push('\n');

    c.push_str("⚙️ *PERFORMA & MEMORI*\n");
    spec_line(specs, "Chipset", &["Chipset"], &mut c);
    spec_line(specs, "CPU", &["CPU"], &mut c);
    spec_line(specs, "GPU", &["GPU"], &mut c);
    spec_line(specs, "RAM", &["RAM"], &mut c);
    spec_line(specs, "Storage", &["Memori Internal"], &mut c);
    c.push('\n');

    c.push_str("📸 *KAMERA UTAMA*\n");
    spec_line(specs, "Jumlah", &["Jumlah Kamera"], &mut c);
    spec_line(specs, "Konfigurasi", &["Konfigurasi"], &mut c);
    spec_line(specs, "Video", &["Resolusi Video"], &mut c);
    c.push('\n');

    c.push_str("🔋 *BATERAI*\n");
    spec_line(specs, "Kapasitas", &["Kapasitas"], &mut c);
    spec_line(specs, "Charging", &["Daya Pengisian"], &mut c);
    c.push('\n');

    c.push_str("📡 *KONEKTIVITAS & LAINNYA*\n");
    spec_line(specs, "OS Saat Rilis", &["OS (Saat Rilis)"], &mut c);
    spec_line(specs, "NFC", &["NFC"], &mut c);
    spec_line(specs, "Audio Jack", &["Jack 3.5mm"], &mut c);
    spec_line(specs, "Sensor", &["Sensor"], &mut c);

    c.push_str("\n🔗 *Sumber:* Carisinyal");
    c
}

/// Balasan kegagalan — semua jalur `catch` di plugin ini berakhir di sini.
fn failed() -> Reply {
    Reply::text(FETCH_FAILED).with_reaction("❌")
}

#[plugin(
    name = "spekhp",
    alias = ["spek"],
    category = "search",
    description = "Mencari spesifikasi lengkap smartphone dari Carisinyal.",
    usage = ".spekhp <nama hp>",
    example = ".spekhp oppo a3s",
    cooldown = 5,
    energi = 2,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn spekhp(ctx: Ctx) -> Reply {
    let text = ctx.text.clone();

    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `axios.get` di JS melempar untuk status 4xx/5xx, jadi `terima_semua_status`
    // sengaja tidak dipakai: halaman 404 harus masuk `catch` seperti di JS.
    let get = |url: &str| {
        ctx.http
            .call(&HttpCall::get(url).header("User-Agent", USER_AGENT))
            .ok()
    };

    let Some(list) = get(&search_url(&text)) else {
        return failed();
    };

    let hits = parse_search(&list.text());
    if hits.is_empty() {
        return Reply::text(not_found(&text)).with_reaction("❌");
    }

    let phone = pick_phone(&hits);
    // `detail(phone.url)` dengan `url` undefined memanggil axios tanpa URL ->
    // error -> `catch` -> pesan gagal.
    let Some(url) = phone.url.as_deref() else {
        return failed();
    };
    let Some(page) = get(url) else {
        return failed();
    };
    let html = page.text();

    let caption = format_caption(&first_h1(&html), &parse_specs(&html));

    match meta_content(&html, "property", "og:image") {
        Some(image) if !image.is_empty() => {
            Reply::image(MediaSource::url(image), caption).with_reaction("✅")
        }
        _ => Reply::text(caption).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    /// Stub dua halaman: daftar (`carisinyal.com/?s=`) dan detail (host lain).
    #[derive(Debug)]
    struct Stub {
        list: Option<String>,
        detail: Option<String>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(list: &str, detail: &str) -> Arc<Stub> {
            Arc::new(Stub {
                list: Some(list.to_string()),
                detail: Some(detail.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn empty() -> Arc<Stub> {
            Arc::new(Stub {
                list: None,
                detail: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let body = if url.starts_with("https://carisinyal.com/?s=") {
                self.list.clone()
            } else {
                self.detail.clone()
            };
            let body = body.ok_or_else(|| oktz_plugin_api::HttpError {
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
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
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
            command: "spekhp".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("spekhp").unwrap().handler)(ctx)
    }

    fn body_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    const DAFTAR: &str = r#"<div class="wrap">
      <div class="oxy-post">
        <h2><a class="oxy-post-title" href="https://carisinyal.com/htc/one-m9/">HTC One M9 Plus</a></h2>
        <span class="oxy-post-meta">Tablet</span>
      </div>
      <div class="oxy-post">
        <h2><a class="oxy-post-title" href="https://carisinyal.com/oppo/a3s/">OPPO A3s</a></h2>
        <span class="oxy-post-meta">PONSEL</span>
      </div>
    </div>"#;

    const DETAIL: &str = r#"<html><head>
      <meta property="og:image" content="https://carisinyal.com/img/a3s.jpg">
      <meta name="description" content="Spesifikasi OPPO A3s">
    </head><body>
      <h1>OPPO A3s</h1>
      <table class="box-info">
        <tr class="box-baris"><td class="kolom-satu">Rilis</td><td class="kolom-dua">2018/07</td></tr>
        <tr class="box-baris"><td class="kolom-satu">Jaringan</td><td class="kolom-dua">GSM / HSPA / LTE</td></tr>
        <tr class="box-baris"><td class="kolom-satu">Jenis</td><td class="kolom-dua">IPS LCD</td></tr>
        <tr class="box-baris"><td class="kolom-satu">Chipset</td><td class="kolom-dua">MediaTek Helio P23</td></tr>
        <tr class="box-baris"><td class="kolom-satu">RAM</td><td class="kolom-dua">3 GB RAM</td></tr>
        <tr class="box-baris"><td class="kolom-satu">Kapasitas</td><td class="kolom-dua">4230 mAh</td></tr>
        <tr class="box-baris"><td class="kolom-satu">Jumlah Kamera</td><td class="kolom-dua">1 Kamera</td></tr>
        <tr class="box-baris"><td class="kolom-satu">Sensor</td><td class="kolom-dua">Accelerometer, proximity</td></tr>
      </table>
    </body></html>"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("spekhp").unwrap();
        assert_eq!(p.meta.name, "spekhp");
        assert_eq!(p.meta.alias, ["spek"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Mencari spesifikasi lengkap smartphone dari Carisinyal."
        );
        assert_eq!(p.meta.usage, ".spekhp <nama hp>");
        assert_eq!(p.meta.example, ".spekhp oppo a3s");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn the_alias_resolves_to_this_plugin() {
        assert_eq!(oktz_plugin_api::find("spek").unwrap().meta.name, "spekhp");
    }

    #[test]
    fn an_empty_keyword_shows_the_usage_card_without_a_request() {
        let http = Stub::empty();
        assert_eq!(body_of(&run(http.clone(), "")), help("."));
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_search_url_encodes_the_keyword() {
        assert_eq!(
            search_url("oppo a3s"),
            "https://carisinyal.com/?s=oppo%20a3s"
        );
        assert_eq!(
            search_url("a&b"),
            "https://carisinyal.com/?s=a%26b"
        );
    }

    #[test]
    fn every_post_becomes_one_hit_and_a_blank_title_is_skipped() {
        let html = r#"<div class="oxy-post"><a class="oxy-post-title" href="/a/">  </a><span class="oxy-post-meta">Tablet</span></div>
        <div class="oxy-post"><a class="oxy-post-title" href="/b/">Samsung A55</a><span class="oxy-post-meta">PONSEL</span></div>
        <div class="oxy-post"><a class="oxy-post-title">Tanpa href</a></div>"#;
        let hits = parse_search(html);
        assert_eq!(hits.len(), 2, "judul kosong harus dilewati: {hits:?}");
        assert_eq!(hits[0].title, "Samsung A55");
        assert_eq!(hits[0].kind, "PONSEL");
        assert_eq!(hits[0].url.as_deref(), Some("/b/"));
        assert_eq!(hits[1].title, "Tanpa href");
        assert_eq!(hits[1].url, None, "href tidak ada -> undefined");
    }

    #[test]
    fn the_phone_hit_wins_over_the_first_result() {
        let hits = parse_search(DAFTAR);
        assert_eq!(hits.len(), 2);
        assert_eq!(pick_phone(&hits).title, "OPPO A3s", "meta PONSEL harus menang");
        assert_eq!(pick_phone(&hits[1..]).title, "OPPO A3s");
    }

    #[test]
    fn a_hit_without_the_phone_meta_falls_back_to_the_first_one() {
        let html = r#"<div class="oxy-post"><a class="oxy-post-title" href="/a/">Tablet X</a><span class="oxy-post-meta">Tablet</span></div>"#;
        let hits = parse_search(html);
        assert_eq!(pick_phone(&hits).title, "Tablet X");
    }

    #[test]
    fn the_spec_table_becomes_key_value_pairs_with_the_last_value_winning() {
        let html = r#"<table class="box-info">
          <tr class="box-baris"><td class="kolom-satu">RAM</td><td class="kolom-dua">3 GB</td></tr>
          <tr class="box-baris"><td class="kolom-satu">RAM</td><td class="kolom-dua">4 GB</td></tr>
          <tr class="box-baris"><td class="kolom-satu">CPU</td><td class="kolom-dua"></td></tr>
          <tr class="box-baris"><td class="kolom-satu"></td><td class="kolom-dua">x</td></tr>
          <tr><td class="kolom-satu">Di luar</td><td class="kolom-dua">x</td></tr>
        </table>"#;
        let specs = parse_specs(html);
        assert_eq!(specs.get(&["RAM"]), Some("4 GB"), "nilai kedua menimpa");
        assert_eq!(specs.get(&["CPU"]), None, "baris dengan nilai kosong diabaikan");
        assert_eq!(specs.get(&["Di luar"]), None, "tr tanpa class box-baris diabaikan");
        assert_eq!(specs.get(&["Tidak ada", "RAM"]), Some("4 GB"), "kunci dicoba berurutan");
    }

    #[test]
    fn the_caption_lists_only_the_specs_that_exist() {
        let specs = parse_specs(DETAIL);
        let c = format_caption(&first_h1(DETAIL), &specs);
        assert!(c.starts_with("📱 *SPESIFIKASI OPPO A3S*\n\n"), "{c}");
        assert!(c.contains("📅 *Rilis:* 2018/07\n"), "{c}");
        assert!(c.contains("📶 *Jaringan:* GSM / HSPA / LTE\n\n"), "{c}");
        assert!(c.contains("🖥️ *LAYAR*\n- *Tipe:* IPS LCD\n"), "{c}");
        assert!(c.contains("⚙️ *PERFORMA & MEMORI*\n- *Chipset:* MediaTek Helio P23\n"), "{c}");
        assert!(c.contains("📸 *KAMERA UTAMA*\n- *Jumlah:* 1 Kamera\n"), "{c}");
        assert!(c.contains("🔋 *BATERAI*\n- *Kapasitas:* 4230 mAh\n"), "{c}");
        assert!(c.contains("- *Sensor:* Accelerometer, proximity\n"), "{c}");
        assert!(!c.contains("NFC"), "baris tanpa nilai tidak boleh muncul: {c}");
        assert!(c.ends_with("\n🔗 *Sumber:* Carisinyal"), "{c}");
    }

    #[test]
    fn an_empty_spec_table_still_renders_every_heading() {
        let c = format_caption("HP Tanpa Data", &Specs::default());
        // JS `carisinyal.js:156` men-uppercase judul, dan baris 159 menambah
        // satu `\n` kosong sebelum heading LAYAR meski data kosong.
        assert_eq!(c, format!("📱 *SPESIFIKASI {}*\n\n\n🖥️ *LAYAR*\n\n⚙️ *PERFORMA & MEMORI*\n\n📸 *KAMERA UTAMA*\n\n🔋 *BATERAI*\n\n📡 *KONEKTIVITAS & LAINNYA*\n\n🔗 *Sumber:* Carisinyal", "HP TANPA DATA"));
    }

    #[test]
    fn the_metadata_scrapers_read_the_title_and_the_image() {
        assert_eq!(first_h1(DETAIL), "OPPO A3s");
        assert_eq!(
            meta_content(DETAIL, "property", "og:image").as_deref(),
            Some("https://carisinyal.com/img/a3s.jpg")
        );
        assert_eq!(
            meta_content(DETAIL, "name", "description").as_deref(),
            Some("Spesifikasi OPPO A3s")
        );
        assert_eq!(meta_content("<html></html>", "property", "og:image"), None);
    }

    #[test]
    fn a_hit_with_an_image_becomes_an_image_reply() {
        let http = Stub::new(DAFTAR, DETAIL);
        let r = run(http.clone(), "oppo a3s");
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url("https://carisinyal.com/img/a3s.jpg"))
        );
        assert!(body_of(&r).contains("- *Chipset:* MediaTek Helio P23"));
        assert_eq!(http.seen.lock().unwrap().len(), 2, "daftar lalu detail");
    }

    #[test]
    fn a_detail_page_without_an_image_falls_back_to_plain_text() {
        let html = "<h1>HP Murahan</h1>";
        let r = run(Stub::new(DAFTAR, html), "hp");
        assert!(body_of(&r).starts_with("📱 *SPESIFIKASI HP MURAHAN*"), "{}", body_of(&r));
        assert!(!r.has_media());
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn an_empty_result_page_reports_not_found() {
        for html in [
            "<html><body>tidak ada</body></html>",
            r#"<div class="oxy-post"><a class="oxy-post-title" href="/a/">  </a></div>"#,
        ] {
            let r = run(Stub::new(html, DETAIL), "hp mitos");
            assert_eq!(body_of(&r), not_found("hp mitos"), "{html}");
            assert_eq!(reaction_of(&r), Some("❌"), "{html}");
        }
    }

    #[test]
    fn a_dead_search_page_reports_the_failure_message() {
        let http = Stub::empty();
        let r = run(http.clone(), "oppo");
        assert_eq!(body_of(&r), FETCH_FAILED);
        assert_eq!(reaction_of(&r), Some("❌"));
        assert_eq!(http.seen.lock().unwrap().len(), 1, "hanya request daftar");
    }

    #[test]
    fn a_hit_without_a_url_fails_like_axios_without_a_url() {
        let html = r#"<div class="oxy-post"><a class="oxy-post-title">Tanpa href</a><span class="oxy-post-meta">PONSEL</span></div>"#;
        let r = run(Stub::new(html, DETAIL), "oppo");
        assert_eq!(body_of(&r), FETCH_FAILED);
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn a_very_long_keyword_is_sent_whole() {
        let http = Stub::new(DAFTAR, DETAIL);
        let _ = run(http.clone(), &"a".repeat(4_000));
        let seen = http.seen.lock().unwrap();
        assert!(seen[0].contains(&"a".repeat(4_000)), "{}", seen[0].len());
    }
}