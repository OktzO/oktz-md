//! `plugins/ai/feelbetter.js` — chat dengan FeelBetterBot.
//!
//! `ponytail:` header `cookie: feelbet-memory=…` tidak bisa dikirim
//! (`HttpGet` tidak punya setter header per-request), jadi `makeMemoryId()`
//! (`src/scraper/feeb.js:11-18`) sengaja tidak ditulis — id-nya cuma dipakai
//! untuk header itu. `saluranCtx()` juga tidak ada padanannya di `Reply`.

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};
use serde_json::{Value};


use super::aiassist::{AiFailure, clip, failed};

/// `API` (`src/scraper/feeb.js:3`).
const ENDPOINT: &str = "https://feelbetterbot.com/";

const SYSTEM: &str = "Kamu adalah asisten AI yang dibuat oleh Ditzzx. Ikuti bahasa yang digunakan user dalam percakapan. Jika user memakai bahasa Indonesia, jawab dalam bahasa Indonesia yang natural, santai, jelas, dan mudah dipahami. Jangan tiba-tiba pindah bahasa kecuali user memintanya. Jika user bertanya siapa pembuatmu, penciptamu, developermu, atau siapa yang membuatmu, jawab bahwa pembuatmu adalah Ditzzx.";

const GREETING: &str = "Hi, I'm FeelBetterBot — I'm here to listen and help you carry whatever feels heavy, without judgment. I draw on gentle, proven ways of work through hard things, but mostly I just want to understand what you're going through. So, how are you doing right now?";

/// `parseChunk` (`feeb.js:20-41`).
///
/// `JSON.parse` gagal → `catch` mengembalikan `data` mentah, bukan string kosong.
fn parse_chunk(line: &str) -> String {
    let mut data = line.trim();
    if data.is_empty() || data == "[DONE]" {
        return String::new();
    }
    if let Some(rest) = data.strip_prefix("data:") {
        data = rest.trim();
    }
    if data.is_empty() || data == "[DONE]" {
        return String::new();
    }
    match serde_json::from_str::<Value>(data) {
        Ok(v) => {
            if let Some(s) = v.as_str() {
                return s.to_string();
            }
            for key in ["content", "text", "delta", "message", "response", "answer"] {
                if let Some(s) = v.get(key).and_then(Value::as_str) {
                    return s.to_string();
                }
            }
            v.pointer("/choices/0/delta/content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        }
        Err(_) => data.to_string(),
    }
}

/// `FeelBetter(prompt)` (`src/scraper/feeb.js:43-108`).
fn feel_better(http: &dyn HttpGet, prompt: &str) -> Result<String, AiFailure> {
    let body = serde_json::json!({
        "messages": [
            { "role": "system", "content": SYSTEM },
            { "role": "assistant", "content": GREETING },
            { "role": "user", "content": prompt },
        ],
    });
    let res = http.post_json(ENDPOINT, &body).map_err(|_| AiFailure::Transport)?;
    let raw = res.text();
    // `feeb.js:90-93` — `lines.pop()` menyisakan potongan terakhir di buffer,
    // dan `feeb.js:103-106` memproses sisa itu kalau tidak kosong.
    let mut parts: Vec<&str> = raw.split('\n').collect();
    let buffer = parts.pop().unwrap_or("");
    let mut answer: String = parts.iter().map(|l| parse_chunk(l)).collect();
    if !buffer.trim().is_empty() {
        answer.push_str(&parse_chunk(buffer));
    }
    Ok(answer)
}

/// `feelbetter.js:24-33`.
fn help(prefix: &str) -> String {
    format!(
        "💚 *FeelBetterBot*\n\n\
         AI yang siap mendengarkan curhatan kamu — tanpa menghakimi, dengan hangat dan empatik.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}feelbetter <curhatan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}feelbetter lagi sedih nih*\n\
         > *{prefix}feelbetter aku capek banget belakangan*\n\n\
         _Bot ini bukan pengganti profesional, tapi bisa jadi tempat curhat yang aman_"
    )
}

#[plugin(
    name = "feelbetter",
    alias = ["feelbetterbot", "healing"],
    category = "ai",
    description = "Chat dengan FeelBetterBot — AI yang siap mendengarkan tanpa menghakimi",
    usage = ".feelbetter <curhat/pertanyaan>",
    example = ".feelbetter lagi sedih nih",
    cooldown = 10,
    energi = 2,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn feelbetter(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match feel_better(&*ctx.http, &text) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("FeelBetter Gagal", &e)).with_reaction("☢")
        }
        Err(AiFailure::Transport) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢"),
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

    fn run(http: Arc<Stub>, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "feelbetter".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("feelbetter").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("feelbetter").unwrap();
        assert_eq!(p.meta.name, "feelbetter");
        assert_eq!(p.meta.alias, ["feelbetterbot", "healing"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Chat dengan FeelBetterBot — AI yang siap mendengarkan tanpa menghakimi"
        );
        assert_eq!(p.meta.usage, ".feelbetter <curhat/pertanyaan>");
        assert_eq!(p.meta.example, ".feelbetter lagi sedih nih");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["feelbetter", "feelbetterbot", "healing"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("feelbetter"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("*FeelBetterBot*"), "{t}");
        assert!(t.contains("*.feelbetter <curhatan>*"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn every_known_chunk_shape_is_recognised() {
        for line in [
            r#"{"content":"a"}"#,
            r#"{"text":"a"}"#,
            r#"{"delta":"a"}"#,
            r#"{"message":"a"}"#,
            r#"{"response":"a"}"#,
            r#"{"answer":"a"}"#,
            r#"{"choices":[{"delta":{"content":"a"}}]}"#,
            r#""a""#,
        ] {
            assert_eq!(parse_chunk(line), "a", "baris {line} tidak dikenali");
        }
    }

    #[test]
    fn noise_is_dropped_but_unparsable_text_is_kept_verbatim() {
        assert_eq!(parse_chunk(""), "");
        assert_eq!(parse_chunk("   "), "");
        assert_eq!(parse_chunk("[DONE]"), "");
        assert_eq!(parse_chunk("data: [DONE]"), "");
        assert_eq!(parse_chunk(r#"data: {"content":"a"}"#), "a");
        assert_eq!(parse_chunk("bukan json"), "bukan json");
        assert_eq!(parse_chunk(r#"{"nope":1}"#), "");
    }

    #[test]
    fn a_normal_answer_is_joined_reacted_and_sent() {
        let http = Stub::ok(
            "{\"content\":\"Aku \"}\ndata: {\"delta\":\"dengar\"}\ndata: [DONE]\n",
        );
        let r = run(http.clone(), "lagi sedih nih");
        assert_eq!(body(&r), "Aku dengar");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        let msgs = sent["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3, "sistem + sapaan + user");
        assert_eq!(msgs[0]["role"], "system");
        assert_eq!(msgs[1]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "lagi sedih nih");
    }

    #[test]
    fn a_trailing_chunk_without_newline_is_still_counted() {
        let http = Stub::ok("{\"content\":\"a\"}\n{\"content\":\"b\"}");
        assert_eq!(body(&run(http.clone(), "x")), "ab", "sisa buffer ikut diproses");
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template(".", "feelbetter"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}