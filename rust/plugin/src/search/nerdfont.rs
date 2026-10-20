//! `plugins/search/nerdfont.js` — daftar font Nerd Font untuk dipilih.
//!
//! `ponytail:` JS memakai cheerio. Crate ini tidak punya parser HTML dan
//! menambah dependency baru dilarang, jadi yang dipakai di sini hanya operasi
//! yang sama dengan selector yang dibaca plugin: `div.item`,
//! `span.nerd-font-invisible-text`, `div` pertama, `a.font-preview`,
//! `a.nf-oct-link_external`, dan `a.nf-fa-download`.
//!
//! Dua anomali JS dipertahankan apa adanya:
//! 1. `textContent.match(/Version:…/)[1]` melempar kalau pola tidak cocok —
//!    satu item tanpa `Version:`/`Info:`/gambar membuat SELURUH permintaan
//!    gagal. `parse_fonts` mengembalikan `Err` untuk itu, persis `throw`.
//! 2. `sock.sendMessage` dengan `interactiveButtons` + `buttonParamsJson`
//!    diteruskan apa adanya: `single_select` + `sections[0].rows` jadi
//!    `InteractiveButton`.

use oktz_plugin_api::registry::InteractiveButton;
use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::json;


/// `nerdfont.js:5` — satu-satunya endpoint.
pub(crate) const URL: &str = "https://www.nerdfonts.com/font-downloads";

/// Satu font hasil parse (`nerdfont.js:29-37`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Font {
    pub name: String,
    pub version: String,
    pub info: String,
    pub preview_image: String,
    pub preview_url: String,
    pub download_url: String,
}

/// `nerdfont.js:73` — satu baris tombol. Bentuk JSON yang dibaca Baileys.
pub(crate) fn baris(font: &Font, prefix: &str) -> (String, String, String, String) {
    (
        format!("Font {}", font.name),
        font.info.clone(),
        format!("Version: {}", font.version),
        format!("{prefix}nerdfont-ambil {}", font.name),
    )
}

/// `nerdfont.js:15-16` — `/Version:\s*([^\n\r]+)/` lalu `.trim()`. `null` kalau
/// pola tidak ada — pemanggilnya melempar `TypeError`.
fn setelah_label(teks: &str, label: &str) -> Option<String> {
    let at = teks.find(label)?;
    let rest = &teks[at + label.len()..];
    let value: String = rest
        .chars()
        .take_while(|c| *c != '\n' && *c != '\r')
        .collect();
    Some(value.trim().to_string())
}

/// `nerdfont.js:24` — `/background-image\s*:\s*url\s*\(\s*['"]?([^'"]+)['"]?\s*\)/i`.
///
/// Regex JS memakai flag `i`, jadi pencocokan dilakukan pada huruf kecil;
/// spasi di sekitar `:` dan `(` diizinkan.
fn url_dari_style(style: &str) -> Option<String> {
    let low = style.to_lowercase();
    let at = low.find("background-image")?;
    let mut rest = &low[at + "background-image".len()..];
    rest = rest.trim_start().strip_prefix(':')?.trim_start();
    rest = rest.trim_start().strip_prefix("url").unwrap_or(rest).trim_start();
    rest = rest.strip_prefix('(')?.trim_start();
    let inner = rest
        .strip_prefix('\'')
        .or_else(|| rest.strip_prefix('"'))
        .unwrap_or(rest);
    let end = inner.find(['\'', '"', ')'])?;
    Some(inner[..end].to_string())
}

/// Tag pertama yang benar-benar membuka tag: batas setelah nama tag wajib
/// `>`, spasi, `/`, atau `\n`, supaya `<item>` tidak tertangkap `<i…`.
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

/// Teks di dalam tag pertama setelah `from`.
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

/// Tag `<a>` pertama yang punya kelas `class` di daftar kelasnya.
fn link_dengan_kelas(block: &str, class: &str) -> Option<(usize, Option<String>)> {
    let mut at = 0;
    while let Some(pos) = buka_tag(block, at, "a") {
        if tag_punya_kelas(block, pos, class) {
            return Some((pos, atribut(block, pos, "href")));
        }
        at = pos + 2;
    }
    None
}

/// `nerdfonts()` (`nerdfont.js:10-40`) — `div.item` dipisah satu per satu.
///
/// Batas tiap blok adalah kemunculan `div class="item"` BERIKUTNYA, bukan
/// `</div>` pertama: `</div>` pertama milik `div` dalam yang memuat
/// `Version:`/`Info:`, sehingga memakainya sebagai batas membuat link download
/// berada di luar blok. HTML yang dipakai `nerdfonts.com` tidak bersarang item, jadi
/// batas ini setara dengan "`$(…)` berikutnya".
///
/// `Err` berarti `textContent.match(..)[1]` melempar `TypeError`, persis
/// seperti di JS: satu item yang tidak punya `Version:`, `Info:`, atau
/// `background-image` menggagalkan seluruh permintaan.
pub(crate) fn parse_fonts(html: &str) -> Result<Vec<Font>, String> {
    // Posisi semua `div class="item"`.
    let mut starts = Vec::new();
    let mut at = 0;
    while let Some(pos) = buka_tag(html, at, "div") {
        at = pos + 4;
        if tag_punya_kelas(html, pos, "item") {
            starts.push(pos);
        }
    }

    let mut out = Vec::new();
    for (n, &pos) in starts.iter().enumerate() {
        let end = starts.get(n + 1).copied().unwrap_or(html.len());
        let block = &html[pos..end];

        let name = block
            .find("nerd-font-invisible-text")
            .and_then(|i| isi_tag(block, i, "span"))
            .unwrap_or_default();
        // `$(rynn).find("div").first()` — descendant PERTAMA, jadi blok
        // `div.item` itu sendiri tidak termasuk. `block` selalu dimulai di
        // `<` tag pembuka itu, jadi pencarian mulai dari indeks 1.
        let first_div = buka_tag(block, 1, "div").ok_or_else(gagal)?;
        let text = isi_tag(block, first_div, "div").ok_or_else(gagal)?;
        let version = setelah_label(&text, "Version:").ok_or_else(gagal)?;
        let info = setelah_label(&text, "Info:").ok_or_else(gagal)?;
        let (preview_at, _) = link_dengan_kelas(block, "font-preview").ok_or_else(gagal)?;
        let style = atribut(block, preview_at, "style").unwrap_or_default();
        let src = url_dari_style(&style).ok_or_else(gagal)?;
        let preview_url = link_dengan_kelas(block, "nf-oct-link_external")
            .and_then(|(_, href)| href);
        let download_url = link_dengan_kelas(block, "nf-fa-download")
            .and_then(|(_, href)| href);

        // `if (name && download_url)` — `download_url` undefined tidak lolos.
        if !name.is_empty()
            && let Some(download_url) = download_url
        {
            out.push(Font {
                name,
                version,
                info,
                preview_image: format!("https://www.nerdfonts.com{src}"),
                preview_url: preview_url.unwrap_or_default(),
                download_url,
            });
        }
    }
    Ok(out)
}

/// `TypeError: textContent.match(..) is not a function` — pesan yang sama
/// untuk keempat kegagalan, karena semuanya dari ekspresi yang sama.
fn gagal() -> String {
    "Cannot read properties of undefined (reading '1')".to_string()
}

/// Tag yang dibuka di `pos` punya kelas `class` di daftar kelasnya.
///
/// `class="item active"` juga cocok — cheerio memakai selector kelas, bukan
/// kecocokan string utuh. Atribut dibaca dari teks tag utuh supaya urutan
/// atribut tidak berpengaruh.
fn tag_punya_kelas(html: &str, pos: usize, class: &str) -> bool {
    let Some(open_end) = html[pos..].find('>') else {
        return false;
    };
    let open_end = pos + open_end + 1;
    let Some(tag_end) = html[open_end..].find('<') else {
        return false;
    };
    let tag = &html[pos..open_end + tag_end];
    let Some(mut at) = tag.find("class=\"") else {
        return false;
    };
    at += 7;
    let Some(close) = tag[at..].find('"') else {
        return false;
    };
    tag[at..at + close]
        .split_whitespace()
        .any(|k| k == class)
}

/// `nerdfont.js:71-92` — kartu interaktif beserta barisnya.
pub(crate) fn kartu(fonts: &[Font], prefix: &str) -> Reply {
    let rows: Vec<serde_json::Value> = fonts
        .iter()
        .map(|font| {
            let (header, title, description, id) = baris(font, prefix);
            json!({
                "header": header,
                "title": title,
                "description": description,
                "id": id,
            })
        })
        .collect();
    let params = json!({
        "title": "Pilih Font Disini",
        "sections": [{
            "title": "Aku harap, font ini dapat membantu kamu",
            "highlight_label": "Font Pilihan",
            "rows": rows,
        }],
    });

    Reply::interactive("Silahkan pilih font yang ingin kamu download")
        .with_footer("Click tombol di bawah ini")
        .with_button(InteractiveButton::new("single_select", params))
}

#[plugin(
    name = "nerdfont",
    category = "search",
    description = "Cari font di DaFont",
    usage = ".nerdfont <query>",
    example = ".nerdfont Coolvetica",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn nerdfont(ctx: Ctx) -> Reply {
    // `nerdfont.js` TIDAK pernah membaca `m.text`: parameter `<query>` di
    // `usage` cuma sisa. Permintaan tetap jalan untuk input kosong.
    let Ok(res) = ctx.http.get(URL) else {
        return Reply::text(crate::common::error_template(&ctx));
    };
    let html = res.text();

    match parse_fonts(&html) {
        Ok(fonts) => kartu(&fonts, &ctx.prefix),
        Err(_) => Reply::text(crate::common::error_template(&ctx)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        body: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn html(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Ok(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self
                .body
                .clone()
                .map_err(|()| oktz_plugin_api::HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
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
            command: "nerdfont".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("nerdfont").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const SEBUAH_ITEM: &str = r#"<div class="item">
      <span class="nerd-font-invisible-text">Coolvetica</span>
      <div>Version: 1.1.0
Info: A Font based on Font-A-Ware
Creator: Arthur Deng</div>
      <a class="font-preview" style="background-image: url('/preview/coolvetica.png')"></a>
      <a class="nf-oct-link_external" href="https://github.com/ryanoasis/nerd-fonts">github</a>
      <a class="nf-fa-download" href="/releases/Coolvetica.zip">download</a>
    </div>"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("nerdfont").unwrap();
        assert_eq!(p.meta.name, "nerdfont");
        assert!(p.meta.alias.is_empty(), "alias JS kosong");
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari font di DaFont");
        assert_eq!(p.meta.usage, ".nerdfont <query>");
        assert_eq!(p.meta.example, ".nerdfont Coolvetica");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn one_item_becomes_a_font_with_every_field() {
        let fonts = parse_fonts(SEBUAH_ITEM).expect("harus berhasil");
        assert_eq!(fonts.len(), 1);
        assert_eq!(fonts[0].name, "Coolvetica");
        assert_eq!(fonts[0].version, "1.1.0");
        assert_eq!(fonts[0].info, "A Font based on Font-A-Ware");
        assert_eq!(
            fonts[0].preview_image,
            "https://www.nerdfonts.com/preview/coolvetica.png"
        );
        assert_eq!(fonts[0].preview_url, "https://github.com/ryanoasis/nerd-fonts");
        assert_eq!(fonts[0].download_url, "/releases/Coolvetica.zip");
    }

    #[test]
    fn the_prefix_is_added_to_the_background_url_only() {
        let fonts = parse_fonts(SEBUAH_ITEM).unwrap();
        assert!(fonts[0].preview_image.starts_with("https://www.nerdfonts.com/"));
        assert!(
            !fonts[0].preview_image.contains("https://www.nerdfonts.comhttps"),
            "path absolut tidak digandakan"
        );
    }

    #[test]
    fn a_quoted_style_url_is_still_extracted() {
        // `&quot;` di HTML adalah hasil entity-decode; cheerio melihatnya
        // sebagai `"`, sementara parser manual di sini tidak meng-decode
        // entity. Yang dijamin adalah kedua bentuk kutip yang ditulis langsung.
        assert_eq!(
            url_dari_style(r#"background-image: url('/p.png')"#),
            Some("/p.png".to_string())
        );
        assert_eq!(
            url_dari_style(r#"background-image:url("/p.png")"#),
            Some("/p.png".to_string())
        );
        assert_eq!(
            url_dari_style("BACKGROUND-IMAGE:   URL( '/p.png' )"),
            Some("/p.png".to_string()),
            "pola JS memakai flag `i` dan mengizinkan spasi"
        );
        assert!(url_dari_style("background-color: red").is_none());
        assert!(url_dari_style("").is_none());
    }

    #[test]
    fn two_items_are_read_in_document_order() {
        let html = format!("{SEBUAH_ITEM}\n{SEBUAH_ITEM}");
        let fonts = parse_fonts(&html).unwrap();
        assert_eq!(fonts.len(), 2);
        assert_eq!(fonts[1].name, "Coolvetica");
    }

    #[test]
    fn a_div_without_the_item_class_is_skipped() {
        let html = r#"<div class="lain">
          <span class="nerd-font-invisible-text">Bukan font</span>
          <div>Version: 1
Info: Y</div>
        </div>"#;
        assert!(parse_fonts(html).unwrap().is_empty());
    }

    #[test]
    fn an_item_without_a_download_link_is_dropped_not_fatal() {
        let html = r#"<div class="item">
      <span class="nerd-font-invisible-text">Tanpa Link</span>
      <div>Version: 1
Info: Y</div>
      <a class="font-preview" style="background-image: url('/p.png')"></a>
    </div>"#;
        assert!(parse_fonts(html).unwrap().is_empty());
    }

    #[test]
    fn an_item_without_a_name_is_dropped() {
        let html = r#"<div class="item">
      <div>Version: 1
Info: Y</div>
      <a class="font-preview" style="background-image: url('/p.png')"></a>
      <a class="nf-fa-download" href="/d.zip"></a>
    </div>"#;
        assert!(parse_fonts(html).unwrap().is_empty());
    }

    #[test]
    fn a_missing_version_label_kills_the_whole_page_like_the_javascript_match() {
        // `textContent.match(/Version:…/)[1]` = `TypeError` di JS, jadi
        // `nerdfonts()` melempar dan handler menjawab template error.
        let html = r#"<div class="item">
      <span class="nerd-font-invisible-text">X</span>
      <div>Info: Y</div>
      <a class="font-preview" style="background-image: url('/p.png')"></a>
      <a class="nf-fa-download" href="/d.zip"></a>
    </div>"#;
        assert!(parse_fonts(html).is_err());
        let r = run(Stub::html(html), "");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "nerdfont"));
    }

    #[test]
    fn a_missing_info_label_also_kills_the_page() {
        let html = r#"<div class="item">
      <span class="nerd-font-invisible-text">X</span>
      <div>Version: 1</div>
      <a class="font-preview" style="background-image: url('/p.png')"></a>
      <a class="nf-fa-download" href="/d.zip"></a>
    </div>"#;
        assert!(parse_fonts(html).is_err());
    }

    #[test]
    fn a_missing_preview_style_kills_the_page() {
        let html = r#"<div class="item">
      <span class="nerd-font-invisible-text">X</span>
      <div>Version: 1
Info: Y</div>
      <a class="nf-fa-download" href="/d.zip"></a>
    </div>"#;
        assert!(parse_fonts(html).is_err());
    }

    #[test]
    fn the_label_capture_stops_at_the_line_break() {
        assert_eq!(setelah_label("Version: 1.1.0\nInfo: X", "Version:").unwrap(), "1.1.0");
        assert_eq!(setelah_label("Version:  2.0  \r\nInfo: X", "Version:").unwrap(), "2.0");
        assert_eq!(setelah_label("Version:\nInfo: X", "Version:").unwrap(), "");
        assert_eq!(setelah_label("Info saja", "Version:"), None);
    }

    #[test]
    fn empty_and_broken_pages_do_not_panic() {
        for html in ["", "<html></html>", "<div class=\"item\"></div>", "<div class=\"item\">"] {
            let _ = parse_fonts(html);
            let r = run(Stub::html(html), "");
            assert!(!text_of(&r).is_empty(), "html {html:?}");
        }
    }

    #[test]
    fn a_huge_page_still_parses_without_panicking() {
        let html = SEBUAH_ITEM.repeat(500);
        let fonts = parse_fonts(&html).unwrap();
        assert_eq!(fonts.len(), 500);
        let r = kartu(&fonts, ".");
        assert_eq!(text_of(&r), "Silahkan pilih font yang ingin kamu download");
        let Reply::Interactive { buttons, .. } = r.without_mutations() else {
            panic!("harus kartu interaktif");
        };
        assert_eq!(buttons.len(), 1);
        let rows = buttons[0].params["sections"][0]["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 500);
        assert_eq!(rows[499]["id"], ".nerdfont-ambil Coolvetica");
    }

    #[test]
    fn the_card_carries_the_button_rows_in_order() {
        let fonts = parse_fonts(SEBUAH_ITEM).unwrap();
        let r = kartu(&fonts, "!");
        let Reply::Interactive {
            body,
            footer,
            buttons,
        } = r.without_mutations()
        else {
            panic!("harus kartu interaktif");
        };
        assert_eq!(body, "Silahkan pilih font yang ingin kamu download");
        assert_eq!(footer.as_deref(), Some("Click tombol di bawah ini"));
        assert_eq!(buttons[0].name, "single_select");
        assert_eq!(buttons[0].params["title"], "Pilih Font Disini");
        assert_eq!(
            buttons[0].params["sections"][0]["highlight_label"],
            "Font Pilihan"
        );
        assert_eq!(
            buttons[0].params["sections"][0]["rows"],
            json!([{
                "header": "Font Coolvetica",
                "title": "A Font based on Font-A-Ware",
                "description": "Version: 1.1.0",
                "id": "!nerdfont-ambil Coolvetica",
            }])
        );
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "nerdfont"));
        let seen = Stub::offline();
        let _ = run(seen.clone(), "Coolvetica");
        assert_eq!(seen.seen.lock().unwrap()[0], URL);
    }

    #[test]
    fn odd_and_very_long_inputs_still_fetch_the_page() {
        let http = Stub::html(SEBUAH_ITEM);
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", &"x".repeat(5000)] {
            let r = run(http.clone(), input);
            assert!(text_of(&r).contains("Silahkan pilih font"), "input {input:?}");
        }
        assert_eq!(http.seen.lock().unwrap().len(), 6, "input diabaikan, bukan guard");
    }
}