//! `plugins/search/npm.js` — cari package di registry NPM.
//!
//! JS memakai `fetch` global, bukan `axios`: `fetch` hanya melempar untuk
//! error jaringan, TIDAK untuk status 4xx/5xx. Di API plugin `ctx.http.get`
//! mengembalikan `Err` untuk keduanya, jadi status 4xx/5px ikut masuk `catch`.
//! Bedanya dicatat di sini, bukan ditutup.
//!
//! `ponytail:` `data.objects.slice(0, 8).forEach` tidak menambah baris baru di
//! akhir tiap paket (`> ⭐ Score: …` tanpa `\n`), jadi paket berikutnya menempel
//! ke baris yang sama. Anomali itu ikut dipertahankan.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::{js_truthy};

/// `npm.js:34` — `size=10`, meski hanya 8 yang ditampilkan.
pub(crate) fn api_url(query: &str) -> String {
    format!(
        "https://registry.npmjs.com/-/v1/search?text={}&size=10",
        crate::tools::phisingataubukan::encode_uri_component(query)
    )
}

/// `npm.js:22-27` — kartu pakai.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}npm <query>`\n\n\
         > Contoh:\n\
         > `{prefix}npm axios`"
    )
}

/// `npm.js:39-43` — tidak ada hasil.
pub(crate) const TIDAK_ADA: &str = "❌ *ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> Package \"{query}\" tidak ditemukan";

/// `npm.js:51` — `Math.round((item.score?.final || 0) * 100)`.
///
/// `Math.round` V8 membulatkan ke atas untuk tepat `.5`, sedangkan
/// `f64::round` Rust setengah ke arah nol, jadi pembulatan ditulis eksplisit.
pub(crate) fn score_persen(item: &Value) -> i64 {
    let final_score = match item.get("score").and_then(|s| s.get("final")) {
        Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()) => {
            n.as_f64().unwrap()
        }
        _ => 0.0,
    };
    let scaled = final_score * 100.0;
    if !scaled.is_finite() {
        return 0;
    }
    let bawah = scaled.floor();
    let naik = if scaled - bawah >= 0.5 { 1 } else { 0 };
    (bawah as i64) + naik
}

/// `npm.js:45-63` — teks hasil; 8 paket pertama saja.
pub(crate) fn build_text(objects: &[Value], query: &str, total: Option<&Value>) -> String {
    let total = match total {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => "undefined".to_string(),
        Some(other) => other.to_string(),
    };
    let mut text = String::from("📦 *ɴᴘᴍ sᴇᴀʀᴄʜ*\n\n");
    text.push_str(&format!("> Query: `{query}`\n"));
    text.push_str(&format!("> Found: {total} packages\n\n"));
    for (i, item) in objects.iter().take(8).enumerate() {
        let pkg = &item["package"];
        let name = pkg.get("name").and_then(Value::as_str).unwrap_or("undefined");
        text.push_str(&format!("{}. *{name}*\n", i + 1));
        let version = pkg.get("version").and_then(Value::as_str).unwrap_or("undefined");
        text.push_str(&format!("> 📌 v{version}\n"));
        // `if (pkg.description)` — string kosong falsy, jadi bloknya dilewati.
        if let Some(desc) = pkg.get("description").and_then(Value::as_str)
            && !desc.is_empty()
        {
            let chars: Vec<char> = desc.chars().collect();
            let cut: String = chars.iter().take(50).collect();
            text.push_str(&format!(
                "> 📝 {cut}{}\n",
                if chars.len() > 50 { "..." } else { "" }
            ));
        }
        let link = pkg
            .pointer("/links/npm")
            .and_then(Value::as_str)
            .unwrap_or("-");
        text.push_str(&format!("> 🔗 {link}\n"));
        if let Some(author) = pkg.pointer("/author/name").and_then(Value::as_str)
            && js_truthy(pkg.pointer("/author/name"))
        {
            text.push_str(&format!("> 👤 {author}\n"));
        }
        // Tanpa `\n` di akhir — persis seperti di JS.
        text.push_str(&format!("> ⭐ Score: {}%", score_persen(item)));
    }
    text
}

/// `!data.objects || data.objects.length === 0` (`npm.js:38`).
pub(crate) fn objects_of(data: &Value) -> Vec<Value> {
    data.get("objects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

#[plugin(
    name = "npm",
    alias = ["npmsearch", "npmjs", "npmfind"],
    category = "search",
    description = "Search package di NPM registry",
    usage = ".npm <query>",
    example = ".npm axios",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn npm(ctx: Ctx) -> Reply {
    let query = ctx.args.join(" ");

    if query.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `await m.react("🕕")` di `:30` sudah jalan sebelum request, jadi reaksi
    // terakhir pada cabang gagal adalah "☢".
    let Ok(res) = ctx.http.get(&api_url(&query)) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };

    // `fetch` tidak melempar untuk status 4xx/5xx; `ctx.http.get` sudah
    // mengembalikan `Err` untuk itu, jadi `json()` di sini hanya gagal kalau
    // body memang bukan JSON.
    let data = res.json().unwrap_or(Value::Null);
    let objects = objects_of(&data);

    if objects.is_empty() {
        return Reply::text(TIDAK_ADA.replace("{query}", &query)).with_reaction("❌");
    }

    Reply::text(build_text(&objects, &query, data.get("total"))).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use std::sync::{Arc, Mutex};

    const ME: &str = "628111@s.whatsapp.net";

    #[derive(Debug)]
    struct Stub {
        body: Option<Value>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn with(v: Value) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(v),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let v = self
                .body
                .clone()
                .ok_or_else(|| oktz_plugin_api::HttpError { message: "offline".into() })?;
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(v.to_string().into_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
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
            command: "npm".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("npm").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("npm").unwrap();
        assert_eq!(p.meta.name, "npm");
        assert_eq!(p.meta.alias, ["npmsearch", "npmjs", "npmfind"]);
        assert_eq!(p.meta.category, "search");
        assert_eq!(p.meta.description, "Search package di NPM registry");
        assert_eq!(p.meta.usage, ".npm <query>");
        assert_eq!(p.meta.example, ".npm axios");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["npmsearch", "npmjs", "npmfind"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "npm");
        }
    }

    #[test]
    fn empty_query_shows_the_help_card() {
        let http = Stub::offline();
        for text in ["", "   ", "\t\n"] {
            assert_eq!(
                text_of(&run(http.clone(), text)),
                "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n> `.npm <query>`\n\n> Contoh:\n> `.npm axios`",
                "input {text:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
        assert!(help("#").contains("`#npm axios`"));
    }

    #[test]
    fn the_query_is_url_encoded_in_the_registry_search() {
        let http = Stub::with(serde_json::json!({ "objects": [], "total": 0 }));
        let _ = run(http.clone(), "@babel/core");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            "https://registry.npmjs.com/-/v1/search?text=%40babel%2Fcore&size=10"
        );
    }

    #[test]
    fn a_hit_renders_one_block_per_package() {
        let http = Stub::with(serde_json::json!({
            "total": 1234,
            "objects": [
                { "package": {
                    "name": "axios", "version": "1.7.2",
                    "description": "Promise based HTTP client",
                    "links": { "npm": "https://npmjs.com/package/axios" },
                    "author": { "name": "Matt Johnson" },
                }, "score": { "final": 0.987 }},
                { "package": { "name": "tanpa", "version": "0.0.1" }, "score": {} },
            ],
        }));
        let r = run(http, "axios");
        let t = text_of(&r);
        assert_eq!(reaction_of(&r), Some("✅"));
        assert!(t.starts_with("📦 *ɴᴘᴍ sᴇᴀʀᴄʜ*\n\n> Query: `axios`\n> Found: 1234 packages\n\n"), "{t}");
        assert!(t.contains("1. *axios*\n> 📌 v1.7.2\n"), "{t}");
        assert!(t.contains("> 📝 Promise based HTTP client\n"), "{t}");
        assert!(t.contains("> 🔗 https://npmjs.com/package/axios\n"), "{t}");
        assert!(t.contains("> 👤 Matt Johnson\n"), "{t}");
        assert!(t.contains("> ⭐ Score: 99%"), "{t}");
        // Paket kedua tidak punya description/links/author.
        assert!(t.contains("2. *tanpa*\n> 📌 v0.0.1\n> 🔗 -\n> ⭐ Score: 0%"), "{t}");
    }

    #[test]
    fn packages_run_together_because_the_js_loop_adds_no_newline() {
        let objects = vec![
            serde_json::json!({ "package": { "name": "a" }, "score": { "final": 0.5 } }),
            serde_json::json!({ "package": { "name": "b" }, "score": { "final": 0.5 } }),
        ];
        let t = build_text(&objects, "q", Some(&serde_json::json!(2)));
        assert!(
            t.contains("> ⭐ Score: 50%2. *b*"),
            "tidak ada newline di akhir blok: {t:?}"
        );
    }

    #[test]
    fn a_description_longer_than_fifty_characters_is_cut_with_an_ellipsis() {
        let desc = "x".repeat(51);
        let objects = vec![serde_json::json!({ "package": { "name": "p", "description": desc } })];
        let t = build_text(&objects, "q", None);
        assert!(t.contains(&format!("> 📝 {}...\n", "x".repeat(50))), "{t}");
        assert_eq!(t.matches('x').count(), 50);

        let desc = "y".repeat(50);
        let objects = vec![serde_json::json!({ "package": { "name": "p", "description": desc } })];
        let t = build_text(&objects, "q", None);
        assert!(t.contains(&format!("> 📝 {}\n", "y".repeat(50))), "{t}");
        assert!(!t.contains("..."), "tepat 50 tidak dipotong: {t}");
    }

    #[test]
    fn only_eight_packages_are_listed_even_though_ten_were_requested() {
        let objects: Vec<Value> = (0..10)
            .map(|i| serde_json::json!({ "package": { "name": format!("p{i}") } }))
            .collect();
        let t = build_text(&objects, "q", None);
        assert!(t.contains("8. *p7*"), "{t}");
        assert!(!t.contains("*p8*"), "paket ke-9 terpotong: {t}");
    }

    #[test]
    fn the_score_rounds_halves_up_like_math_round() {
        assert_eq!(score_persen(&serde_json::json!({ "score": { "final": 0.5 } })), 50);
        assert_eq!(score_persen(&serde_json::json!({ "score": { "final": 0.999 } })), 100);
        assert_eq!(score_persen(&serde_json::json!({ "score": { "final": 0.005 } })), 1);
        assert_eq!(score_persen(&serde_json::json!({ "score": {} })), 0);
        assert_eq!(score_persen(&serde_json::json!({})), 0);
        assert_eq!(score_persen(&serde_json::json!({ "score": { "final": null } })), 0);
        // `|| 0` menutup nilai 0 dan string kosong, jadi keduanya jadi 0.
        assert_eq!(score_persen(&serde_json::json!({ "score": { "final": 0 } })), 0);
    }

    #[test]
    fn an_empty_or_absent_objects_field_reports_not_found() {
        for body in [serde_json::json!({ "objects": [] }), serde_json::json!({}), serde_json::json!("bukan json")] {
            let r = run(Stub::with(body), "zzz");
            assert_eq!(
                text_of(&r),
                "❌ *ᴛɪᴅᴀᴋ ᴅɪᴛᴇᴍᴜᴋᴀɴ*\n\n> Package \"zzz\" tidak ditemukan"
            );
            assert_eq!(reaction_of(&r), Some("❌"));
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "axios");
        assert_eq!(text_of(&r), crate::common::error_template_bawaan(".", "npm"));
        assert_eq!(reaction_of(&r), Some("☢"));
    }

    #[test]
    fn odd_and_very_long_queries_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b", "?#&="] {
            let r = run(
                Stub::with(serde_json::json!({ "objects": [], "total": 0 })),
                input,
            );
            assert!(text_of(&r).contains("tidak ditemukan"), "input {input:?}");
        }
        let panjang = "a".repeat(5000);
        let r = run(Stub::with(serde_json::json!({ "objects": [], "total": 0 })), &panjang);
        assert!(text_of(&r).contains(&panjang.chars().take(200).collect::<String>()));
    }

    #[test]
    fn a_missing_total_field_prints_undefined() {
        let objects = vec![serde_json::json!({ "package": { "name": "x" } })];
        assert!(build_text(&objects, "q", None).contains("> Found: undefined packages"));
        assert!(build_text(&objects, "q", Some(&serde_json::json!(null))).contains("> Found: undefined packages"));
    }
}