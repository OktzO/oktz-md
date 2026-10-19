//! `plugins/search/soundcloud.js` — cari lagu di SoundCloud lewat m.soundcloud.com.
//!
//! `ponytail:` JS memakai `cheerio` untuk ambil teks `#__NEXT_DATA__` lalu
//! `JSON.parse`. `oktz-plugins` tidak punya parser HTML, jadi elemen
//! `#__NEXT_DATA__` dicari manual dan isi teksnya di-parse sebagai JSON.
//!
//! `ponytail:` `new URL(image, url).href` untuk resolve gambar — di sini
//! URL relatif diselesaikan terhadap base.
//!
//! `ponytail:` `sock.sendMedia(..., { type: 'image' })` menjadi
//! `Reply::image(MediaSource::Url)` dengan caption.
//!
//! `ponytail:` `m.react` dua kali (`🕕` lalu `✅`) — hanya reaksi akhir.
//!
//! Catatan: baris `pub mod soundcloud;` perlu ditambahkan ke
//! `rust/crates/oktz-plugins/src/search/mod.rs` oleh orchestrator (tidak boleh
//! disentuh agent porting per aturan PORT-BRIEF #2).

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

/// Satu hasil track dari SoundCloud.
#[derive(Clone, Debug)]
struct Track {
    title: String,
    url: String,
    artwork: Option<String>,
    plays: String,
    likes: String,
    comments: String,
    reposts: String,
}

/// Cari lagu: ambil HTML, ekstraksi #__NEXT_DATA__, parse JSON, map tracks.
fn sc_search(http: &dyn oktz_plugin_api::HttpGet, query: &str) -> Result<Vec<Track>, String> {
    let url = format!("https://m.soundcloud.com/search?q={}", encode_query(query));
    let res = http
        .get(&url)
        .map_err(|e| format!("Gagal mencari: {}", e.message))?;
    let html = res.text();
    let json_text = extract_next_data(&html);
    if json_text.is_empty() {
        return Ok(Vec::new());
    }
    let json: Value = serde_json::from_str(&json_text).map_err(|e| format!("JSON gagal: {e}"))?;
    let tracks = json
        .get("props")
        .and_then(|v| v.get("pageProps"))
        .and_then(|v| v.get("initialStoreState"))
        .and_then(|v| v.get("entities"))
        .and_then(|v| v.get("tracks"))
        .and_then(|v| v.as_object())
        .map_or(Vec::new(), |obj| {
            obj.values()
                .filter_map(track_from_value)
                .filter(|t| !t.title.is_empty())
                .collect()
        });
    Ok(tracks)
}

/// Ekstraksi isi teks dari elemen `#__NEXT_DATA__`.
fn extract_next_data(html: &str) -> String {
    let pola = r#"id="__NEXT_DATA__""#;
    // Kemunculan pertama saja — JS `cheerio` mengambil elemen pertama.
    if let Some(abs) = html.find(pola) {
        // Cari `>` setelah atribut
        let after = &html[abs + pola.len()..];
        let gt = match after.find('>') {
            Some(g) => g,
            None => return String::new(),
        };
        let content_start = abs + pola.len() + gt + 1;
        let close = r#"</script>"#;
        if let Some(cs) = html[content_start..].find(close) {
            return html[content_start..content_start + cs].to_string();
        }
        return html[content_start..].to_string();
    }
    String::new()
}

/// Map satu objek track JSON ke struct Track.
fn track_from_value(v: &Value) -> Option<Track> {
    let data = v.get("data")?;
    let title = data.get("title").and_then(Value::as_str).unwrap_or("-").to_string();
    let url = data.get("permalink_url").and_then(Value::as_str).unwrap_or("-").to_string();
    let artwork = data.get("artwork_url").and_then(Value::as_str).map(|s| s.to_string());
    let plays = data.get("playback_count").and_then(Value::as_u64).map_or("-".to_string(), |n| n.to_string());
    let likes = data.get("likes_count").and_then(Value::as_u64).map_or("-".to_string(), |n| n.to_string());
    let comments = data.get("comment_count").and_then(Value::as_u64).map_or("-".to_string(), |n| n.to_string());
    let reposts = data.get("reposts_count").and_then(Value::as_u64).map_or("-".to_string(), |n| n.to_string());

    Some(Track { title, url, artwork, plays, likes, comments, reposts })
}

/// Encode query parameter seperti `encodeURIComponent` JS.
fn encode_query(q: &str) -> String {
    let mut out = String::with_capacity(q.len());
    for b in q.bytes() {
        let c = b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')') {
            out.push(c);
        } else if c == ' ' {
            out.push_str("%20");
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Resolve URL relatif terhadap base URL.
fn resolve_url(url: &str, base_url: &str) -> String {
    if url.is_empty() {
        return String::new();
    }
    if url.starts_with("http://") || url.starts_with("https://") {
        return url.to_string();
    }
    if url.starts_with("//") {
        return format!("https:{url}");
    }
    if url.starts_with('/') {
        let host = base_url.split("://").nth(1).and_then(|b| b.split('/').next()).unwrap_or("");
        return format!("https://{host}{url}");
    }
    url.to_string()
}

fn help(prefix: &str) -> String {
    format!(
        "🎵 *SOUNDCLOUD SEARCH* 🎵\n\nHalo kak! Mau cari lagu apa hari ini?\n\n*Cara Pakai:*\n👉 `{prefix}soundcloud <judul lagu>`\n\n*Contoh:*\n`{prefix}soundcloud Only We Know`"
    )
}

#[plugin(
    name = "soundcloud",
    alias = ["scsearch", "scs"],
    category = "search",
    description = "Cari lagu di SoundCloud",
    usage = ".soundcloud <judul>",
    example = ".soundcloud Only We Know",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn soundcloud(ctx: Ctx) -> Reply {
    if ctx.args.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let query = ctx.args.join(" ");
    let tracks = match sc_search(ctx.http.as_ref(), &query) {
        Ok(t) => t,
        Err(_) => {
            return Reply::text("❌ Maaf kak, terjadi kesalahan sistem!");
        }
    };

    if tracks.is_empty() {
        return Reply::text("❌ Aduh kak, lagunya nggak ketemu nih! Coba cari dengan judul yang beda ya. 😭");
    }

    let thumb = tracks.iter().find(|t| t.artwork.is_some()).and_then(|t| t.artwork.as_deref());
    let mut txt = "🎧 *HASIL PENCARIAN SOUNDCLOUD* 🎧\n\n".to_string();
    let mut content_txt = String::new();
    let limit = tracks.len().min(5);
    for (i, d) in tracks.iter().enumerate().take(limit) {
        content_txt.push_str(&format!("🎵 *Title :* {}\n", d.title));
        content_txt.push_str(&format!("🔗 *Url :* {}\n", d.url));
        content_txt.push_str(&format!("👁️ *Views :* {}\n", d.plays));
        content_txt.push_str(&format!("❤️ *Likes :* {}\n", d.likes));
        content_txt.push_str(&format!("💬 *Comments :* {}\n", d.comments));
        content_txt.push_str(&format!("🔁 *Reposts :* {}", d.reposts));
        if i < limit - 1 {
            content_txt.push_str("\n\n");
        }
    }
    // Trim baris kosong seperti JS `.split("\n").map(line => line.trim() ? line : "").join("\n")`
    let lines: Vec<&str> = content_txt.split('\n').filter(|l| !l.is_empty()).collect();
    let content_txt = lines.join("\n");
    txt.push_str(&content_txt);
    txt.push_str("\n\n");
    txt.push_str(&format!("Kalo mau download lagunya, pake fitur `{}`playsc aja kak! 😉", ctx.prefix));

    let base_url = "https://m.soundcloud.com";
    if let Some(thumb) = thumb {
        let thumb_url = resolve_url(thumb, base_url);
        return Reply::image(MediaSource::url(thumb_url), txt.trim().to_string()).with_reaction("✅");
    }

    Reply::text(txt.trim().to_string()).with_reaction("✅")
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
            command: "soundcloud".into(),
            http,
            ..Ctx::default()
        };
        soundcloud(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    const HTML_WITH_DATA: &str = r#"<html><body><script id="__NEXT_DATA__" type="application/json">
{"props":{"pageProps":{"initialStoreState":{"entities":{"tracks":{"1":{"data":{"id":1,"title":"Lagu Satu","permalink_url":"https://soundcloud.com/1","user_id":100,"artwork_url":"https://a/1.jpg","duration":215000,"playback_count":4321,"likes_count":10,"comment_count":5,"reposts_count":2,"created_at":"2024-01-01"}},"2":{"data":{"id":2,"title":"Lagu Dua","permalink_url":"https://soundcloud.com/2","user_id":200,"artwork_url":null,"duration":180000,"playback_count":100,"likes_count":5,"comment_count":0,"reposts_count":0,"created_at":"2024-01-02"}}}}}}}}</script></body></html>"#;

    const HTML_WITHOUT_DATA: &str = "<html><body>tidak ada data</body></html>";

    #[test]
    fn extract_next_data_mengambil_json() {
        let json = extract_next_data(HTML_WITH_DATA);
        assert!(json.contains("\"title\":\"Lagu Satu\""));
        assert!(json.contains("\"title\":\"Lagu Dua\""));
    }

    #[test]
    fn extract_next_data_kosong_kalau_tidak_ada() {
        assert_eq!(extract_next_data(HTML_WITHOUT_DATA), "");
    }

    #[test]
    fn track_from_value_ekstrak_semua_field() {
        let json = serde_json::json!({
            "data": {
                "id": 1,
                "title": "Lagu Satu",
                "permalink_url": "https://soundcloud.com/1",
                "user_id": 100,
                "artwork_url": "https://a/1.jpg",
                "duration": 215000,
                "playback_count": 4321,
                "likes_count": 10,
                "comment_count": 5,
                "reposts_count": 2,
                "created_at": "2024-01-01"
            }
        });
        let t = track_from_value(&json).unwrap();
        assert_eq!(t.title, "Lagu Satu");
        assert_eq!(t.artwork, Some("https://a/1.jpg".to_string()));
        assert_eq!(t.plays, "4321");
    }

    #[test]
    fn track_dengan_artwork_null() {
        let json = serde_json::json!({
            "data": {
                "id": 2,
                "title": "Lagu Dua",
                "permalink_url": "https://soundcloud.com/2",
                "artwork_url": null
            }
        });
        let t = track_from_value(&json).unwrap();
        assert_eq!(t.artwork, None);
    }

    #[test]
    fn sc_search_mengembalikan_dua_track() {
        let http = StubHttp::body(HTML_WITH_DATA);
        let tracks = sc_search(http.as_ref(), "jazz").unwrap();
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].title, "Lagu Satu");
        assert_eq!(tracks[1].title, "Lagu Dua");
    }

    #[test]
    fn tanpa_argument_tampilkan_karcis_pakai() {
        let r = run(StubHttp::offline(), "");
        assert_eq!(text_of(&r), help("."));
    }

    #[test]
    fn tidak_ditemukan_tampilkan_pesan() {
        let http = StubHttp::body(HTML_WITHOUT_DATA);
        let r = run(http, "tidakada");
        assert_eq!(text_of(&r), "❌ Aduh kak, lagunya nggak ketemu nih! Coba cari dengan judul yang beda ya. 😭");
    }

    #[test]
    fn encode_query_spasi_dengan_persen20() {
        assert_eq!(encode_query("a b"), "a%20b");
        assert_eq!(encode_query("Only We Know"), "Only%20We%20Know");
    }

    #[test]
    fn resolve_url_relative() {
        assert_eq!(
            resolve_url("/img/x.jpg", "https://m.soundcloud.com"),
            "https://m.soundcloud.com/img/x.jpg"
        );
    }

    #[test]
    fn jaringan_gagal_menggunakan_pesan_khusus() {
        let r = run(StubHttp::offline(), "jazz");
        assert_eq!(text_of(&r), "❌ Maaf kak, terjadi kesalahan sistem!");
    }
}