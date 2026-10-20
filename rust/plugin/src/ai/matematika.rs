//! `plugins/ai/matematika.js` — AI pemecah soal matematika.
//!
//! `ponytail:` header `User-Agent` per-request (`matematika.js:35`) tidak bisa
//! dikirim — `HttpGet` tidak punya setter header. reacted `🕕` (`matematika.js:27`)
//! dilewati karena `Reply::Reaction` cuma menampung satu emoji; reaksi akhir
//! (`❌`/`✅`/`☢`) yang dikirim.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `matematika.js:30`.
const ENDPOINT: &str = "https://api.nexray.eu.cc/ai/mathgpt";

/// Nilai falsy JS (`!v`) — `matematika.js:39`.
fn falsy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        _ => false,
    }
}

/// `matematika.js:24`.
fn help(prefix: &str) -> String {
    format!(
        "📐 *ᴍᴀᴛʜ ɢᴘᴛ*\n\n\
         > Masukkan soal matematika\n\n\
         `Contoh: {prefix}matematika 2+2 berapa?`"
    )
}

#[plugin(
    name = "matematika",
    alias = ["mathgpt", "math", "mathsolver"],
    category = "ai",
    description = "AI untuk menyelesaikan soal matematika",
    usage = ".matematika <soal>",
    example = ".matematika 2+2 berapa?",
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
pub fn matematika(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let url = format!("{ENDPOINT}?text={}", encode_uri_component(&text));
    // `matematika.js:31` — kegagalan jaringan dan HTTP alike berakhir di
    // `catch`, jadi keduanya dibedakan dari respons `status: false`.
    let Some(body) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        return Reply::text(crate::common::error_template(&ctx)).with_reaction("☢");
    };

    if falsy(body.get("status")) || falsy(body.get("result")) {
        return Reply::text("⚠️ Gagal memproses soal matematika.").with_reaction("❌");
    }

    let answer = match body.get("result") {
        Some(Value::String(s)) => s.clone(),
        // `matematika.js:47` menempelkan nilai apa adanya lewat template literal.
        Some(other) => other.to_string(),
        None => String::new(),
    };
    Reply::text(answer).with_reaction("✅")
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
        seen: Mutex<Vec<String>>,
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
            self.seen.lock().unwrap().push(url.into());
            self.respond()
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("plugin ini hanya GET")
        }
    }

    fn run(http: Arc<Stub>, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "matematika".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("matematika").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("matematika").unwrap();
        assert_eq!(p.meta.name, "matematika");
        assert_eq!(p.meta.alias, ["mathgpt", "math", "mathsolver"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "AI untuk menyelesaikan soal matematika"
        );
        assert_eq!(p.meta.usage, ".matematika <soal>");
        assert_eq!(p.meta.example, ".matematika 2+2 berapa?");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["matematika", "mathgpt", "math", "mathsolver"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("matematika"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("📐 *ᴍᴀᴛʜ ɢᴘᴛ*"), "{t}");
        assert!(t.contains(".matematika 2+2 berapa?"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn falsiness_follows_javascript() {
        assert!(falsy(None));
        assert!(falsy(Some(&Value::Null)));
        assert!(falsy(Some(&Value::Bool(false))));
        assert!(falsy(Some(&Value::String(String::new()))));
        assert!(falsy(Some(&Value::from(0))));
        assert!(!falsy(Some(&Value::from(true))));
        assert!(!falsy(Some(&Value::from(1))));
        assert!(!falsy(Some(&Value::from("x"))));
        assert!(!falsy(Some(&Value::from(vec![1]))));
    }

    #[test]
    fn a_solved_question_is_answered_and_reacted() {
        let http = Stub::ok(r#"{"status":true,"result":"2+2 = 4"}"#);
        let r = run(http.clone(), "2+2 berapa?");
        assert_eq!(body(&r), "2+2 = 4");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0], format!("{ENDPOINT}?text=2%2B2%20berapa%3F"));
    }

    #[test]
    fn a_rejected_answer_shows_the_warning_and_the_red_cross() {
        for payload in [
            r#"{"status":false,"result":"4"}"#,
            r#"{"status":true,"result":""}"#,
            r#"{"status":true}"#,
        ] {
            let r = run(Stub::ok(payload), "soal");
            assert_eq!(body(&r), "⚠️ Gagal memproses soal matematika.", "{payload}");
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            ));
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "2+2 berapa?");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "matematika"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn a_non_json_body_lands_in_the_catch_branch() {
        let r = run(Stub::ok("<html>gateway</html>"), "soal");
        assert_eq!(body(&r), crate::common::error_template_bawaan(".", "matematika"));
    }
}