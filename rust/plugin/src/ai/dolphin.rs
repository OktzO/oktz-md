//! `plugins/ai/dolphin.js` — Dolphin AI 24B (chat.dphn.ai).
//!
//! Scraper `dolphinAI` ada di dalam file plugin sendiri, jadi tidak ada
//! helper bersama yang perlu dipinjam.
//!
//! `ponytail:` header `origin`, `referer`, dan `user-agent` khusus tidak bisa
//! dikirim (`HttpGet` tidak punya setter header per-request).

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};
use serde_json::{Value};


const ENDPOINT: &str = "https://chat.dphn.ai/api/chat";

/// `TEMPLATES` (`dolphin.js:19`).
const TEMPLATES: [&str; 5] = [
    "logical",
    "creative",
    "summarize",
    "code-beginner",
    "code-advanced",
];

/// `text.match(/^--(\S+)\s+/)` lalu `text.replace(match[0], '').trim()`
/// (`dolphin.js:70-77`).
///
/// Mengembalikan `(template, sisaPertanyaan)`. `\s+` serakah jadi seluruh
/// spasi setelah nama ikut terpotong; `\S+` terlalu sedikit (huruf/karakter
/// angka) tidak dianggap template yang sah dan teksnya dibiarkan utuh.
fn split_template(text: &str) -> (Option<&'static str>, &str) {
    let Some(rest) = text.strip_prefix("--") else {
        return (None, text);
    };
    let Some(end) = rest.find(char::is_whitespace) else {
        return (None, text);
    };
    let requested = rest[..end].to_lowercase();
    let Some(known) = TEMPLATES.iter().find(|t| **t == requested.as_str()) else {
        return (None, text);
    };
    (Some(*known), rest[end..].trim())
}

/// `dolphinAI` (`dolphin.js:21-46`).
///
/// `data.split('\n\n').filter(l => l.startsWith('data: {'))` lalu
/// `JSON.parse(l.substring(6))`. Parse yang gagal melempar keluar dari
/// `.map` di JS — jadi di sini juga jadi `Err`, bukan dilewati diam-diam.
fn dolphin_ai(http: &dyn HttpGet, question: &str, template: &str) -> Result<String, String> {
    let body = serde_json::json!({
        "messages": [{ "role": "user", "content": question }],
        "model": "dolphinserver:24B",
        "template": template,
    });
    let res = http.post_json(ENDPOINT, &body).map_err(|e| e.message)?;
    let raw = res.text();
    let mut result = String::new();
    for chunk in raw.split("\n\n") {
        if !chunk.starts_with("data: {") {
            continue;
        }
        let v: Value = serde_json::from_str(&chunk[6..]).map_err(|e| e.to_string())?;
        if let Some(c) = v.pointer("/choices/0/delta/content").and_then(Value::as_str) {
            result.push_str(c);
        }
    }
    if result.is_empty() {
        return Err("Tidak ada respon dari AI".to_string());
    }
    Ok(result)
}

/// `dolphin.js:53-65`.
fn help(prefix: &str) -> String {
    format!(
        "🐬 *ᴅᴏʟᴘʜɪɴ ᴀɪ*\n\n\
         > Chat dengan Dolphin AI 24B Model\n\n\
         ╭┈┈⬡「 📋 *ᴛᴇᴍᴘʟᴀᴛᴇs* 」\n\
         ┃ • `logical` - Jawaban logis\n\
         ┃ • `creative` - Jawaban kreatif\n\
         ┃ • `summarize` - Ringkasan\n\
         ┃ • `code-beginner` - Kode pemula\n\
         ┃ • `code-advanced` - Kode lanjutan\n\
         ╰┈┈┈┈┈┈┈┈⬡\n\n\
         > *Contoh:*\n\
         > {prefix}dolphin apa itu AI?\n\
         > {prefix}dolphin --creative buat puisi"
    )
}

#[plugin(
    name = "dolphin",
    alias = ["dolphinai", "dphn"],
    category = "ai",
    description = "Chat dengan Dolphin AI (24B Model)",
    usage = ".dolphin <pertanyaan> atau .dolphin --<template> <pertanyaan>",
    example = ".dolphin jelaskan tentang AI",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn dolphin(ctx: Ctx) -> Reply {
    let raw = ctx.text.trim();
    if raw.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let (template, question) = split_template(raw);
    if question.is_empty() {
        return Reply::text("❌ Masukkan pertanyaan!");
    }

    match dolphin_ai(&*ctx.http, question, template.unwrap_or("logical")) {
        Ok(answer) => Reply::text(answer).with_reaction("✅"),
        Err(_) => Reply::text(crate::common::error_template(&ctx)).with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpResponse};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        resp: Option<String>,
        seen: Mutex<Vec<(String, String)>>,
    }

    impl Stub {
        fn ok(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(body.to_string()),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                resp: None,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn respond(&self) -> Result<HttpResponse, HttpError> {
            let Some(text) = &self.resp else {
                return Err(oktz_plugin_api::HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.clone().into_bytes()),
                    mimetype: Some("text/event-stream".into()),
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push((url.into(), String::new()));
            self.respond()
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen
                .lock()
                .unwrap()
                .push((url.into(), body.to_string()));
            self.respond()
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "dolphin".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("dolphin").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    fn chunk(content: &str) -> String {
        format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{content}\"}}}}]}}")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("dolphin").unwrap();
        assert_eq!(p.meta.name, "dolphin");
        assert_eq!(p.meta.alias, ["dolphinai", "dphn"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan Dolphin AI (24B Model)");
        assert_eq!(
            p.meta.usage,
            ".dolphin <pertanyaan> atau .dolphin --<template> <pertanyaan>"
        );
        assert_eq!(p.meta.example, ".dolphin jelaskan tentang AI");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["dolphin", "dolphinai", "dphn"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("dolphin"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_lists_every_template() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "   ");
        let t = body(&__balasan);
        for name in TEMPLATES {
            assert!(t.contains(&format!("`{name}`")), "{name} hilang: {t}");
        }
        // `dolphin.js:63-64` tidak memakai `*` di sekitar contoh perintah.
        assert!(t.contains("> .dolphin --creative buat puisi"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn a_known_template_prefix_is_split_off_case_insensitively() {
        assert_eq!(
            split_template("--CREATIVE   buat puisi"),
            (Some("creative"), "buat puisi")
        );
        assert_eq!(
            split_template("--code-beginner x"),
            (Some("code-beginner"), "x")
        );
        assert_eq!(split_template("apa itu AI?"), (None, "apa itu AI?"));
    }

    #[test]
    fn an_unknown_or_incomplete_prefix_leaves_the_text_untouched() {
        assert_eq!(split_template("--ngawur soal"), (None, "--ngawur soal"));
        assert_eq!(split_template("--creative"), (None, "--creative"));
        assert_eq!(split_template("--"), (None, "--"));
        assert_eq!(split_template("-creative x"), (None, "-creative x"));
    }

    #[test]
    fn a_bare_template_never_matches_because_serialize_trims_first() {
        // `serialize.js:407` sudah `.trim()` sebelum handler jalan, dan
        // `/^--(\S+)\s+/` butuh spasi setelah nama. Jadi `.dolphin --creative`
        // tidak pernah kena cabang `❌ Masukkan pertanyaan!` (`dolphin.js:79-81`)
        // — teks utuh justru dikirim ke API.
        let http = Stub::ok(&format!("{}\n\n", chunk("Tekscreative")));
        let r = run(http.clone(), "--creative   ");
        assert_eq!(body(&r), "Tekscreative");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["messages"][0]["content"], "--creative");
        assert_eq!(sent["template"], "logical");
    }

    #[test]
    fn the_default_template_is_logical_and_is_sent_with_the_model_name() {
        let http = Stub::ok(&format!("{}\n\n", chunk("AI itu")));
        let r = run(http.clone(), "apa itu AI?");
        assert_eq!(body(&r), "AI itu");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["template"], "logical");
        assert_eq!(sent["model"], "dolphinserver:24B");
        assert_eq!(sent["messages"][0]["content"], "apa itu AI?");
    }

    #[test]
    fn a_chosen_template_reaches_the_api_and_the_prefix_is_stripped() {
        let http = Stub::ok(&format!("{}\n\n", chunk("Puisi")));
        let _ = run(http.clone(), "--creative  buat puisi");
        let sent: Value = serde_json::from_str(&http.seen.lock().unwrap()[0].1).unwrap();
        assert_eq!(sent["template"], "creative");
        assert_eq!(sent["messages"][0]["content"], "buat puisi");
    }

    #[test]
    fn a_stream_without_content_is_the_same_error_the_js_throws() {
        let http = Stub::ok("data: {\"choices\":[]}\n\n");
        let r = run(http.clone(), "x");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "dolphin"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "dolphin"));
    }
}