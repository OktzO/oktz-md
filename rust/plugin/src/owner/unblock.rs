//! `plugins/owner/unblock.js` — buka blokir nomor WhatsApp.
//!
//! `ponytail:` `sock.updateBlockStatus(jid, 'unblock')` tidak di-port — `Ctx`
//! tidak punya `sock`. `unblock.js` TIDAK punya guard "jangan buka blokir bot
//! sendiri" seperti `block.js`, jadi file ini tidak menambahkannya sendiri.
//!
//! Resolusi target diambil dari `block.rs` (urutan mention → quote → angka →
//! chat private harus identik di kedua plugin); lihat laporan batch.

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::owner::block::{INVALID_NUMBER, resolve_target, sock_unavailable, Target};

pub(crate) const USAGE: &str = "⚠️ *ᴄᴀʀᴀ ᴘᴀᴋᴀɪ*\n\n\
     > `.unblock 628xxx` — Unblock via nomor\n\
     > `.unblock` (reply pesan) — Unblock pengirim\n\
     > `.unblock @mention` — Unblock yang di-mention\n\
     > `.unblock` (di private chat) — Unblock user ini";

#[plugin(
    name = "unblock",
    alias = ["unblocknomor"],
    category = "owner",
    description = "Buka blokir nomor WhatsApp",
    usage = ".unblock <nomor/reply/mention>",
    example = ".unblock 628xxx",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn unblock(ctx: Ctx) -> Reply {
    let target = match resolve_target(&ctx) {
        Target::Jid(jid) => jid,
        Target::InvalidNumber => return Reply::text(INVALID_NUMBER),
        Target::None => return Reply::text(USAGE),
    };
    let number = target.split('@').next().unwrap_or(&target);
    Reply::text(sock_unavailable("Unblock", number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{PluginEntry, QuotedKind, QuotedMessage, find};

    /// `find("unblock")` tidak bisa dipakai di test: `unban.js` juga
    /// mendaftarkan `unblock` sebagai alias (`plugins/owner/unban.js:6`).
    /// `unblocknomor` milik plugin ini saja.
    fn entry() -> &'static PluginEntry {
        find("unblocknomor").expect("unblock harus terdaftar lewat alias unblocknomor")
    }

    fn ctx(is_group: bool, args: &[&str], mentioned: &[&str], quoted: Option<QuotedMessage>) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "unblock".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            is_group,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            mentioned: mentioned.iter().map(|s| (*s).to_string()).collect(),
            quoted,
            ..Default::default()
        }
    }

    fn run(c: Ctx) -> String {
        (entry().handler)(c)
            .text_body()
            .expect("harus ada teks")
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = entry();
        assert_eq!(p.meta.name, "unblock");
        // JS `name: ['unblock','unblocknomor']` — entri kedua jadi alias.
        assert_eq!(p.meta.alias, ["unblocknomor"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Buka blokir nomor WhatsApp");
        assert_eq!(p.meta.usage, ".unblock <nomor/reply/mention>");
        assert_eq!(p.meta.example, ".unblock 628xxx");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }

    #[test]
    fn the_second_name_in_the_js_array_resolves_as_an_alias() {
        assert_eq!(find("unblocknomor").unwrap().meta.name, "unblock");
    }

    /// Tabrakan warisan JS: `unban.js` mendaftarkan `unblock` sebagai alias,
    /// `unblock.js` mendaftarkannya sebagai nama. `find("unblock")` mengembalikan
    /// salah satu dari keduanya. Tidak dirapikan diam-diam; lihat laporan batch.
    #[test]
    fn the_unblock_command_is_shared_with_unban_just_like_in_javascript() {
        let owner = find("unblock").unwrap().meta.name.clone();
        assert!(
            owner == "unblock" || owner == "unban",
            "pemilik `unblock` harus salah satu dari dua plugin JS: {owner}"
        );
    }

    #[test]
    fn the_usage_text_is_the_unblock_wording_not_the_block_one() {
        assert!(USAGE.contains(".unblock 628xxx"));
        assert!(
            !USAGE.contains(".block"),
            "menyalin usage blokir akan boda user: {USAGE}"
        );
        assert_ne!(USAGE, crate::owner::block::USAGE);
    }

    #[test]
    fn the_shared_resolver_prefers_a_mention_over_a_quote_and_a_number() {
        let t = resolve_target(&ctx(
            true,
            &["628777"],
            &["628999@s.whatsapp.net"],
            Some(QuotedMessage {
                id: "Q".into(),
                sender: "628888@s.whatsapp.net".into(),
                chat: "628111@s.whatsapp.net".into(),
                text: String::new(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            timestamp: 0,
        }),
        ));
        assert!(matches!(t, Target::Jid(j) if j == "628999@s.whatsapp.net"));
    }

    #[test]
    fn a_number_keeps_only_digits_and_gets_the_wa_server() {
        // `+62 811-1` -> hanya digit: 628111
        assert_eq!(
            run(ctx(true, &["+62 811-1"], &[], None)),
            sock_unavailable("Unblock", "628111")
        );
    }

    #[test]
    fn a_bare_command_in_a_group_shows_the_unblock_usage() {
        assert_eq!(run(ctx(true, &[], &[], None)), USAGE);
    }

    #[test]
    fn a_bare_command_in_private_targets_the_chat_itself() {
        assert_eq!(
            run(ctx(false, &[], &[], None)),
            sock_unavailable("Unblock", "628111")
        );
    }

    #[test]
    fn a_number_without_digits_is_rejected_with_the_shared_message() {
        assert_eq!(run(ctx(true, &["-"], &[], None)), INVALID_NUMBER);
    }
}
