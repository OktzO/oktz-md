//! `plugins/search/searchthatsong.js` — mencari detail lagu dari lirik/potongan kata.
//!
//! `ponytail:` dua permintaan HTTP berturut-turut dengan cookie sesi
//! (`route-preview` lalu full search) dan pengiriman image + teks lirik +
//! audio berurutan. `Reply::sequence` digunakan, tetapi `quoted: m` pada tiap
//! pesan tidak tersedia di `Ctx`, jadi pesan tidak mengutip pemicu.
//! Header kustom (Cookie, User-Agent) dikirim lewat `HttpCall::call`.
//!
//! Catatan: baris `pub mod searchthatsong;` perlu ditambah ke `search/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, HttpCall, MediaSource, Reply, plugin};
use serde_json::{json, Value};

const BASE: &str = "https://searchthatsong.com";
const UA: &str = "Mozilla/5.0 (Linux; Android 10; Mobile) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Mobile Safari/537.36 VyrSTS/1.0";

fn header_dasar() -> HttpCall {
    HttpCall::post("")
        .header("Accept", "application/json, text/plain, */*")
        .header("User-Agent", UA)
        .header("Accept-Language", "id-ID,id;q=0.9,en-US;q=0.8,en;q=0.7")
        .header("Cache-Control", "no-cache")
        .header("Pragma", "no-cache")
}

/// Ekstrak `session_id` dari header `set-cookie`.
fn extract_session_id(headers: &[(String, String)]) -> Option<String> {
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("set-cookie") {
            if let Some(m) = v.find("session_id=") {
                let after = &v[m + "session_id=".len()..];
                let end = after.find(';').unwrap_or(after.len());
                return Some(after[..end].to_string());
            }
        }
    }
    None
}

/// Satu permintaan JSON POST, mengembalikan (status, headers, body-parse).
fn json_post(http: &dyn oktz_plugin_api::HttpGet, url: &str, body: &Value, cookie: Option<&str>) -> Result<(u16, Vec<(String, String)>, Value), oktz_plugin_api::HttpError> {
    let mut req = header_dasar();
    req.url = url.to_string();
    if let Some(c) = cookie {
        req = req.header("Cookie", c);
    }
    let res = http.call(&req.json(body))?;
    let body_val = res.json().unwrap_or(Value::Null);
    Ok((res.status, res.headers, body_val))
}

/// `search(query)` — dua langkah: route-preview lalu full search.
fn search(http: &dyn oktz_plugin_api::HttpGet, query: &str) -> Result<Value, oktz_plugin_api::HttpError> {
    let (status, headers, route_data) = json_post(
        http,
        &format!("{BASE}/api/search/route-preview"),
        &serde_json::json!({ "query": query }),
        None,
    )?;
    if status != 200 {
        return Err(oktz_plugin_api::HttpError {
            message: format!("request failed [rp] status={status}"),
        });
    }
    let session_id = extract_session_id(&headers);
    let sid = session_id.as_deref().or_else(|| route_data.get("session_id").and_then(Value::as_str)).unwrap_or("");
    let route_preview = route_data.get("route").or(Some(&route_data)).and_then(Value::as_str).unwrap_or("");

    let cookie = if !sid.is_empty() { Some(format!("session_id={sid}")) } else { None };
    let (_s2, _h2, full) = json_post(
        http,
        BASE,
        &serde_json::json!({ "data": query, "route_preview": route_preview, "search_mode": "web_search" }),
        cookie.as_deref(),
    )?;
    Ok(full)
}

fn build_result(raw: &Value) -> Value {
    let a = raw.get("answer").unwrap_or(raw);
    // Nilai kosong dan "n/a" jadi null, sama seperti JS.
    let teks_atau_null = |kunci: &str| {
        let v = a.get(kunci).and_then(Value::as_str).unwrap_or("");
        if v.is_empty() || v == "n/a" { None } else { Some(v.to_string()) }
    };
    let lirik = teks_atau_null("plain_lyrics");
    let potongan = teks_atau_null("most_relevant_chunk");
    serde_json::json!({
        "song": a.get("song").or_else(|| a.get("title")).and_then(Value::as_str),
        "artist": a.get("artist").and_then(Value::as_str),
        "album": a.get("album").and_then(Value::as_str),
        "year": a.get("year_song_released").or_else(|| a.get("year")).and_then(Value::as_str),
        "genre": a.get("genre").and_then(Value::as_str),
        "confidence": a.get("router_confidence").and_then(Value::as_str),
        "lyrics": lirik,
        "relevant_chunk": potongan,
        "preview_url": a.get("preview_audio_url").or_else(|| a.get("youtube_url")).and_then(Value::as_str),
        "album_artwork": a.get("album_artwork_url").or_else(|| a.get("album_artwork")).and_then(Value::as_str),
        "youtube_url": a.get("Youtube_URL").or_else(|| a.get("youtube_url")).and_then(Value::as_str),
    })
}

#[plugin(
    name = "searchthatsong",
    alias = ["sts", "carilagu"],
    category = "search",
    description = "Mencari detail sebuah lagu dari lirik atau potongan kata",
    usage = ".searchthatsong <potongan lirik>",
    example = ".searchthatsong ku menangis membayangkan",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn searchthatsong(ctx: Ctx) -> Reply {
    let query = if !ctx.args.is_empty() {
        ctx.args.join(" ")
    } else {
        ctx.text.trim().to_string()
    };
    if query.is_empty() {
        return Reply::text("❌ Masukkan potongan lirik atau nama lagu yang ingin dicari.\n\nContoh: `.sts ku menangis membayangkan`");
    }

    let result = match search(ctx.http.as_ref(), &query) {
        Ok(v) => v,
        Err(_) => {
            return Reply::text("😔 Terjadi kesalahan saat mencari lagu. Server mungkin sedang bermasalah.");
        }
    };

    let r = build_result(&result);
    let song = r.get("song").and_then(Value::as_str);
    if song.is_none() {
        return Reply::text("⚠️ Lagu tidak ditemukan. Coba gunakan lirik yang lebih spesifik.");
    }
    let song = song.unwrap();

    let mut caption = format!("🎵 *SEARCH THAT SONG* 🎵\n\n*Judul:* {song}\n");
    if let Some(artist) = r.get("artist").and_then(Value::as_str) {
        caption.push_str(&format!("*Artis:* {artist}\n"));
    }
    if let Some(album) = r.get("album").and_then(Value::as_str) {
        caption.push_str(&format!("*Album:* {album}\n"));
    }
    if let Some(year) = r.get("year").and_then(Value::as_str) {
        caption.push_str(&format!("*Tahun:* {year}\n"));
    }
    if let Some(genre) = r.get("genre").and_then(Value::as_str) {
        caption.push_str(&format!("*Genre:* {genre}\n"));
    }
    if let Some(youtube) = r.get("youtube_url").and_then(Value::as_str) {
        caption.push_str(&format!("*YouTube:* {youtube}\n"));
    }
    if let Some(chunk) = r.get("relevant_chunk").and_then(Value::as_str) {
        caption.push_str(&format!("\n*Match:* {chunk}\n"));
    }

    let artwork = r.get("album_artwork").and_then(Value::as_str).unwrap_or("");
    let lyrics = r.get("lyrics").and_then(Value::as_str).unwrap_or("");
    let preview = r.get("preview_url").and_then(Value::as_str).unwrap_or("");

    let mut msgs: Vec<Reply> = Vec::new();
    if !artwork.is_empty() {
        msgs.push(Reply::image(MediaSource::url(artwork), caption.clone()));
    } else {
        msgs.push(Reply::text(caption.clone()));
    }
    if !lyrics.is_empty() {
        msgs.push(Reply::text(format!("*Lirik:*\n\n{lyrics}")));
    }
    if !preview.is_empty() {
        msgs.push(Reply::audio(MediaSource::url(preview)));
    }

    Reply::sequence(msgs).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct Stub {
        responses: Mutex<Vec<String>>,
        headers_list: Mutex<Vec<Vec<(String, String)>>>,
        seen: Mutex<Vec<String>>,
        hit: Mutex<usize>,
    }

    impl Stub {
        fn ordered(responses: Vec<&str>) -> Arc<Stub> {
            Arc::new(Stub {
                responses: Mutex::new(responses.into_iter().map(|s| s.to_string()).collect()),
                headers_list: Mutex::new(Vec::new()),
                seen: Mutex::new(Vec::new()),
                hit: Mutex::new(0),
            })
        }
    }

    impl HttpGet for Stub {
        fn call(&self, req: &HttpCall) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(req.url.clone());
            let mut hit = self.hit.lock().unwrap();
            let i = *hit;
            *hit += 1;
            self.headers_list.lock().unwrap().push(req.headers.clone());
            let responses = self.responses.lock().unwrap();
            let body = responses.get(i).cloned().unwrap_or_default();
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: Vec::new(),
            })
        }
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("pakai call")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let args: Vec<String> = text.split_whitespace().skip(1).map(|s| s.to_string()).collect();
        let ctx = Ctx {
            text: text.to_string(),
            args,
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            command: "searchthatsong".into(),
            http,
            ..Default::default()
        };
        searchthatsong(ctx)
    }

    fn body(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or("").to_string()
    }

    fn reaction(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji.as_str()),
            _ => None,
        }
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("searchthatsong").unwrap();
        assert_eq!(p.meta.name, "searchthatsong");
        assert_eq!(p.meta.alias, ["sts", "carilagu"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
    }

    #[test]
    fn tanpa_query_tampilkan_petunjuk() {
        let r = run(Stub::ordered(vec![]), ".searchthatsong");
        assert!(body(&r).contains("Masukkan potongan lirik"));
        assert_eq!(reaction(&r), None);
    }

    #[test]
    fn dua_permintaan_http_dibuat_dengan_cookie_sesi() {
        let route = serde_json::json!({
            "session_id": "abc123",
            "route": "web_search"
        });
        let full = serde_json::json!({
            "answer": {
                "song": "Ku Menangis Membayangkan",
                "artist": "Artis Contoh",
                "album": "Album X",
                "year": "2026",
                "genre": "Pop",
                "youtube_url": "https://youtube.com/watch?v=abc",
                "router_confidence": "0.95"
            }
        });
        let http = Stub::ordered(vec![
            route.to_string(),
            full.to_string(),
        ]);
        let r = run(http.clone(), ".searchthatsong ku menangis membayangkan");
        assert!(body(&r).contains("Ku Menangis Membayangkan"));
        assert!(body(&r).contains("Artis Contoh"));
        assert_eq!(reaction(&r), Some("✅"));
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 2, "dua permintaan HTTP");
        assert!(seen[0].contains("/api/search/route-preview"));
        assert!(seen[1].ends_with("/") || seen[1] == BASE);
    }

    #[test]
    fn lagu_tidak_ditemukan() {
        let http = Stub::ordered(vec![
            serde_json::json!({ "session_id": "x", "route": "r" }).to_string(),
            serde_json::json!({ "answer": {} }).to_string(),
        ]);
        let r = run(http, ".searchthatsong xyz");
        assert!(body(&r).contains("Lagu tidak ditemukan"));
        assert_eq!(reaction(&r), Some("✅"));
    }

    #[test]
    fn kegagalan_jaringan() {
        let http = Stub::ordered(vec![]);
        let r = run(http, ".searchthatsong ku menangis");
        assert!(body(&r).contains("kesalahan saat mencari lagu"));
    }
}
