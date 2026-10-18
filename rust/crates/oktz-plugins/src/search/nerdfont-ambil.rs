//! `plugins/search/nerdfont-ambil.js` — unduh satu font Nerd Font.
//!
//! Parser HTML-nya identik dengan `search::nerdfont` (`nerdfonts()` disalin
//! apa adanya di kedua file JS), jadi helper-nya dipinjam dari sana.
//!
//! `ponytail:` `sharp` tidak ada di API plugin: `jpegThumbnail` hasil
//! `.resize(50, 50)` di `nerdfont-ambil.js:78-86` tidak ikut. Sisanya
//! (dokumen zip + caption) ada padanannya di `Reply::Document`.
//!
//! `ponytail:` `sock.sendMessage` di `:74` tidak di-`await`, jadi handler JS
//! selalu selesai tanpa balasan apa pun; di sini dokumennya dikembalikan
//! sebagai balasan karena itu satu-satunya jalanpipeline untuk mengirim media.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::search::nerdfont::{Font, URL, parse_fonts};

/// `nerdfont-ambil.js:67` — guard nama font kosong.
pub(crate) fn butuh_nama() -> String {
    "*NERD FONT*\n\n> Masukan nama font yang ingin didownload".to_string()
}

/// `nerdfont-ambil.js:73` — nama tidak ada di daftar.
pub(crate) fn tidak_ditemukan(query: &str) -> String {
    format!("❌ Font *{query}* tidak ditemukan")
}

/// `nerdfont-ambil.js:87-88` — caption dokumen.
pub(crate) fn caption(prefix: &str) -> String {
    format!("*Done*\nJika kamu ingin mendownload lagi, ketik {prefix}nerdfont lagi")
}

/// `res.find((d) => d?.name.toLowerCase() === query.toLowerCase())` (`:70`) —
/// `find` mengembalikan item PERTAMA, bukan yang terbaik.
pub(crate) fn cari<'a>(fonts: &'a [Font], query: &str) -> Option<&'a Font> {
    let q = query.to_lowercase();
    fonts
        .iter()
        .find(|f| f.name.to_lowercase() == q)
}

#[plugin(
    name = "nerdfont-ambil",
    alias = ["dafont-ambil", "fontambil"],
    category = "search",
    description = "Cari font di DaFont",
    usage = ".nerdfont-ambil <query>",
    example = ".nerdfont-ambil Coolvetica",
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
pub fn nerdfont_ambil(ctx: Ctx) -> Reply {
    // `m.text?.trim()?.toLowerCase()` (`:65`) — pesan "tidak ditemukan" ikut
    // memuat versi kecil itu, bukan input mentah.
    let query = ctx.text.trim().to_lowercase();

    if query.is_empty() {
        return Reply::text(butuh_nama());
    }

    let Ok(res) = ctx.http.get(URL) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command));
    };
    let html = res.text();

    // `parse_fonts` `Err` = `TypeError` dari `match(..)[1]`, sama seperti di JS.
    let fonts = match parse_fonts(&html) {
        Ok(f) => f,
        Err(_) => return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)),
    };

    let Some(font) = cari(&fonts, &query) else {
        return Reply::text(tidak_ditemukan(&query));
    };

    // `document: { url }, fileName: data.name, mimetype: "application/zip"`.
    // `jpegThumbnail` butuh `sharp` — tidak ada di API plugin.
    Reply::Document {
        media: Some(
            MediaSource::url(&font.download_url).with_mimetype("application/zip"),
        ),
        file_name: font.name.clone(),
        caption: caption(&ctx.prefix),
        mentions: Vec::new(),
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
            command: "nerdfont-ambil".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("nerdfont-ambil").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const HALAMAN: &str = r#"<div class="item">
      <span class="nerd-font-invisible-text">Coolvetica</span>
      <div>Version: 1.1.0
Info: A Font based on Font-A-Ware</div>
      <a class="font-preview" style="background-image: url('/preview/coolvetica.png')"></a>
      <a class="nf-oct-link_external" href="https://github.com/ryanoasis/nerd-fonts">g</a>
      <a class="nf-fa-download" href="/releases/Coolvetica.zip">d</a>
    </div>
    <div class="item">
      <span class="nerd-font-invisible-text">Hack Nerd Font</span>
      <div>Version: 3.3.0
Info: A patched font</div>
      <a class="font-preview" style="background-image: url('/preview/hack.png')"></a>
      <a class="nf-fa-download" href="/releases/Hack.zip">d</a>
    </div>"#;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("nerdfont-ambil").unwrap();
        assert_eq!(p.meta.name, "nerdfont-ambil");
        assert_eq!(p.meta.alias, ["dafont-ambil", "fontambil"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari font di DaFont");
        assert_eq!(p.meta.usage, ".nerdfont-ambil <query>");
        assert_eq!(p.meta.example, ".nerdfont-ambil Coolvetica");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["dafont-ambil", "fontambil"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "nerdfont-ambil"
            );
        }
    }

    #[test]
    fn an_empty_name_asks_for_one_without_fetching() {
        let http = Stub::offline();
        for text in ["", "   ", "\t\n"] {
            assert_eq!(
                text_of(&run(http.clone(), text)),
                "*NERD FONT*\n\n> Masukan nama font yang ingin didownload",
                "input {text:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn an_exact_name_sends_the_zip_document() {
        let r = run(Stub::html(HALAMAN), "Coolvetica");
        let inner = r.without_mutations();
        match inner {
            Reply::Document {
                media,
                file_name,
                caption,
                ..
            } => {
                assert_eq!(file_name, "Coolvetica");
                assert_eq!(
                    caption,
                    "*Done*\nJika kamu ingin mendownload lagi, ketik .nerdfont lagi"
                );
                let src = media.as_ref().expect("media harus ada");
                assert_eq!(src.mimetype(), Some("application/zip"));
                match src {
                    MediaSource::Url { url, .. } => {
                        assert_eq!(url, "/releases/Coolvetica.zip")
                    }
                    other => panic!("harus Url, dapat {other:?}"),
                }
            }
            other => panic!("harus Document, dapat {other:?}"),
        }
    }

    #[test]
    fn the_name_match_ignores_case_on_both_sides() {
        let fonts = parse_fonts(HALAMAN).unwrap();
        assert_eq!(cari(&fonts, "coolvetica").unwrap().name, "Coolvetica");
        assert_eq!(cari(&fonts, "COOLVETICA").unwrap().name, "Coolvetica");
        assert_eq!(cari(&fonts, "Hack Nerd Font").unwrap().name, "Hack Nerd Font");
        assert!(cari(&fonts, "coolv").is_none(), "pencocokan persis, bukan sebagian");
        assert!(cari(&fonts, "coolvetica ").is_none(), "spasi excess tidak dipangkas");
        assert!(cari(&fonts, "").is_none());
    }

    #[test]
    fn an_unknown_name_reports_it_in_lowercase() {
        let r = run(Stub::html(HALAMAN), "TIDAK ADA");
        assert_eq!(text_of(&r), "❌ Font *tidak ada* tidak ditemukan");
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "Coolvetica");
        assert_eq!(text_of(&r), crate::common::error_template(".", "nerdfont-ambil"));
    }

    #[test]
    fn a_broken_page_reports_the_error_template_instead_of_panicking() {
        // Tanpa `Version:` -> `match(..)[1]` melempar di JS.
        let rusak = r#"<div class="item">
      <span class="nerd-font-invisible-text">X</span>
      <div>Info: Y</div>
      <a class="nf-fa-download" href="/d.zip"></a>
    </div>"#;
        let r = run(Stub::html(rusak), "X");
        assert_eq!(text_of(&r), crate::common::error_template(".", "nerdfont-ambil"));
    }

    #[test]
    fn empty_and_hostile_pages_do_not_panic() {
        for html in ["", "<html></html>", "<div class=\"item\"></div>", "<div class=\"item\">"] {
            let r = run(Stub::html(html), "apa saja");
            assert!(!text_of(&r).is_empty(), "html {html:?}");
        }
    }

    #[test]
    fn odd_and_very_long_names_do_not_panic() {
        for input in [
            "<b>", "a|b", "🎨", "'\"", "a\u{202e}b", "Coolvetica\u{0}",
            &"x".repeat(5000),
        ] {
            let r = run(Stub::html(HALAMAN), input);
            assert!(!text_of(&r).is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn the_caption_follows_the_prefix_from_the_context() {
        assert_eq!(
            caption("!"),
            "*Done*\nJika kamu ingin mendownload lagi, ketik !nerdfont lagi"
        );
    }
}