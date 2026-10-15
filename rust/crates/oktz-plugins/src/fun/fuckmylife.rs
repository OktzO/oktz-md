//! `plugins/fun/fuckmylife.js` — cerita FML random dari `api.neoxr.eu`.
//!
//! `ponytail:` `Reply::Reaction` hanya bisa membawa satu emoji, sedangkan JS
//! bereaksi `🕕` lebih dulu lalu menggantinya dengan `❌`/`✅`/`☢`. Yang ikut
//! hanya reaksi terakhir.
//!
//! `ponytail:` `config.APIkey?.neoxr` (`config.js:327` = `process.env
//! .APIKEY_NEOXR || ""`) tidak dibawa `Ctx`, jadi kuncinya dikunci kosong —
//! pola yang sama dipakai `info/jadwalbola.rs` dan `ai/anime-gen.rs`.
//!
//! `ponytail:` `Ctx` tidak membawa `m.pushName`, jadi `{pushName}` pada template
//! error selalu jatuh ke `User` seperti `pushName || 'User'` di JS saat
//! `pushName` kosong.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;

/// `config.APIkey?.neoxr || ''` (`fuckmylife.js:4`).
const NEOXR_APIKEY: &str = "";

/// `❌ Gagal mengambil FML story` (`fuckmylife.js:30`).
const GAGAL: &str = "❌ Gagal mengambil FML story";

fn url() -> String {
    format!("https://api.neoxr.eu/api/fml?apikey={NEOXR_APIKEY}")
}

/// `te(prefix, command, pushName)` (`src/lib/error.js`).
///
/// `config.js:153` mengeset `errorTemplate`, jadi template bawaan di
/// `error.js` praktis tidak pernah terpakai. `||` pada `prefix` dan `command`
/// tetap ditulis karena `Ctx` mengizinkan keduanya kosong, sedangkan string
/// non-kosong apa adanya adalah satu-satunya nilai truthy yang mungkin.
pub(crate) fn error_reply(prefix: &str, command: &str) -> String {
    let prefix = if prefix.is_empty() { "." } else { prefix };
    let command = if command.is_empty() { "?" } else { command };
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

/// `data.data.text` — string dipakai apa adanya, nilai lain dirender seperti
/// `${value}` di JS (pola yang sama dipakai `common::joined_strings`).
pub(crate) fn text_value(v: &Value) -> String {
    v.as_str().map_or_else(|| v.to_string(), str::to_string)
}

#[plugin(
    name = "fuckmylife",
    alias = ["fml"],
    category = "fun",
    description = "Random FML story",
    usage = ".fuckmylife",
    example = ".fuckmylife",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn fuckmylife(ctx: Ctx) -> Reply {
    // `data?.status` memakai optional chaining, jadi `null` dari `f()` tidak
    // melempar — justru masuk ke cabang `❌ Gagal` di bawah.
    let payload = ctx.http.get(&url()).ok().and_then(|r| r.json());

    let text = payload
        .as_ref()
        .filter(|d| js_truthy(d.get("status")))
        .and_then(|d| d.get("data"))
        .and_then(|d| d.get("text"))
        .filter(|t| js_truthy(Some(*t)));

    match text {
        Some(t) => Reply::text(text_value(t)).with_reaction("✅"),
        None => Reply::text(GAGAL).with_reaction("❌"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{
        HttpBody, HttpError, HttpGet, HttpResponse, find, new_session_store,
    };
    use std::sync::{Arc, Mutex};

    /// Stub HTTP: satu body JSON untuk semua URL, atau `Err` kalau `None`.
    #[derive(Debug)]
    struct Stub {
        body: Option<&'static str>,
        seen: Mutex<Vec<String>>,
    }

    impl Stub {
        fn json(body: &'static str) -> Arc<Stub> {
            Arc::new(Stub {
                body: Some(body),
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                body: None,
                seen: Mutex::new(Vec::new()),
            })
        }
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            let Some(body) = self.body else {
                return Err(HttpError {
                    message: "offline".into(),
                });
            };
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(body.as_bytes()),
                    mimetype: Some("application/json".into()),
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unimplemented!("cuma GET")
        }
    }

    fn run(http: Arc<Stub>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fuckmylife".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            sessions: new_session_store(),
            http,
            ..Default::default()
        };
        (find("fuckmylife").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    fn reaction_of(r: &Reply) -> String {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => emoji.clone(),
            other => panic!("harus dibungkus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let m = &find("fuckmylife").unwrap().meta;
        assert_eq!(m.name, "fuckmylife");
        assert_eq!(m.alias, ["fml"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Random FML story");
        assert_eq!(m.usage, ".fuckmylife");
        assert_eq!(m.example, ".fuckmylife");
        assert_eq!(m.cooldown, 5);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        assert_eq!(find("fml").unwrap().meta.name, "fuckmylife");
    }

    #[test]
    fn a_successful_response_is_replied_verbatim_and_reacts_success() {
        let r = run(Stub::json(r#"{"status":true,"data":{"text":"FML hari ini"}}"#));
        assert_eq!(text_of(&r), "FML hari ini");
        assert_eq!(reaction_of(&r), "✅");
    }

    #[test]
    fn the_text_is_sent_untouched_even_with_markdown_and_newlines() {
        let body = r#"{"status":true,"data":{"text":"*a* _b_\n> c `d` ✨"}}"#;
        assert_eq!(text_of(&run(Stub::json(body))), "*a* _b_\n> c `d` ✨");
    }

    #[test]
    fn an_offline_request_falls_into_the_failed_branch_not_the_error_template() {
        // JS pakai `data?.status`, jadi `f()` yang mengembalikan `null` tidak
        // melempar — ia masuk cabang `❌ Gagal`, bukan `catch`.
        let r = run(Stub::offline());
        assert_eq!(text_of(&r), "❌ Gagal mengambil FML story");
        assert_eq!(reaction_of(&r), "❌");
    }

    #[test]
    fn missing_status_data_or_text_all_fail_the_same_way() {
        for body in [
            r#"{}"#,
            r#"{"status":false,"data":{"text":"x"}}"#,
            r#"{"status":0,"data":{"text":"x"}}"#,
            r#"{"status":true}"#,
            r#"{"status":true,"data":null}"#,
            r#"{"status":true,"data":{}}"#,
            r#"{"status":true,"data":{"text":""}}"#,
            r#"{"status":true,"data":{"text":null}}"#,
            r#"bukan json"#,
        ] {
            let r = run(Stub::json(body));
            assert_eq!(text_of(&r), "❌ Gagal mengambil FML story", "body {body}");
            assert_eq!(reaction_of(&r), "❌", "body {body}");
        }
    }

    #[test]
    fn the_request_url_is_the_neoxr_fml_endpoint_with_an_empty_key() {
        let http = Stub::json(r#"{"status":true,"data":{"text":"x"}}"#);
        let _ = run(http.clone());
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            ["https://api.neoxr.eu/api/fml?apikey="]
        );
    }

    #[test]
    fn the_error_template_keeps_the_js_fallbacks_for_empty_prefix_and_command() {
        // `prefix || '.'` dan `command || '?'` dari `error.js`.
        assert!(error_reply("", "").contains("command `.?`"), "prefix/command kosong");
        assert!(
            error_reply("!", "fml").contains("command `!fml`"),
            "prefix non-kosong dipakai apa adanya"
        );
        assert!(error_reply(".", "fml").ends_with("_Jika masalah berlanjut, silahkan hubungi owner bot_"));
    }
}