//! `plugins/ai/wormgpt.js` — chat dengan WormGPT (uncensored AI).
//!
//! `wormgpt.js:32` memanggil `fetchAiText('wormgpt', …)`
//! (`src/lib/stalker-fallback.js:265`), yang mencoba cuki lalu nexray.
//! Cabang cuki butuh `config.APIkey.cuki`, yang tidak dibawa `Ctx`; di JS tanpa
//! key isian cuki selalu melempar `MissingApiKeyError`, jadi nexray adalah
//! jalur yang benar-benar tembus. Hanya nexray yang ditulis di sini.
//!
//! `ponytail:` reacted `🕕` (`wormgpt.js:29`) dilewati karena `Reply::Reaction`
//! cuma menampung satu emoji; reaksi akhir (`✅`/`☢`) yang dikirim.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::js_truthy;
use crate::fun::tod::error_template;
use crate::tools::phisingataubukan::encode_uri_component;

/// `NEXRAY_BASES[0]` + `nexrayPaths.wormgpt` (`stalker-fallback.js:4,268,298`).
const ENDPOINT: &str = "https://api.nexray.eu.cc/ai/chatgpt";

/// `wormgpt.js:24-26`.
fn help(prefix: &str) -> String {
    format!(
        "🐛 *ᴡᴏʀᴍ ɢᴘᴛ*\n\n\
         > Masukkan pertanyaan\n\n\
         `Contoh: {prefix}wormgpt perkenalkan dirimu`"
    )
}

#[plugin(
    name = "wormgpt",
    alias = ["worm"],
    category = "ai",
    description = "Chat dengan WormGPT (uncensored AI)",
    usage = ".wormgpt <pertanyaan>",
    example = ".wormgpt perkenalkan dirimu",
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
pub fn wormgpt(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let url = format!("{ENDPOINT}?text={}", encode_uri_component(&text));
    // Semua kegagalan provider (termasuk cuki yang key-nya kosong) berakhir di
    // satu `catch` yang sama di `wormgpt.js:36-40`, jadi pesannya sama.
    let Some(body) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    if !js_truthy(body.get("status")) || !js_truthy(body.get("result")) {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    }

    let answer = match body.get("result") {
        Some(serde_json::Value::String(s)) => s.trim().to_string(),
        Some(other) => other.to_string().trim().to_string(),
        None => String::new(),
    };
    Reply::text(answer).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
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
        fn post_json(&self, _: &str, _: &serde_json::Value) -> Result<HttpResponse, HttpError> {
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
            command: "wormgpt".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("wormgpt").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    fn emoji(r: &Reply) -> &str {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji,
            other => panic!("harus dibungkus reaksi, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("wormgpt").unwrap();
        assert_eq!(p.meta.name, "wormgpt");
        assert_eq!(p.meta.alias, ["worm"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan WormGPT (uncensored AI)");
        assert_eq!(p.meta.usage, ".wormgpt <pertanyaan>");
        assert_eq!(p.meta.example, ".wormgpt perkenalkan dirimu");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["wormgpt", "worm"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("wormgpt"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("🐛 *ᴡᴏʀᴍ ɢᴘᴛ*"), "{t}");
        assert!(t.contains(".wormgpt perkenalkan dirimu"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn only_the_nexray_provider_is_reachable_and_the_answer_is_trimmed() {
        let http = Stub::ok(r#"{"status":true,"result":" Aku WormGPT. "}"#);
        let r = run(http.clone(), "perkenalkan dirimu");
        assert_eq!(body(&r), "Aku WormGPT.", "hasil harus di-trim");
        assert_eq!(emoji(&r), "✅");

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1, "cuki tidak boleh dipanggil");
        assert_eq!(
            seen[0],
            // `encodeURIComponent` mengubah tiap spasi jadi `%20` (`stalker-fallback.js:298`).
        format!("{ENDPOINT}?text=perkenalkan%20dirimu")
        );
        assert!(!seen[0].starts_with("https://api.cuki.biz.id"));
    }

    #[test]
    fn a_question_with_symbols_is_percent_encoded() {
        let http = Stub::ok(r#"{"status":true,"result":"ok"}"#);
        run(http.clone(), "apa itu AI?");
        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen[0], format!("{ENDPOINT}?text=apa%20itu%20AI%3F"));
    }

    #[test]
    fn a_non_string_result_is_stringified_like_js() {
        let http = Stub::ok(r#"{"status":true,"result":42}"#);
        assert_eq!(body(&run(http.clone(), "x")), "42");
    }

    #[test]
    fn a_rejected_answer_reports_the_error_template() {
        for payload in [
            r#"{"status":false,"result":"x"}"#,
            r#"{"status":true,"result":""}"#,
            r#"{"status":true}"#,
            r#"bukan json"#,
        ] {
            let r = run(Stub::ok(payload), "pertanyaan");
            assert_eq!(body(&r), error_template(".", "wormgpt"), "{payload}");
            assert_eq!(emoji(&r), "☢");
        }
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "pertanyaan");
        assert_eq!(body(&r), error_template(".", "wormgpt"));
        assert_eq!(emoji(&r), "☢");
    }
}