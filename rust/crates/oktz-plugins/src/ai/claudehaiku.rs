//! `plugins/ai/claudehaiku.js` — Claude Haiku 4.5 via OverChat.
//!
//! File ini juga menjadi rumah helper OverChat untuk `gpt5.rs` — keduanya POST
//! ke `api.overchat.ai/v1/chat/completions` dengan bentuk body dan parser SSE
//! yang identik, hanya `model`/`personaId`/pesan sistem yang beda.
//!
//! `ponytail:` tiga hal tidak bisa di-port dan tidak ada padanannya di `Ctx`/
//! `Reply`, jadi tidak ditulis (bukan di shortcut):
//! 1. Header per-request (`x-device-uuid`, `sec-ch-ua*`, `origin`, `referer`).
//! 2. `saluranCtx()` (`src/lib/context.js:151`) yang ditempel sebagai
//!    `contextInfo` forwarded-newsletter — `Reply` tidak punya field context.
//! 3. `crypto.randomUUID()` diganti generator UUID v4 di [`super::aiassist`]
//!    (pola hex yang sama, sumber acaknya `crate::common::roll`).

use oktz_plugin_api::{Ctx, HttpGet, Reply, plugin};
use serde_json::{Value};


use super::aiassist::{AiFailure, clip, failed, uuid_v4};

/// `API` (`src/scraper/claudehaiku.js:3`).
const ENDPOINT: &str = "https://api.overchat.ai/v1/chat/completions";

const MODEL: &str = "claude-haiku-4-5-20251001";
const PERSONA: &str = "claude-haiku-4-5-landing";

/// Parser SSE OpenAI-compatible (`claudehaiku.js:84-101`).
///
/// `buffer.split("\n")` + `lines.pop()`: potongan terakhir dibuang kalau body
/// tidak diakhiri newline. Ditiru di sini.
pub(crate) fn sse_answer(raw: &str) -> String {
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut answer = String::new();
    for line in lines.iter().take(lines.len() - 1) {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if let Some(c) = v.pointer("/choices/0/delta/content").and_then(Value::as_str) {
            answer.push_str(c);
        }
    }
    answer
}

/// POST ke OverChat dan rakit `choices[0].delta.content` dari SSE.
pub(crate) fn overchat_chat(
    http: &dyn HttpGet,
    model: &str,
    persona_id: &str,
    prompt: &str,
    system: Option<&str>,
) -> Result<String, AiFailure> {
    let mut messages = vec![serde_json::json!({
        "id": uuid_v4(),
        "role": "user",
        "content": prompt,
    })];
    if let Some(s) = system {
        messages.push(serde_json::json!({ "id": uuid_v4(), "role": "system", "content": s }));
    }

    let body = serde_json::json!({
        "chatId": uuid_v4(),
        "model": model,
        "messages": messages,
        "personaId": persona_id,
        "frequency_penalty": 0,
        "max_tokens": 4000,
        "presence_penalty": 0,
        "stream": true,
        "temperature": 0.5,
        "top_p": 0.95,
    });

    let res = http.post_json(ENDPOINT, &body).map_err(|_| AiFailure::Transport)?;
    Ok(sse_answer(&res.text()))
}

/// `claudehaiku.js:25-32`.
fn help(prefix: &str) -> String {
    format!(
        "🤍 *Claude Haiku 4.5*\n\n\
         Tanya apa aja ke AI Claude Haiku — cepat dan ringan, cocok buat pertanyaan sehari-hari.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}claudehaiku <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}claudehaiku Jelaskan teori relativitas*\n\
         > *{prefix}claudehaiku Tips biar produktif*\n\n\
         _Respons cepat, tapi tetap cerdas_"
    )
}

#[plugin(
    name = "claudehaiku",
    alias = ["claude", "haiku", "chiku"],
    category = "ai",
    description = "Chat dengan Claude Haiku 4.5 via OverChat",
    usage = ".claudehaiku <pertanyaan>",
    example = ".claudehaiku Jelaskan teori relativitas",
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
pub fn claudehaiku(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    const SYSTEM: &str = "Ikuti bahasa user dan jawab dengan gaya natural, singkat, dan jelas.";
    match overchat_chat(&*ctx.http, MODEL, PERSONA, &text, Some(SYSTEM)) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("Claude Haiku Gagal", &e)).with_reaction("☢")
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
            command: "claudehaiku".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("claudehaiku").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("claudehaiku").unwrap();
        assert_eq!(p.meta.name, "claudehaiku");
        assert_eq!(p.meta.alias, ["claude", "haiku", "chiku"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan Claude Haiku 4.5 via OverChat");
        assert_eq!(p.meta.usage, ".claudehaiku <pertanyaan>");
        assert_eq!(p.meta.example, ".claudehaiku Jelaskan teori relativitas");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["claudehaiku", "claude", "haiku", "chiku"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("claudehaiku"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("*Claude Haiku 4.5*"), "{t}");
        assert!(t.contains("*.claudehaiku <pertanyaan>*"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn sse_deltas_are_joined_and_noise_is_skipped() {
        assert_eq!(
            sse_answer(
                "data: {\"choices\":[{\"delta\":{\"content\":\"Halo\"}}]}\n\
                 event: ping\n\
                 data: {\"choices\":[{\"delta\":{\"content\":\" dunia\"}}]}\n\
                 data: [DONE]\n\
                 data: bukan json\n"
            ),
            "Halo dunia"
        );
    }

    #[test]
    fn the_last_unterminated_line_is_dropped_like_the_js_buffer() {
        assert_eq!(
            sse_answer("data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\ndata: {\"choices\":[{\"delta\":{\"content\":\"b\"}}]}"),
            "a"
        );
    }

    #[test]
    fn a_normal_answer_reacts_and_sends_the_model_output() {
        let http = Stub::ok("data: {\"choices\":[{\"delta\":{\"content\":\"Relativitas\"}}]}\n");
        let r = run(http.clone(), "Jelaskan teori relativitas");
        assert_eq!(body(&r), "Relativitas");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["model"], MODEL);
        assert_eq!(sent["personaId"], PERSONA);
        assert_eq!(sent["stream"], true);
        assert_eq!(sent["messages"].as_array().unwrap().len(), 2);
        assert_eq!(sent["messages"][0]["role"], "user");
        assert_eq!(sent["messages"][1]["role"], "system");
        assert_eq!(sent["messages"][1]["content"], "Ikuti bahasa user dan jawab dengan gaya natural, singkat, dan jelas.");
    }

    #[test]
    fn an_overlong_answer_is_cut_with_an_ellipsis() {
        let big = "y".repeat(6000);
        let http = Stub::ok(&format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{big}\"}}}}]}}\n"));
        let __balasan = run(http.clone(), "panjang");
        let t = body(&__balasan);
        assert!(t.ends_with("..."));
        assert_eq!(t.chars().filter(|c| *c == 'y').count(), 4096);
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template(".", "claudehaiku"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}