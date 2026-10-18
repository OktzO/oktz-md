//! `plugins/group/pinchat.js` — pin pesan yang di-reply.
//!
//! Nama plugin di JS adalah `gcpinchat` (`pinchat.js:2`) sementara nama
//! filenya `pinchat.js`; yang terdaftar tetap `gcpinchat`.
//!
//! Yang dipindah: gerbang "wajib reply" dan perhitungan durasi pin. Pin
//! sebenarnya `sock.sendMessage(m.chat, { pin: pinKey, type: 1, time })`
//! (`pinchat.js:50-54`) tidak ada di API plugin, jadi setelah gerbang lolos
//! plugin diam — bukan mengirim "Success pin pesan ini".

use oktz_plugin_api::{Ctx, Reply, plugin};

use crate::common::parse_int;

/// `pinchat.js:31-37` — default satu hari, `args[0]` berupa jam 1..720.
pub(crate) fn pin_duration(args: &[String]) -> i64 {
    let mut duration = 86_400;
    if let Some(first) = args.first().filter(|a| !a.is_empty())
        && let Some(hours) = parse_int(first)
        && (1..=720).contains(&hours)
    {
        duration = hours * 3600;
    }
    duration
}

/// `pinchat.js:21-27`. Teksnya memakai `.pinchat` literal, bukan `m.prefix`.
const NO_QUOTE: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
> Reply pesan yang ingin di-pin!\n\n\
*Cara penggunaan:*\n\
> Reply pesan → ketik `.pinchat`\n\
> Optional: `.pinchat 24` (pin 24 jam)";

#[plugin(
    name = "gcpinchat",
    alias = ["pinmsg", "pinpesan"],
    category = "group",
    description = "Pin pesan penting di grup",
    usage = ".gcpinchat (reply pesan)",
    example = ".gcpinchat",
    cooldown = 10,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn pinchat(ctx: Ctx) -> Reply {
    let Some(quoted) = ctx.quoted() else {
        return Reply::text(NO_QUOTE);
    };

    // `quoted.key.id` tidak ada di `QuotedMessage`; `QuotedMessage::id` adalah
    // padanannya. `ponytail:` `key.rawParticipant`/`key.participant` juga tidak
    // terbawa — tidak masalah karena pin-nya sendiri tidak di-port.
    let _ = quoted;
    let _duration = pin_duration(&ctx.args);

    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedKind;
    use oktz_plugin_api::QuotedMessage;
    use oktz_plugin_api::find;

    fn ctx_with_quoted(args: &[&str]) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "pinchat".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: args.iter().map(|s| (*s).to_string()).collect(),
            quoted: Some(QuotedMessage {
                id: "M1".into(),
                sender: "628999@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: "penting".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..Default::default()
        }
    }

    fn run(args: &[&str]) -> Reply {
        (find("gcpinchat").unwrap().handler)(ctx_with_quoted(args))
    }

    fn body(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("gcpinchat").unwrap();
        assert_eq!(e.meta.name, "gcpinchat", "nama plugin bukan nama file");
        assert_eq!(e.meta.alias, ["pinmsg", "pinpesan"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(e.meta.description, "Pin pesan penting di grup");
        assert_eq!(e.meta.usage, ".gcpinchat (reply pesan)");
        assert_eq!(e.meta.example, ".gcpinchat");
        assert_eq!(e.meta.cooldown, 10);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        assert!(e.meta.is_group && e.meta.is_admin && e.meta.is_bot_admin);
        assert!(!e.meta.is_private && !e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["pinmsg", "pinpesan"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("gcpinchat"),
                "alias {a}"
            );
        }
        // `pinchat` milik plugin owner (`plugins/owner/pinchat.js:2`), bukan
        // plugin grup ini — grup menamai dirinya `gcpinchat` (`pinchat.js:2`).
        assert_ne!(
            find("pinchat").map(|e| e.meta.name.as_str()),
            Some("gcpinchat"),
            "`pinchat` tidak boleh resolve ke plugin grup ini"
        );
    }

    #[test]
    fn without_a_reply_it_explains_how_to_pin() {
        let mut ctx = ctx_with_quoted(&[]);
        ctx.quoted = None;
        let r = (find("gcpinchat").unwrap().handler)(ctx);
        assert_eq!(
            body(&r),
            "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
             > Reply pesan yang ingin di-pin!\n\n\
             *Cara penggunaan:*\n\
             > Reply pesan → ketik `.pinchat`\n\
             > Optional: `.pinchat 24` (pin 24 jam)"
        );
    }

    #[test]
    fn the_default_duration_is_one_day() {
        assert_eq!(pin_duration(&[]), 86_400);
        assert_eq!(pin_duration(&["".into()]), 86_400);
    }

    #[test]
    fn only_hours_between_one_and_seven_hundred_twenty_are_accepted() {
        assert_eq!(pin_duration(&["1".into()]), 3600);
        assert_eq!(pin_duration(&["24".into()]), 86_400);
        assert_eq!(pin_duration(&["720".into()]), 720 * 3600);
        // Di luar rentang, bukan angka, dan sisa `parseInt` — semua default.
        for bad in ["0", "721", "-5", "abc", "24jam", ""] {
            assert_eq!(pin_duration(&[bad.into()]), 86_400, "arg {bad:?}");
        }
        // `parseInt` berhenti di karakter non-digit pertama.
        assert_eq!(pin_duration(&["12.5".into()]), 12 * 3600);
    }

    #[test]
    fn a_replied_message_passes_the_guard_and_then_goes_silent() {
        let r = run(&["24"]);
        assert!(r.is_none(), "pin tidak di-port; jangan mengarang");
        assert!(r.mutations().is_empty());
    }
}
