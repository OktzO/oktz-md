//! `plugins/group/htpremium.js` — hidetag dengan pemisah `|` untuk custom tag.
//!
//! `pluginConfig.name` di JS adalah array
//! `['htpremium', 'hidetagpremium', 'htprem']`. `PluginMeta.name` hanya
//! `String`, jadi elemen pertama jadi `name` dan sisanya jadi `alias` —
//! `matches()` tetap menerima ketiganya. Pola yang sama dipakai
//! [`crate::group::autodl`].
//!
//! `ponytail:`
//! 1. Cabang media di reply-mode butuh `quoted.download()`
//!    (`downloadContentMessage`) yang tidak ada di `Ctx`, jadi berhenti di
//!    `Reply::None`.
//! 2. `contextInfo.groupMentions: [{ groupJid, groupSubject: cmd }]` di mode
//!    teks tidak punya padanan di `Reply::TextQuoted` — teks tetap `@chat` +
//!    body dan mention per-jid, tapi tag `@everyone`/custom subject hilang.
//!    Upgrade path: tambah field group-mention ke `oktz-plugin-api` kalau
//!    klien lokal benar-benar relies pada `groupSubject`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::roster_jids;

pub(crate) const EMPTY: &str = "❌ *Pesan kosong*";

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📢 *HIDETAG PREMIUM*\n\n\
         • Reply pesan lalu ketik `{prefix}ht`\n\
         • Atau ketik `{prefix}ht <custom tag> | <pesan>`\n\n\
         • Contoh: `{prefix}ht everyone | hai semua`\n\n\
         Support: teks, gambar, video, sticker, audio, dokumen"
    )
}

#[plugin(
    name = "htpremium",
    alias = ["hidetagpremium", "htprem"],
    category = "group",
    description = "Hidetag dengan support reply pesan (teks/media)",
    usage = ".htprem [pesan] atau reply pesan",
    example = ".htprem atau reply pesan lalu .htprem",
    cooldown = 30,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn htpremium(ctx: Ctx) -> Reply {
    let mentions = roster_jids(&ctx);
    // Destructure JS: `const [cmd, text] = (m.text || '').split('|')`.
    // Tanpa trim, segmen tambahan diabaikan, `undefined` = tidak ada.
    // `cmd` hanya jadi `groupSubject` yang tidak bisa di-port — lihat `ponytail:`.
    let mut it = ctx.text.split('|');
    let _cmd = it.next().unwrap_or_default();
    let text = it.next().map(str::to_string);

    if let Some(quoted) = ctx.quoted() {
        // Cabang media tidak bisa di-port (lihat `ponytail:` modul).
        if quoted.is_media() {
            return Reply::None;
        }
        let final_text = text
            .clone()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| quoted.text.clone());
        if final_text.is_empty() {
            return Reply::text(EMPTY);
        }
        // `sock.sendMessage` tanpa `{ quoted }` di JS — teks polos.
        return Reply::Text {
            text: final_text,
            mentions,
        };
    }

    let Some(text) = text.filter(|t| !t.is_empty()) else {
        return Reply::text(help(&ctx.prefix));
    };

    // `cmd` hanya dipakai sebagai `groupSubject`, yang tidak bisa di-port.
    Reply::TextQuoted {
        text: format!("@{} {} ", ctx.chat, text),
        mentions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::GroupRoster;
    use oktz_plugin_api::{Participant, QuotedKind, QuotedMessage};

    fn quoted(kind: QuotedKind, text: &str) -> QuotedMessage {
        QuotedMessage {
            id: "MSG1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: text.into(),
            kind,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn ctx_of(text: &str, q: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "htprem".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: text.into(),
            args: text.split_whitespace().map(str::to_string).collect(),
            is_group: true,
            roster: GroupRoster {
                participants: vec![
                    Participant { jid: "628111@s.whatsapp.net".into(), is_admin: true, is_owner: true, data: Default::default() },
                    Participant { jid: "628999@lid".into(), is_admin: false, is_owner: false, data: Default::default() },
                ],
                is_bot_admin: false,
            },
            quoted: q,
            ..Default::default()
        }
    }

    fn run(text: &str, q: Option<QuotedMessage>) -> Reply {
        (oktz_plugin_api::find("htpremium").unwrap().handler)(ctx_of(text, q))
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions(r: &Reply) -> Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } | Reply::TextQuoted { mentions, .. } => mentions.clone(),
            other => panic!("harus teks, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("htpremium").unwrap();
        assert_eq!(p.meta.name, "htpremium");
        assert_eq!(p.meta.alias, ["hidetagpremium", "htprem"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Hidetag dengan support reply pesan (teks/media)"
        );
        assert_eq!(p.meta.usage, ".htprem [pesan] atau reply pesan");
        assert_eq!(p.meta.example, ".htprem atau reply pesan lalu .htprem");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn the_other_js_names_resolve_to_the_same_plugin() {
        assert_eq!(
            oktz_plugin_api::find("hidetagpremium").unwrap().meta.name,
            "htpremium"
        );
        assert_eq!(oktz_plugin_api::find("htprem").unwrap().meta.name, "htpremium");
    }

    #[test]
    fn text_mode_tags_the_chat_and_quotes_the_trigger() {
        // `split('|')` tanpa trim: spasi setelah `|` ikut terbawa, sama
        // seperti di JS.
        let r = run("everyone | hai semua", None);
        assert!(matches!(r, Reply::TextQuoted { .. }), "mode teks meng-quote m");
        assert_eq!(body(&r), "@120363@g.us  hai semua ");
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net", "628999@lid"]);
    }

    #[test]
    fn segments_after_the_second_pipe_are_ignored() {
        let r = run("everyone | hai | aba", None);
        assert_eq!(body(&r), "@120363@g.us  hai  ");
    }

    #[test]
    fn no_text_and_no_reply_prints_the_help() {
        for t in ["", "everyone", "everyone |"] {
            let r = run(t.clone(), None);
            assert_eq!(
                body(&r),
                "📢 *HIDETAG PREMIUM*\n\n• Reply pesan lalu ketik `.ht`\n• Atau ketik `.ht <custom tag> | <pesan>`\n\n• Contoh: `.ht everyone | hai semua`\n\nSupport: teks, gambar, video, sticker, audio, dokumen",
                "input {t:?}"
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn reply_mode_prefers_the_text_after_the_pipe() {
        let r = run("everyone | baru", Some(quoted(QuotedKind::Text, "pesan lama")));
        assert!(matches!(r, Reply::Text { .. }), "reply-mode tidak meng-quote m");
        assert_eq!(body(&r), " baru");
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net", "628999@lid"]);
    }

    #[test]
    fn reply_mode_falls_back_to_the_quoted_text() {
        let r = run("everyone", Some(quoted(QuotedKind::Text, "pesan lama")));
        assert_eq!(body(&r), "pesan lama");
    }

    #[test]
    fn an_empty_reply_with_no_argument_is_refused() {
        let r = run("everyone", Some(quoted(QuotedKind::Text, "")));
        assert_eq!(body(&r), EMPTY);
    }

    #[test]
    fn every_quoted_media_kind_stops_at_the_unavailable_download() {
        for kind in [
            QuotedKind::Image,
            QuotedKind::Video,
            QuotedKind::Audio,
            QuotedKind::Sticker,
            QuotedKind::Document,
        ] {
            let r = run("everyone | baru", Some(quoted(kind.clone(), "cap")));
            assert!(r.is_none(), "jenis {}", kind.wire_type());
            assert!(r.mutations().is_empty(), "jenis {}", kind.wire_type());
        }
    }
}