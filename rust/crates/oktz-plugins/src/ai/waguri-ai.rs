//! `plugins/ai/waguri-ai.js` — chat dengan Waguri-san (backend UnlimitedAI).
//!
//! `UnlimitedAI` + parser-nya dipinjam dari [`super::aiassist`]; yang berbeda
//! hanya kartu penggunaan, nama karakter, dan judul pesan error.
//!
//! `ponytail:` header `cookie`/`sec-ch-ua*`/`origin`/`referer` tidak bisa
//! dikirim, dan status >= 400 jadi `Err` di `UreqHttp` — lihat `aiassist.rs`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::fun::tod::error_template;

use super::aiassist::{AiFailure, clip, failed, unlimited_ai};

/// `waguri-ai.js:23-30`.
fn help(prefix: &str) -> String {
    format!(
        "👓 *Waguri-san*\n\n\
         > Gadis pemalu dari \"The Girl I Like Forgot Her Glasses\"\n\
         > Manis, perhatian, dan sering salah tingkah~\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}waguri-ai <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}waguri-ai Waguri-san, halo!*"
    )
}

#[plugin(
    name = "waguri-ai",
    alias = ["waguriai", "waguri"],
    category = "ai",
    description = "Chat dengan Waguri-san — Gadis pemalu yang lupa kacamata",
    usage = ".waguri-ai <pertanyaan>",
    example = ".waguri-ai Waguri-san, halo!",
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
pub fn waguri_ai(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }
    match unlimited_ai(&*ctx.http, "waguri-ai", &text) {
        Ok(answer) => Reply::text(clip(&answer)).with_reaction("✅"),
        Err(AiFailure::Rejected(e)) => {
            Reply::text(failed("Waguri AI Error", &e)).with_reaction("☢")
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
        status: u16,
        seen: Mutex<Vec<(String, String)>>,
    }

    impl Stub {
        fn ok(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(body.to_string()),
                status: 200,
                seen: Mutex::new(Vec::new()),
            })
        }
        fn offline() -> Arc<Stub> {
            Arc::new(Stub {
                resp: None,
                status: 200,
                seen: Mutex::new(Vec::new()),
            })
        }
        /// Status di luar 2xx supaya cabang `!response.ok` di JS terjangkau.
        fn refused(body: &str) -> Arc<Stub> {
            Arc::new(Stub {
                resp: Some(body.to_string()),
                status: 403,
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
                status: self.status,
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
            command: "waguri-ai".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("waguri-ai").unwrap().handler)(ctx)
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
        let p = oktz_plugin_api::find("waguri-ai").unwrap();
        assert_eq!(p.meta.name, "waguri-ai");
        assert_eq!(p.meta.alias, ["waguriai", "waguri"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Chat dengan Waguri-san — Gadis pemalu yang lupa kacamata"
        );
        assert_eq!(p.meta.usage, ".waguri-ai <pertanyaan>");
        assert_eq!(p.meta.example, ".waguri-ai Waguri-san, halo!");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["waguri-ai", "waguriai", "waguri"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("waguri-ai"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        for text in ["", "   "] {
            let __balasan = run(http.clone(), text);
            let t = body(&__balasan);
            assert!(t.contains("👓 *Waguri-san*"), "{text:?}");
            assert!(t.contains("*.waguri-ai <pertanyaan>*"), "{text:?}");
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_waguri_character_prompt_is_the_one_sent() {
        let http = Stub::ok(
            "{\"type\":\"delta\",\"delta\":\"A-ano...\"}\n{\"type\":\"delta\",\"delta\":\"Gomen...\"}\n",
        );
        let r = run(http.clone(), "Waguri-san, halo!");
        assert_eq!(body(&r), "A-ano...Gomen...");
        assert_eq!(emoji(&r), "✅");

        let sent = http.seen.lock().unwrap()[0].1.clone();
        let v: Value = serde_json::from_str(&sent).unwrap();
        let system = v["messages"][0]["content"].as_str().unwrap();
        assert!(system.starts_with("Kamu adalah Waguri-san"), "{system}");
        assert!(
            system.ends_with("Pertanyaan user: Waguri-san, halo!"),
            "{system}"
        );
    }

    #[test]
    fn an_overlong_answer_is_clipped_to_4096_characters() {
        let big = "a".repeat(5000);
        let http = Stub::ok(&format!("{{\"type\":\"delta\",\"delta\":\"{big}\"}}\n"));
        let t = body(&run(http.clone(), "panjang")).to_string();
        assert_eq!(t.chars().count(), 4099, "4096 karakter + tiga titik");
        assert!(t.ends_with("..."));
    }

    #[test]
    fn a_rejected_response_uses_the_waguri_error_title() {
        let r = run(Stub::refused("- tamu"), "halo");
        assert_eq!(body(&r), "❌ *Waguri AI Error*\n\n> - tamu");
        assert_eq!(emoji(&r), "☢");
    }

    #[test]
    fn an_empty_rejection_falls_back_to_the_generic_line() {
        // `result.error || "Gagal mendapatkan respons"` — teks kosong pakai
        // kalimat bawaan.
        let r = run(Stub::refused(""), "halo");
        assert_eq!(
            body(&r),
            "❌ *Waguri AI Error*\n\n> Gagal mendapatkan respons"
        );
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), error_template(".", "waguri-ai"));
        assert_eq!(emoji(&r), "☢");
    }
}