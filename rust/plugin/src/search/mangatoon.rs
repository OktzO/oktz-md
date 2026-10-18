//! `plugins/search/mangatoon.js` — cari komik di Mangatoon lewat API Cuki.
//!
//! Kunci `config.APIkey?.cuki` dibaca dari `ctx.config()`. Cabang "API Key
//! belum diset" (`mangatoon.js:53`) benar kalau kuncinya kosong — persis
//! seperti `process.env.APIKEY_CUKI || ""` di `config.js`.
//!
//! `ponytail:` `sock.sendMedia(m.chat, cover, caption.trim(), m, {type:'image'})`
//! ada padanannya: `Reply::Image` dengan `MediaSource::url`.
//!
//! `ponytail:` `trimText` memakai `value.length` dan `value.slice`, yaitu
//! satuan UTF-16 V8. Di sini dipakai `.chars()` — beda hanya untuk emoji dan
//! karakter astral di luar BMP.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `mangatoon.js:50` — kartu pakai saat `m.text` kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!("📚 *MANGATOON SEARCH*\n\n> Contoh:\n`{prefix}mangatoon love`")
}

/// `mangatoon.js:54` — teks literal, bukan lewat `explainFailure`.
pub(crate) const MISSING_KEY: &str =
    "❌ *API Key belum diset!*\n\n> Isi `APIKEY_CUKI` di file `.env`, lalu restart bot";

/// `mangatoon.js:23-28` — spasi beruntai dirapatkan jadi satu, string kosong
/// jadi `-`, dan teks panjang dipotong pada `max` karakter (bukan `max + 1`).
pub(crate) fn trim_text(text: Option<&str>, max: usize) -> String {
    let value = collapse_spaces(text.unwrap_or(""));
    if value.is_empty() {
        return "-".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max {
        return value;
    }
    chars.into_iter().take(max).collect::<String>() + "..."
}

/// `String.prototype.replace(/\s+/g, " ")` — hanya whitespace ASCII dan
/// Unicode yang dikenal Rust; hasil `trim()` diambil dari sisanya.
fn collapse_spaces(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_ws = false;
    for c in raw.chars() {
        if c.is_whitespace() {
            in_ws = true;
        } else {
            if in_ws {
                out.push(' ');
                in_ws = false;
            }
            out.push(c);
        }
    }
    if in_ws {
        out.push(' ');
    }
    out.trim().to_string()
}

/// `x || fallback` untuk nilai yang masuk ke template literal.
fn or_truthy(value: Option<&Value>, fallback: &str) -> String {
    match value {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0) => n.to_string(),
        Some(Value::Bool(true)) => "true".to_string(),
        _ => fallback.to_string(),
    }
}

/// `mangatoon.js:69-77` — caption hasil, sudah di-`trim()` seperti di JS.
pub(crate) fn caption(items: &[Value], data: &Value, query: &str) -> String {
    let mut txt = String::from("📚 *MANGATOON SEARCH*\n\n");
    txt.push_str(&format!(
        "🔎 *Query:* {}\n",
        or_truthy(data.get("query"), query)
    ));
    txt.push_str(&format!(
        "📦 *Total:* {}\n",
        or_truthy(data.get("total"), &items.len().to_string())
    ));
    txt.push_str(&format!(
        "🌐 *Source:* {}\n\n",
        or_truthy(data.get("source"), "mangatoon.mobi")
    ));
    for (i, item) in items.iter().enumerate() {
        txt.push_str(&format!(
            "*{}.* {}\n   ├ {}\n\n",
            i + 1,
            trim_text(item.get("title").and_then(Value::as_str), 60),
            interp(item.get("link"))
        ));
    }
    txt.trim().to_string()
}

/// `${value}` di JS — `undefined` untuk field yang hilang, `null` untuk
/// `null`, string apa adanya, nilai lain sebagai JSON kompak.
pub(crate) fn interp(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `mangatoon.js:61-62` — grup `results.komik[].items[]` diratakan jadi satu
/// daftar lalu dipotong 10.
pub(crate) fn flat_items(data: &Value) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    if let Some(groups) = data.pointer("/results/komik").and_then(Value::as_array) {
        for group in groups {
            if let Some(items) = group.get("items").and_then(Value::as_array) {
                out.extend(items.iter().cloned());
            }
        }
    }
    out.truncate(10);
    out
}

/// `fetchMangatoon` (`mangatoon.js:30-44`) — endpoint + dua header.
fn api_url(query: &str, api_key: &str) -> String {
    format!(
        "https://api.cuki.biz.id/api/search/mangatoon?apikey={}&query={}",
        encode_uri_component(api_key),
        encode_uri_component(query)
    )
}

#[plugin(
    name = "mangatoon",
    alias = ["mtoon", "mangatoonsearch", "searchmangatoon"],
    category = "search",
    description = "Cari komik di Mangatoon",
    usage = ".mangatoon <query>",
    example = ".mangatoon love",
    cooldown = 8,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn mangatoon(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `mangatoon.js:53` dicek sebelum `m.react('🔍')` di `:57`, jadi cabang ini
    // tidak bereaksi sama sekali.
    if ctx.config().api_keys.cuki.is_empty() {
        return Reply::text(MISSING_KEY);
    }

    // `axios.get` melempar untuk status >= 400 dan error transport; keduanya
    // ditangkap `catch` yang sama.
    let Ok(res) = ctx
        .http
        .call(
            &oktz_plugin_api::HttpCall::get(api_url(query, &ctx.config().api_keys.cuki))
                .header("x-api-key", &ctx.config().api_keys.cuki)
                .header("user-agent", "Mozilla/5.0"),
        )
    else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    // `if (!data?.status || !data?.data?.results) throw` (`:39`) — `results`
    // harus ada, tapi `status`/`data` yang salah bentuk hanya menilai
    // `results`-nya.
    let body = res.json().unwrap_or(Value::Null);
    let Some(data) = body.get("data") else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };
    if !crate::common::js_truthy(body.get("status")) || data.get("results").is_none() {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    }

    let items = flat_items(data);
    if items.is_empty() {
        return Reply::text(format!(
            "❌ Tidak ditemukan komik Mangatoon untuk: {query}"
        ))
        .with_reaction("❌");
    }

    let cap = caption(&items, data, query);
    // `items[0]?.image` — cover hanya diambil dari item pertama, bukan dari item
    // mana pun yang punya gambar.
    match items[0].get("image").and_then(Value::as_str) {
        Some(cover) if !cover.is_empty() => {
            Reply::image(MediaSource::url(cover), cap).with_reaction("✅")
        }
        _ => Reply::text(cap).with_reaction("✅"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    // Selalu gagal: jalur HTTP di JS tidak terjangkau selama kunci kosong,
    // jadi stub hanya dipakai untuk membuktikan tidak ada request yang dikirim.
    #[derive(Debug, Default)]
    struct Stub {
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Err(oktz_plugin_api::HttpError {
                message: "offline".into(),
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
            command: "mangatoon".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("mangatoon").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("mangatoon").unwrap();
        assert_eq!(p.meta.name, "mangatoon");
        assert_eq!(
            p.meta.alias,
            ["mtoon", "mangatoonsearch", "searchmangatoon"]
        );
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari komik di Mangatoon");
        assert_eq!(p.meta.usage, ".mangatoon <query>");
        assert_eq!(p.meta.example, ".mangatoon love");
        assert_eq!(p.meta.cooldown, 8);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mtoon", "mangatoonsearch", "searchmangatoon"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mangatoon");
        }
    }

    #[test]
    fn empty_input_shows_the_help_card_without_reacting() {
        let http = Stub::offline();
        for text in ["", "   ", "\n\t "] {
            assert_eq!(
                text_of(&run(http.clone(), text)),
                "📚 *MANGATOON SEARCH*\n\n> Contoh:\n`.mangatoon love`",
                "input {text:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_help_card_uses_the_prefix_from_the_context() {
        let ctx = Ctx {
            prefix: "!".into(),
            command: "mtoon".into(),
            text: String::new(),
            ..Default::default()
        };
        assert!(help(&ctx.prefix).contains("`!mangatoon love`"));
    }

    #[test]
    fn a_query_hits_the_missing_key_branch_without_a_request() {
        // `config.APIkey.cuki` tidak ada di `Ctx`, jadi cabang ini selalu benar
        // — persis seperti `canvas::pakustad`.
        let http = Stub::offline();
        let r = run(http.clone(), "love");
        assert_eq!(
            text_of(&r),
            "❌ *API Key belum diset!*\n\n> Isi `APIKEY_CUKI` di file `.env`, lalu restart bot"
        );
        // `m.react('🔍')` ada setelah guard, jadi tidak ada reaksi di sini.
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn odd_and_very_long_queries_still_land_on_the_missing_key_branch() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", "  \t "] {
            let r = run(Stub::offline(), input);
            let t = text_of(&r);
            assert!(
                t == MISSING_KEY || t.starts_with("📚 *MANGATOON SEARCH*"),
                "input {input:?} -> {t}"
            );
        }
        let panjang = "komik ".repeat(3000);
        assert_eq!(text_of(&run(Stub::offline(), &panjang)), MISSING_KEY);
    }

    #[test]
    fn trim_text_collapses_whitespace_and_replaces_blanks() {
        assert_eq!(trim_text(None, 60), "-");
        assert_eq!(trim_text(Some(""), 60), "-");
        assert_eq!(trim_text(Some("   \n\t "), 60), "-");
        assert_eq!(trim_text(Some("  One   Piece \n Vol 1 "), 60), "One Piece Vol 1");
        assert_eq!(trim_text(Some("pendek"), 60), "pendek");
    }

    #[test]
    fn trim_text_cuts_exactly_at_the_limit() {
        let lima = "abcde";
        assert_eq!(trim_text(Some(lima), 5), "abcde", "tepat 5 tidak dipotong");
        assert_eq!(trim_text(Some(lima), 4), "abcd...");
        assert_eq!(trim_text(Some(lima), 1), "a...");
        assert_eq!(trim_text(Some(lima), 0), "...", "max 0 tetap Potong");
    }

    #[test]
    fn comic_groups_flatten_into_one_numbered_caption() {
        let data = serde_json::json!({
            "query": "love",
            "total": 42,
            "source": "mangatoon.mobi",
            "results": { "komik": [
                { "items": [
                    { "title": "  Love   Story ", "link": "https://a/1" },
                    { "title": "Love 2", "link": "https://a/2" },
                ]},
                { "items": [ { "title": "Love 3", "link": "https://a/3" } ]},
                { "bukanArray": true },
            ]},
        });
        let items = flat_items(&data);
        assert_eq!(items.len(), 3, "grup tanpa items diabaikan");
        let t = caption(&items, &data, "cari");
        assert!(
            t.starts_with("📚 *MANGATOON SEARCH*\n\n🔎 *Query:* love\n📦 *Total:* 42\n🌐 *Source:* mangatoon.mobi\n\n"),
            "{t}"
        );
        assert!(t.contains("*1.* Love Story\n   ├ https://a/1\n"), "{t}");
        assert!(t.ends_with("*3.* Love 3\n   ├ https://a/3"), "caption di-trim: {t:?}");
    }

    #[test]
    fn missing_caption_fields_fall_back_the_way_js_does() {
        let data = serde_json::json!({ "query": "", "total": 0, "source": "", "results": {} });
        let items = flat_items(&data);
        assert!(items.is_empty());
        let t = caption(&items, &data, "cari");
        assert!(t.contains("🔎 *Query:* cari\n"), "query kosong -> fallback: {t}");
        assert!(t.contains("📦 *Total:* 0\n"), "total 0 -> panjang items: {t}");
        // Tanpa item, baris terakhir adalah `Source` dan caption sudah di-trim,
        // jadi tanpa newline penutup.
        assert!(t.ends_with("🌐 *Source:* mangatoon.mobi"), "{t}");
    }

    #[test]
    fn an_item_without_a_link_prints_undefined() {
        let data = serde_json::json!({ "results": { "komik": [ { "items": [ { "title": "Tanpa link" } ] } ] } });
        let items = flat_items(&data);
        let t = caption(&items, &data, "q");
        assert!(t.ends_with("*1.* Tanpa link\n   ├ undefined"), "`item.link` undefined: {t}");
        assert_eq!(interp(None), "undefined");
        assert_eq!(interp(Some(&Value::Null)), "null");
    }

    #[test]
    fn at_most_ten_comics_are_listed() {
        let semua: Vec<Value> = (0..14)
            .map(|i| serde_json::json!({ "title": format!("Komik {i}"), "link": format!("https://a/{i}") }))
            .collect();
        let data = serde_json::json!({ "results": { "komik": [ { "items": semua } ] } });
        let items = flat_items(&data);
        assert_eq!(items.len(), 10);
        let t = caption(&items, &data, "q");
        assert!(t.contains("*10.* Komik 9\n"), "{t}");
        assert!(!t.contains("Komik 10"), "hasil ke-11 terpotong: {t}");
    }

    #[test]
    fn a_missing_results_key_flattens_to_nothing() {
        // `data.results` ada tapi bukan objek dengan `komik` — `flatMap` JS
        // menolak `entry?.items` yang bukan array.
        let data = serde_json::json!({ "results": { "komik": "bukan array" } });
        assert!(flat_items(&data).is_empty());
        assert!(flat_items(&serde_json::json!({})).is_empty());
    }
}