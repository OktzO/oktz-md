//! `plugins/ai/qwen3.js` — chat dengan Qwen3 80B via OverChat.
//!
//! Scraper `Qwen3` (`src/scraper/qwen3.js`) memakai endpoint, model,
//! `personaId`, dan bentuk body yang sama persis dengan
//! `src/scraper/claudehaiku.js`, jadi parser SSE-nya dipinjam dari
//! [`super::claudehaiku`] lewat `overchat_chat`.
//!
//! `ponytail:` tidak ditulis:
//! 1. Header per-request (`x-device-uuid`, `sec-ch-ua*`, `origin`, `referer`)
//!    — `HttpGet` tidak punya setter header.
//! 2. `saluranCtx()` (`qwen3.js:52`) yang ditempel sebagai `contextInfo`
//!    forwarded-newsletter — `Reply` tidak punya field context.
//! 3. Cabang `status:false` (`qwen3.js:41-46`) praktis tidak terjangkau:
//!    `UreqHttp` sudah mengubah status >= 400 jadi `Err`, jadi yang tampil
//!    template error `catch`, bukan `❌ *Qwen3 Gagal*`.

use oktz_plugin_api::{Ctx, Reply, plugin};


use super::aiassist::{AiFailure, clip, failed};
use super::claudehaiku::overchat_chat;

const MODEL: &str = "alibaba/qwen3-next-80b-a3b-instruct";
const PERSONA: &str = "qwen-3-landing";

/// Pesan sistem yang dipisah sendiri di `qwen3.js:27-31`.
const SYSTEM: &str = "Ikuti bahasa user dan jawab dengan gaya natural, singkat, dan jelas.";

/// `qwen3.js:24-33`.
fn help(prefix: &str) -> String {
    format!(
        "🔵 *Qwen3 80B*\n\n\
         Tanya apa aja ke AI Qwen3 — model besar dari Alibaba yang jago bahasa apa aja.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}qwen3 <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}qwen3 Apa itu machine learning?*\n\
         > *{prefix}qwen3 Buat resep masakan Indonesia*\n\n\
         _Model 80B, jadi agak lama tapi jawabannya mantap_"
    )
}

#[plugin(
    name = "qwen3",
    alias = ["qwen", "qw3"],
    category = "ai",
    description = "Chat dengan Qwen3 80B via OverChat",
    usage = ".qwen3 <pertanyaan>",
    example = ".qwen3 Apa itu machine learning?",
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
pub fn qwen3(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    match overchat_chat(&*ctx.http, MODEL, PERSONA, &text, Some(SYSTEM)) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => Reply::text(failed("Qwen3 Gagal", &e)).with_reaction("☢"),
        Err(AiFailure::Transport) => {
            Reply::text(crate::common::error_template(&ctx)).with_reaction("☢")
        }
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
        fn get(&self, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya POST JSON")
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

    fn sse(chunks: &[&str]) -> String {
        let mut out = String::new();
        for c in chunks {
            out.push_str(&format!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":\"{c}\"}}}}]}}\n"
            ));
        }
        out
    }

    fn run(http: Arc<Stub>, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "qwen3".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("qwen3").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("qwen3").unwrap();
        assert_eq!(p.meta.name, "qwen3");
        assert_eq!(p.meta.alias, ["qwen", "qw3"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan Qwen3 80B via OverChat");
        assert_eq!(p.meta.usage, ".qwen3 <pertanyaan>");
        assert_eq!(p.meta.example, ".qwen3 Apa itu machine learning?");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["qwen3", "qwen", "qw3"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("qwen3"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_lists_the_examples_without_calling_the_api() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "   ");
        let t = body(&__balasan);
        assert!(t.contains("🔵 *Qwen3 80B*"), "{t}");
        assert!(t.contains("*.qwen3 Apa itu machine learning?*"), "{t}");
        assert!(
            t.contains("_Model 80B, jadi agak lama tapi jawabannya mantap_"),
            "{t}"
        );
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_request_carries_the_model_the_persona_and_the_system_prompt() {
        let http = Stub::ok(&sse(&["Halo"]));
        let r = run(http.clone(), "apa itu machine learning");
        assert_eq!(body(&r), "Halo");

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["model"], MODEL);
        assert_eq!(sent["personaId"], PERSONA);
        assert_eq!(sent["stream"], true);
        assert_eq!(sent["messages"][0]["role"], "user");
        assert_eq!(sent["messages"][0]["content"], "apa itu machine learning");
        assert_eq!(sent["messages"][1]["role"], "system");
        assert_eq!(sent["messages"][1]["content"], SYSTEM);
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_long_answer_is_cut_at_4096_characters_with_an_ellipsis() {
        let long = "a".repeat(5000);
        let http = Stub::ok(&sse(&[&long]));
        let r = run(http.clone(), "panjang");
        let t = body(&r);
        assert!(t.ends_with("..."), "{}", &t[t.len().saturating_sub(8)..]);
        assert_eq!(t.chars().filter(|c| *c == 'a').count(), 4096);
    }

    #[test]
    fn a_transport_failure_falls_back_to_the_shared_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "qwen3"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn a_rejected_response_uses_the_qwen3_failure_card() {
        // `UreqHttp` mengubah status >= 400 jadi `Err`, jadi jalur yang tampil
        // nyata adalah template error. Teks kartu `Qwen3 Gagal` tetap diuji
        // di sini supaya tidak ikut hilang diam-diam.
        assert_eq!(failed("Qwen3 Gagal", "502"), "❌ *Qwen3 Gagal*\n\n> 502");
        assert_eq!(
            failed("Qwen3 Gagal", ""),
            "❌ *Qwen3 Gagal*\n\n> Gagal mendapatkan respons"
        );
    }
}
