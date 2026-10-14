//! `plugins/ai/quillbot.js` — parafrasa teks dengan Quillbot.
//!
//! Nama plugin di JS memang `quilbot` (typo) dengan alias `quillbot`
//! (`quillbot.js:4-5`), jadi keduanya dipakai apa adanya supaya alias itu
//! tetap resolve.
//!
//! `ponytail:` header `User-Agent` per-request (`quillbot.js:32`) tidak bisa
//! dikirim — `HttpGet` tidak punya setter header. reacted `🕕`
//! (`quillbot.js:26`) dilewati karena `Reply::Reaction` cuma menampung satu
//! emoji; reaksi akhir (`❌`/`✅`/`☢`) yang dikirim.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `quillbot.js:29`.
const ENDPOINT: &str = "https://api.nexray.eu.cc/ai/quillbot";

/// Nilai falsy JS (`!v`) — `quillbot.js:38`.
fn falsy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => true,
        Some(Value::Bool(b)) => !b,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        _ => false,
    }
}

#[plugin(
    name = "quilbot",
    alias = ["quillbot", "parafrase"],
    category = "ai",
    description = "AI Quillbot untuk menulis ulang atau menyempurnakan kalimat",
    usage = ".quilbot <teks>",
    example = ".quilbot Saya sedang makan nasi di rumah",
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
pub fn quilbot(ctx: Ctx) -> Reply {
    // `quillbot.js:20` — `m.args.join(' ')` jatuh ke `m.text` kalau kosong.
    let text = if ctx.args.is_empty() {
        ctx.text.trim().to_string()
    } else {
        ctx.args.join(" ")
    };

    if text.is_empty() {
        return Reply::text(
            "❌ Masukkan teks yang ingin disempurnakan.\n\n\
             Contoh: `.quilbot Saya sedang makan nasi di rumah`",
        );
    }

    let url = format!("{ENDPOINT}?text={}", encode_uri_component(&text));
    // `quillbot.js:46-50` memakai pesan sendiri, bukan template error global.
    let Some(body) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        return Reply::text("😔 Terjadi kesalahan saat memproses teks ke Quillbot.").with_reaction("☢");
    };

    if falsy(body.get("status")) || falsy(body.get("result")) {
        return Reply::text("⚠️ Quillbot gagal memproses teks.").with_reaction("❌");
    }

    let answer = match body.get("result") {
        Some(Value::String(s)) => s.clone(),
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

    /// `text` dan `args` bisa diisi terpisah untuk menguji fallback `quillbot.js:20`.
    fn run(http: Arc<Stub>, text: &str, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "quilbot".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("quilbot").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("quilbot").unwrap();
        assert_eq!(p.meta.name, "quilbot", "typo nama di JS harus dipertahankan");
        assert_eq!(p.meta.alias, ["quillbot", "parafrase"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "AI Quillbot untuk menulis ulang atau menyempurnakan kalimat"
        );
        assert_eq!(p.meta.usage, ".quilbot <teks>");
        assert_eq!(p.meta.example, ".quilbot Saya sedang makan nasi di rumah");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn the_alias_is_what_users_type_while_the_name_stays_the_js_typo() {
        for k in ["quilbot", "quillbot", "parafrase"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("quilbot"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_text_asks_for_input_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "  ", "");
        let t = body(&__balasan);
        assert!(t.contains("❌ Masukkan teks yang ingin disempurnakan."), "{t}");
        assert!(t.contains("`.quilbot Saya sedang makan nasi di rumah`"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn args_win_and_the_raw_text_is_the_fallback() {
        let http = Stub::ok(r#"{"status":true,"result":"ok"}"#);
        run(http.clone(), "teks mentah", "satu dua");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?text=satu%20dua")
        );

        let http = Stub::ok(r#"{"status":true,"result":"ok"}"#);
        run(http.clone(), "  teks mentah  ", "");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?text=teks%20mentah")
        );
    }

    #[test]
    fn a_paraphrase_is_sent_and_reacted() {
        let http = Stub::ok(r#"{"status":true,"result":"Saya sedang makan nasi di rumah."}"#);
        let r = run(http.clone(), "", "Saya sedang makan nasi di rumah");
        assert_eq!(body(&r), "Saya sedang makan nasi di rumah.");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_rejected_answer_shows_the_quillbot_warning() {
        for payload in [
            r#"{"status":false,"result":"x"}"#,
            r#"{"status":true,"result":""}"#,
            r#"{"status":true}"#,
        ] {
            let r = run(Stub::ok(payload), "teks", "");
            assert_eq!(body(&r), "⚠️ Quillbot gagal memproses teks.", "{payload}");
            assert!(matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "❌"
            ));
        }
    }

    #[test]
    fn a_transport_failure_uses_its_own_message_not_the_error_template() {
        let r = run(Stub::offline(), "", "teks");
        assert_eq!(
            body(&r),
            "😔 Terjadi kesalahan saat memproses teks ke Quillbot."
        );
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn a_non_json_body_lands_in_the_catch_branch() {
        let r = run(Stub::ok("<html>gateway</html>"), "", "teks");
        assert_eq!(
            body(&r),
            "😔 Terjadi kesalahan saat memproses teks ke Quillbot."
        );
    }
}