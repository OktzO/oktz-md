//! `plugins/canvas/pakustad.js` — tanya "pak ustad" (gambar) lewat Cuki.
//!
//! `pluginConfig.name` di JS berupa array tiga nama
//! (`['pakustad', 'pak-ustad', 'tanyaustad']`); yang pertama jadi `name`, sisa
//! nya jadi `alias` — pola yang sama dengan `crate::fun::husbualbum`.
//! Kategori di JS `fun`, bukan nama folder `canvas`; nilai `category` di
//! `#[plugin]` disalin apa adanya dari `pluginConfig`.
//!
//! `ponytail:` `config.APIkey?.cuki` tidak ada di `Ctx`, jadi dikunci `""` —
//! pola yang sama dengan `crate::ai::anime_gen` dan `crate::info::jadwalbola`.
//! Efeknya cabang "API Key belum diset" (`pakustad.js:34`) selalu benar dan
//! jalur HTTP setelahnya tidak terjangkau sampai router menyediakan config.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};
use serde_json::Value;

use crate::tools::phisingataubukan::encode_uri_component;

/// `config.APIkey?.cuki || ""` (`pakustad.js:4`).
const CUKI_APIKEY: &str = "";

/// `pakustad.js:25-29` — cabang `if (!text)`.
fn help(prefix: &str) -> String {
    format!(
        "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
         > `{prefix}pakustad <pertanyaan>`\n\n\
         > Contoh: `{prefix}pakustad kenapa aku ganteng`"
    )
}

/// `pakustad.js:35` — teks literal, bukan lewat `explainFailure`.
const MISSING_KEY: &str =
    "❌ *API Key belum diset!*\n\n> Isi `APIKEY_CUKI` di file `.env`, lalu restart bot";

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

/// `m.text || m.quoted?.text` (`pakustad.js:22`).
pub(crate) fn effective_text(ctx: &Ctx) -> String {
    if !ctx.text.is_empty() {
        return ctx.text.clone();
    }
    ctx.quoted().map(|q| q.text.clone()).unwrap_or_default()
}

#[plugin(
    name = "pakustad",
    alias = ["pak-ustad", "tanyaustad"],
    category = "fun",
    description = "Tanya pak ustad (gambar)",
    usage = ".pakustad <pertanyaan>",
    example = ".pakustad kenapa aku ganteng",
    cooldown = 10,
    energi = 1,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pakustad(ctx: Ctx) -> Reply {
    let text = effective_text(&ctx);

    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `await m.react('🕕')` di `:32` sudah jalan sebelum cabang ini, jadi reaksi
    // satu-satunya pada cabang ini adalah "🕕".
    if CUKI_APIKEY.is_empty() {
        return Reply::text(MISSING_KEY).with_reaction("🕕");
    }

    let url = format!(
        "https://api.cuki.biz.id/api/canvas/ustadz?apikey={CUKI_APIKEY}&text={}",
        encode_uri_component(&text)
    );

    // `f()` mengembalikan `null` untuk status >= 400 dan error jaringan, jadi
    // `ctx.http.get(..)` yang `Err` meniru keduanya.
    //
    // `const { results } = await f(apiUrl)` melempar `TypeError` kalau `f`
    // mengembalikan `null`, dan `results.url` melempor kalau `results` tidak
    // punya `url`. Keduanya jatuh ke `catch` yang sama, jadi satu syarat
    // di sini sudah cukup.
    let image = ctx
        .http
        .get(&url)
        .ok()
        .and_then(|r| r.json())
        .and_then(|body: Value| {
            body.pointer("/results/url")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    let Some(image) = image else {
        return Reply::text(error_template(&ctx.prefix, &ctx.command)).with_reaction("☢");
    };

    // `sock.sendMedia(m.chat, results.url, text, m, { type: 'image' })`.
    Reply::image(MediaSource::url(image), text).with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};
    use serde_json::json;

    const ME: &str = "628111@s.whatsapp.net";

    fn ctx_of(text: &str, quoted: Option<&str>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "pakustad".into(),
            sender: ME.into(),
            chat: ME.into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            quoted: quoted.map(|t| QuotedMessage {
                id: "MSG1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: ME.into(),
                text: t.into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        }
    }

    fn run(ctx: Ctx) -> Reply {
        (oktz_plugin_api::find("pakustad").unwrap().handler)(ctx)
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
        let meta = oktz_plugin_api::find("pakustad").unwrap().meta.clone();
        assert_eq!(meta.name, "pakustad");
        assert_eq!(meta.alias, ["pak-ustad", "tanyaustad"]);
        assert_eq!(meta.category, "fun");
        assert_eq!(meta.description, "Tanya pak ustad (gambar)");
        assert_eq!(meta.usage, ".pakustad <pertanyaan>");
        assert_eq!(meta.example, ".pakustad kenapa aku ganteng");
        assert_eq!(meta.cooldown, 10);
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
    fn every_name_from_the_js_array_resolves_here() {
        // `name: ['pakustad', 'pak-ustad', 'tanyaustad']` di JS.
        for a in ["pakustad", "pak-ustad", "tanyaustad"] {
            assert_eq!(
                oktz_plugin_api::find(a).unwrap().meta.name,
                "pakustad",
                "nama {a}"
            );
        }
    }

    #[test]
    fn a_question_reaches_the_missing_key_branch_with_the_clock_reaction() {
        let r = run(ctx_of("kenapa aku ganteng", None));
        assert!(!r.has_media());
        assert_eq!(text_of(&r), MISSING_KEY);
        // `m.react('🕕')` di `:32` tetap reaksi terakhir pada cabang ini.
        assert_eq!(reaction_of(&r), Some("🕕"));
    }

    #[test]
    fn empty_input_shows_the_help_without_reacting() {
        for (text, quoted) in [("", None), ("", Some(""))] {
            let r = run(ctx_of(text, quoted));
            assert_eq!(text_of(&r), help("."), "text {text:?} quoted {quoted:?}");
            assert!(!r.has_media());
            assert_eq!(reaction_of(&r), None);
        }
    }

    #[test]
    fn the_quoted_text_is_used_when_there_is_no_own_text() {
        // `m.text || m.quoted?.text` — teks sendiri menang kalau ada.
        assert_eq!(effective_text(&ctx_of("", Some("apakah aku hebat"))), "apakah aku hebat");
        assert_eq!(
            effective_text(&ctx_of("teks sendiri", Some("teks kutipan"))),
            "teks sendiri"
        );
        assert_eq!(effective_text(&ctx_of("", None)), "");
        assert_eq!(effective_text(&ctx_of("", Some(""))), "");

        let r = run(ctx_of("", Some("apakah aku hebat")));
        assert_eq!(text_of(&r), MISSING_KEY);
        assert_eq!(reaction_of(&r), Some("🕕"));
    }

    #[test]
    fn the_help_message_keeps_the_backtick_delimiters_from_js() {
        // JS memakai backtick di dalam blok bold, bukan asterisk.
        assert_eq!(
            help("."),
            "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
             > `.pakustad <pertanyaan>`\n\n\
             > Contoh: `.pakustad kenapa aku ganteng`"
        );
        assert!(help("#").contains("`#pakustad <pertanyaan>`"));
    }

    #[test]
    fn the_missing_key_text_names_the_env_variable_in_backticks() {
        assert_eq!(
            MISSING_KEY,
            "❌ *API Key belum diset!*\n\n\
             > Isi `APIKEY_CUKI` di file `.env`, lalu restart bot"
        );
    }

    #[test]
    fn odd_and_very_long_questions_do_not_panic() {
        for input in ["<b>", "a|b", "🎨", "'\"", "a\u{202e}b"] {
            let r = run(ctx_of(input, None));
            assert_eq!(text_of(&r), MISSING_KEY, "input {input:?}");
        }
        let panjang = "apa ".repeat(2000);
        let r = run(ctx_of(&panjang, None));
        assert_eq!(text_of(&r), MISSING_KEY);
    }

    #[test]
    fn error_template_falls_back_like_the_js_replacements() {
        assert_eq!(
            error_template("", ""),
            "☢ Kayaknya command `.?` lagi ada kendala\n\
             Silahkan coba lagi nanti, User\n\n\
             _Jika masalah berlanjut, silahkan hubungi owner bot_"
        );
        assert!(error_template("#", "pakustad").contains("#pakustad"));
    }

    // Bentuk respons sukses dipakai hanya sebagai dokumentasi bentuk JSON yang
    // dibaca plugin; jalur sebenarnya tidak terjangkau selama kunci kosong.
    #[test]
    fn the_success_shape_needs_results_url() {
        let body = json!({ "results": { "url": "https://cdn.example/ustadz.png" } });
        assert_eq!(
            body.pointer("/results/url").and_then(Value::as_str),
            Some("https://cdn.example/ustadz.png")
        );
        let kosong = json!({});
        assert!(kosong.pointer("/results/url").is_none());
    }
}