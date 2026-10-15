//! `plugins/group/getpp.js` — kirim foto profil target (mention/reply).
//!
//! Yang dipindah: seluruh penentuan target (`m.quoted` > `m.mentionedJid[0]` >
//! angka di `m.args[0]` > `m.sender`), karena itu murni string.
//!
//! `ponytail:` pengambilan avatar `sock.profilePictureUrl(target, 'image')`
//! (`getpp.js:34`) dan pengirimannya `sock.sendMedia(...)` (`:39`) tidak ada
//! di API plugin, jadi tidak ada gambar yang dikirim — termasuk fallback
//! `https://files.catbox.moe/ejy4ky.jpg` yang di JS hanya dipakai kalau
//! avatar target gagal diambil.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `getpp.js:18-28`. `num.startsWith('0')` → `62` + sisa digit, persis
/// konversi awalan `0` ala Indonesia.
pub(crate) fn resolve_target(ctx: &Ctx) -> String {
    if let Some(q) = ctx.quoted() {
        return q.sender.clone();
    }
    if let Some(jid) = ctx.mentioned.first() {
        return jid.clone();
    }
    if let Some(arg) = ctx.args.first() {
        let mut num: String = arg.chars().filter(char::is_ascii_digit).collect();
        if num.starts_with('0') {
            num = format!("62{}", &num[1..]);
        }
        return format!("{num}@s.whatsapp.net");
    }
    ctx.sender.clone()
}

#[plugin(
    name = "getpp",
    alias = ["pp", "profilepic", "avatar"],
    category = "group",
    description = "Ambil foto profil target (mention/reply)",
    usage = ".getpp @user",
    example = ".getpp @628xxx",
    cooldown = 5,
    energi = 0,
    is_enabled = true,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn getpp(ctx: Ctx) -> Reply {
    // `let target = …` + `const targetNum = …` tetap dihitung supaya pilihan
    // target tidak ikut hilang, tapi tidak ada yang bisa dikirim.
    let _target_num = resolve_target(&ctx)
        .split('@')
        .next()
        .unwrap_or_default()
        .to_string();
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::QuotedKind;
    use oktz_plugin_api::QuotedMessage;
    use oktz_plugin_api::find;

    fn base() -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "getpp".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let e = find("getpp").unwrap();
        assert_eq!(e.meta.name, "getpp");
        assert_eq!(e.meta.alias, ["pp", "profilepic", "avatar"]);
        assert_eq!(e.meta.category, "group");
        assert_eq!(
            e.meta.description,
            "Ambil foto profil target (mention/reply)"
        );
        assert_eq!(e.meta.usage, ".getpp @user");
        assert_eq!(e.meta.example, ".getpp @628xxx");
        assert_eq!(e.meta.cooldown, 5);
        assert_eq!(e.meta.energi, 0);
        assert!(e.meta.is_enabled);
        // `isGroup: false` dan `isPrivate: false` di JS — plugin ini tidak
        // boleh jalan di mana pun sampai ada jalur avatar.
        assert!(!e.meta.is_group && !e.meta.is_private);
        assert!(!e.meta.is_admin && !e.meta.is_bot_admin);
        assert!(!e.meta.is_owner && !e.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["pp", "profilepic", "avatar"] {
            assert_eq!(
                find(a).map(|e| e.meta.name.as_str()),
                Some("getpp"),
                "alias {a}"
            );
        }
    }

    #[test]
    fn without_quote_mention_or_number_the_sender_is_the_target() {
        let ctx = base();
        assert_eq!(resolve_target(&ctx), "628111@s.whatsapp.net");
    }

    #[test]
    fn a_mention_wins_over_a_number_argument() {
        let ctx = Ctx {
            mentioned: vec!["628999@s.whatsapp.net".into()],
            args: vec!["628777".into()],
            ..base()
        };
        assert_eq!(resolve_target(&ctx), "628999@s.whatsapp.net");
    }

    #[test]
    fn a_reply_wins_over_everything() {
        let ctx = Ctx {
            mentioned: vec!["628999@s.whatsapp.net".into()],
            args: vec!["628777".into()],
            quoted: Some(QuotedMessage {
                id: "M1".into(),
                sender: "628555:2@s.whatsapp.net".into(),
                chat: "120363@g.us".into(),
                text: "hai".into(),
                kind: QuotedKind::Text,
                mimetype: None,
                file_name: None,
                from_me: false,
            }),
            ..base()
        };
        assert_eq!(resolve_target(&ctx), "628555:2@s.whatsapp.net");
    }

    #[test]
    fn digits_are_extracted_from_a_noisy_argument() {
        for (arg, expect) in [
            ("@628123", "628123@s.whatsapp.net"),
            ("+62 8123", "628123@s.whatsapp.net"),
            ("0812-3456", "628123456@s.whatsapp.net"),
            // Tanpa digit sama sekali → `'' + '@s.whatsapp.net'`, bukan error.
            ("halo", "@s.whatsapp.net"),
        ] {
            let ctx = Ctx {
                args: vec![arg.into()],
                ..base()
            };
            assert_eq!(resolve_target(&ctx), expect, "arg {arg:?}");
        }
    }

    #[test]
    fn a_leading_zero_becomes_62_exactly_once() {
        let ctx = Ctx {
            args: vec!["00".into()],
            ..base()
        };
        assert_eq!(
            resolve_target(&ctx),
            "620@s.whatsapp.net",
            "'62' + num.slice(1)"
        );
    }

    #[test]
    fn it_sends_nothing_instead_of_faking_a_profile_picture() {
        let ctx = Ctx {
            mentioned: vec!["628999@s.whatsapp.net".into()],
            ..base()
        };
        let r = (find("getpp").unwrap().handler)(ctx);
        assert!(
            r.is_none(),
            "tidak ada `profilePictureUrl`, jadi tidak ada gambar"
        );
        assert!(!r.has_media());
        assert!(r.mutations().is_empty());
    }
}
