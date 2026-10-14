//! `plugins/ai/prabowo-ai.js` — chat dengan Pak Prabowo (backend UnlimitedAI).
//!
//! `UnlimitedAI` + parser-nya dipinjam dari [`super::aiassist`]; yang berbeda
//! hanya kartu penggunaan dan nama karakter.
//!
//! `ponytail:` header `cookie`/`sec-ch-ua*`/`origin`/`referer` tidak bisa
//! dikirim, dan status >= 400 jadi `Err` di `UreqHttp` — lihat `aiassist.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::fun::tod::error_template;

use super::aiassist::{AiFailure, clip, failed, unlimited_ai};

/// `prabowo-ai.js:22-30`.
fn help(prefix: &str) -> String {
    format!(
        "🇮🇩 *Pak Prabowo*\n\n\
         > Pria Sawit — Presiden RI\n\
         > Tegas, patriotik, dan karismatik\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}prabowo-ai <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}prabowo-ai Saudara, kita harus berdaulat!*"
    )
}

#[plugin(
    name = "prabowo-ai",
    alias = ["prabowoi", "prabowo", "pakprabowo"],
    category = "ai",
    description = "Chat dengan Pak Prabowo — Pria Sawit",
    usage = ".prabowo-ai <pertanyaan>",
    example = ".prabowo-ai Saudara, kita harus berdaulat!",
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
pub fn prabowo_ai(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match unlimited_ai(&*ctx.http, "prabowo-ai", &text) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("Prabowo AI Error", &e)).with_reaction("☢")
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
                    mimetype: Some("application/json".into()),
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
            command: "prabowo-ai".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("prabowo-ai").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("prabowo-ai").unwrap();
        assert_eq!(p.meta.name, "prabowo-ai");
        assert_eq!(p.meta.alias, ["prabowoi", "prabowo", "pakprabowo"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan Pak Prabowo — Pria Sawit");
        assert_eq!(p.meta.usage, ".prabowo-ai <pertanyaan>");
        assert_eq!(p.meta.example, ".prabowo-ai Saudara, kita harus berdaulat!");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["prabowo-ai", "prabowoi", "prabowo", "pakprabowo"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("prabowo-ai"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("Pria Sawit — Presiden RI"), "{t}");
        assert!(t.contains("*.prabowo-ai <pertanyaan>*"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_prabowo_character_prompt_is_the_one_sent() {
        let http = Stub::ok("{\"type\":\"delta\",\"delta\":\"Saudara-saudara!\"}\n");
        let r = run(http.clone(), "Saudara, kita harus berdaulat!");
        assert_eq!(body(&r), "Saudara-saudara!");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let sent = http.seen.lock().unwrap()[0].1.clone();
        let v: Value = serde_json::from_str(&sent).unwrap();
        let system = v["messages"][0]["content"].as_str().unwrap();
        assert!(
            system.starts_with("Kamu adalah Prabowo Subianto"),
            "{system}"
        );
        assert!(system.ends_with("Pertanyaan user: Saudara, kita harus berdaulat!"));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), error_template(".", "prabowo-ai"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}