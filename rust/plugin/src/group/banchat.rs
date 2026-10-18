//! `plugins/group/banchat.js` — `groups[chat].isBanned`.
//!
//! Cabang unban memakai `sock.sendMessage(..., { quoted: m })` (bukan
//! `m.reply`), jadi ia `Reply::TextQuoted`; cabang ban memakai `m.reply` jadi
//! `Reply::Text`. Keduanya menyebut pengirim lewat `mentions: [m.sender]`.
//!
//! `ponytail:` `db.setGroup(m.chat, { ...groupData, isBanned })` menyalin
//! seluruh objek grup. `Mutation::SetGroup` sudah merge, jadi hanya field
//! yang berubah yang perlu dikirim — hasil di store sama.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use crate::common::jid_number;

/// `m.groupMetadata.subject || 'Unknown'` — `handler.js:327` mengisi
/// `groups[chat].name` dari `meta.subject`.
pub(crate) const NO_SUBJECT: &str = "Unknown";

pub(crate) fn subject_or_unknown(ctx: &Ctx) -> String {
    ctx.group_field("name")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(NO_SUBJECT)
        .to_string()
}

/// `['unbanchat', 'unbangroup'].includes(cmd)` — hanya dua alias itu, bukan
/// kebalikan dari `cmd.startsWith('un')`.
pub(crate) fn is_unban(command: &str) -> bool {
    matches!(command, "unbanchat" | "unbangroup")
}

pub(crate) fn already_open() -> &'static str {
    "⚠️ *ɢʀᴜᴘ ᴛɪᴅᴀᴋ ᴅɪʙᴀɴ*\n\n> Grup ini tidak dalam status banned.\n> Semua user bisa menggunakan bot."
}

pub(crate) fn already_banned() -> &'static str {
    "⚠️ *ɢʀᴜᴘ sᴜᴅᴀʜ ᴅɪʙᴀɴ*\n\n> Grup ini sudah dalam status banned.\n> Gunakan `.unbanchat` untuk membuka akses."
}

pub(crate) fn unbanned(group: &str, sender_num: &str) -> String {
    format!(
        "✅ *ɢʀᴜᴘ ᴅɪ-ᴜɴʙᴀɴ*\n\n╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n┃ 📛 ɢʀᴜᴘ: *{group}*\n┃ 📊 sᴛᴀᴛᴜs: *✅ AKTIF*\n┃ 👤 ᴜɴʙᴀɴ ᴏʟᴇʜ: @{sender_num}\n╰┈┈⬡\n\n> Semua member sekarang bisa menggunakan bot kembali."
    )
}

pub(crate) fn banned(group: &str, sender_num: &str) -> String {
    format!(
        "🚫 *ɢʀᴜᴘ ᴅɪʙᴀɴ*\n\n╭┈┈⬡「 📋 *ᴅᴇᴛᴀɪʟ* 」\n┃ 📛 ɢʀᴜᴘ: *{group}*\n┃ 📊 sᴛᴀᴛᴜs: *🔴 BANNED*\n┃ 👤 ʙᴀɴ ᴏʟᴇʜ: @{sender_num}\n╰┈┈⬡\n\n> Member biasa tidak bisa menggunakan bot di grup ini.\n> Hanya owner yang bisa menggunakan bot."
    )
}

#[plugin(
    name = "banchat",
    alias = ["bangroup", "bangrup", "unbanchat", "unbangroup"],
    category = "group",
    description = "Ban grup dari penggunaan bot (hanya owner yang bisa akses)",
    usage = ".banchat",
    example = ".banchat",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = true
)]
pub fn banchat(ctx: Ctx) -> Reply {
    let banned_now = crate::common::js_truthy(ctx.group_field("isBanned"));
    let group = subject_or_unknown(&ctx);
    let sender_num = jid_number(&ctx.sender);
    let write = |value: bool| {
        Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ "isBanned": value }),
        }
    };

    if is_unban(&ctx.command) {
        if !banned_now {
            return Reply::text(already_open());
        }
        return Reply::TextQuoted {
            text: unbanned(&group, sender_num),
            mentions: vec![ctx.sender.clone()],
        }
        .with_mutation(write(false));
    }

    if banned_now {
        return Reply::text(already_banned());
    }

    Reply::Text {
        text: banned(&group, sender_num),
        mentions: vec![ctx.sender.clone()],
    }
    .with_mutation(write(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value};

    fn run(command: &str, group: Value) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: command.into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: true,
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("banchat").unwrap().handler)(ctx)
    }

    fn body(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn write(r: &Reply) -> &Value {
        assert_eq!(r.mutations().len(), 1, "harus satu SetGroup");
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    fn mentions(r: &Reply) -> Vec<String> {
        match r.without_mutations() {
            Reply::Text { mentions, .. } | Reply::TextQuoted { mentions, .. } => mentions.clone(),
            other => panic!("harus teks, dapat {other:?}"),
        }
    }

    fn quoted(r: &Reply) -> bool {
        matches!(r.without_mutations().tanpa_reaksi(), Reply::TextQuoted { .. })
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("banchat").unwrap();
        assert_eq!(p.meta.name, "banchat");
        assert_eq!(
            p.meta.alias,
            ["bangroup", "bangrup", "unbanchat", "unbangroup"]
        );
        assert_eq!(p.meta.category, "group");
        assert_eq!(
            p.meta.description,
            "Ban grup dari penggunaan bot (hanya owner yang bisa akses)"
        );
        assert_eq!(p.meta.usage, ".banchat");
        assert_eq!(p.meta.example, ".banchat");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner, "isOwner: true — satu-satunya plugin group ini");
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
        assert!(!p.meta.is_bot_admin && !p.meta.is_premium);
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["bangroup", "bangrup", "unbanchat", "unbangroup"] {
            assert_eq!(
                oktz_plugin_api::find(a).map(|p| p.meta.name.as_str()),
                Some("banchat")
            );
        }
    }

    #[test]
    fn the_two_helper_literals_are_the_js_templates_verbatim() {
        // Substring yang paling rawan salah transkripsi (small-caps unicode).
        assert!(already_open().starts_with("⚠️ *ɢʀᴜᴘ ᴛɪᴅᴀᴋ ᴅɪʙᴀɴ*"));
        assert!(already_banned().starts_with("⚠️ *ɢʀᴜᴘ sᴜᴅᴀʜ ᴅɪʙᴀɴ*"));
        assert!(already_banned().contains("Gunakan `.unbanchat` untuk"));
        assert!(banned("G", "1").starts_with("🚫 *ɢʀᴜᴘ ᴅɪʙᴀɴ*"));
        assert!(unbanned("G", "1").starts_with("✅ *ɢʀᴜᴘ ᴅɪ-ᴜɴʙᴀɴ*"));
    }

    #[test]
    fn banning_writes_true_and_mentions_the_owner() {
        let r = run("banchat", serde_json::json!({ "name": "Grup Diskusi" }));
        let text = body(&r);
        assert_eq!(text, banned("Grup Diskusi", "628111"));
        for part in [
            "┃ 📛 ɢʀᴜᴘ: *Grup Diskusi*",
            "┃ 📊 sᴛᴀᴛᴜs: *🔴 BANNED*",
            "┃ 👤 ʙᴀɴ ᴏʟᴇʜ: @628111",
            "> Hanya owner yang bisa menggunakan bot.",
        ] {
            assert!(text.contains(part), "kurang {part:?} → {text}");
        }
        assert_eq!(write(&r), &serde_json::json!({"isBanned": true}));
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net"]);
        assert!(!quoted(&r), "JS m.reply tanpa opsi quoted");
    }

    #[test]
    fn a_group_that_is_already_banned_is_left_alone() {
        for truthy in [serde_json::json!(true), serde_json::json!(1), serde_json::json!("ya"), serde_json::json!({})] {
            let r = run("bangroup", serde_json::json!({ "isBanned": truthy.clone() }));
            assert_eq!(body(&r), already_banned(), "isBanned={truthy}");
            assert!(r.mutations().is_empty(), "isBanned={truthy}");
        }
    }

    #[test]
    fn unbanning_a_banned_group_writes_false_and_quotes_the_command() {
        let r = run("unbanchat", serde_json::json!({ "name": "Grup Diskusi", "isBanned": true }));
        let text = body(&r);
        assert_eq!(text, unbanned("Grup Diskusi", "628111"));
        for part in [
            "┃ 📊 sᴛᴀᴛᴜs: *✅ AKTIF*",
            "┃ 👤 ᴜɴʙᴀɴ ᴏʟᴇʜ: @628111",
            "> Semua member sekarang bisa menggunakan bot kembali.",
        ] {
            assert!(text.contains(part), "kurang {part:?} → {text}");
        }
        assert_eq!(write(&r), &serde_json::json!({"isBanned": false}));
        assert_eq!(mentions(&r), ["628111@s.whatsapp.net"]);
        assert!(quoted(&r), "JS kirim dengan opsi quoted");
    }

    #[test]
    fn unbanning_a_group_that_is_not_banned_writes_nothing() {
        let r = run("unbangroup", serde_json::json!({}));
        assert_eq!(body(&r), already_open());
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_group_row_without_a_name_falls_back_to_unknown() {
        for stored in [serde_json::json!({}), serde_json::json!({ "name": "" })] {
            let r = run("banchat", stored);
            assert!(body(&r).contains("┃ 📛 ɢʀᴜᴘ: *Unknown*"), "{}", body(&r));
        }
    }
}
