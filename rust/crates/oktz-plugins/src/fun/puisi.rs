//! `plugins/fun/puisi.js` — puisi random dari `api.neoxr.eu`.
//!
//! `ponytail:` `Reply::Reaction` hanya bisa membawa satu emoji, sedangkan JS
//! bereaksi `🕕` lebih dulu lalu menggantinya dengan `❌`/`✅`/`☢`. Yang ikut
//! hanya reaksi terakhir.
//!
//! `ponytail:` `config.APIkey?.neoxr` (`config.js:327` = `process.env
//! .APIKEY_NEOXR || ""`) tidak dibawa `Ctx`, jadi kuncinya dikunci kosong.
//!
//! `ponytail:` `Ctx` tidak membawa `m.pushName`, jadi `{pushName}` pada template
//! error selalu jatuh ke `User`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::js_truthy;
use crate::fun::fuckmylife::{error_reply, text_value};

/// `config.APIkey?.neoxr || ''` (`puisi.js:4`).
const NEOXR_APIKEY: &str = "";

/// `❌ Gagal mengambil puisi` (`puisi.js:30`).
const GAGAL: &str = "❌ Gagal mengambil puisi";

fn url() -> String {
    format!("https://api.neoxr.eu/api/puisi?apikey={NEOXR_APIKEY}")
}

#[plugin(
    name = "puisi",
    alias = ["puisiku", "sajak"],
    category = "fun",
    description = "Random puisi Indonesia",
    usage = ".puisi",
    example = ".puisi",
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
pub fn puisi(ctx: Ctx) -> Reply {
    // `f()` mengembalikan `null` untuk status >= 400 maupun request yang gagal
    // (`src/lib/http.js:44-59`), sedangkan baris berikutnya menulis
    // `res.status` TANPA `?.` — jadi `null` melempar TypeError dan masuk
    // `catch`. Berbeda dari `fuckmylife.js` yang pakai `data?.status`.
    let payload = match ctx.http.get(&url()) {
        Ok(res) => match res.json() {
            Some(v) => v,
            None => return error_branch(&ctx),
        },
        Err(_) => return error_branch(&ctx),
    };

    // `!res.status || !res.data?.text`
    if !js_truthy(payload.get("status")) {
        return Reply::text(GAGAL).with_reaction("❌");
    }
    let Some(text) = payload
        .get("data")
        .and_then(|d| d.get("text"))
        .filter(|t| js_truthy(Some(*t)))
    else {
        return Reply::text(GAGAL).with_reaction("❌");
    };

    Reply::text(text_value(text)).with_reaction("✅")
}

fn error_branch(ctx: &Ctx) -> Reply {
    Reply::text(error_reply(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{
        HttpBody, HttpError, HttpGet, HttpResponse, find, new_session_store,
    };
    use serde_json::Value;
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
            command: "puisi".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            sessions: new_session_store(),
            http,
            ..Default::default()
        };
        (find("puisi").unwrap().handler)(ctx)
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
        let m = &find("puisi").unwrap().meta;
        assert_eq!(m.name, "puisi");
        assert_eq!(m.alias, ["puisiku", "sajak"]);
        assert_eq!(m.category, "fun");
        assert_eq!(m.description, "Random puisi Indonesia");
        assert_eq!(m.usage, ".puisi");
        assert_eq!(m.example, ".puisi");
        assert_eq!(m.cooldown, 5);
        assert_eq!(m.energi, 0);
        assert!(!m.is_private && !m.is_group && !m.is_owner && !m.is_premium);
        for a in ["puisiku", "sajak"] {
            assert_eq!(find(a).unwrap().meta.name, "puisi", "alias {a}");
        }
    }

    #[test]
    fn a_successful_response_is_replied_verbatim_and_reacts_success() {
        let body = r#"{"status":true,"data":{"text":"Rindu itu,\nmenyulam malam ☾\n  tanpa bintang."}}"#;
        let r = run(Stub::json(body));
        assert_eq!(
            text_of(&r),
            "Rindu itu,\nmenyulam malam ☾\n  tanpa bintang."
        );
        assert_eq!(reaction_of(&r), "✅");
    }

    #[test]
    fn an_offline_request_reaches_the_catch_branch_with_the_config_template() {
        // `res.status` tanpa `?.` pada `null` = TypeError -> `catch` -> `te()`.
        let r = run(Stub::offline());
        assert!(text_of(&r).starts_with("☢ Kayaknya command `.puisi`"), "{r:?}");
        assert!(
            text_of(&r).ends_with("_Jika masalah berlanjut, silahkan hubungi owner bot_"),
            "{}",
            text_of(&r)
        );
        assert_eq!(reaction_of(&r), "☢");
    }

    #[test]
    fn a_non_json_body_also_reaches_the_catch_branch() {
        let r = run(Stub::json("<html>error</html>"));
        assert!(text_of(&r).starts_with("☢ Kayaknya command `.puisi`"));
        assert_eq!(reaction_of(&r), "☢");
    }

    #[test]
    fn missing_status_data_or_text_take_the_failed_branch() {
        for body in [
            r#"{}"#,
            r#"{"status":false,"data":{"text":"x"}}"#,
            r#"{"status":null,"data":{"text":"x"}}"#,
            r#"{"status":true}"#,
            r#"{"status":true,"data":null}"#,
            r#"{"status":true,"data":{}}"#,
            r#"{"status":true,"data":{"text":""}}"#,
        ] {
            let r = run(Stub::json(body));
            assert_eq!(text_of(&r), "❌ Gagal mengambil puisi", "body {body}");
            assert_eq!(reaction_of(&r), "❌", "body {body}");
        }
    }

    #[test]
    fn the_request_url_is_the_neoxr_puisi_endpoint_with_an_empty_key() {
        let http = Stub::json(r#"{"status":true,"data":{"text":"x"}}"#);
        let _ = run(http.clone());
        assert_eq!(
            http.seen.lock().unwrap().as_slice(),
            ["https://api.neoxr.eu/api/puisi?apikey="]
        );
    }
}