//! `plugins/ai/kobo-ai.js` — chat dengan Kobo Kanaeru (backend UnlimitedAI).
//!
//! `UnlimitedAI` + parser-nya dipinjam dari [`super::aiassist`]; yang berbeda
//! hanya kartu penggunaan dan nama karakter.
//!
//! `ponytail:` header `cookie`/`sec-ch-ua*`/`origin`/`referer` tidak bisa
//! dikirim, dan status >= 400 jadi `Err` di `UreqHttp` — lihat `aiassist.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};


use super::aiassist::{AiFailure, clip, failed, unlimited_ai};

/// `kobo-ai.js:22-30`.
fn help(prefix: &str) -> String {
    format!(
        "🌬️ *Kobo Kanaeru*\n\n\
         > VTuber Hololive Indonesia Gen 3\n\
         > Wind Shaman yang cheerfull dan suka prank!\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}kobo-ai <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}kobo-ai Kobo lagi apa?*"
    )
}

#[plugin(
    name = "kobo-ai",
    alias = ["koboai", "kobo"],
    category = "ai",
    description = "Chat dengan Kobo Kanaeru — VTuber Hololive ID",
    usage = ".kobo-ai <pertanyaan>",
    example = ".kobo-ai Kobo lagi apa?",
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
pub fn kobo_ai(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match unlimited_ai(&*ctx.http, "kobo-ai", &text) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("Kobo AI Error", &e)).with_reaction("☢")
        }
        Err(AiFailure::Transport) => Reply::text(crate::common::error_template(&ctx))
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
                return Err(oktz_plugin_api::HttpError {
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
            command: "kobo-ai".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("kobo-ai").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("kobo-ai").unwrap();
        assert_eq!(p.meta.name, "kobo-ai");
        assert_eq!(p.meta.alias, ["koboai", "kobo"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Chat dengan Kobo Kanaeru — VTuber Hololive ID"
        );
        assert_eq!(p.meta.usage, ".kobo-ai <pertanyaan>");
        assert_eq!(p.meta.example, ".kobo-ai Kobo lagi apa?");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["kobo-ai", "koboai", "kobo"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("kobo-ai"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        for text in ["", "  "] {
            let __balasan = run(http.clone(), text);
            let t = body(&__balasan);
            assert!(t.contains("VTuber Hololive Indonesia Gen 3"), "{text:?}");
            assert!(t.contains("*.kobo-ai <pertanyaan>*"), "{text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_kobo_character_prompt_is_the_one_sent() {
        let http = Stub::ok("{\"type\":\"delta\",\"delta\":\"DAJOOR!\"}\n{\"type\":\"delta\",\"delta\":\"HMPH!\"}\n");
        let r = run(http.clone(), "Kobo lagi apa?");
        assert_eq!(body(&r), "DAJOOR!HMPH!");

        let sent = http.seen.lock().unwrap()[0].1.clone();
        let v: Value = serde_json::from_str(&sent).unwrap();
        let system = v["messages"][0]["content"].as_str().unwrap();
        assert!(system.starts_with("Kamu adalah Kobo Kanaeru"), "{system}");
        assert!(system.ends_with("Pertanyaan user: Kobo lagi apa?"), "{system}");
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "kobo-ai"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}