//! `plugins/search/movieku.js` — cari film di Movieku beserta link download.
//!
//! `ponytail:` JS memakai cheerio untuk mem-parse halaman detail. Crate ini tidak
//! punya parser HTML dan menambah dependency baru dilarang, jadi yang dipakai di
//! sini hanya operasi yang sama dengan selector yang dibaca plugin: `h1`,
//! `.entry-content p`, `img[src*="wp-content/uploads"]`, `ul li`, `a` di dalam
//! `li`, `a[href*="abyssplayer"]`, `strong`, dan `a` di dalam `parent()`.
//!
//! `ponytail:` JS mengirim DUA pesan saat ada lebih dari satu hasil: kartu
//! poster dulu, lalu daftar "HASIL LAINNYA". `Reply` hanya satu pesan, jadi
//! yang terkirim kartu pertamanya; teks daftar tetap ditulis sebagai fungsi
//! murni.
#![allow(clippy::needless_range_loop)]

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `movieku.js:23`.
pub(crate) const ENDPOINT: &str = "https://movieku.rest/wp-admin/admin-ajax.php";

/// `movieku.js:95-104` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎬 *MOVIEKU*\n\n\
         Fitur ini membantu kamu mencari informasi lengkap tentang film dari database Movieku, termasuk sinopsis, detail film, dan link download dalam berbagai kualitas resolusi\n\n\
         *Cara pakai:*\n\
         > `{prefix}movieku <judul film>`\n\n\
         *Contoh:*\n\
         > `{prefix}movieku avengers`\n\
         > `{prefix}movieku one piece`\n\n\
         _Hasil pencarian akan menampilkan film yang paling relevan dengan judul yang kamu cari_"
    )
}

/// `movieku.js:114` — tidak ada hasil.
pub(crate) fn tidak_ditemukan(query: &str) -> String {
    format!("❌ Film dengan kata kunci *{query}* tidak ditemukan, coba gunakan judul yang lebih spesifik ya")
}

/// `movieku.js:161-170` — blok "HASIL LAINNYA". Maksimal 10 judul dari indeks 1.
#[allow(dead_code)]
pub(crate)
fn hasil_lainnya(movies: &[Value]) -> String {
    let mut list = String::from("🎬 *HASIL LAINNYA*\n\n");
    list.push_str(&format!(
        "Ditemukan *{}* film yang cocok dengan pencarianmu, berikut daftar lengkapnya:\n\n",
        movies.len()
    ));
    // `for (let i = 1; i < maxShow; i++)` — maxShow = min(len, 10), dan i mulai
    // dari 1, jadi film PERTAMA tidak pernah ikut (sudah jadi kartu utama).
    let max_show = movies.len().min(10);
    for i in 1..max_show {
        let m = &movies[i];
        list.push_str(&format!(
            "- *{}*\n  > {}\n\n",
            interp(m.get("post_title")),
            interp(m.get("post_link"))
        ));
    }
    if movies.len() > 10 {
        list.push_str(&format!("_...dan {} film lainnya_", movies.len() - 10));
    }
    list.trim().to_string()
}

/// `${value}` di JS.
fn interp(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

// ── SELECTOR MINIMAL ────────────────────────────────────────────────────────

/// `<tag` yang benar-benar membuka tag.
fn buka_tag(html: &str, from: usize, tag: &str) -> Option<usize> {
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

/// Isi tag pertama setelah `from`.
fn isi_tag(html: &str, from: usize, tag: &str) -> Option<String> {
    let open_end = html[from..].find('>')? + from + 1;
    let close = format!("</{tag}>");
    let end = html[open_end..].find(&close)?;
    Some(html[open_end..open_end + end].trim().to_string())
}

/// Nilai atribut pertama `name` pada tag yang dibuka di `from`.
fn atribut(html: &str, from: usize, name: &str) -> Option<String> {
    let open_end = html[from..].find('>')? + from + 1;
    let tag_end = html[open_end..].find('<')? + open_end;
    let tag = &html[from..tag_end];
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    Some(tag[start..end].to_string())
}

/// `$(el).text()` — gabungan semua text node, tag dibuang.
fn teks_dalam(html: &str, from: usize, to: usize) -> String {
    /// Tag HTML tanpa pasangan penutup; isinya tidak pernah di-rekursi.
    const KOSONG: [&str; 7] = ["br", "img", "hr", "input", "meta", "link", "source"];

    let mut out = String::new();
    let mut cursor = from;
    while cursor < to {
        let Some(rel) = html[cursor..to].find('<') else {
            out.push_str(&html[cursor..to]);
            break;
        };
        let open = cursor + rel;
        out.push_str(&html[cursor..open]);
        let Some(gt) = html[open..to].find('>') else {
            break;
        };
        let tag_end = open + gt + 1;
        let tag = &html[open..tag_end];
        let name: String = tag[1..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        if tag.starts_with('/') || tag.ends_with("/>") || KOSONG.contains(&name.as_str()) {
            cursor = tag_end;
            continue;
        }
        let close_tag = format!("</{name}>");
        match html[tag_end..to].find(&close_tag) {
            Some(rel) => {
                let inner_end = tag_end + rel;
                out.push_str(&teks_dalam(html, tag_end, inner_end));
                cursor = inner_end + close_tag.len();
            }
            // Penutup tidak ada (HTML rusak): sisa blok diperlakukan sebagai
            // isi, bukan di-drop.
            None => {
                out.push_str(&teks_dalam(html, tag_end, to));
                cursor = to;
            }
        }
    }
    out.trim().to_string()
}

/// `$(el).find("a").map(text).get().join(", ")` — daftar `<a>` descendant.
fn teks_link(block: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut at = 0;
    while let Some(pos) = buka_tag(block, at, "a") {
        parts.push(isi_tag(block, pos, "a").unwrap_or_default());
        at = pos + 2;
    }
    parts.join(", ")
}

/// `$(el).find("a").first().text().trim()`.
fn teks_link_pertama(block: &str) -> String {
    let mut at = 0;
    while let Some(pos) = buka_tag(block, at, "a") {
        if let Some(t) = isi_tag(block, pos, "a") {
            return t;
        }
        at = pos + 2;
    }
    String::new()
}

/// Detail film hasil parse (`movieku.js:71`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Detail {
    pub title: String,
    pub synopsis: String,
    pub poster: String,
    pub genre: String,
    pub release: String,
    pub duration: String,
    pub director: String,
    pub country: String,
    pub quality: String,
    pub score: String,
    pub rating: String,
    pub stars: String,
    pub stream: String,
    /// Label kualitas → (server → URL).
    pub downloads: Vec<(String, Vec<(String, String)>)>,
}

/// `getMovieDetail` (`movieku.js:36-72`).
pub(crate) fn parse_detail(html: &str) -> Detail {
    let mut d = Detail {
        title: buka_tag(html, 0, "h1")
            .and_then(|i| isi_tag(html, i, "h1"))
            .unwrap_or_default(),
        synopsis: html
            .find("entry-content")
            .and_then(|i| buka_tag(html, i, "p"))
            .and_then(|i| isi_tag(html, i, "p"))
            .unwrap_or_default(),
        stream: String::new(),
        ..Detail::default()
    };

    // `$('img[src*="wp-content/uploads"]').first()`.
    let mut at = 0;
    while let Some(pos) = buka_tag(html, at, "img") {
        let Some(src) = atribut(html, pos, "src") else {
            at = pos + 4;
            continue;
        };
        if src.contains("wp-content/uploads") {
            d.poster = src;
            break;
        }
        at = pos + 4;
    }

    // `a[href*="abyssplayer"]` pertama.
    let mut at = 0;
    while let Some(pos) = buka_tag(html, at, "a") {
        if atribut(html, pos, "href")
            .is_some_and(|h| h.contains("abyssplayer"))
        {
            d.stream = atribut(html, pos, "href").unwrap_or_default();
            break;
        }
        at = pos + 2;
    }

    // `$("ul li").each(..)` — hanya `<li>` PERTAMA yang ada di setiap `<ul>`,
    // tapi JS memproses semua; di sini semua `<li>` diproses.
    let mut cursor = 0;
    while let Some(ul) = buka_tag(html, cursor, "ul") {
        let Some(ul_end) = html[ul..].find("</ul>").map(|i| ul + i) else {
            break;
        };
        let list = &html[ul..ul_end];
        cursor = ul_end + 5;
        let mut row = 0;
        while let Some(li) = buka_tag(list, row, "li") {
            let Some(li_end) = list[li..].find("</li>").map(|i| li + i) else {
                break;
            };
            let block = &list[li..li_end];
            row = li_end + 5;
            let text = teks_dalam(block, 0, block.len());
            if let Some(rest) = text.strip_prefix("Genre:") {
                // JS memakai `$(el).find("a")`, bukan sisa teks; sisa teks
                // dipakai hanya sebagai penanda bahwa baris ini genre.
                if !rest.trim().is_empty() {
                    d.genre = teks_link(block);
                }
            } else if let Some(rest) = text.strip_prefix("Release:") {
                d.release = rest.trim().to_string();
            } else if let Some(rest) = text.strip_prefix("Duration:") {
                d.duration = rest.trim().to_string();
            } else if text.starts_with("Director:") {
                d.director = teks_link_pertama(block);
            } else if text.starts_with("Country:") {
                d.country = teks_link_pertama(block);
            } else if let Some(rest) = text.strip_prefix("Quality:") {
                d.quality = rest.trim().to_string();
            } else if let Some(rest) = text.strip_prefix("Score:") {
                d.score = rest.trim().to_string();
            } else if let Some(rest) = text.strip_prefix("Rating:") {
                d.rating = rest.trim().to_string();
            } else if text.starts_with("Stars:") {
                d.stars = teks_link(block);
            }
        }
    }

    // `$("strong").each(..)` — label kualitas, lalu `<a>` di dalam
    // `$(el).parent()`.
    let mut at = 0;
    while let Some(pos) = buka_tag(html, at, "strong") {
        at = pos + 7;
        let Some(label) = isi_tag(html, pos, "strong") else {
            continue;
        };
        if !["1080p", "720p", "480p", "360p"].contains(&label.as_str()) {
            continue;
        }
        let links_html = match induk_dari(html, pos) {
            Some((mulai, selesai)) => &html[mulai..selesai],
            None => "",
        };
        let mut links: Vec<(String, String)> = Vec::new();
        let mut a_at = 0;
        while let Some(a) = buka_tag(links_html, a_at, "a") {
            a_at = a + 2;
            let label_a = isi_tag(links_html, a, "a").unwrap_or_default();
            let href = atribut(links_html, a, "href").unwrap_or_default();
            links.push((label_a, href));
        }
        if let Some(slot) = d.downloads.iter_mut().find(|(k, _)| *k == label) {
            slot.1 = links;
        } else {
            d.downloads.push((label, links));
        }
    }

    d
}

/// `$(el).parent()` — rentang isi tag yang masih terbuka di `pos`.
///
/// Tanpa DOM, tag pembuka terdekat yang tag penutupnya berada SETELAH `pos`
/// dipakai. Hanya tag blok yang diperiksa; `<strong>` dan `<a>` sendiri
/// selalu lebih kurung dari `<p>`/`<li>` yang memuatnya, jadi tidak mungkin
/// menjadi induk yang benar. Tag yang tidak punya pasangan penutup (HTML
/// rusak) diabaikan — persis `parent()` cheerio pada dokumen yang sama.
fn induk_dari(html: &str, pos: usize) -> Option<(usize, usize)> {
    const BLOK: [&str; 6] = ["p", "div", "li", "td", "section", "article"];
    let mut terbaik: Option<(usize, usize, usize)> = None;
    for tag in BLOK {
        let mut at = 0;
        while let Some(open) = buka_tag(html, at, tag) {
            if open >= pos {
                break;
            }
            at = open + 1;
            let Some(content_start) = html[open..].find('>').map(|i| open + i + 1) else {
                continue;
            };
            let close = format!("</{tag}>");
            let Some(close_at) = html[pos..].find(&close).map(|i| pos + i) else {
                continue;
            };
            if terbaik.is_none_or(|(o, _, _)| open > o) {
                terbaik = Some((open, content_start, close_at));
            }
        }
    }
    terbaik.map(|(_, mulai, selesai)| (mulai, selesai))
}

/// `formatDownloads` (`movieku.js:74-89`) — kualitas SELALU di urutan tetap,
/// bukan urutan kemunculan di HTML.
pub(crate) fn format_downloads(downloads: &[(String, Vec<(String, String)>)]) -> String {
    if downloads.is_empty() {
        return String::new();
    }
    let mut txt = String::from("\n🔽 *LINK DOWNLOAD*\n\n");
    for q in ["1080p", "720p", "480p", "360p"] {
        let Some((_, links)) = downloads.iter().find(|(k, _)| k == q) else {
            continue;
        };
        if links.is_empty() {
            continue;
        }
        txt.push_str(&format!("📀 *{q}*\n"));
        for (server, url) in links {
            txt.push_str(&format!("- {server}: {url}\n"));
        }
        txt.push('\n');
    }
    txt
}

/// `movieku.js:120-146` — kartu film.
pub(crate) fn format_movie(detail: &Detail, movie: &Value, query: &str) -> String {
    let judul = if detail.title.is_empty() {
        let dari_post = movie.get("post_title").and_then(Value::as_str).unwrap_or("");
        if dari_post.is_empty() {
            query.to_uppercase()
        } else {
            dari_post.to_string()
        }
    } else {
        detail.title.clone()
    };
    let mut txt = format!("🎬 *{judul}*\n\n");
    if !detail.synopsis.is_empty() {
        let chars: Vec<char> = detail.synopsis.chars().collect();
        let synopsis = if chars.len() > 500 {
            format!("{}...", chars[..497].iter().collect::<String>())
        } else {
            detail.synopsis.clone()
        };
        txt.push_str(&format!("📝 *Sinopsis:*\n{synopsis}\n\n"));
    }
    txt.push_str("📋 *DETAIL FILM*\n\n");
    let baris: [(&str, &String); 9] = [
        ("🎭 Genre: ", &detail.genre),
        ("📅 Rilis: ", &detail.release),
        ("⏱️ Durasi: ", &detail.duration),
        ("📺 Kualitas: ", &detail.quality),
        ("🌍 Negara: ", &detail.country),
        ("🎬 Sutradara: ", &detail.director),
        ("⭐ Rating: ", &detail.rating),
        ("📊 Score: ", &detail.score),
        ("🌟 Pemeran: ", &detail.stars),
    ];
    for (label, value) in baris {
        if !value.is_empty() {
            txt.push_str(&format!("{label}*{value}*\n"));
        }
    }
    if !detail.stream.is_empty() {
        txt.push_str(&format!("\n▶️ *Streaming:* {}\n", detail.stream));
    }
    txt.push_str(&format_downloads(&detail.downloads));
    txt.push_str(&format!(
        "🔗 {}",
        interp(movie.get("post_link"))
    ));
    txt
}

/// `movieku.js:22-34` — POST form-urlencoded ke admin-ajax.
fn search_request(query: &str) -> HttpCall {
    HttpCall::post(ENDPOINT)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("X-Requested-With", "XMLHttpRequest")
        .form(format!(
            "action=ts_ac_do_search&ts_ac_query={}",
            encode_uri_component(query)
        ))
}

#[plugin(
    name = "movieku",
    alias = ["movie", "film", "carifilm", "carimovie"],
    category = "search",
    description = "Cari dan tampilkan informasi film lengkap dari Movieku beserta link download dalam berbagai kualitas",
    usage = ".movieku <judul film>",
    example = ".movieku avengers",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn movieku(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `m.react("🔍")` di `:107` sudah jalan sebelum request.
    let Ok(res) = ctx.http.call(&search_request(query)) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };

    // `res.data?.post?.[0]?.all || []`.
    let movies: Vec<Value> = res
        .json()
        .and_then(|b| b.pointer("/post/0/all").cloned())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();

    if movies.is_empty() {
        return Reply::text(tidak_ditemukan(query)).with_reaction("❌");
    }

    let movie = &movies[0];
    let link = movie
        .get("post_link")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let Ok(detail_res) = ctx.http.get(&link) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };
    let detail = parse_detail(&detail_res.text());
    let txt = format_movie(&detail, movie, query);

    // `m.react("✅")` di `:148` terjadi sebelum pengiriman media.
    let poster = if detail.poster.is_empty() {
        movie
            .get("post_image")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    } else {
        detail.poster.clone()
    };
    if poster.is_empty() {
        Reply::text(txt).with_reaction("✅")
    } else {
        Reply::image(MediaSource::url(poster), txt).with_reaction("✅")
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    #![allow(clippy::field_reassign_with_default)]
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        /// (prefix URL, status, body).
        pages: Vec<(String, u16, String)>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn new(pages: &[(&str, u16, &str)]) -> Arc<Stub> {
            Arc::new(Stub {
                pages: pages
                    .iter()
                    .map(|(u, s, b)| ((*u).to_string(), *s, (*b).to_string()))
                    .collect(),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                pages: vec![],
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    /// Satu sumber untuk GET dan `call`: plugin memakai POST lewat `call()`
    /// dan GET biasa lewat `get()`.
    impl Stub {
        fn jawab(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let hit = self.pages.iter().find(|(u, _, _)| url.starts_with(u.as_str()));
            let Some((_, status, body)) = hit else {
                return Err(oktz_plugin_api::HttpError { message: "tanpa stub".into() });
            };
            Ok(HttpResponse {
                status: *status,
                body: HttpBody::Bytes {
                    data: Arc::from(body.clone().into_bytes()),
                    mimetype: Some("text/html".into()),
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.jawab(url)
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma POST")
        }

        fn call(&self, req: &oktz_plugin_api::HttpCall) -> Result<HttpResponse, HttpError> {
            self.jawab(&req.url)
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "movieku".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("movieku").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const DETAIL: &str = r#"<html><body>
      <h1>Avengers: Endgame</h1>
      <img src="https://movieku.rest/wp-content/uploads/2019/poster.jpg" alt="p">
      <img src="https://movieku.rest/other/Logo.png" alt="l">
      <div class="entry-content"><p>SetelahLOCEAN Events</p><p>Paragraf kedua</p></div>
      <ul>
        <li>Genre: <a href="/a">Action</a><a href="/b">Sci-Fi</a></li>
        <li>Release: 2019</li>
        <li>Duration: 3j 11m</li>
        <li>Director: <a href="/d">Anthony Russo</a></li>
        <li>Country: <a href="/c">USA</a></li>
        <li>Quality: BluRay</li>
        <li>Score: 8.4</li>
        <li>Rating: PG-13</li>
        <li>Stars: <a href="/s">A</a><a href="/t">B</a></li>
      </ul>
      <a href="https://abyssplayer.to/watch/xyz">Nonton</a>
      <p><strong>720p</strong><a href="https://dl/720-a">Server A</a></p>
      <p><strong>1080p</strong><a href="https://dl/1080-a">Mirror 1</a><a href="https://dl/1080-b">Mirror 2</a></p>
      <p><strong>360p</strong><a href="https://dl/360-a">Server A</a><a href="https://dl/360-b">Server B</a></p>
    </body></html>"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("movieku").unwrap();
        assert_eq!(p.meta.name, "movieku");
        assert_eq!(p.meta.alias, ["movie", "film", "carifilm", "carimovie"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(
            p.meta.description,
            "Cari dan tampilkan informasi film lengkap dari Movieku beserta link download dalam berbagai kualitas"
        );
        assert_eq!(p.meta.usage, ".movieku <judul film>");
        assert_eq!(p.meta.example, ".movieku avengers");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        // `film` sengaja tidak diuji: `plugins/search/film.js` memakai `film`
        // sebagai `name`, dan `getPlugin` mengutamakan `commands` di atas
        // `aliases` (`src/lib/plugins.js:334`), jadi `.film` tetap ke plugin
        // film — tabrakan ini tercatat di `common::JS_COLLISIONS`.
        for a in ["movie", "carifilm", "carimovie"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "movieku");
        }
        assert_eq!(oktz_plugin_api::find("film").unwrap().meta.name, "film");
    }

    #[test]
    fn an_empty_title_shows_the_help_card() {
        let http = Stub::offline();
        for text in ["", "  ", "\t\n"] {
            let t = text_of(&run(http.clone(), text));
            assert!(t.starts_with("🎬 *MOVIEKU*\n\n"), "input {text:?}");
            assert!(t.contains("> `.movieku <judul film>`"), "{t}");
            assert!(t.ends_with("_Hasil pencarian akan menampilkan film yang paling relevan dengan judul yang kamu cari_"), "{t}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
        assert!(help("!").contains("`!movieku one piece`"));
    }

    #[test]
    fn the_detail_page_yields_every_field_the_card_uses() {
        let d = parse_detail(DETAIL);
        assert_eq!(d.title, "Avengers: Endgame");
        assert_eq!(d.synopsis, "SetelahLOCEAN Events", "p pertama saja");
        assert_eq!(
            d.poster,
            "https://movieku.rest/wp-content/uploads/2019/poster.jpg",
            "hanya img yang memuat wp-content/uploads"
        );
        assert_eq!(d.genre, "Action, Sci-Fi", "link digabung dengan koma");
        assert_eq!(d.release, "2019");
        assert_eq!(d.duration, "3j 11m");
        assert_eq!(d.director, "Anthony Russo");
        assert_eq!(d.country, "USA");
        assert_eq!(d.quality, "BluRay");
        assert_eq!(d.score, "8.4");
        assert_eq!(d.rating, "PG-13");
        assert_eq!(d.stars, "A, B");
        assert_eq!(d.stream, "https://abyssplayer.to/watch/xyz");
    }

    #[test]
    fn downloads_keep_a_fixed_quality_order_not_the_html_order() {
        let d = parse_detail(DETAIL);
        let labels: Vec<&str> = d.downloads.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(labels, ["720p", "1080p", "360p"], "urutan kemunculan");
        let txt = format_downloads(&d.downloads);
        assert!(txt.starts_with("\n🔽 *LINK DOWNLOAD*\n\n"), "{txt:?}");
        let pos1080 = txt.find("📀 *1080p*").unwrap();
        let pos720 = txt.find("📀 *720p*").unwrap();
        let pos360 = txt.find("📀 *360p*").unwrap();
        assert!(pos1080 < pos720 && pos720 < pos360, "urutan tetap: {txt}");
        assert!(txt.contains("- Mirror 1: https://dl/1080-a\n"), "{txt}");
        assert!(txt.contains("- Server A: https://dl/360-a\n"), "{txt}");
        assert!(txt.contains("- Server B: https://dl/360-b\n"), "{txt}");
        assert!(!txt.contains("📀 *480p*"), "kualitas absen tidak dicetak: {txt}");
        // `<a>` milik paragraf lain tidak ikut masuk ke label sebelumnya.
        assert!(!txt.contains("https://dl/720-a\n📀"), "{txt}");
    }

    #[test]
    fn an_empty_download_map_produces_nothing() {
        assert_eq!(format_downloads(&[]), "");
        assert_eq!(format_downloads(&[("1080p".into(), vec![])]), "\n🔽 *LINK DOWNLOAD*\n\n",
            "kualitas tanpa link tetap mencetak header");
    }

    #[test]
    fn the_card_skips_empty_detail_rows() {
        let movie = serde_json::json!({ "post_link": "https://movieku.rest/film/avengers", "post_image": "https://i/p.jpg" });
        let d = parse_detail(DETAIL);
        let txt = format_movie(&d, &movie, "avengers");
        assert!(txt.starts_with("🎬 *Avengers: Endgame*\n\n📝 *Sinopsis:*\nSetelahLOCEAN Events\n\n"), "{txt}");
        assert!(txt.contains("🎭 Genre: *Action, Sci-Fi*\n"), "{txt}");
        assert!(txt.contains("📅 Rilis: *2019*\n"), "{txt}");
        assert!(txt.contains("▶️ *Streaming:* https://abyssplayer.to/watch/xyz\n"), "{txt}");
        assert!(txt.ends_with("🔗 https://movieku.rest/film/avengers"), "{txt}");

        let kosong = Detail::default();
        let txt = format_movie(&kosong, &Value::Null, "one piece");
        assert!(txt.starts_with("🎬 *ONE PIECE*\n\n📋 *DETAIL FILM*\n\n🔗 undefined"), "{txt}");
        assert!(!txt.contains("Genre:"), "baris kosong dilewati: {txt}");
        assert!(!txt.contains("Sinopsis"), "sinopsis kosong dilewati: {txt}");
    }

    #[test]
    fn a_title_from_the_search_row_is_used_when_the_page_has_no_h1() {
        let movie = serde_json::json!({ "post_title": "Interstellar", "post_link": "https://x" });
        let txt = format_movie(&Detail::default(), &movie, "interstellar");
        assert!(txt.starts_with("🎬 *Interstellar*\n"), "{txt}");
    }

    #[test]
    fn a_synopsis_over_five_hundred_characters_is_cut_at_497() {
        let mut d = Detail::default();
        d.synopsis = "y".repeat(600);
        let movie = serde_json::json!({ "post_link": "https://x" });
        let txt = format_movie(&d, &movie, "q");
        assert!(txt.contains(&format!("📝 *Sinopsis:*\n{}...\n", "y".repeat(497))), "{txt}");
        d.synopsis = "z".repeat(500);
        let txt = format_movie(&d, &movie, "q");
        assert!(txt.contains(&format!("📝 *Sinopsis:*\n{}\n\n", "z".repeat(500))), "tepat 500 tidak dipotong");
        assert!(!txt.contains("..."), "{txt}");
    }

    #[test]
    fn the_other_hits_list_starts_from_the_second_movie() {
        let movies: Vec<Value> = (0..4)
            .map(|i| serde_json::json!({ "post_title": format!("Film {i}"), "post_link": format!("https://x/{i}") }))
            .collect();
        let txt = hasil_lainnya(&movies);
        assert!(txt.starts_with("🎬 *HASIL LAINNYA*\n\nDitemukan *4* film yang cocok dengan pencarianmu, berikut daftar lengkapnya:\n\n"), "{txt}");
        assert!(txt.contains("- *Film 1*\n  > https://x/1\n"), "{txt}");
        // Baris terakhir sudah di-trim, jadi tanpa newline penutup.
        assert!(txt.ends_with("- *Film 3*\n  > https://x/3"), "{txt}");
        assert!(!txt.contains("Film 0"), "film pertama jadi kartu utama: {txt}");
    }

    #[test]
    fn a_long_result_list_is_capped_at_ten_with_a_tail_count() {
        let movies: Vec<Value> = (0..15)
            .map(|i| serde_json::json!({ "post_title": format!("Film {i}"), "post_link": format!("https://x/{i}") }))
            .collect();
        let txt = hasil_lainnya(&movies);
        assert!(txt.contains("- *Film 9*\n"), "indeks 1..9: {txt}");
        assert!(!txt.contains("Film 10"), "{txt}");
        assert!(txt.ends_with("_...dan 5 film lainnya_"), "{txt}");
    }

    #[test]
    fn an_empty_search_result_reports_not_found() {
        let http = Stub::new(&[(ENDPOINT, 200, r#"{"post":[{"all":[]}]}"#)]);
        let r = run(http, "zzzz");
        assert_eq!(
            text_of(&r),
            "❌ Film dengan kata kunci *zzzz* tidak ditemukan, coba gunakan judul yang lebih spesifik ya"
        );
    }

    #[test]
    fn a_malformed_search_payload_is_treated_as_no_result() {
        for body in ["{}", r#"{"post":[]}"#, r#"{"post":[{"all":{}}]}"#, "bukan json"] {
            let http = Stub::new(&[(ENDPOINT, 200, body)]);
            let r = run(http, "q");
            assert!(text_of(&r).contains("tidak ditemukan"), "body {body}");
        }
    }

    #[test]
    fn a_full_round_trip_renders_the_card_with_the_detail_poster() {
        let http = Stub::new(&[
            (
                ENDPOINT,
                200,
                r#"{"post":[{"all":[{"post_title":"Avengers","post_link":"https://movieku.rest/film/avengers","post_image":"https://i/fallback.jpg"}]}]}"#,
            ),
            ("https://movieku.rest/film/", 200, DETAIL),
        ]);
        let r = run(http.clone(), "avengers");
        // `.with_reaction("✅")` membungkus balasan; `without_mutations`
        // berhenti di `Reaction`, jadi isinya harus diturunkan satu lapis.
        let mut cur = r.without_mutations();
        while let Reply::Reaction { reply, .. } = cur {
            cur = reply;
        }
        match cur {
            Reply::Image { caption, media, .. } => {
                assert!(caption.contains("🎬 *Avengers: Endgame*"), "{caption}");
                let src = media.as_ref().unwrap();
                match src {
                    MediaSource::Url { url, .. } => assert_eq!(
                        url,
                        "https://movieku.rest/wp-content/uploads/2019/poster.jpg",
                        "poster halaman menang atas post_image"
                    ),
                    other => panic!("harus Url, dapat {other:?}"),
                }
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen[0], ENDPOINT);
        assert_eq!(seen[1], "https://movieku.rest/film/avengers");
    }

    #[test]
    fn a_post_image_is_used_when_the_page_has_no_poster() {
        let http = Stub::new(&[
            (
                ENDPOINT,
                200,
                r#"{"post":[{"all":[{"post_link":"https://movieku.rest/film/x","post_image":"https://i/fallback.jpg"}]}]}"#,
            ),
            ("https://movieku.rest/film/x", 200, "<html><h1>X</h1></html>"),
        ]);
        let r = run(http, "x");
        assert!(r.has_media(), "post_image dipakai");
        assert!(text_of(&r).contains("🎬 *X*"), "{}", text_of(&r));
    }

    #[test]
    fn without_any_poster_the_card_is_sent_as_plain_text() {
        let http = Stub::new(&[
            (
                ENDPOINT,
                200,
                r#"{"post":[{"all":[{"post_link":"https://movieku.rest/film/y"}]}]}"#,
            ),
            ("https://movieku.rest/film/y", 200, "<html><h1>Y</h1></html>"),
        ]);
        let r = run(http, "y");
        assert!(!r.has_media());
        assert!(text_of(&r).contains("🔗 https://movieku.rest/film/y"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "avengers");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "movieku"));
    }

    #[test]
    fn the_search_is_a_form_post_with_the_two_javascript_headers() {
        let call = search_request("one piece");
        assert_eq!(call.method, oktz_plugin_api::HttpMethod::Post);
        assert_eq!(call.url, ENDPOINT);
        assert!(call.headers.iter().any(|(k, v)| k == "Content-Type"
            && v == "application/x-www-form-urlencoded"));
        assert!(call
            .headers
            .iter()
            .any(|(k, v)| k == "X-Requested-With" && v == "XMLHttpRequest"));
        match &call.body {
            Some(HttpBody::Form(f)) => assert_eq!(
                f,
                "action=ts_ac_do_search&ts_ac_query=one%20piece"
            ),
            other => panic!("harus Form, dapat {other:?}"),
        }
    }

    #[test]
    fn empty_and_hostile_detail_pages_do_not_panic() {
        for html in [
            "",
            "<html></html>",
            "<ul><li>Genre:</li></ul>",
            "<div class=\"item\"><strong>1080p</strong></div>",
            "<a href=\"https://abyssplayer.to/\">",
            &"x".repeat(50_000),
        ] {
            let d = parse_detail(html);
            let txt = format_movie(&d, &Value::Null, "q");
            assert!(!txt.is_empty(), "html {html:.20}?");
        }
    }

    #[test]
    fn odd_and_very_long_titles_do_not_panic() {
        for input in ["<b>", "a|b", "🎬", "'\"", "a\u{202e}b", "%00"] {
            let http = Stub::new(&[(ENDPOINT, 200, r#"{"post":[{"all":[]}]}"#)]);
            assert!(
                text_of(&run(http, input)).contains("tidak ditemukan"),
                "input {input:?}"
            );
        }
        let panjang = "film ".repeat(3000);
        let http = Stub::new(&[(ENDPOINT, 200, r#"{"post":[{"all":[]}]}"#)]);
        assert!(text_of(&run(http, &panjang)).contains(&panjang[..20]));
    }
}