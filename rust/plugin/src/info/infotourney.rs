//! `plugins/info/infotourney.js` — info turnamen Mobile Legends dari infotourney.com.
//!
//! `ponytail:` JS memakai `cheerio` untuk parse `.items-row .item`, `h2[itemprop="name"] a`,
//! `p img`, `time[itemprop="datePublished"]`, dan `p[style="text-align: center;"]`.
//! `oktz-plugins` tidak punya parser HTML, jadi semua dipakai manual di bawah.
//!
//! `ponytail:` `timeHelper.fromTimestamp(datePublished, "DD/MM/YYYY HH:mm")` — tanpa
//! crate tanggal, string datetime dikembalikan apa adanya kalau tidak bisa di-parse.
//!
//! `ponytail:` `sock.sendMedia(..., { type: 'image' })` menjadi
//! `Reply::image(MediaSource::Url)` dengan caption. URL di-fetch pipeline saat kirim.
//!
//! `ponytail:` `m.react` dua kali (`🕕` lalu `✅`/`❌`) — hanya reaksi akhir yang terkirim.
//!
//! Catatan: baris `pub mod infotourney;` perlu ditambahkan ke
//! `rust/crates/oktz-plugins/src/info/mod.rs` oleh orchestrator (tidak boleh
//! disentuh agent porting per aturan PORT-BRIEF #2).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

/// Satu turnamen dari halaman infotourney.
#[derive(Clone, Debug)]
struct Tournament {
    title: String,
    image_url: String,
    date_published: String,
    description: String,
    info: String,
    url: String,
}

/// Ambil halaman dan parse semua turnamen.
fn fetch_tournaments(http: &dyn oktz_plugin_api::HttpGet) -> Result<Vec<Tournament>, String> {
    let res = http
        .get("https://infotourney.com/tournament/mobile-legends")
        .map_err(|e| format!("Gagal mengambil halaman: {}", e.message))?;
    let html = res.text();
    Ok(parse_tournaments(&html, "https://infotourney.com/tournament/mobile-legends"))
}

/// Parse `.items-row .item` dari HTML.
fn parse_tournaments(html: &str, base_url: &str) -> Vec<Tournament> {
    let mut turnamen = Vec::new();
    // Cari setiap `.item` di dalam `.items-row`
    let mut i = 0;
    while i < html.len() {
        let Some(item_pos) = html[i..].find("class=\"item\"") else { break };
        let abs = i + item_pos;
        // Cari tag pembuka div.item
        let tag_start = html[..abs].rfind('<').unwrap_or(abs);
        let tag_end = match html[abs..].find('>') {
            Some(e) => abs + e + 1,
            None => break,
        };
        // Cari penutup div.item
        let after = &html[tag_end..];
        let close_pos = after.find("</div>").unwrap_or(after.len());
        let akhir = (tag_end + close_pos + 6).min(html.len());
        let item_html = &html[tag_start..akhir];

        let title = tag_text(item_html, "a").unwrap_or_default();
        let link = attr_href(item_html).unwrap_or_default();
        let image = attr_src(item_html).unwrap_or_default();
        let date_raw = attr_value(item_html, "datetime").unwrap_or_default();

        let date_published = if !date_raw.is_empty() {
            format_tanggal(&date_raw)
        } else {
            String::new()
        };

        let description_html = tag_inner_html(item_html, "p[style=\"text-align: center;\"]");
        let (description, info) = split_description(&description_html);

        let image_url = resolve_url(&image, base_url);
        let url = resolve_url(&link, base_url);

        if !title.is_empty() {
            turnamen.push(Tournament {
                title,
                image_url,
                date_published,
                description,
                info,
                url,
            });
        }
        i = akhir;
    }
    turnamen
}

/// Nilai atribut `nama` dari kemunculan pertama di HTML.
fn attr_value(html: &str, nama: &str) -> Option<String> {
    let pola = format!("{nama}=\"");
    let pos = html.find(&pola)?;
    let mulai = pos + pola.len();
    let akhir = html[mulai..].find('"')?;
    Some(html[mulai..mulai + akhir].to_string())
}

/// Nilai `href` dari atribut pertama.
fn attr_href(html: &str) -> Option<String> {
    attr_value(html, "href")
}

/// Nilai `src` dari atribut pertama.
fn attr_src(html: &str) -> Option<String> {
    attr_value(html, "src")
}

/// Teks di dalam tag pertama bernama `nama` (tanpa tag pembuka ber-atribut).
fn tag_text(html: &str, nama: &str) -> Option<String> {
    let open = format!("<{nama}");
    let mut start = 0;
    while let Some(pos) = html[start..].find(&open) {
        let abs = start + pos;
        let sisa = &html[abs + open.len()..];
        if (sisa.starts_with('>') || sisa.starts_with(' '))
            && let Some(gt) = html[abs..].find('>')
        {
            let isi = abs + gt + 1;
            let close = format!("</{nama}>");
            if let Some(cs) = html[isi..].find(&close) {
                return Some(html[isi..isi + cs].to_string());
            }
        }
        start = abs + open.len();
    }
    None
}

/// Isi innerHTML dari tag pertama yang cocok.
fn tag_inner_html(html: &str, _selector: &str) -> String {
    // Cari <p style="..."> ... </p>
    let pola = r#"<p style="text-align: center;">"#;
    let Some(start) = html.find(pola) else {
        return String::new();
    };
    let after = start + pola.len();
    let Some(end) = html[after..].find("</p>") else {
        return String::new();
    };
    html[after..after + end].to_string()
}

/// Split deskripsi HTML pada `<br>`, ganti `&nbsp;` dengan spasi.
fn split_description(html: &str) -> (String, String) {
    let decoded = decode_html_entities(html);
    let parts: Vec<&str> = decoded.split("<br>").collect();
    let description = parts.first().map(|s| s.trim().to_string()).unwrap_or_default();
    let info = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
    (description, info)
}

/// Decode entitas HTML yang lazim.
fn decode_html_entities(raw: &str) -> String {
    raw.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// Resolve URL relatif terhadap base_url.
fn resolve_url(url: &str, base_url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    if url.starts_with("//") {
        return format!("https:{url}");
    }
    if url.starts_with('/') {
        if let Some(base) = base_url.split("://").nth(1) {
            let host = base.split('/').next().unwrap_or(base);
            return format!("https://{host}{url}");
        }
        return url.to_string();
    }
    url.to_string()
}

/// Format tanggal dari string datetime. Tanpa crate tanggal, coba parsing sederhana
/// atau balik apa adanya.
fn format_tanggal(date_str: &str) -> String {
    if date_str.is_empty() {
        return String::new();
    }
    // Coba parse ISO 8601 sederhana: "2024-01-01T12:00:00" atau "2024-01-01 12:00:00"
    let s = date_str.trim();
    if let Some(pos) = s.find('T') {
        let date_part = &s[..pos];
        let time_part = &s[pos + 1..];
        let time_part = time_part.split('.').next().unwrap_or(time_part);
        let time_part = time_part.split('+').next().unwrap_or(time_part);
        if date_part.contains('-') {
            // Sudah format tanggal, gabungkan
            return format!("{} {}", date_part, time_part);
        }
    }
    s.to_string()
}

#[plugin(
    name = "infotourney",
    alias = ["tourney", "turnamen", "mltourney"],
    category = "info",
    description = "Info turnamen Mobile Legends terbaru",
    usage = ".infotourney",
    example = ".infotourney",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn infotourney(ctx: Ctx) -> Reply {
    let turnamen = match fetch_tournaments(ctx.http.as_ref()) {
        Ok(t) => t,
        Err(_) => {
            return Reply::text("❌ Tidak ada turnamen yang ditemukan");
        }
    };

    if turnamen.is_empty() {
        return Reply::text("❌ Tidak ada turnamen yang ditemukan");
    }

    let tampil = turnamen.iter().take(5).collect::<Vec<_>>();

    let mut text = "🏆 *ɪɴꜰᴏ ᴛᴜʀɴᴀᴍᴇɴ ᴍᴏʙɪʟᴇ ʟᴇɢᴇɴᴅs*\n\n> 5 Turnamen Terbaru\n\n".to_string();
    for (i, t) in tampil.iter().enumerate() {
        text.push_str(&format!("{}. *{}*\n", i + 1, t.title));
        if !t.date_published.is_empty() {
            text.push_str(&format!("📅 {}\n", t.date_published));
        } else {
            text.push_str("📅 N/A\n");
        }
        if !t.description.is_empty() {
            text.push_str(&format!("📝 {}\n", t.description));
        }
        if !t.info.is_empty() {
            text.push_str(&format!("⚠️ {}\n", t.info));
        }
        text.push_str(&format!("🔗 {}\n\n", t.url));
    }

    if let Some(first) = tampil.first()
        && !first.image_url.is_empty()
    {
        return Reply::image(MediaSource::url(first.image_url.clone()), text)
            .with_reaction("✅");
    }

    Reply::text(text).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::Value;
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

    fn run(http: Arc<StubHttp>) -> Reply {
        let ctx = Ctx {
            text: String::new(),
            args: Vec::new(),
            sender: SENDER.into(),
            chat: SENDER.into(),
            prefix: ".".into(),
            command: "infotourney".into(),
            http,
            ..Ctx::default()
        };
        infotourney(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const HTML: &str = r#"<div class="items-row">
  <div class="item">
    <h2 itemprop="name"><a href="/turnamen/ml1">Turnamen ML 1</a></h2>
    <p><img src="/img/ml1.jpg"></p>
    <time itemprop="datePublished" datetime="2024-01-15T10:00:00"></time>
    <p style="text-align: center;">Deskripsi 1<br>Info 1</p>
  </div>
  <div class="item">
    <h2 itemprop="name"><a href="/turnamen/ml2">Turnamen ML 2</a></h2>
    <p><img src="/img/ml2.jpg"></p>
    <time itemprop="datePublished" datetime="2024-01-16T14:30:00"></time>
    <p style="text-align: center;">Deskripsi 2<br>Info 2</p>
  </div>
</div>"#;

    #[test]
    fn parse_tournaments_ekstrak_dua() {
        let turnamen = parse_tournaments(HTML, "https://infotourney.com/tournament/mobile-legends");
        assert_eq!(turnamen.len(), 2);
        assert_eq!(turnamen[0].title, "Turnamen ML 1");
        assert_eq!(turnamen[0].image_url, "https://infotourney.com/img/ml1.jpg");
        assert_eq!(turnamen[0].url, "https://infotourney.com/turnamen/ml1");
        assert_eq!(turnamen[1].title, "Turnamen ML 2");
    }

    #[test]
    fn deskripsi_di_split_pada_br() {
        let (desc, info) = split_description(r#"Deskripsi 1<br>Info 1"#);
        assert_eq!(desc, "Deskripsi 1");
        assert_eq!(info, "Info 1");
    }

    #[test]
    fn nbsp_diganti_spasi() {
        assert_eq!(
            decode_html_entities("a&nbsp;b"),
            "a b"
        );
    }

    #[test]
    fn url_relatif_diresolve() {
        assert_eq!(
            resolve_url("/img/x.jpg", "https://infotourney.com/tournament/mobile-legends"),
            "https://infotourney.com/img/x.jpg"
        );
        assert_eq!(
            resolve_url("//cdn.com/x.jpg", "https://infotourney.com/"),
            "https://cdn.com/x.jpg"
        );
    }

    #[test]
    fn tanggal_diformat_sederhana() {
        assert_eq!(
            format_tanggal("2024-01-15T10:00:00"),
            "2024-01-15 10:00:00"
        );
    }

    #[test]
    fn halaman_kosong_menampilkan_tidak_ada() {
        let r = run(StubHttp::body("<html></html>"));
        assert_eq!(text_of(&r), "❌ Tidak ada turnamen yang ditemukan");
    }

    #[test]
    fn jaringan_gagal_menampilkan_pesan() {
        let r = run(StubHttp::offline());
        assert_eq!(text_of(&r), "❌ Tidak ada turnamen yang ditemukan");
    }

    #[test]
    fn turnamen_dengan_gambar_mengembalikan_image_reply() {
        let http = StubHttp::body(HTML);
        let r = run(http);
        match r.without_mutations().tanpa_reaksi() {
            Reply::Image { media, caption, .. } => {
                let src = media.as_ref().expect("harus ada media");
                assert_eq!(src, &MediaSource::url("https://infotourney.com/img/ml1.jpg"));
                assert!(caption.contains("Turnamen ML 1"));
            }
            other => panic!("harus Image, dapat {other:?}"),
        }
    }
}