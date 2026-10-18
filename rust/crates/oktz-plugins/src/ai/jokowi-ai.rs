//! `plugins/ai/jokowi-ai.js` — chat dengan Pak Jokowi (backend UnlimitedAI).
//!
//! Pemanggilan `UnlimitedAI` dan seluruh parsing-nya ada di
//! [`super::aiassist`] — helper dipakai bersama `aiassist`, `kobo-ai`, dan
//! `prabowo-ai` di direktori ini.
//!
//! `ponytail:` header `cookie`/`sec-ch-ua*`/`origin`/`referer` tidak bisa
//! dikirim (`HttpGet` tidak punya setter header), dan `UreqHttp` mengubah
//! status >= 400 jadi `Err` — jadi cabang `❌ *Jokowi AI Error*` hanya
//! terjangkau lewat implementasi `HttpGet` lain, bukan lewat `UreqHttp`.

use oktz_plugin_api::{Ctx, Reply, plugin};


use super::aiassist::{AiFailure, clip, failed, unlimited_ai};

/// `jokowi-ai.js:22-30`.
fn help(prefix: &str) -> String {
    format!(
        "🏛️ *Pak Jokowi*\n\n\
         > Pria Solo — Mantan Presiden RI\n\
         > Sederhana, bijak, dan suka blusukan\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}jokowi-ai <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}jokowi-ai Pak, gimana kabar?*"
    )
}

#[plugin(
    name = "jokowi-ai",
    alias = ["jokowiai", "jokowi", "pakjokowi"],
    category = "ai",
    description = "Chat dengan Pak Jokowi — Pria Solo",
    usage = ".jokowi-ai <pertanyaan>",
    example = ".jokowi-ai Pak, gimana kabar?",
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
pub fn jokowi_ai(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match unlimited_ai(&*ctx.http, "jokowi-ai", &text) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("Jokowi AI Error", &e)).with_reaction("☢")
        }
        Err(AiFailure::Transport) => Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
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
            command: "jokowi-ai".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("jokowi-ai").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("jokowi-ai").unwrap();
        assert_eq!(p.meta.name, "jokowi-ai");
        assert_eq!(p.meta.alias, ["jokowiai", "jokowi", "pakjokowi"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan Pak Jokowi — Pria Solo");
        assert_eq!(p.meta.usage, ".jokowi-ai <pertanyaan>");
        assert_eq!(p.meta.example, ".jokowi-ai Pak, gimana kabar?");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["jokowi-ai", "jokowiai", "jokowi", "pakjokowi"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("jokowi-ai"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let r = run(http.clone(), "");
        let t = body(&r);
        assert!(t.contains("Pria Solo — Mantan Presiden RI"), "{t}");
        assert!(t.contains("*.jokowi-ai <pertanyaan>*"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_jokowi_character_prompt_is_the_one_sent() {
        let http = Stub::ok("{\"type\":\"delta\",\"delta\":\"Sami-sami\"}\n");
        let r = run(http.clone(), "Pak, gimana kabar?");
        assert_eq!(body(&r), "Sami-sami");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        let system = sent["messages"][0]["content"].as_str().unwrap();
        assert!(system.starts_with("Kamu adalah Joko Widodo"), "{system}");
        assert!(system.ends_with("Pertanyaan user: Pak, gimana kabar?"), "{system}");
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template(".", "jokowi-ai"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}