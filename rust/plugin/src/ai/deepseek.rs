//! `plugins/ai/deepseek.js` — DeepSeek V4 thinking/reasoning (notegpt.io).
//!
//! `ponytail:` header `Cookie` (analytics `_ga`/`_gid`/`sbox-guid` +
//! `anonymous_user_id`) tidak bisa dikirim, jadi `makeCookieHeader`,
//! `makeSboxGuid`, dan `randomNumber` (`src/scraper/deepseek.js:13-37`)
//! sengaja tidak ditulis — kodenya hanya dipakai untuk header itu. `history`
//! juga selalu kosong: satu-satunya pemanggil tidak pernah mengirimnya.
//!
//! `ponytail:` JS memakai `validateStatus: () => true`, jadi respons 4xx/5xx
//! tetap dibaca dan biasanya berakhir sebagai `success:false`. `UreqHttp`
//! membalik itu jadi `Err`, sehingga jalur 403 yang paling mungkin terjadi
//! jatuh ke template error `catch` — bukan `❌ *DeepSeek Gagal*`.

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value};


use super::aiassist::uuid_v4;

/// `BASE` + path (`src/scraper/deepseek.js:4,95`).
const ENDPOINT: &str = "https://notegpt.io/api/v2/chat/stream";

/// `reasoningPreview` — `deepseek.js:50-54`.
const REASONING_PREVIEW: usize = 800;

/// `reply.slice(0, 4096) + "\n\n... (dipotong)"` (`deepseek.js:61-63`).
const MAX_LEN: usize = 4096;

/// `parseSSE` (`src/scraper/deepseek.js:52-74`).
///
/// Bedanya dari parser OverChat: `history_messages` selalu kosong dan
/// `rawBody.split(/\r?\n/)` memecah SELURUH body — tidak ada `lines.pop()`,
/// jadi potongan terakhir yang tidak diakhiri newline tetap ikut dibaca.
fn parse_sse(raw: &str) -> (String, String) {
    let mut answer = String::new();
    let mut reasoning = String::new();
    for line in raw.split('\n').map(str::trim) {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        if let Some(r) = v.get("reasoning").and_then(Value::as_str) {
            reasoning.push_str(r);
        }
        if let Some(t) = v.get("text").and_then(Value::as_str) {
            answer.push_str(t);
        }
        // `if (json.done) break` — JS truthy, jadi `false`/null/0 tidak break.
        match v.get("done") {
            None | Some(Value::Null) | Some(Value::Bool(false)) => {}
            _ => break,
        }
    }
    (answer, reasoning)
}

/// `reply += "💭 *Proses Berpikir:*\n" + preview.replace(/\n/g, "\n> ") + "\n\n"`
/// (`deepseek.js:49-55`).
fn reasoning_block(reasoning: &str) -> String {
    let preview: String = if reasoning.chars().count() > REASONING_PREVIEW {
        let head: String = reasoning.chars().take(REASONING_PREVIEW).collect();
        format!("{head}...")
    } else {
        reasoning.to_string()
    };
    format!(
        "💭 *Proses Berpikir:*\n{}\n\n",
        preview.replace('\n', "\n> ")
    )
}

/// `deepseek.js:24-32`.
fn help(prefix: &str) -> String {
    format!(
        "🧠 *DeepSeek V4*\n\n\
         AI yang bisa mikir dulu sebelum jawab — cocok buat pertanyaan yang butuh penalaran.\n\n\
         *PENGGUNAAN:*\n\
         > *{prefix}deepseek <pertanyaan>*\n\n\
         *CONTOH:*\n\
         > *{prefix}deepseek Jelaskan black hole*\n\
         > *{prefix}deepseek Buat kode sorting algorithm*\n\n\
         _Bot akan mikir dulu, baru jawab — jadi agak lama sedikit_"
    )
}

#[plugin(
    name = "deepseek",
    alias = ["ds", "dsv4", "deepthink"],
    category = "ai",
    description = "Chat dengan DeepSeek V4 (thinking/reasoning)",
    usage = ".deepseek <pertanyaan>",
    example = ".deepseek Jelaskan black hole",
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
pub fn deepseek(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    let body = serde_json::json!({
        "message": text,
        "language": "auto",
        "model": "deepseek-v4-flash",
        "tone": "default",
        "length": "moderate",
        "conversation_id": uuid_v4(),
        "image_urls": [],
        "history_messages": [],
        "chat_mode": "deep_think",
    });

    let raw = match ctx.http.post_json(ENDPOINT, &body) {
        Ok(r) => r.text(),
        Err(_) => {
            return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
                .with_reaction("☢");
        }
    };
    let (answer, reasoning) = parse_sse(&raw);
    if answer.is_empty() && reasoning.is_empty() {
        return Reply::text("❌ *DeepSeek Gagal*\n\n> Gagal mendapatkan respons").with_reaction("☢");
    }

    let mut reply = String::new();
    if !reasoning.is_empty() {
        reply.push_str(&reasoning_block(&reasoning));
    }
    reply.push_str(&answer);
    if reply.chars().count() > MAX_LEN {
        let head: String = reply.chars().take(MAX_LEN).collect();
        reply = format!("{head}\n\n... (dipotong)");
    }
    Reply::text(reply).with_reaction("✅")
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
                    mimetype: Some("text/event-stream".into()),
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
            command: "deepseek".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: args.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("deepseek").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("deepseek").unwrap();
        assert_eq!(p.meta.name, "deepseek");
        assert_eq!(p.meta.alias, ["ds", "dsv4", "deepthink"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.description, "Chat dengan DeepSeek V4 (thinking/reasoning)");
        assert_eq!(p.meta.usage, ".deepseek <pertanyaan>");
        assert_eq!(p.meta.example, ".deepseek Jelaskan black hole");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["deepseek", "ds", "dsv4", "deepthink"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("deepseek"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_question_shows_the_usage_card_without_a_request() {
        let http = Stub::ok("");
        let __balasan = run(http.clone(), "");
        let t = body(&__balasan);
        assert!(t.contains("*DeepSeek V4*"), "{t}");
        assert!(t.contains("*.deepseek <pertanyaan>*"), "{t}");
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn reasoning_and_text_are_collected_separately() {
        let (a, r) = parse_sse(
            "data: {\"reasoning\":\"mikir \"}\n\
             data: {\"text\":\"jawab \"}\n\
             data: {\"text\":\"dua\"}\n\
             data: [DONE]\n",
        );
        assert_eq!(a, "jawab dua");
        assert_eq!(r, "mikir ");
    }

    #[test]
    fn a_done_flag_stops_the_loop_but_false_does_not() {
        let (a, _) = parse_sse("data: {\"text\":\"a\"}\ndata: {\"done\":true}\ndata: {\"text\":\"b\"}\n");
        assert_eq!(a, "a", "setelah done parsing berhenti");
        let (a2, _) = parse_sse("data: {\"text\":\"a\",\"done\":false}\ndata: {\"text\":\"b\"}\n");
        assert_eq!(a2, "ab", "done:false bukan break");
    }

    #[test]
    fn the_last_line_without_a_newline_is_still_read() {
        // Tidak ada `lines.pop()` di `parseSSE` — seluruh body dipecah.
        assert_eq!(parse_sse("data: {\"text\":\"akhir\"}").0, "akhir");
    }

    #[test]
    fn a_normal_reply_puts_the_reasoning_block_before_the_answer() {
        let http = Stub::ok("data: {\"reasoning\":\"langkah 1\\nlangkah 2\"}\ndata: {\"text\":\"Jawaban\"}\n");
        let t = body(&run(http.clone(), "Jelaskan black hole")).to_string();
        assert_eq!(
            t,
            // `replace(/\n/g, "\n> ")` hanya menyisipkan `> ` SETELAH newline,
            // jadi baris pertama tidak dapat `> `.
            "💭 *Proses Berpikir:*\nlangkah 1\n> langkah 2\n\nJawaban"
        );
        assert!(matches!(
            run(Stub::ok("data: {\"text\":\"x\"}\n"), "x").without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));

        let seen = http.seen.lock().unwrap().clone();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, ENDPOINT);
        let sent: Value = serde_json::from_str(&seen[0].1).unwrap();
        assert_eq!(sent["chat_mode"], "deep_think");
        assert_eq!(sent["model"], "deepseek-v4-flash");
        assert_eq!(sent["message"], "Jelaskan black hole");
        assert_eq!(sent["history_messages"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn an_answer_without_reasoning_has_no_thinking_block() {
        let http = Stub::ok("data: {\"text\":\"Cuma jawaban\"}\n");
        assert_eq!(body(&run(http.clone(), "x")), "Cuma jawaban");
    }

    #[test]
    fn long_reasoning_is_previewed_to_800_characters() {
        let long = "z".repeat(2000);
        let http = Stub::ok(&format!("data: {{\"reasoning\":\"{long}\"}}\n"));
        let t = body(&run(http.clone(), "x")).to_string();
        assert!(t.starts_with("💭 *Proses Berpikir:*\n"), "{t}");
        assert!(t.contains("z".repeat(800).as_str()), "800 karakter ikut");
        assert!(!t.contains(&"z".repeat(801)), "tidak lebih dari 800");
    }

    #[test]
    fn an_overlong_reply_is_cut_with_the_dipotong_marker() {
        let big = "q".repeat(6000);
        let http = Stub::ok(&format!("data: {{\"text\":\"{big}\"}}\n"));
        let t = body(&run(http.clone(), "x")).to_string();
        assert!(t.ends_with("\n\n... (dipotong)"), "{}", &t[t.len() - 20..]);
        assert_eq!(t.matches('q').count(), MAX_LEN);
    }

    #[test]
    fn an_empty_sse_body_is_the_gagal_branch() {
        let r = run(Stub::ok("data: [DONE]\n"), "x");
        assert_eq!(body(&r), "❌ *DeepSeek Gagal*\n\n> Gagal mendapatkan respons");
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }

    #[test]
    fn a_transport_failure_reports_the_error_template() {
        let r = run(Stub::offline(), "halo");
        assert_eq!(body(&r), crate::common::error_template(".", "deepseek"));
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}