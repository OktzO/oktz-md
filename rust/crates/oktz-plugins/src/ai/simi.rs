//! `plugins/ai/simi.js` — ngobrol santai bareng SimiSimi.
//!
//! `ponytail:` header `User-Agent` per-request (`simi.js:33`) tidak bisa
//! dikirim — `HttpGet` tidak punya setter header. reacted `🕕` (`simi.js:26`)
//! dilewati karena `Reply::Reaction` hanya menampung satu emoji; reaksi
//! terakhir (`❌`/`✅`/`☢`) yang ikut.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;
use crate::tools::phisingataubukan::encode_uri_component;

const ENDPOINT: &str = "https://api.nexray.eu.cc/ai/simisimi";

#[plugin(
    name = "simi",
    alias = ["simisimi"],
    category = "ai",
    description = "Ngobrol santai bareng SimiSimi",
    usage = ".simi <pesan>",
    example = ".simi Halo Simi!",
    cooldown = 3,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn simi(ctx: Ctx) -> Reply {
    // `simi.js:20` — `m.args.join(" ")` dulu, `m.text` cuma fallback.
    let text = if ctx.args.is_empty() {
        ctx.text.trim().to_string()
    } else {
        ctx.args.join(" ")
    };

    if text.is_empty() {
        return Reply::text("❌ Mau ngobrol apa sama Simi?\n\nContoh: `.simi Halo Simi!`");
    }

    let url = format!("{ENDPOINT}?text={}", encode_uri_component(&text));
    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text("😔 Simi gagal membalas pesanmu.").with_reaction("☢");
    };
    // `axios` `silentJSONParsing` aktif secara default: body non-JSON tidak
    // melempar, tapi dipakai apa adanya sebagai string. Jadi `data.status`
    // tetap `undefined` dan jatuh ke cabang "ngambek" di bawah.
    let data = res.json().unwrap_or(Value::Null);

    // `simi.js:38` — `!data.status || !data.result`.
    if !js_truthy(data.get("status")) || !js_truthy(data.get("result")) {
        return Reply::text("⚠️ Simi lagi ngambek, nggak mau balas.").with_reaction("❌");
    }

    let answer = match data.get("result") {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
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

    fn run(http: Arc<Stub>, text: &str, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "simi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("simi").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("simi").unwrap();
        assert_eq!(p.meta.name, "simi");
        assert_eq!(p.meta.alias, ["simisimi"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Ngobrol santai bareng SimiSimi");
        assert_eq!(p.meta.usage, ".simi <pesan>");
        assert_eq!(p.meta.example, ".simi Halo Simi!");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["simi", "simisimi"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("simi"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_message_asks_what_to_say_without_calling_the_api() {
        let http = Stub::ok("");
        for (text, args) in [("", ""), ("   ", ""), ("", "  ")] {
            let r = run(http.clone(), text, args);
            assert_eq!(
                body(&r),
                "❌ Mau ngobrol apa sama Simi?\n\nContoh: `.simi Halo Simi!`",
                "text={text:?} args={args:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn args_win_and_the_trimmed_text_is_the_fallback() {
        let http = Stub::ok(r#"{"status":true,"result":"hai"}"#);
        let _ = run(http.clone(), "teks mentah", "Halo Simi!");
        // `encodeURIComponent` tidak menyentuh `!'()*~`, jadi `!` tetap apa adanya.
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?text=Halo%20Simi!")
        );

        let http = Stub::ok(r#"{"status":true,"result":"hai"}"#);
        let _ = run(http.clone(), "  Halo Simi!  ", "");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?text=Halo%20Simi!")
        );
    }

    #[test]
    fn a_normal_answer_is_reacted_with_a_check() {
        let http = Stub::ok(r#"{"status":true,"result":"Hai juga!"}"#);
        let r = run(http.clone(), "", "Halo");
        assert_eq!(body(&r), "Hai juga!");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_rejected_or_empty_result_shows_the_stuck_warning() {
        for payload in [
            r#"{"status":false,"result":"x"}"#,
            r#"{"status":true,"result":""}"#,
            r#"{"status":true}"#,
            "bukan json",
        ] {
            let r = run(Stub::ok(payload), "", "Halo");
            assert_eq!(
                body(&r),
                "⚠️ Simi lagi ngambek, nggak mau balas.",
                "{payload}"
            );
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            ));
        }
    }

    #[test]
    fn a_transport_failure_uses_its_own_message_not_the_error_template() {
        let r = run(Stub::offline(), "", "Halo");
        assert_eq!(body(&r), "😔 Simi gagal membalas pesanmu.");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}
