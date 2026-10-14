//! `plugins/canvas/iqc.js` — kirim URL gambar fake quote iOS ke WhatsApp.
//!
//! Tidak ada HTTP di plugin ini: JS juga tidak memanggil axios, hanya
//! menyerahkan URL ke Baileys (`image: { url }`).
//!
//! `ponytail:` cabang `catch` di JS (`iqc.js:38-42`) menangkap kegagalan
//! `sock.sendMessage`. Pengiriman sekarang dikerjakan pipeline, jadi cabang
//! itu tidak lagi punya pemicu di dalam handler.
//!
//! `pluginConfig` tidak punya key `example`, jadi `example` kosong — bukan
//! diisi tebakan.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

use crate::tools::phisingataubukan::encode_uri_component;

/// Cabang `if (!targetText)` (`iqc.js:21-27`).
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "💬 *FITUR FAKE QUOTE iOS*\n\n\
         Fitur ini digunakan untuk membuat gambar quote elegan bergaya iOS dengan sangat cepat.\n\n\
         *Cara Penggunaan:*\n\
         - Ketik *{prefix}iqc <teks kamu>*\n\
         - Atau kamu bisa membalas (reply) pesan teks orang lain dengan perintah *{prefix}iqc*\n\n\
         _Pesan tersebut akan secara otomatis diubah menjadi quote keren!_"
    )
}

/// `text || (m.quoted && m.quoted.text ? m.quoted.text : "")` (`iqc.js:18`).
///
/// `m.quoted.text` kosong (`""`) diperlakukan sama dengan tidak ada quote —
/// itu yang membuat `m.quoted.text ?` Guard di JS.
pub(crate) fn target_text(text: &str, quoted: Option<&str>) -> String {
    if !text.is_empty() {
        return text.to_string();
    }
    quoted.filter(|q| !q.is_empty()).unwrap_or("").to_string()
}

/// `https://my.izuka-api.xyz/api/canvas/iqc?text=<text>` (`iqc.js:32`).
pub(crate) fn api_url(target: &str) -> String {
    format!(
        "https://my.izuka-api.xyz/api/canvas/iqc?text={}",
        encode_uri_component(target)
    )
}

#[plugin(
    name = "iqc",
    alias = ["qc2"],
    category = "canvas",
    description = "Membuat Fake Quote iOS style secara instan.",
    usage = ".iqc [text/reply]",
    example = "",
    cooldown = 5,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn iqc(ctx: Ctx) -> Reply {
    let quoted = ctx.quoted().map(|q| q.text.as_str());
    let target = target_text(&ctx.text, quoted);
    if target.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `await m.react('🕕')` lalu `await m.react('✅')` — `Reply` hanya menyimpan
    // satu reaksi, jadi yang dipegang adalah reaksi terakhir.
    Reply::image(MediaSource::url(api_url(&target)), "").with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{QuotedKind, QuotedMessage};

    fn run(text: &str, quoted: Option<&str>) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "iqc".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            quoted: quoted.map(|t| QuotedMessage {
                id: "MSG1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "628111@s.whatsapp.net".into(),
                text: t.into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        };
        (oktz_plugin_api::find("iqc").unwrap().handler)(ctx)
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
        let meta = oktz_plugin_api::find("iqc").unwrap().meta.clone();
        assert_eq!(meta.name, "iqc");
        assert_eq!(meta.alias, ["qc2"]);
        assert_eq!(meta.category, "canvas");
        assert_eq!(
            meta.description,
            "Membuat Fake Quote iOS style secara instan."
        );
        assert_eq!(meta.usage, ".iqc [text/reply]");
        assert_eq!(meta.cooldown, 5);
        assert_eq!(meta.energi, 2);
        assert!(!meta.is_group);
        assert!(!meta.is_private);
        assert!(!meta.is_owner);
        assert!(!meta.is_premium);
        // `pluginConfig` JS tidak punya key `example`.
        assert_eq!(meta.example, "");
    }

    #[test]
    fn the_alias_resolves_to_iqc() {
        assert_eq!(oktz_plugin_api::find("qc2").unwrap().meta.name, "iqc");
    }

    #[test]
    fn typed_text_becomes_the_image_url() {
        let r = run("halo dunia", None);
        assert!(r.has_media());
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url(
                "https://my.izuka-api.xyz/api/canvas/iqc?text=halo%20dunia"
            ))
        );
        assert_eq!(reaction_of(&r), Some("✅"));
    }

    #[test]
    fn a_quoted_text_is_used_when_no_text_is_typed() {
        let r = run("", Some("teks yang dibalas"));
        assert!(r.has_media());
        assert_eq!(
            r.media_source(),
            Some(&MediaSource::url(
                "https://my.izuka-api.xyz/api/canvas/iqc?text=teks%20yang%20dibalas"
            ))
        );
    }

    #[test]
    fn typed_text_wins_over_the_quoted_text() {
        assert_eq!(
            target_text("yang diketik", Some("yang di-quote")),
            "yang diketik"
        );
    }

    #[test]
    fn no_text_and_no_quote_shows_the_help_without_reacting() {
        for (text, quoted) in [("", None), ("", Some("")), ("   ", Some(""))] {
            let r = run(text.clone(), quoted);
            assert_eq!(text_of(&r), help("."), "text {text:?} quoted {quoted:?}");
            assert!(!r.has_media());
            assert_eq!(reaction_of(&r), None, "guard ini sebelum `m.react`");
        }
    }

    #[test]
    fn special_characters_are_percent_encoded_not_dropped() {
        assert_eq!(
            api_url("a&b c?d=e#f"),
            "https://my.izuka-api.xyz/api/canvas/iqc?text=a%26b%20c%3Fd%3De%23f"
        );
        assert!(api_url("100%Pure").ends_with("100%25Pure"));
        // CJK ditulis sebagai escape supaya file ini tetap tanpa huruf non-Latin.
        assert!(api_url("\u{65E5}\u{672C}\u{8A9E}").ends_with("%E6%97%A5%E6%9C%AC%E8%AA%9E"));
    }

    #[test]
    fn a_very_long_text_does_not_panic() {
        let panjang = "a".repeat(10_000);
        let r = run(&panjang, None);
        assert!(r.has_media());
        assert_eq!(
            r.media_source().map(MediaSource::label).as_deref(),
            Some("url")
        );
    }
}
