//! `plugins/ai/gita.js` — chat dengan Gita GPT (Bhagavad Gita AI).
//!
//! `gita.js:28` memanggil `fetchAiText('gita', …)`
//! (`src/lib/stalker-fallback.js:265`), yang mencoba cuki lalu nexray.
//! Cabang cuki butuh `config.APIkey.cuki`, yang tidak dibawa `Ctx`; di JS tanpa
//! key isian cuki selalu melempar `MissingApiKeyError`, jadi nexray adalah
//! jalur yang benar-benar tembus. Hanya nexray yang ditulis di sini.
//!
//! `ponytail:` reacted `🕕` (`gita.js:25`) dilewati karena `Reply::Reaction`
//! cuma menampung satu emoji; reaksi akhir (`✅`/`☢`) yang dikirim.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::fun::tod::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `NEXRAY_BASES[0]` + `nexrayPaths.gita` (`stalker-fallback.js:4,267,298`).
const ENDPOINT: &str = "https://api.nexray.eu.cc/ai/gitagpt";

/// Nilai falsy JS (`!v`) — `stalker-fallback.js:299`.
fn falsy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        _ => false,
    }
}

/// `gita.js:22`.
fn help(prefix: &str) -> String {
    format!(
        "📿 *ɢɪᴛᴀ ɢᴘᴛ*\n\n\
         > Masukkan pertanyaan\n\n\
         `Contoh: {prefix}gita What is dharma?`"
    )
}

#[plugin(
    name = "gita",
    alias = ["gitagpt", "bhagavadgita"],
    category = "ai",
    description = "Chat dengan Gita GPT (Bhagavad Gita AI)",
    usage = ".gita <pertanyaan>",
    example = ".gita What is dharma?",
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
pub fn gita(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let url = format!("{ENDPOINT}?text={}", encode_uri_component(&text));
    // `gita.js:33-36` — semua kegagalan provider (termasuk cuki yang key-nya
    // kosong) berakhir di satu `catch` yang sama, jadi pesannya sama.
    let Some(body) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    if falsy(body.get("status")) || falsy(body.get("result")) {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    }

    let answer = match body.get("result") {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(other) => other.to_string().trim().to_string(),
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
            command: "gita".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("gita").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("gita").unwrap();
        assert_eq!(p.meta.name, "gita");
        assert_eq!(p.meta.alias, ["gitagpt", "bhagavadgita"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan Gita GPT (Bhagavad Gita AI)");
        assert_eq!(p.meta.usage, ".gita <pertanyaan>");
        assert_eq!(p.meta.example, ".gita What is dharma?");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["gita", "gitagpt", "bhagavadgita"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("gita"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("📿 *ɢɪᴛᴀ ɢᴘᴛ*"), "{t}");
        assert!(t.contains("`.gita What is dharma?`"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn only_the_nexray_provider_is_reachable() {
        let http = Stub::ok(r#"{"status":true,"result":" Dharma is duty. "}"#);
        let r = run(http.clone(), "What is dharma?");
        assert_eq!(body(&r), "Dharma is duty.", "hasil harus di-trim");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "cuki tidak boleh dipanggil");
        assert_eq!(seen[0], format!("{ENDPOINT}?text=What%20is%20dharma%3F"));
        assert!(!seen[0].starts_with("https://api.cuki.biz.id"), "cuki tak bisa dipakai");
    }

    #[test]
    fn an_empty_provider_answer_is_still_sent_as_the_nexray_result() {
        // `stalker-fallback.js:300-301` menolak `result` kosong sebelum trim,
        // jadi string spasi ikut lolos sebagai-isinya.
        let r = run(Stub::ok(r#"{"status":true,"result":"   "}"#), "dharma");
        assert_eq!(body(&r), "");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_rejected_answer_reports_the_error_template() {
        for payload in [
            r#"{"status":false,"result":"x"}"#,
            r#"{"status":true,"result":""}"#,
            r#"{"status":true}"#,
        ] {
            let r = run(Stub::ok(payload), "dharma");
            assert_eq!(body(&r), error_template(".", "gita"), "{payload}");
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            ));
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "dharma");
        assert_eq!(body(&r), error_template(".", "gita"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}