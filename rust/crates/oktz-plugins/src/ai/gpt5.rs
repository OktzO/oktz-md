//! `plugins/ai/gpt5.js` — GPT-4.1 Nano via OverChat.
//!
//! POST + parser SSE-nya dipinjam dari [`super::claudehaiku`] (endpoint,
//! bentuk body, dan parsing identik); yang berbeda hanya `model`, `personaId`,
//! dan tidak adanya pesan sistem.
//!
//! `ponytail:` header per-request dan `salunanCtx()` tidak punya padanan di
//! `Ctx`/`Reply` — lihat catatan di `claudehaiku.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::fun::tod::error_template;

use super::aiassist::{AiFailure, clip, failed};
use super::claudehaiku::overchat_chat;

/// `src/scraper/gpt5.js:11,30`.
const MODEL: &str = "openai/gpt-4.1-nano-2025-04-14";
const PERSONA: &str = "gpt-4o-landing";

/// `gpt5.js:25-33`.
fn help(prefix: &str) -> String {
    format!(
        "🤖 *GPT-4.1 Nano*\n\n\
         Tanya apa aja ke AI, nanti dijawab pakai model GPT-4.1 Nano.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}gpt5 <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}gpt5 Apa itu quantum computing?*\n\
         > *{prefix}gpt5 Buat puisi tentang Indonesia*\n\n\
         _Jawaban bisa agak lama, sabar ya_"
    )
}

#[plugin(
    name = "gpt5",
    alias = ["gpt5nano", "gpt41"],
    category = "ai",
    description = "Chat dengan GPT-4.1 Nano via OverChat",
    usage = ".gpt5 <pertanyaan>",
    example = ".gpt5 Apa itu quantum computing?",
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
pub fn gpt5(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match overchat_chat(&*ctx.http, MODEL, PERSONA, &text, None) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("GPT-5 Gagal", &e)).with_reaction("☢")
        }
        Err(AiFailure::Transport) => Reply::text(error_template(&ctx.prefix, &ctx.command))
            .with_reaction("☢"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::Value;
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
            command: "gpt5".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("gpt5").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gpt5").unwrap();
        assert_eq!(p.meta.name, "gpt5");
        assert_eq!(p.meta.alias, ["gpt5nano", "gpt41"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan GPT-4.1 Nano via OverChat");
        assert_eq!(p.meta.usage, ".gpt5 <pertanyaan>");
        assert_eq!(p.meta.example, ".gpt5 Apa itu quantum computing?");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["gpt5", "gpt5nano", "gpt41"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("gpt5"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "   ");
        let t = body(&__balasan);
        assert!(t.contains("`GPT-4.1 Nano`"), "{t}");
        assert!(t.contains("*.gpt5 <pertanyaan>*"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_nano_model_is_requested_with_a_single_user_message() {
        let http = Stub::ok("data: {\"choices\":[{\"delta\":{\"content\":\"Quantum\"}}]}\n");
        let r = run(http.clone(), "Apa itu quantum computing?");
        assert_eq!(body(&r), "Quantum");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].0.starts_with("https://api.overchat.ai/"));
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["model"], MODEL);
        assert_eq!(sent["personaId"], PERSONA);
        let msgs = sent["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 1, "gpt5.js tidak menambah pesan sistem");
        assert_eq!(msgs[0]["role"], "user");
        assert_eq!(msgs[0]["content"], "Apa itu quantum computing?");
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), error_template(".", "gpt5"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}