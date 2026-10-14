//! `plugins/group/hidetag2.js` — hidetag dengan gaya `fakeQuoted`.
//!
//! `pluginConfig.name` sudah string tunggal (`'hidetag2'`) dengan alias
//! `['h2', 'ht2']`, jadi tidak perlu pemecahan array seperti di
//! [`crate::group::ht`].
//!
//! `ponytail:` dua hal di JS tidak bisa di-port.
//! 1. Cabang media (gambar/video/sticker/audio/dokumen) butuh `q.download()`
//!    (`downloadContentMessage`) yang tidak ada di `Ctx`, jadi berhenti di
//!    `Reply::None`.
//! 2. `fakeQuoted` (remoteJid `status@broadcast`, participant `0@s.whatsapp.net`,
//!    conversation = `config.bot.name`) tidak punya padanan di `Reply`; teks
//!    dikirim polos. Upgrade path: tambah varian quoted-message di
//!    `oktz-plugin-api` kalau `fakeQuoted` benar-benar dipakai klien.
//!
//! JS membakar dua reaction: `📢` sebelum kirim lalu `✅` setelahnya pada mode
//! teks saja. `Reply` hanya punya satu reaction, jadi mode teks memakai `✅`.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::roster_jids;

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📢 *HIDETAG 2*\n\n\
         • `{prefix}h2 <text>`\n\
         • Reply pesan + `{prefix}h2`"
    )
}

#[plugin(
    name = "hidetag2",
    alias = ["h2", "ht2"],
    category = "group",
    description = "Hidetag dengan fakeQuoted styling",
    usage = ".h2 <text> atau reply pesan",
    example = ".h2 Pengumuman penting!",
    cooldown = 30,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn hidetag2(ctx: Ctx) -> Reply {
    let text = ctx.text.trim();
    let mentions = roster_jids(&ctx);

    if text.is_empty() && ctx.quoted().is_none() {
        return Reply::text(help(&ctx.prefix));
    }

    if let Some(quoted) = ctx.quoted() {
        // Cabang media tidak bisa di-port (lihat `ponytail:` modul).
        if quoted.is_media() {
            return Reply::None;
        }
        return Reply::Text {
            text: quoted.text.clone(),
            mentions,
        };
    }

    Reply::Text {
        text: text.to_string(),
        mentions,
    }
    .with_reaction("✅")
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
            command: "h2".into(),
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
        (oktz_plugin_api::find("hidetag2").unwrap().handler)(ctx_of(text, q))
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn mentions(r: &Reply) -> Vec<String> {
        // `with_reaction` membungkus jadi `Reaction { reply }`, jadi teksnya
        // dibaca satu lapis lebih dalam dari `without_mutations()`.
        match r.without_mutations() {
            Reply::Text { mentions, .. } => mentions.clone(),
            Reply::Reaction { reply, .. } => mentions(reply),
            other => panic!("harus teks, dapat {other:?}"),
        }
    }

    fn reaction(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("hidetag2").unwrap();
        assert_eq!(p.meta.name, "hidetag2");
        assert_eq!(p.meta.alias, ["h2", "ht2"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Hidetag dengan fakeQuoted styling");
        assert_eq!(p.meta.usage, ".h2 <text> atau reply pesan");
        assert_eq!(p.meta.example, ".h2 Pengumuman penting!");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_to_the_same_plugin() {
        assert_eq!(oktz_plugin_api::find("h2").unwrap().meta.name, "hidetag2");
        assert_eq!(oktz_plugin_api::find("ht2").unwrap().meta.name, "hidetag2");
    }

    #[test]
    fn text_mode_tags_the_whole_roster_and_reacts() {
        let r = run("  Pengumuman penting! ", None);
        assert_eq!(body(&r), "Pengumuman penting!", "fullArgs di-trim");
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net", "628999@lid"]);
        assert!(r.mutations().is_empty());
        assert_eq!(reaction(&r), Some("✅"), "reaction ✅ dipakai untuk mode teks");
    }

    #[test]
    fn no_text_and_no_reply_prints_the_help() {
        for t in ["", "   "] {
            let r = run(t.clone(), None);
            assert_eq!(
                body(&r),
                "📢 *HIDETAG 2*\n\n• `.h2 <text>`\n• Reply pesan + `.h2`",
                "input {t:?}"
            );
            assert_eq!(reaction(&r), None, "reaction 📢 hanya setelah guard help");
        }
    }

    #[test]
    fn reply_mode_forwards_the_quoted_text_without_the_typed_argument() {
        let r = run("diabaikan", Some(quoted(QuotedKind::Text, "pesan lama")));
        assert_eq!(body(&r), "pesan lama");
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net", "628999@lid"]);
        assert_eq!(reaction(&r), None, "JS cuma react di mode teks");
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