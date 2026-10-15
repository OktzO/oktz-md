//! `plugins/ai/muslimai.js` — tanya-jawab soal Islam dan Al-Quran.
//!
//! `_parse` (`muslimai.js:43-52`) memecah body NDJSON per baris dan menyambung
//! `p.data` hanya untuk `type === "text"`; kalau tidak ada satu pun potongan,
//! body mentah dikembalikan apa adanya.
//!
//! `ponytail:` reacted `🕕` (`muslimai.js:67`) dilewati karena `Reply::Reaction`
//! cuma menampung satu emoji; reaksi akhir (`✅`/`☢`) yang dikirim.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value, json};

use crate::fun::tod::error_template;

/// `muslimai.js:20`.
const ENDPOINT: &str = "https://www.muslimai.io/api/chat";

/// `muslimai.js:24-26` — id statis, bukan `distinctId` per pengguna.
const DISTINCT_ID: &str = "019e7d1d-e8a4-702d-96b8-defd87522114";

/// `String(v)` JS untuk nilai yang ditempel dengan `+=` (`muslimai.js:48`).
fn js_string(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Array(a)) => a.iter().map(|v| js_string(Some(v))).collect::<Vec<_>>().join(","),
        Some(Value::Object(_)) => "[object Object]".into(),
    }
}

/// `_parse` (`muslimai.js:43-52`).
fn parse(res: &str) -> String {
    let mut txt = String::new();
    for line in res.split('\n') {
        // `catch {}` di JS: baris yang bukan JSON (termasuk baris kosong) dilewati.
        let Ok(p) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if p.get("type").and_then(Value::as_str) == Some("text") {
            txt.push_str(&js_string(p.get("data")));
        }
    }
    if txt.is_empty() { res.to_string() } else { txt }
}

/// `muslimai.js:64`.
fn help(prefix: &str) -> String {
    format!(
        "☪️ *ᴍᴜsʟɪᴍ ᴀɪ*\n\n\
         > Masukkan pertanyaan tentang Islam\n\n\
         `Contoh: {prefix}muslimai Apa itu sholat?`"
    )
}

#[plugin(
    name = "muslimai",
    alias = ["islamai", "quranai"],
    category = "ai",
    description = "AI untuk bertanya tentang Islam dan Al-Quran",
    usage = ".muslimai <pertanyaan>",
    example = ".muslimai Apa itu sholat?",
    cooldown = 5,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn muslimai(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `_body` (`muslimai.js:28-33`) — `fetch`, bukan axios, jadi status HTTP
    // tidak diperiksa; `HttpGet` justru mengembalikan `Err` untuk >=400 dan
    // itu masuk `catch`, sama seperti `fetch` yang melempar untuk status itu.
    let body = json!({ "query": text, "distinctId": DISTINCT_ID });
    let Ok(res) = ctx.http.post_json(ENDPOINT, &body) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    Reply::text(parse(&res.text())).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use oktz_plugin_api::HttpGet;
    use serde_json::Value;
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
                return Err(HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(text.clone().into_bytes()),
                    mimetype: Some("application/x-ndjson".into()),
                },
                headers: vec![],
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
        }
        fn post_json(&self, url: &str, body: &Value) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push((url.into(), body.to_string()));
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

    fn run(http: Arc<Stub>, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "muslimai".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("muslimai").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("muslimai").unwrap();
        assert_eq!(p.meta.name, "muslimai");
        assert_eq!(p.meta.alias, ["islamai", "quranai"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "AI untuk bertanya tentang Islam dan Al-Quran");
        assert_eq!(p.meta.usage, ".muslimai <pertanyaan>");
        assert_eq!(p.meta.example, ".muslimai Apa itu sholat?");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["muslimai", "islamai", "quranai"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("muslimai"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("☪️ *ᴍᴜsʟɪᴍ ᴀɪ*"), "{t}");
        assert!(t.contains(".muslimai Apa itu sholat?"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn text_chunks_are_concatenated_in_order() {
        // `res.split("\n")` (`muslimai.js:45`) — tiap baris harus JSON utuh,
        // jadi antar chunk wajib ada newline.
        let raw = concat!(
            r#"{"type":"text","data":"Sholat adalah "}"#,
            "\n",
            r#"{"type":"tool","data":"diabaikan"}"#,
            "\n",
            r#"{"type":"text","data":"tiang lima waktu."}"#,
            "\n"
        );
        let http = Stub::ok(raw);
        let r = run(http.clone(), "Apa itu sholat?");
        assert_eq!(body(&r), "Sholat adalah tiang lima waktu.");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["query"], "Apa itu sholat?");
        assert_eq!(sent["distinctId"], DISTINCT_ID);
    }

    #[test]
    fn a_body_without_text_chunks_is_returned_verbatim() {
        let raw = "<html>gateway</html>";
        assert_eq!(parse(raw), raw, "body mentah jadi cadangan");
        assert_eq!(parse(""), "");
        assert_eq!(parse(r#"{"type":"tool","data":"x"}"#), r#"{"type":"tool","data":"x"}"#);
    }

    #[test]
    fn non_json_lines_are_skipped_not_fatal() {
        let raw = "bukan json\n{\"type\":\"text\",\"data\":\"a\"}\n\nlagi\n{\"type\":\"text\",\"data\":\"b\"}\n";
        assert_eq!(parse(raw), "ab");
    }

    #[test]
    fn string_coercion_follows_javascript_for_missing_data() {
        assert_eq!(parse("{\"type\":\"text\"}"), "undefined");
        assert_eq!(parse("{\"type\":\"text\",\"data\":null}"), "null");
        assert_eq!(parse("{\"type\":\"text\",\"data\":5}"), "5");
        assert_eq!(parse("{\"type\":\"text\",\"data\":{}}"), "[object Object]");
        assert_eq!(parse("{\"type\":\"text\",\"data\":[1,2]}"), "1,2");
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "Apa itu sholat?");
        assert_eq!(body(&r), error_template(".", "muslimai"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}