//! `plugins/search/applemusic.js` — cari lagu di Apple Music (backend YT Music).
//!
//! `ponytail:` JS memakai `resolver.resolve('ytmusic', …)` — lapisan backend
//! berlapis: scraper lokal `ytmusic-api` dulu, baru agregator. Backend lokal
//! itu modul npm dengan state internal, dan `src/lib/resolve.js` belum
//! di-port, jadi yang dipakai di sini hanya backend agregator yang sudah
//! menjadi cadangan di JS (`api.nexray.eu.cc/search/applemusic`, path tanpa
//! prefix `/api` — menambahkannya membuatnya 404). Bentuk field sudah
//! dinormalkan di `normalize` (`src/capabilities/ytmusic.js`), jadi yang dibaca
//! di sini bentuk yang sama dengan yang dibaca plugin JS.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

pub(crate) const ENDPOINT: &str = "https://api.nexray.eu.cc/search/applemusic";

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `{p}applemusic <query>`\n\n\
         > Contoh:\n> `{p}applemusic Best Friend`",
        p = prefix
    )
}

pub(crate) fn not_found(query: &str) -> String {
    format!("❌ Tidak ditemukan hasil untuk: {query}")
}

/// `te(prefix, command, pushName)` dengan template default (`error.js`).
/// `ponytail:` `Ctx` tidak membawa `pushName` dan plugin tidak bisa membaca
/// `config.errorTemplate`, jadi keduanya jatuh ke fallback JS.
pub(crate) fn error_reply(prefix: &str, command: &str) -> String {
    format!(
        "☢ *ᴇʀʀᴏʀ*\n\n> Terjadi kesalahan pada command `{prefix}{command}`\n\
         > Silahkan coba lagi nanti, User\n\n_Jika masalah berlanjut, hubungi owner_"
    )
}

/// `t.subtitle || 'Unknown'` — `normalize` tidak memetakan `artist`, jadi
/// bentuk agregator hanya membawa `subtitle`.
pub(crate) fn artist(track: &Value) -> String {
    match track.get("subtitle").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => "Unknown".to_string(),
    }
}

/// Lima hasil teratas, persis format `applemusic.js:46-51`. `m.reply(txt.trim())`
/// membuang spasi akhir, jadi `render` juga.
pub(crate) fn render(query: &str, tracks: &[Value]) -> String {
    let mut txt = format!("🍎 *ᴀᴘᴘʟᴇ ᴍᴜsɪᴄ sᴇᴀʀᴄʜ*\n\n> Query: *{query}*\n\n");
    for (i, t) in tracks.iter().take(5).enumerate() {
        txt.push_str(&format!(
            "*{}.* ```{}```\n   ├ 📀 `{}`\n   └ 🔗 `{}`\n\n",
            i + 1,
            t.get("title").and_then(Value::as_str).unwrap_or(""),
            artist(t),
            t.get("url").and_then(Value::as_str).unwrap_or("")
        ));
    }
    txt.trim().to_string()
}

#[plugin(
    name = "applemusic",
    alias = ["amusic", "am"],
    category = "search",
    description = "Cari lagu di Apple Music",
    usage = ".applemusic <query>",
    example = ".applemusic Best Friend",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn applemusic(ctx: Ctx) -> Reply {
    let query = ctx.text.trim().to_string();
    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let url = format!("{ENDPOINT}?q={}", encode(&query));
    let error = error_reply(&ctx.prefix, &ctx.command);
    let body = match ctx.http.get(&url).ok().and_then(|r| r.json()) {
        Some(v) => v,
        None => return Reply::text(error),
    };

    let tracks: Vec<Value> = body
        .get("result")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if tracks.is_empty() {
        return Reply::text(not_found(&query));
    }
    Reply::text(render(&query, &tracks))
}

/// `URLSearchParams.set` meng-encode query sebagai
/// `application/x-www-form-urlencoded` — byte non-ASCII jadi `%XX` uppercase
/// dan spasi jadi `+`.
fn encode(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for b in raw.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(*b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{Ctx, HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    /// HTTP yang dicatat: mengembalikan body yang sudah disiapkan, atau gagal
    /// total. Offline — test tidak pernah menyentuh jaringan.
    #[derive(Debug)]
    struct Stub {
        result: Result<String, ()>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn body(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                result: Ok(v.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn raw(text: &str) -> Arc<Stub> {
            Arc::new(Stub {
                result: Ok(text.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                result: Err(()),
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let text = self
                .result
                .clone()
                .map_err(|()| HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "applemusic".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("applemusic").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    /// Varian dimiliki: untuk `owned(run(..))` di dalam assert.
    fn owned(r: Reply) -> String {
        r.text_body().map(str::to_string).unwrap_or_default()
    }

    fn tracks(n: usize) -> Vec<Value> {
        (0..n)
            .map(|i| {
                json!({
                    "title": format!("Lagu {i}"),
                    "subtitle": format!("Artist {i}"),
                    "url": format!("https://youtu.be/abc{i}"),
                })
            })
            .collect()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("applemusic").unwrap();
        assert_eq!(p.meta.name, "applemusic");
        assert_eq!(p.meta.alias, ["amusic", "am"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari lagu di Apple Music");
        assert_eq!(p.meta.usage, ".applemusic <query>");
        assert_eq!(p.meta.example, ".applemusic Best Friend");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["amusic", "am"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "applemusic");
        }
    }

    #[test]
    fn an_empty_query_shows_the_usage_card_without_touching_the_network() {
        let http = Stub::body(json!({}));
        for text in ["", "   "] {
            let r = run(http.clone(), text);
            let t = text_of(&r);
            assert_eq!(t, help("."), "input {text:?}");
            assert!(t.contains("`.applemusic <query>`"), "{t}");
        }
        assert!(http.seen.lock().unwrap().is_empty(), "tidak boleh ada request");
    }

    #[test]
    fn results_are_numbered_with_artist_and_link() {
        let http = Stub::body(json!({ "status": true, "result": tracks(2) }));
        let r = run(http.clone(), "Best Friend");
        let t = text_of(&r);
        assert!(t.starts_with("🍎 *ᴀᴘᴘʟᴇ ᴍᴜsɪᴄ sᴇᴀʀᴄʜ*"), "{t}");
        assert!(t.contains("> Query: *Best Friend*"), "{t}");
        assert!(t.contains("*1.* ```Lagu 0```"), "{t}");
        assert!(t.contains("├ 📀 `Artist 0`"), "{t}");
        assert!(t.contains("└ 🔗 `https://youtu.be/abc0`"), "{t}");
        assert!(t.contains("*2.* ```Lagu 1```"), "{t}");
        assert_eq!(r.mutations().len(), 0);

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(
            seen[0].starts_with("https://api.nexray.eu.cc/search/applemusic?q="),
            "path tanpa prefix `/api`: {}",
            seen[0]
        );
    }

    #[test]
    fn only_the_first_five_tracks_are_listed() {
        let http = Stub::body(json!({ "result": tracks(9) }));
        let t = owned(run(http, "x"));
        assert!(t.contains("*5.*"), "lima hasil: {t}");
        assert!(!t.contains("*6.*"), "enam hasil terpotong: {t}");
    }

    #[test]
    fn a_missing_subtitle_falls_back_to_unknown() {
        let http = Stub::body(json!({ "result": [
            { "title": "T", "subtitle": "", "url": "u" },
            { "title": "T2", "url": "u2" }
        ] }));
        let t = owned(run(http, "x"));
        assert_eq!(t.matches("📀 `Unknown`").count(), 2, "{t}");
    }

    #[test]
    fn an_empty_result_list_reports_not_found() {
        for body in [json!({ "status": false, "result": [] }), json!({}), json!({"result": null})] {
            let http = Stub::body(body.clone());
            let t = owned(run(http, "tidak ada"));
            assert_eq!(t, not_found("tidak ada"), "body {body}");
        }
    }

    #[test]
    fn a_transport_or_parsing_failure_reports_the_generic_error_template() {
        let http = Stub::offline();
        assert_eq!(owned(run(http, "x")), error_reply(".", "applemusic"));

        let http = Stub::raw("<html>bukan json</html>");
        let t = owned(run(http, "x"));
        assert!(t.starts_with("☢ *ᴇʀʀᴏʀ*"), "{t}");
        assert!(t.contains("command `.applemusic`"), "{t}");
    }

    #[test]
    fn the_query_is_url_encoded_the_way_url_search_params_does_it() {
        assert_eq!(encode("Best Friend"), "Best+Friend");
        assert_eq!(encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(encode("küfte"), "k%C3%BCfte");
        assert_eq!(encode("a/b?c#d"), "a%2Fb%3Fc%23d");
        assert_eq!(encode("safe-._*"), "safe-._*", "karakter unreserved tak di-encode");
    }
}
