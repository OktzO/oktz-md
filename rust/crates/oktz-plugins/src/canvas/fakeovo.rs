//! `plugins/canvas/fakeovo.js` — gambar aplikasi OVO palsu dari API
//! `kyzznekoo`.
//!
//! `ponytail:` axios di JS mengirim dua header (`User-Agent` dan
//! `Content-Type`). `HttpGet` tidak punya setter header per-request, jadi dua
//! header itu tidak ikut.
//!
//! `ponytail:` `console.error("[FakeOvo Error]", e)` di cabang `catch` tidak
//! punya padanan — `Reply` tidak membawa log.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

/// Cabang `if (!text)` (`fakeovo.js:21`).
const FORMAT_SALAH: &str = "Format salah!\n\n> Contoh: .fake-ovo Budi";

/// Caption sukses (`fakeovo.js:33`).
const CAPTION: &str = "✅ Berhasil membuat fake ovo";

/// `https://kyzznekoo.zone.id/api/canvas/fake-ovo?q=<text>` (`fakeovo.js:25`).
pub(crate) fn api_url(text: &str) -> String {
    format!(
        "https://kyzznekoo.zone.id/api/canvas/fake-ovo?q={}",
        encode_uri_component(text)
    )
}

#[plugin(
    name = "fakeovo",
    alias = ["fake-ovo", "fakeovo"],
    category = "canvas",
    description = "Membuat canvas fake ovo",
    usage = ".fake-ovo <nama>",
    example = ".fake-ovo Jokowi",
    cooldown = 5,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn fakeovo(ctx: Ctx) -> Reply {
    if ctx.text.is_empty() {
        return Reply::text(FORMAT_SALAH);
    }

    // `await m.react("🕕")` lalu `await m.react("✅")` / `"❌"` — `Reply` hanya
    // menyimpan satu reaksi, jadi yang dipegang adalah reaksi terakhir.
    let Ok(res) = ctx.http.get(&api_url(&ctx.text)) else {
        return Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("❌");
    };

    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), CAPTION).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct Stub {
        seen: Mutex<Vec<String>>,
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(vec![0x89, 0x50, 0x4E]),
                    mimetype: None,
                },
                headers: vec![],
            })
        }
        fn post_json(&self, _: &str, _: &Value) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
        fn post_form(&self, _: &str, _: &str) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
        fn post_multipart(
            &self,
            _: &str,
            _: &[(&str, HttpBody)],
        ) -> Result<HttpResponse, HttpError> {
            unreachable!("cuma GET")
        }
    }

    fn stub() -> Arc<Stub> {
        Arc::new(Stub {
            seen: Mutex::new(vec![]),
        })
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fake-ovo".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("fakeovo").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn reaction_of(r: &Reply) -> Option<&str> {
        let mut cur = r;
        loop {
            match cur {
                Reply::Reaction { emoji, .. } => return Some(emoji),
                Reply::Mutate { reply, .. } => cur = reply,
                _ => return None,
            }
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let meta = oktz_plugin_api::find("fakeovo").unwrap().meta.clone();
        assert_eq!(meta.name, "fakeovo");
        assert_eq!(meta.alias, ["fake-ovo", "fakeovo"]);
        assert_eq!(meta.category, "canvas");
        assert_eq!(meta.description, "Membuat canvas fake ovo");
        assert_eq!(meta.usage, ".fake-ovo <nama>");
        assert_eq!(meta.example, ".fake-ovo Jokowi");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 2);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
        assert!(!meta.is_owner);
        assert!(!meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_to_fakeovo() {
        assert_eq!(
            oktz_plugin_api::find("fake-ovo").unwrap().meta.name,
            "fakeovo"
        );
        assert_eq!(
            oktz_plugin_api::find("fakeovo").unwrap().meta.name,
            "fakeovo"
        );
    }

    #[test]
    fn a_name_produces_the_image() {
        let s = stub();
        let r = run(s.clone(), "Jokowi");
        assert!(r.has_media());
        assert_eq!(
            s.seen.lock().unwrap()[0],
            "https://kyzznekoo.zone.id/api/canvas/fake-ovo?q=Jokowi"
        );
        assert_eq!(text_of(&r), CAPTION);
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("bytes(3)")
        );
    }

    #[test]
    fn empty_text_shows_the_format_error_without_reaching_the_network() {
        let s = stub();
        let r = run(s.clone(), "");
        assert_eq!(text_of(&r), FORMAT_SALAH);
        assert!(!r.has_media());
        assert_eq!(reaction_of(&r), None);
        assert!(s.seen.lock().unwrap().is_empty());
    }

    #[test]
    fn the_name_is_percent_encoded_verbatim() {
        assert_eq!(
            api_url("Budi Santoso"),
            "https://kyzznekoo.zone.id/api/canvas/fake-ovo?q=Budi%20Santoso"
        );
        assert!(api_url("a&b=c").ends_with("?q=a%26b%3Dc"));
        // Spasi diruby `+` — itu `URLSearchParams`, bukan `encodeURIComponent`.
        assert!(api_url("a+b").ends_with("?q=a%2Bb"));
    }

    #[test]
    fn whitespace_only_text_is_still_sent_like_in_js() {
        // `if (!text)` hanya menolak string kosong; spasi masih truthy.
        let s = stub();
        let r = run(s.clone(), "   ");
        assert!(r.has_media());
        assert_eq!(
            s.seen.lock().unwrap()[0],
            "https://kyzznekoo.zone.id/api/canvas/fake-ovo?q=%20%20%20"
        );
    }

    #[test]
    fn unicode_and_very_long_names_do_not_panic() {
        let panjang = "nama ".repeat(2000);
        assert!(run(stub(), &panjang).has_media());
        // CJK ditulis sebagai escape supaya file ini tetap tanpa huruf
        // non-Latin; emoji di bawah tetap literal karena bukan huruf.
        let s = stub();
        assert!(run(s.clone(), "\u{65E5}\u{672C}\u{8A9E} \u{1F4B0}").has_media());
        assert!(s.seen.lock().unwrap()[0].contains("%F0%9F%92%B0"));
    }

    #[test]
    fn http_failure_falls_back_to_the_error_template_with_a_red_cross() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "fake-ovo".into(),
            text: "Budi".into(),
            ..Default::default()
        };
        let r = (oktz_plugin_api::find("fakeovo").unwrap().handler)(ctx);
        assert!(!r.has_media());
        assert_eq!(text_of(&r), crate::common::error_template(".", "fake-ovo"));
        assert_eq!(reaction_of(&r), Some("❌"));
    }
}
