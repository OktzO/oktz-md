//! `plugins/search/mcpedl.js` — cari map dan addon Minecraft PE lewat API Cuki.
//!
//! Kunci `config.APIkey?.cuki` dibaca dari `ctx.config()` — sama seperti
//! `search::mangatoon`. Cabang "API Key belum diset" (`mcpedl.js:53`) benar
//! kalau kuncinya kosong.
//!
//! `ponytail:` `trimText` memakai satuan UTF-16 V8 (`length`/`slice`); di sini
//! `.chars()`. Sama seperti `search::mangatoon`.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::{error_template, js_truthy};
use crate::tools::phisingataubukan::encode_uri_component;

/// `mcpedl.js:50` — kartu pakai; contoh memakai command `mcpe`, bukan
/// `mcpedl`, persis seperti di JS.
pub(crate) fn help(prefix: &str) -> String {
    format!("🧱 *MCPEDL SEARCH*\n\n> Contoh:\n`{prefix}mcpe survival`")
}

/// `mcpedl.js:54` — teks literal.
pub(crate) const MISSING_KEY: &str =
    "❌ *API Key belum diset!*\n\n> Isi `APIKEY_CUKI` di file `.env`, lalu restart bot";

/// `mcpedl.js:23-28` — sama seperti `search::mangatoon::trim_text`.
pub(crate) fn trim_text(text: Option<&str>, max: usize) -> String {
    let mut value = String::new();
    let mut in_ws = false;
    for c in text.unwrap_or("").chars() {
        if c.is_whitespace() {
            in_ws = true;
        } else {
            if in_ws {
                value.push(' ');
                in_ws = false;
            }
            value.push(c);
        }
    }
    if in_ws {
        value.push(' ');
    }
    let value = value.trim().to_string();
    if value.is_empty() {
        return "-".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max {
        return value;
    }
    chars.into_iter().take(max).collect::<String>() + "..."
}

/// `${value}` di JS — `undefined` untuk field yang hilang.
fn interp(value: Option<&Value>) -> String {
    match value {
        None => "undefined".to_string(),
        Some(Value::Null) => "null".to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}

/// `x || fallback` di template literal.
fn or_truthy(value: Option<&Value>, fallback: &str) -> String {
    match value {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0) => n.to_string(),
        Some(Value::Bool(true)) => "true".to_string(),
        _ => fallback.to_string(),
    }
}

/// `mcpedl.js:61` — `result.results` harus array; `slice(0, 10)`.
pub(crate) fn take_items(data: &Value) -> Vec<Value> {
    let mut items = data
        .get("results")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    items.truncate(10);
    items
}

/// `mcpedl.js:68-77` — caption hasil sudah di-`trim()` seperti di JS.
pub(crate) fn caption(items: &[Value], data: &Value, query: &str) -> String {
    let mut txt = String::from("🧱 *MCPEDL SEARCH*\n\n");
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
        or_truthy(data.get("source"), "mcpedl.org")
    ));
    for (i, item) in items.iter().enumerate() {
        txt.push_str(&format!(
            "*{}.* {}\n   ├ ⭐ Rating: {}\n   ├ {}\n\n",
            i + 1,
            trim_text(item.get("title").and_then(Value::as_str), 60),
            or_truthy(item.get("rating"), "-"),
            interp(item.get("link"))
        ));
    }
    txt.trim().to_string()
}

/// `fetchMcpe` (`mcpedl.js:30-44`).
fn api_url(query: &str, api_key: &str) -> String {
    format!(
        "https://api.cuki.biz.id/api/search/mcpedl?apikey={}&query={}",
        encode_uri_component(api_key),
        encode_uri_component(query)
    )
}

#[plugin(
    name = "mcpedl",
    alias = ["mcpe", "mcpesearch", "searchmcpe"],
    category = "search",
    description = "Cari map dan addon Minecraft PE dari MCPEDL",
    usage = ".mcpe <query>",
    example = ".mcpe survival",
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
pub fn mcpedl(ctx: Ctx) -> Reply {
    let query = ctx.text.trim();

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // Dicek sebelum `m.react('🔍')` di `:57`, jadi tanpa reaksi.
    if ctx.config().api_keys.cuki.is_empty() {
        return Reply::text(MISSING_KEY);
    }

    // `axios.get` melempar untuk status >= 400 dan error transport; keduanya
    // masuk `catch` yang sama.
    let Ok(res) = ctx
        .http
        .call(
            &oktz_plugin_api::HttpCall::get(api_url(query, &ctx.config().api_keys.cuki))
                .header("x-api-key", ctx.config().api_keys.cuki)
                .header("user-agent", "Mozilla/5.0"),
        )
    else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    // `if (!data?.status || !Array.isArray(data?.data?.results)) throw` (`:39`).
    // `results` WAJIB array: bentuk lain melempar, bukan jadi daftar kosong.
    let body = res.json().unwrap_or(Value::Null);
    let sah = js_truthy(body.get("status"))
        && body
            .get("data")
            .and_then(|d| d.get("results"))
            .is_some_and(Value::is_array);
    if !sah {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    }
    let data = &body["data"];

    let items = take_items(data);
    if items.is_empty() {
        return Reply::text(format!(
            "❌ Tidak ditemukan hasil MCPEDL untuk: {query}"
        ))
        .with_reaction("❌");
    }

    let cap = caption(&items, data, query);
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
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

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
            Err(HttpError {
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
            command: "mcpedl".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("mcpedl").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("mcpedl").unwrap();
        assert_eq!(p.meta.name, "mcpedl");
        assert_eq!(p.meta.alias, ["mcpe", "mcpesearch", "searchmcpe"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Cari map dan addon Minecraft PE dari MCPEDL");
        assert_eq!(p.meta.usage, ".mcpe <query>");
        assert_eq!(p.meta.example, ".mcpe survival");
        assert_eq!(p.meta.cooldown, 8);
        assert_eq!(p.meta.energi, 1);
        assert!(p.meta.is_enabled);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["mcpe", "mcpesearch", "searchmcpe"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "mcpedl");
        }
    }

    #[test]
    fn empty_input_shows_the_help_card_naming_the_mcpe_alias() {
        let http = Stub::offline();
        for text in ["", "  ", "\t\n"] {
            assert_eq!(
                text_of(&run(http.clone(), text)),
                "🧱 *MCPEDL SEARCH*\n\n> Contoh:\n`.mcpe survival`",
                "input {text:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
        assert!(help("!").contains("`!mcpe survival`"));
    }

    #[test]
    fn a_query_hits_the_missing_key_branch_without_a_request() {
        let http = Stub::offline();
        let r = run(http.clone(), "survival");
        assert_eq!(text_of(&r), MISSING_KEY);
        assert_eq!(reaction_of(&r), None);
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", "%00"] {
            let r = run(Stub::offline(), input);
            let t = text_of(&r);
            assert!(
                t == MISSING_KEY || t.starts_with("🧱 *MCPEDL SEARCH*"),
                "input {input:?} -> {t}"
            );
        }
        let panjang = "map ".repeat(4000);
        assert_eq!(text_of(&run(Stub::offline(), &panjang)), MISSING_KEY);
    }

    #[test]
    fn trim_text_matches_the_javascript_helper() {
        assert_eq!(trim_text(None, 60), "-");
        assert_eq!(trim_text(Some("  \n "), 60), "-");
        assert_eq!(trim_text(Some(" a  b\tc "), 60), "a b c");
        assert_eq!(trim_text(Some("abcde"), 5), "abcde");
        assert_eq!(trim_text(Some("abcde"), 4), "abcd...");
        assert_eq!(trim_text(Some("abcde"), 0), "...");
    }

    #[test]
    fn results_are_capped_at_ten_and_keep_the_rating_line() {
        let semua: Vec<Value> = (0..13)
            .map(|i| json!({ "title": format!("Map {i}"), "link": format!("https://m/{i}") }))
            .collect();
        let data = json!({ "results": semua });
        let items = take_items(&data);
        assert_eq!(items.len(), 10);
        let t = caption(&items, &data, "q");
        assert!(t.starts_with("🧱 *MCPEDL SEARCH*\n\n🔎 *Query:* q\n📦 *Total:* 10\n🌐 *Source:* mcpedl.org\n\n"), "{t}");
        assert!(t.contains("*1.* Map 0\n   ├ ⭐ Rating: -\n   ├ https://m/0\n"), "{t}");
        assert!(t.contains("*10.* Map 9\n"), "{t}");
        assert!(!t.contains("Map 10"), "hasil ke-11 terpotong: {t}");
    }

    #[test]
    fn a_rating_of_zero_still_falls_back_to_the_dash() {
        // `item.rating || '-'` — `0` falsy di JS.
        let data = json!({ "results": [ { "title": "M", "link": "l", "rating": 0 } ] });
        let items = take_items(&data);
        assert!(caption(&items, &data, "q").contains("⭐ Rating: -"), "0 falsy");
    }

    #[test]
    fn a_non_array_results_field_is_never_treated_as_a_hit() {
        // Handler menolak bentuk ini sebelum `take_items` dipanggil, karena di
        // JS `!Array.isArray(..)` melempar.
        let data = json!({ "results": "bukan array" });
        assert!(!data
            .get("results")
            .is_some_and(Value::is_array));
        assert!(take_items(&data).is_empty(), "fungsi murni tetap defensif");
    }

    #[test]
    fn missing_item_fields_print_undefined_like_a_template_literal() {
        let data = json!({ "results": [ { "title": "Tanpa field" } ] });
        let items = take_items(&data);
        let t = caption(&items, &data, "q");
        assert!(t.ends_with("*1.* Tanpa field\n   ├ ⭐ Rating: -\n   ├ undefined"), "{t}");
        assert_eq!(interp(None), "undefined");
        assert_eq!(interp(Some(&json!(null))), "null");
        assert_eq!(interp(Some(&json!(12))), "12");
    }
}