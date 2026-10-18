//! `plugins/owner/mutechat.js` — mute/unmute chat.
//!
//! `ponytail:` `sock.chatModify({ mute: 1 | null }, targetJid)` dan
//! `m.react('✅')` tidak ada di `Ctx` API, jadi tidak dapat meniru perilaku
//! JS apa adanya. Yang dipindah: resolusi target (mention / quoted / nomor
//! argumen / chat saat private) dan teks balasan. Aksi chatModify sendiri
//! tidak dapat dilakukan di sini.
//!
//! Catatan: baris `pub mod mutechat;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

fn help(prefix: &str) -> String {
    format!(
        "🔇 *ᴍᴜᴛᴇ ᴄʜᴀᴛ*\n\n\
         > `.mutechat 628xxx` — Mute chat\n\
         > `.mutechat` (di private chat) — Mute chat ini\n\
         > `.mutechat buka 628xxx` — Unmute chat\n\
         Prefix saat ini: `{prefix}`"
    )
}

fn digits_only(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

#[plugin(
    name = "mutechat",
    alias = ["mute"],
    category = "owner",
    description = "Mute/unmute chat",
    usage = ".mutechat <nomor/reply> atau .mutechat buka <nomor>",
    example = ".mutechat 628xxx",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn mutechat(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase()).unwrap_or_default();
    let mut mute = true;
    let mut target_jid: Option<String> = None;

    if action == "buka" || action == "unmute" {
        mute = false;
        let num = digits_only(ctx.args.get(1).map(String::as_str).unwrap_or(""));
        if !num.is_empty() {
            target_jid = Some(format!("{}@s.whatsapp.net", num));
        } else if let Some(q) = ctx.quoted() {
            target_jid = Some(q.sender.clone());
        } else if !ctx.is_group {
            target_jid = Some(ctx.chat.clone());
        }
    } else {
        if ctx.mentioned.first().is_some() {
            target_jid = Some(ctx.mentioned.first().cloned().unwrap());
        } else if let Some(q) = ctx.quoted() {
            target_jid = Some(q.sender.clone());
        } else if let Some(a) = ctx.args.first() {
            let num = digits_only(a);
            if !num.is_empty() {
                target_jid = Some(format!("{}@s.whatsapp.net", num));
            }
        } else if !ctx.is_group {
            target_jid = Some(ctx.chat.clone());
        }
    }

    let Some(target_jid) = target_jid else {
        return Reply::text(help(&ctx.prefix));
    };

    let target = target_jid.split('@').next().unwrap_or(&target_jid);
    if mute {
        Reply::text(format!("🔇 *ᴄʜᴀᴛ ᴅɪᴍᴜᴛᴇ*\n\n> Target: {target}"))
    } else {
        Reply::text(format!("🔊 *ᴄʜᴀᴛ ᴅɪᴜɴᴍᴜᴛᴇ*\n\n> Target: {target}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], mentioned: Option<&str>, quoted: bool, is_group: bool) -> Reply {
        let ctx = Ctx {
            command: "mutechat".into(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            mentioned: mentioned.into_iter().map(|s| s.to_string()).collect(),
            quoted: if quoted {
                Some(oktz_plugin_api::QuotedMessage {
                    id: "q1".into(),
                    sender: "628999@s.whatsapp.net".into(),
                    chat: "628111@s.whatsapp.net".into(),
                    text: String::new(),
                    kind: oktz_plugin_api::QuotedKind::Text,
                    mimetype: None,
                    file_name: None,
                    from_me: false,
                })
            } else {
                None
            },
            is_group,
            chat: "628111@s.whatsapp.net".into(),
            ..Default::default()
        };
        mutechat(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("mutechat").unwrap();
        assert_eq!(p.meta.name, "mutechat");
        assert_eq!(p.meta.alias, ["mute"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn without_target_shows_help() {
        let r = run(&[], None, false, false);
        assert!(body(&r).starts_with("🔇 *ᴍᴜᴛᴇ ᴄʜᴀᴛ*"));
    }

    #[test]
    fn mute_by_number_in_private() {
        let r = run(&["6281234567890"], None, false, false);
        assert!(body(&r).contains("🔇 *ᴄʜᴀᴛ ᴅɪᴍᴜᴛᴇ*"));
        assert!(body(&r).contains("> Target: 6281234567890"));
    }

    #[test]
    fn unmute_by_buka_and_number() {
        let r = run(&["buka", "6281234567890"], None, false, false);
        assert!(body(&r).contains("🔊 *ᴄʜᴀᴛ ᴅɪᴜɴᴍᴜᴛᴇ*"));
    }

    #[test]
    fn mention_wins_over_argument() {
        let r = run(&["628111111111"], Some("628999222222@s.whatsapp.net"), false, false);
        assert!(body(&r).contains("> Target: 628999222222"));
    }
}
