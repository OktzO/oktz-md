//! `plugins/ai/sologo.js` — buat logo dari prompt, kirim thumbnail + caption.
//!
//! `ponytail:` header `User-Agent` per-request (`sologo.js:32`) tidak bisa
//! dikirim — `HttpGet` tidak punya setter header. reacted `🕕` (`sologo.js:26`)
//! dilewati karena `Reply::Reaction` hanya menampung satu emoji; reaksi
//! terakhir (`❌`/`✅`/`☢`) yang ikut.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::common::js_truthy;
use crate::tools::phisingataubukan::encode_uri_component;

const ENDPOINT: &str = "https://api.nexray.eu.cc/ai/sologo";

/// `${obj.k}` di JS: objek jadi JSON kompak. Field yang tidak ada jadi string
/// kosong — bukan `"undefined"` seperti di JS, mengikuti konvensi plugin
/// sekelas di folder ini (`quillbot.rs`).
fn field(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    }
}

#[plugin(
    name = "sologo",
    alias = ["ailogo", "bikinlogo"],
    category = "ai",
    description = "Membuat logo menggunakan AI dari teks (prompt)",
    usage = ".sologo <prompt>",
    example = ".sologo Kucing lucu warna biru",
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
pub fn sologo(ctx: Ctx) -> Reply {
    // `sologo.js:20` — `m.text` di-`trim()` dulu, `m.args.join(" ")` fallback.
    let prompt = if ctx.text.trim().is_empty() {
        ctx.args.join(" ")
    } else {
        ctx.text.trim().to_string()
    };

    if prompt.is_empty() {
        return Reply::text(
            "❌ Masukkan deskripsi logo yang ingin dibuat.\n\nContoh: `.sologo robot keren warna merah`",
        );
    }

    let url = format!("{ENDPOINT}?prompt={}", encode_uri_component(&prompt));
    let Some(data) = ctx.http.get(&url).ok().and_then(|r| r.json()) else {
        return Reply::text("😔 Terjadi kesalahan saat memproses permintaan ke AI.")
            .with_reaction("☢");
    };

    // `sologo.js:38` — `!data.status || !data.result || data.result.length === 0`.
    let first = data
        .get("result")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .and_then(|a| a.first());
    if !js_truthy(data.get("status")) || first.is_none() {
        return Reply::text("⚠️ AI gagal membuat logo. Coba gunakan prompt (deskripsi) yang lain.")
            .with_reaction("❌");
    }
    let logo = first.expect("dijamin Some oleh guard di atas");

    // `logo.logo_type || "origin"`.
    let logo_type = match logo.get("logo_type") {
        Some(v) if js_truthy(Some(v)) => field(logo, "logo_type"),
        _ => "origin".to_string(),
    };

    let caption = format!(
        "🎨 *SOLOGO AI* 🎨\n\n\
         *Prompt:* {prompt}\n\
         *Judul:* {}\n\
         *Deskripsi:* {}\n\
         *Tipe:* {logo_type}",
        field(logo, "title"),
        field(logo, "desc"),
    );

    // `sock.sendMessage(m.chat, { image: { url: logo.thumbnail }, caption })`.
    Reply::image(MediaSource::url(field(logo, "thumbnail")), caption).with_reaction("✅")
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

    const OK: &str = r#"{"status":true,"result":[
        {"title":"Kucing Biru","desc":"Logo kucing","logo_type":"png","thumbnail":"https://cdn/l.png"}
    ]}"#;

    fn run(http: Arc<Stub>, text: &str, args: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "sologo".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: args.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("sologo").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.text_body().expect("harus ada teks")
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("sologo").unwrap();
        assert_eq!(p.meta.name, "sologo");
        assert_eq!(p.meta.alias, ["ailogo", "bikinlogo"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(
            p.meta.description,
            "Membuat logo menggunakan AI dari teks (prompt)"
        );
        assert_eq!(p.meta.usage, ".sologo <prompt>");
        assert_eq!(p.meta.example, ".sologo Kucing lucu warna biru");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium && !p.meta.is_admin);
        assert!(p.meta.is_enabled);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for k in ["sologo", "ailogo", "bikinlogo"] {
            assert_eq!(
                oktz_plugin_api::find(k).map(|p| p.meta.name.as_str()),
                Some("sologo"),
                "key {k} salah resolve"
            );
        }
    }

    #[test]
    fn an_empty_prompt_asks_for_a_description_without_calling_the_api() {
        let http = Stub::ok("");
        for (text, args) in [("", ""), ("    ", ""), ("", "   ")] {
            let r = run(http.clone(), text, args);
            assert!(
                body(&r).contains("❌ Masukkan deskripsi logo yang ingin dibuat."),
                "text={text:?} args={args:?}"
            );
            assert!(
                body(&r).contains("`.sologo robot keren warna merah`"),
                "text={text:?} args={args:?}"
            );
        }
        assert!(http.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_trimmed_text_wins_and_args_are_the_fallback() {
        let http = Stub::ok(OK);
        let _ = run(http.clone(), "  robot merah  ", "abc def");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?prompt=robot%20merah")
        );

        let http = Stub::ok(OK);
        let _ = run(http.clone(), "   ", "abc def");
        assert_eq!(
            http.seen.lock().unwrap()[0],
            format!("{ENDPOINT}?prompt=abc%20def")
        );
    }

    #[test]
    fn the_first_logo_becomes_the_thumbnail_and_its_caption() {
        let http = Stub::ok(OK);
        let r = run(http.clone(), "kucing biru", "kucing biru");
        let cap = body(&r);
        assert!(cap.starts_with("🎨 *SOLOGO AI* 🎨"), "{cap}");
        assert!(cap.contains("*Prompt:* kucing biru"), "{cap}");
        assert!(cap.contains("*Judul:* Kucing Biru"), "{cap}");
        assert!(cap.contains("*Deskripsi:* Logo kucing"), "{cap}");
        assert!(cap.contains("*Tipe:* png"), "{cap}");
        assert!(
            matches!(r.media_source(), Some(MediaSource::Url { url, .. })
            if url == "https://cdn/l.png")
        );
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "✅"
        ));
    }

    #[test]
    fn a_missing_logo_type_falls_back_to_origin() {
        let http = Stub::ok(
            r#"{"status":true,"result":[{"title":"T","desc":"D","thumbnail":"https://cdn/l.png"}]}"#,
        );
        let __b = run(http.clone(), "x", "x");
        assert!(body(&__b).contains("*Tipe:* origin"));
    }

    #[test]
    fn an_empty_or_rejected_result_list_shows_the_failed_warning() {
        for payload in [
            r#"{"status":true,"result":[]}"#,
            r#"{"status":false,"result":[{"title":"t"}]}"#,
            r#"{"status":true}"#,
            "bukan json",
        ] {
            let r = run(Stub::ok(payload), "kucing", "kucing");
            assert_eq!(
                body(&r),
                "⚠️ AI gagal membuat logo. Coba gunakan prompt (deskripsi) yang lain.",
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
        let r = run(Stub::offline(), "kucing", "kucing");
        assert_eq!(
            body(&r),
            "😔 Terjadi kesalahan saat memproses permintaan ke AI."
        );
        assert!(matches!(
            r.without_mutations(),
            Reply::Reaction { emoji, .. } if emoji == "☢"
        ));
    }
}
