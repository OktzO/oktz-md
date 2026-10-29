//! `plugins/canvas/watercolortext.js` — gambar teks efek watercolor lewat Cuki.
//!
//! Nama plugin di JS bukan nama file: `watercolortext.js` mendaftarkan
//! `.watercolorfx`. Teks bantuan juga memakai literal `watercolortext`, bukan
//! `m.command` — keduanya ikut apa adanya.
//!
//! `ponytail:` `config.APIkey.cuki` tidak ada di `Ctx`, jadi dikunci `""` —
//! pola yang sama dengan `crate::ai::anime_gen` dan `crate::info::jadwalbola`.
//! Efeknya cabang "API key belum diset" (`watercolortext.js:35`) selalu
//! benar dan jalur HTTP setelahnya tidak terjangkau sampai router menyediakan
//! config.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

/// `watercolortext.js:26-28` — cabang `if (!text)`.
fn help(prefix: &str) -> String {
    format!(
        "🎨 *ᴡᴀᴛᴇʀᴄᴏʟᴏʀ ᴛᴇxᴛ*\n\n\
         > Masukkan teks yang ingin dijadikan gambar\n\n\
         `Contoh: {prefix}watercolortext Halo`"
    )
}

/// `explainFailure(new MissingApiKeyError('cuki', 'APIKEY_CUKI'), ..)`
/// (`src/lib/stalker-fallback.js:30-36`).
fn missing_key_message(command: &str) -> String {
    format!(
        "❌ *API Key belum diset!*\n\n\
         > Command `{command}` butuh `APIKEY_CUKI` di `.env`\n\
         > Saat ini belum ada, jadi tidak bisa dipanggil."
    )
}

/// `src/lib/error.js` + `config.errorTemplate` (`config.js:153`).
///
/// `pushName || 'User'` jadi `'User'` karena `m.pushName` tidak ada di `Ctx`;
/// `prefix || '.'` dan `command || '?'` tetap dihormati karena keduanya ada.
pub(crate) fn error_template(prefix: &str, command: &str) -> String {
    let prefix = if prefix.is_empty() { "." } else { prefix };
    let command = if command.is_empty() { "?" } else { command };
    format!(
        "☢ Kayaknya command `{prefix}{command}` lagi ada kendala\n\
         Silahkan coba lagi nanti, User\n\n\
         _Jika masalah berlanjut, silahkan hubungi owner bot_"
    )
}

#[plugin(
    name = "watercolorfx",
    alias = ["watercolor", "wctext"],
    category = "canvas",
    description = "Buat gambar teks dengan efek watercolor",
    usage = ".watercolorfx <teks>",
    example = ".watercolorfx Halo",
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
pub fn watercolortext(ctx: Ctx) -> Reply {
    // `m.args` tidak pernah berisi string kosong, jadi `join(" ")` kosong
    // hanya terjadi saat `args` sendiri kosong.
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `m.react("🕕")` di `:31` terjadi sebelum cabang ini; reaksi terakhir
    // yang dipegang adalah "❌" dari `:36`.
    if ctx.config().api_keys.cuki.is_empty() {
        return Reply::text(missing_key_message(&format!(
            "{}watercolortext",
            ctx.prefix
        )))
        .with_reaction("❌");
    }

    let url = format!(
        "https://api.cuki.biz.id/api/ephoto/watercolortext?apikey={}&query={}",
        ctx.config().api_keys.cuki,
        encode_uri_component(&text)
    );

    // `axios.get` melempar untuk status >= 400 maupun error jaringan, dan
    // keduanya ditangkap `catch` yang sama.
    let Ok(res) = ctx.http.get(&url) else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    Reply::image(
        MediaSource::bytes(res.body.bytes().to_vec()),
        format!("🎨 *Water Color Text*\n\nTeks: {text}"),
    )
    .with_reaction("✅")
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
        bytes: Vec<u8>,
    }

    impl HttpGet for Stub {
        fn get(&self, url: &str) -> Result<HttpResponse, HttpError> {
            self.seen.lock().unwrap().push(url.to_string());
            Ok(HttpResponse {
                status: 200,
                body: HttpBody::Bytes {
                    data: Arc::from(self.bytes.clone()),
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
            bytes: vec![0x89, 0x50, 0x4E, 0x47],
        })
    }

    fn run(http: Arc<Stub>, text: &str) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "watercolorfx".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            http,
            ..Default::default()
        };
        (oktz_plugin_api::find("watercolorfx").unwrap().handler)(ctx)
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
        let meta = oktz_plugin_api::find("watercolorfx").unwrap().meta.clone();
        assert_eq!(meta.name, "watercolorfx");
        assert_eq!(meta.alias, ["watercolor", "wctext"]);
        assert_eq!(meta.category, "canvas");
        assert_eq!(
            meta.description,
            "Buat gambar teks dengan efek watercolor"
        );
        assert_eq!(meta.usage, ".watercolorfx <teks>");
        assert_eq!(meta.example, ".watercolorfx Halo");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 1);
        assert!(meta.is_enabled);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
        assert!(!meta.is_owner);
        assert!(!meta.is_premium);
        assert!(!meta.is_admin);
        assert!(!meta.is_bot_admin);
    }

    #[test]
    fn both_aliases_resolve_to_the_same_plugin() {
        for a in ["watercolorfx", "watercolor", "wctext"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "watercolorfx",
                "alias {a}"
            );
        }
    }

    #[test]
    fn empty_input_shows_the_help_without_reacting() {
        for input in ["", "   ", "\t"] {
            let s = stub();
            let r = run(s.clone(), input);
            assert_eq!(text_of(&r), help("."), "input {input:?}");
            assert!(!r.has_media(), "input {input:?}");
            assert_eq!(
                reaction_of(&r),
                None,
                "cabang ini sebelum `m.react`, jadi tanpa reaksi: {input:?}"
            );
            assert!(s.seen.lock().unwrap().is_empty(), "input {input:?}");
        }
    }

    #[test]
    fn a_normal_query_is_refused_because_the_api_key_is_empty() {
        let s = stub();
        let r = run(s.clone(), "Halo");
        assert!(!r.has_media());
        assert_eq!(text_of(&r), missing_key_message(".watercolortext"));
        assert_eq!(reaction_of(&r), Some("❌"));
        assert!(
            s.seen.lock().unwrap().is_empty(),
            "cabang key kosong tidak boleh menyentuh jaringan"
        );
    }

    #[test]
    fn the_help_and_missing_key_messages_keep_the_js_delimiters() {
        // Backtick, bukan asterisk, di dalam blok `*...*`.
        assert_eq!(
            help("."),
            "🎨 *ᴡᴀᴛᴇʀᴄᴏʟᴏʀ ᴛᴇxᴛ*\n\n\
             > Masukkan teks yang ingin dijadikan gambar\n\n\
             `Contoh: .watercolortext Halo`"
        );
        assert_eq!(
            missing_key_message("#watercolortext"),
            "❌ *API Key belum diset!*\n\n\
             > Command `#watercolortext` butuh `APIKEY_CUKI` di `.env`\n\
             > Saat ini belum ada, jadi tidak bisa dipanggil."
        );
    }

    #[test]
    fn a_very_long_query_does_not_panic() {
        let panjang = "a ".repeat(500) + "b";
        let r = run(stub(), &panjang);
        assert_eq!(text_of(&r), missing_key_message(".watercolortext"));
    }

    #[test]
    fn odd_characters_do_not_panic() {
        for input in ["<script>", "a|b|c", "🎨🎨🎨", "\"'", "%20%"] {
            let r = run(stub(), input);
            assert_eq!(
                text_of(&r),
                missing_key_message(".watercolortext"),
                "input {input:?}"
            );
        }
    }

    #[test]
    fn error_template_falls_back_like_the_js_replacements() {
        assert_eq!(
            error_template("", ""),
            "☢ Kayaknya command `.?` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert!(error_template("#", "watercolorfx").contains("#watercolorfx"));
    }
}