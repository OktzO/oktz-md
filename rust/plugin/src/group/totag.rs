//! `plugins/group/totag.js` — tag semua member dengan meneruskan pesan ter-quote.
//!
//! `ponytail:` inti plugin adalah
//! `sock.sendMessage(chat, { forward: m.quoted.fakeObj || m.quoted })`.
//! `Reply` tidak punya varian forward dan `QuotedMessage` tidak menyimpan
//! `fakeObj`/`rawMessage`, jadi tidak ada payload yang bisa dikirim ulang.
//! Yang bisa di-port adalah guard-nya: konteks reply, reaction 📢/✅, dan
//! pemeriksaan roster kosong. Upgrade path: tambah `Reply::Forward` +
//! `QuotedMessage::raw` kalau forward pesan jadi kebutuhan nyata.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📢 *ᴛᴏᴛᴀɢ*\n\n\
         > Reply pesan yang ingin di-forward ke semua member\n\n\
         > Contoh: Reply pesan lalu ketik `{prefix}totag`"
    )
}

pub(crate) const NO_MEMBERS: &str = "❌ Gagal mendapatkan data member grup";

#[plugin(
    name = "totag",
    alias = ["tagall2", "mentionall"],
    category = "group",
    description = "Tag semua member dengan reply pesan",
    usage = ".totag (reply pesan)",
    example = ".totag",
    cooldown = 30,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn totag(ctx: Ctx) -> Reply {
    if ctx.quoted().is_none() {
        return Reply::text(help(&ctx.prefix));
    }

    if ctx.roster.participants.is_empty() {
        return Reply::text(NO_MEMBERS);
    }

    // Forward tidak portable; reaction saja yang tersisa (lihat `ponytail:`).
    Reply::None.with_reaction("✅")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::registry::GroupRoster;
    use oktz_plugin_api::{Participant, QuotedKind, QuotedMessage};

    fn quoted() -> QuotedMessage {
        QuotedMessage {
            id: "MSG1".into(),
            sender: "628999@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: "pengumuman".into(),
            kind: QuotedKind::Text,
            mimetype: None,
            file_name: None,
            from_me: false,
        }
    }

    fn ctx_of(roster: GroupRoster, q: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "totag".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            text: String::new(),
            args: Vec::new(),
            is_group: true,
            roster,
            quoted: q,
            ..Default::default()
        }
    }

    fn roster(n: usize) -> GroupRoster {
        GroupRoster {
            participants: (0..n)
                .map(|i| Participant {
                    jid: format!("62811{i}@s.whatsapp.net"),
                    is_admin: false,
                    is_owner: false,
                    data: Default::default(),
                })
                .collect(),
            is_bot_admin: true,
        }
    }

    fn run(roster: GroupRoster, q: Option<QuotedMessage>) -> Reply {
        (oktz_plugin_api::find("totag").unwrap().handler)(ctx_of(roster, q))
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn reaction(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("totag").unwrap();
        assert_eq!(p.meta.name, "totag");
        assert_eq!(p.meta.alias, ["tagall2", "mentionall"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Tag semua member dengan reply pesan");
        assert_eq!(p.meta.usage, ".totag (reply pesan)");
        assert_eq!(p.meta.example, ".totag");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin);
        assert!(!p.meta.is_private && !p.meta.is_owner && !p.meta.is_premium);
    }

    #[test]
    fn both_aliases_resolve_to_the_same_plugin() {
        assert_eq!(oktz_plugin_api::find("tagall2").unwrap().meta.name, "totag");
        assert_eq!(oktz_plugin_api::find("mentionall").unwrap().meta.name, "totag");
    }

    #[test]
    fn without_a_quoted_message_it_prints_the_help() {
        let r = run(roster(2), None);
        assert_eq!(
            body(&r),
            "📢 *ᴛᴏᴛᴀɢ*\n\n> Reply pesan yang ingin di-forward ke semua member\n\n> Contoh: Reply pesan lalu ketik `.totag`"
        );
        assert_eq!(reaction(&r), None, "react 📢 baru setelah guard reply");
    }

    #[test]
    fn an_empty_roster_is_refused() {
        let r = run(roster(0), Some(quoted()));
        assert_eq!(body(&r), NO_MEMBERS);
        assert_eq!(reaction(&r), None, "guard roster gagal sebelum react ✅");
    }

    #[test]
    fn a_forward_is_reduced_to_the_reaction_it_can_still_send() {
        let r = run(roster(2), Some(quoted()));
        // `Reply::None.with_reaction(..)` = Reaction yang membungkus None.
        let Reply::Reaction { reply, .. } = r.without_mutations() else {
            panic!("harus reaksi, dapat {:?}", r.without_mutations());
        };
        assert!(reply.is_none(), "forward tidak portable");
        assert_eq!(reaction(&r), Some("✅"));
        assert!(r.mutations().is_empty());
    }
}