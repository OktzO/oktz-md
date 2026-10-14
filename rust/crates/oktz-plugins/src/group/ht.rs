//! `plugins/group/ht.js` — hidetag dengan support reply pesan.
//!
//! `pluginConfig.name` di JS adalah array `['ht', 'hidetag']`.
//! `PluginMeta.name` hanya `String`, jadi elemen pertama jadi `name` dan sisanya
//! jadi `alias` — `matches()` tetap menerima keduanya. Pola yang sama dipakai
//! [`crate::group::autodl`].
//!
//! `ponytail:` cabang media (gambar/video/sticker/audio/dokumen) butuh
//! `quoted.download()` (`downloadContentMessage`) yang tidak ada di `Ctx`, jadi
//! hanya mode teks yang hidup. Reply-mode teks memakai `sock.sendMessage` tanpa
//! `{ quoted: m }` (polos), sedangkan mode teks murni memakai `{ quoted: m }`
//! — perbedaan itu dipertahankan lewat `Reply::Text` vs `Reply::TextQuoted`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::roster_jids;

pub(crate) const EMPTY: &str = "❌ *Pesan kosong*";

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📢 *HIDETAG*\n\n\
         • Reply pesan lalu ketik `{prefix}ht`\n\
         • Atau ketik `{prefix}ht <pesan>`\n\n\
         Support: teks, gambar, video, sticker, audio, dokumen"
    )
}

#[plugin(
    name = "ht",
    alias = ["hidetag"],
    category = "group",
    description = "Hidetag dengan support reply pesan (teks/media)",
    usage = ".ht [pesan] atau reply pesan",
    example = ".ht atau reply pesan lalu .ht",
    cooldown = 30,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true
)]
pub fn ht(ctx: Ctx) -> Reply {
    let text = ctx.text.trim().to_string();
    let mentions = roster_jids(&ctx);

    if let Some(quoted) = ctx.quoted() {
        // Cabang media tidak bisa di-port (lihat `ponytail:` modul).
        if quoted.is_media() {
            return Reply::None;
        }
        let final_text = if text.is_empty() {
            quoted.text.clone()
        } else {
            text.clone()
        };
        if final_text.is_empty() {
            return Reply::text(EMPTY);
        }
        return Reply::Text {
            text: final_text,
            mentions,
        };
    }

    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    Reply::TextQuoted {
        text,
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
            command: "ht".into(),
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
                is_bot_admin: true,
            },
            quoted: q,
            ..Default::default()
        }
    }

    fn run(text: &str, q: Option<QuotedMessage>) -> Reply {
        (oktz_plugin_api::find("ht").unwrap().handler)(ctx_of(text, q))
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
        let p = oktz_plugin_api::find("ht").unwrap();
        assert_eq!(p.meta.name, "ht");
        assert_eq!(p.meta.alias, ["hidetag"], "elemen kedua name JS jadi alias");
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Hidetag dengan support reply pesan (teks/media)"
        );
        assert_eq!(p.meta.usage, ".ht [pesan] atau reply pesan");
        assert_eq!(p.meta.example, ".ht atau reply pesan lalu .ht");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin);
        assert!(!p.meta.is_bot_admin, "isBotAdmin: false di JS");
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn the_second_js_name_resolves_to_the_same_plugin() {
        assert_eq!(oktz_plugin_api::find("hidetag").unwrap().meta.name, "ht");
        // `htprem` milik plugin `htpremium`, bukan milik `ht`.
        assert_eq!(oktz_plugin_api::find("htprem").unwrap().meta.name, "htpremium");
    }

    #[test]
    fn plain_text_is_quoted_back_and_tags_the_whole_roster() {
        let r = run("  rapat  ", None);
        assert!(matches!(r, Reply::TextQuoted { .. }), "mode teks murni meng-quote m");
        assert_eq!(body(&r), "rapat", "fullArgs di-trim");
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net", "628999@lid"]);
    }

    #[test]
    fn no_text_and_no_reply_prints_the_help() {
        for t in ["", "   "] {
            let r = run(t.clone(), None);
            assert_eq!(
                body(&r),
                "📢 *HIDETAG*\n\n• Reply pesan lalu ketik `.ht`\n• Atau ketik `.ht <pesan>`\n\nSupport: teks, gambar, video, sticker, audio, dokumen",
                "input {t:?}"
            );
            assert!(r.mutations().is_empty());
        }
    }

    #[test]
    fn reply_mode_is_plain_text_and_prefers_the_typed_argument() {
        let r = run("dinas", Some(quoted(QuotedKind::Text, "pesan lama")));
        assert!(matches!(r, Reply::Text { .. }), "reply-mode tidak meng-quote m");
        assert_eq!(body(&r), "dinas");
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net", "628999@lid"]);
    }

    #[test]
    fn reply_mode_falls_back_to_the_quoted_text() {
        let r = run("", Some(quoted(QuotedKind::Text, "pesan lama")));
        assert_eq!(body(&r), "pesan lama");
    }

    #[test]
    fn an_empty_reply_with_no_argument_is_refused() {
        let r = run("", Some(quoted(QuotedKind::Text, "")));
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
            let r = run("dinas", Some(quoted(kind.clone(), "cap")));
            assert!(r.is_none(), "jenis {}", kind.wire_type());
            assert!(r.mutations().is_empty(), "jenis {}", kind.wire_type());
        }
    }
}