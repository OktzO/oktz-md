//! `plugins/nsfw/paptt.js` — gambar PAPTT acak dari API `kyzznekoo`.
//!
//! `ponytail:` axios di JS mengirim dua header per-request
//! (`User-Agent` Chrome/150 dan `Content-Type: application/json`). `HttpGet`
//! tidak punya setter header, jadi keduanya tidak ikut — `UreqHttp` sudah
//! memasang UA-nya sendiri untuk GET.
//!
//! `ponytail:` `console.error("[PAPTT Error]", e)` di cabang `catch` tidak punya
//! padanan — `Reply` tidak membawa log.
//!
//! `pluginConfig` JS tidak punya field `example`, jadi `example` kosong di
//! sini — bukan diisi tebakan.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::common::error_template;

/// `paptt.js:22` — URL tanpa query, templatenya tidak pernah berubah.
pub(crate) const API_URL: &str = "https://kyzznekoo.zone.id/api/asupan/paptt";

/// Caption di `sock.sendMessage` (`paptt.js:30`). Asterisk di dalamnya itu
/// markdown WA, bukan karakter biasa.
const CAPTION: &str = "🔞 *PAPTT*";

#[plugin(
    name = "paptt",
    alias = ["paptt"],
    category = "nsfw",
    description = "Random paptt NSFW",
    usage = ".paptt",
    example = "",
    cooldown = 5,
    energi = 2,
    is_enabled = true,
    is_owner = false,
    is_premium = true,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn paptt(ctx: Ctx) -> Reply {
    // `await m.react("🕕")` pembuka di JS digantikan reaksi penutup: `Reply`
    // hanya menyimpan satu reaksi, jadi yang terakhir (`✅`/`❌`) yang pegang.
    let Ok(res) = ctx.http.get(API_URL) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("❌");
    };

    Reply::image(MediaSource::bytes(res.body.bytes().to_vec()), CAPTION).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{HttpBody, HttpError, HttpGet, HttpResponse};
    use serde_json::Value;
    use std::sync::{Arc, Mutex};

    /// Stub yang mencatat URL dan mengembalikan byte apa pun yang diminta.
    #[derive(Debug)]
    struct Stub {
        seen: Mutex<Vec<String>>,
        payload: Vec<u8>,
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.payload.clone()),
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

    fn stub(payload: Vec<u8>) -> Arc<Stub> {
        Arc::new(Stub {
            seen: Mutex::new(Vec::new()),
            payload,
        })
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("paptt").unwrap().handler)(ctx)
    }

    fn ctx_with(http: Arc<Stub>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "paptt".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            http,
            ..Default::default()
        }
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
        let meta = oktz_plugin_api::find("paptt").unwrap().meta.clone();
        assert_eq!(meta.name, "paptt");
        assert_eq!(meta.alias, ["paptt"]);
        assert_eq!(meta.category, "nsfw");
        assert_eq!(meta.description, "Random paptt NSFW");
        assert_eq!(meta.usage, ".paptt");
        assert!(meta.is_premium, "paptt premium di JS");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 2);
        assert!(meta.is_enabled);
        assert!(!meta.is_owner);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
    }

    #[test]
    fn example_is_empty_because_js_has_no_example_field() {
        // `pluginConfig` JS berhenti di `isEnabled` — tidak ada `example`.
        let meta = oktz_plugin_api::find("paptt").unwrap().meta.clone();
        assert_eq!(meta.example, "");
    }

    #[test]
    fn the_alias_is_the_name_itself_and_resolves_to_the_same_plugin() {
        // Alias `"paptt"` identik dengan nama di JS, jadi lookup-nya satu
        // entri saja — bukan dua plugin.
        let meta = oktz_plugin_api::find("paptt").unwrap().meta.clone();
        assert_eq!(meta.alias, ["paptt"]);
        assert_eq!(oktz_plugin_api::find("paptt").unwrap().meta.name, "paptt");
        assert!(
            oktz_plugin_api::all_plugins()
                .iter()
                .all(|e| e.meta.name != "paptt" || e.meta.category == "nsfw"),
            "paptt tidak boleh terdaftar dua kali"
        );
    }

    #[test]
    fn a_successful_fetch_sends_the_image_with_the_exact_caption() {
        let s = stub(vec![0x89, 0x50, 0x4E]);
        let r = run(ctx_with(s.clone()));
        assert!(r.has_media());
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("bytes(3)")
        );
        assert_eq!(text_of(&r), CAPTION);
        assert_eq!(reaction_of(&r), Some("✅"));
        assert_eq!(*s.seen.lock().unwrap(), vec![API_URL.to_string()]);
    }

    #[test]
    fn the_caption_is_exactly_four_ascii_letters_between_the_markers() {
        // `🔞 *PAPTT*` — guard supaya emoji/markdown tidak berubah diam-diam.
        assert_eq!(CAPTION, "\u{1F51E} *PAPTT*");
        assert_eq!(CAPTION.chars().count(), 9);
    }

    #[test]
    fn extra_arguments_are_ignored_like_in_js() {
        // `m.args` tidak pernah dibaca di handler JS.
        let s = stub(vec![1, 2]);
        let ctx = Ctx {
            text: "apa saja 123".into(),
            args: vec!["apa".into(), "saja".into(), "123".into()],
            ..ctx_with(s.clone())
        };
        let r = run(ctx);
        assert!(r.has_media());
        assert_eq!(text_of(&r), CAPTION);
        assert_eq!(*s.seen.lock().unwrap(), vec![API_URL.to_string()]);
    }

    #[test]
    fn an_empty_body_still_becomes_an_image_reply() {
        // JS tidak mengecek panjang buffer — `Buffer.from(res.data)` selalu
        // jalan, walau body kosong.
        let s = stub(Vec::new());
        let r = run(ctx_with(s.clone()));
        assert!(r.has_media());
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("bytes(0)")
        );
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn a_very_large_payload_is_passed_through_untouched() {
        let s = stub(vec![7u8; 200_000]);
        let r = run(ctx_with(s.clone()));
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("bytes(200000)")
        );
    }

    #[test]
    fn http_failure_falls_back_to_the_error_template_with_a_red_cross() {
        // `Ctx::default()` memakai `NoHttp` — selalu `Err`, tanpa jaringan.
        let ctx = Ctx {
            prefix: "!".into(),
            command: "paptt".into(),
            ..Default::default()
        };
        let r = run(ctx);
        assert!(!r.has_media());
        assert_eq!(text_of(&r), error_template("!", "paptt"));
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn an_empty_prefix_and_command_still_render_the_template_verbatim() {
        let r = run(Ctx {
            prefix: String::new(),
            command: String::new(),
            ..Default::default()
        });
        assert_eq!(text_of(&r), error_template("", ""));
        assert_eq!(reaction_of(&r), Some("❌"));
    }

    #[test]
    fn the_plugin_never_writes_to_the_store() {
        let r = run(ctx_with(stub(vec![1])));
        assert!(r.mutations().is_empty());
    }
}