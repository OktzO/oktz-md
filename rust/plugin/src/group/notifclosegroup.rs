//! `plugins/group/notifclosegroup.js` — toggle `groups[chat].notifCloseGroup`.
//!
//! Empat plugin `notif*` punya tubuh yang sama persis (guard admin, status
//! on/off, satu tulis ke grup) dan hanya berbeda di nama field, judul, dan
//! ikon — jadi cabangnya hidup di sini dan tiga saudaranya memanggilnya.
//!
//! `ponytail:` `m.isAdmin` tidak ada di `Ctx`; guard memakainya
//! `ctx.roster.is_admin(sender)`, roster grup untuk pesan yang sama.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use serde_json::Value;

/// `!m.isAdmin && !m.isOwner` — balasan bersama untuk keempat plugin.
pub(crate) const ADMIN_ONLY: &str = "❌ Hanya admin grup yang bisa menggunakan fitur ini";

/// Satu-satunya beda antara `notifclosegroup`/`demote`/`opengroup`/`promote`.
pub(crate) struct NotifToggle {
    /// Kunci di `db.getGroup(m.chat)`.
    pub field: &'static str,
    /// Judul tanpa `*`; balasan status dan on/off merakit dari sini.
    pub title: &'static str,
    /// Ikon di depan judul pada baris status.
    pub emoji: &'static str,
}

/// `group[field] === true ? '✅ Aktif' : '❌ Nonaktif'` — `===`, bukan
/// truthiness, jadi `1` atau `"true"` tetap tampil Nonaktif.
fn status_text(enabled: bool) -> &'static str {
    if enabled { "✅ Aktif" } else { "❌ Nonaktif" }
}

/// Tubuh yang dipakai keempat plugin `notif*`.
pub(crate) fn notif_toggle(ctx: &Ctx, t: &NotifToggle) -> Reply {
    if !ctx.roster.is_admin(&ctx.sender) && !ctx.is_owner {
        return Reply::text(ADMIN_ONLY);
    }

    let arg = ctx.args.first().map(|a| a.to_lowercase());
    let enabled = ctx.group_data.get(t.field).and_then(Value::as_bool) == Some(true);

    match arg.as_deref() {
        Some("on") => Reply::text(format!("✅ *{} ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*", t.title)).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ t.field: true }),
        }),
        Some("off") => {
            Reply::text(format!("❌ *{} ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*", t.title)).with_mutation(Mutation::SetGroup {
                jid: ctx.chat.clone(),
                data: serde_json::json!({ t.field: false }),
            })
        }
        _ => Reply::text(format!(
            "{} *{}*\n\n> Status: {}\n\n*Penggunaan:*\n`{}{} on` - Aktifkan\n`{}{} off` - Nonaktifkan",
            t.emoji,
            t.title,
            status_text(enabled),
            ctx.prefix,
            ctx.command,
            ctx.prefix,
            ctx.command,
        )),
    }
}

#[plugin(
    name = "notifclosegroup",
    alias = ["notifclose"],
    category = "group",
    description = "Toggle notifikasi saat grup ditutup",
    usage = ".notifclosegroup on/off",
    example = ".notifclosegroup on",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn notifclosegroup(ctx: Ctx) -> Reply {
    notif_toggle(
        &ctx,
        &NotifToggle {
            field: "notifCloseGroup",
            title: "ɴᴏᴛɪꜰ ᴄʟᴏsᴇ ɢʀᴏᴜᴘ",
            emoji: "🔒",
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::Participant;
    use oktz_plugin_api::registry::GroupRoster;

    const SENDER: &str = "628111@s.whatsapp.net";

    fn ctx_with_roster(admin: bool, owner: bool) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "notifclosegroup".into(),
            sender: SENDER.into(),
            chat: "120363@g.us".into(),
            is_group: true,
            is_owner: owner,
            roster: GroupRoster {
                participants: vec![Participant {
                    jid: SENDER.into(),
                    is_admin: admin,
                    is_owner: false,
                    data: Value::Null,
                }],
                is_bot_admin: false,
            },
            ..Default::default()
        }
    }

    fn run(group: serde_json::Value, arg: Option<&str>) -> Reply {
        let mut ctx = ctx_with_roster(false, true);
        ctx.group_data = group;
        ctx.args = arg.into_iter().map(String::from).collect();
        (oktz_plugin_api::find("notifclosegroup").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &serde_json::Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetGroup { jid, data } => {
                assert_eq!(jid, "120363@g.us");
                data
            }
            other => panic!("harus SetGroup, dapat {other:?}"),
        }
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("notifclosegroup").unwrap();
        assert_eq!(p.meta.name, "notifclosegroup");
        assert_eq!(p.meta.alias, ["notifclose"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Toggle notifikasi saat grup ditutup");
        assert_eq!(p.meta.usage, ".notifclosegroup on/off");
        assert_eq!(p.meta.example, ".notifclosegroup on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        // `isAdmin`/`isBotAdmin` tidak ada di pluginConfig JS.
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin);
    }

    #[test]
    fn alias_resolves() {
        assert_eq!(oktz_plugin_api::find("notifclose").unwrap().meta.name, "notifclosegroup");
    }

    #[test]
    fn a_member_who_is_neither_admin_nor_owner_is_rejected() {
        let r = (oktz_plugin_api::find("notifclosegroup").unwrap().handler)(ctx_with_roster(false, false));
        assert_eq!(text_of(&r), ADMIN_ONLY);
        assert!(r.mutations().is_empty(), "ditolak berarti tidak boleh menulis");
    }

    #[test]
    fn the_owner_flag_alone_passes_the_guard() {
        let r = (oktz_plugin_api::find("notifclosegroup").unwrap().handler)(ctx_with_roster(false, true));
        assert_ne!(text_of(&r), ADMIN_ONLY);
    }

    #[test]
    fn the_roster_admin_flag_alone_passes_the_guard() {
        let r = (oktz_plugin_api::find("notifclosegroup").unwrap().handler)(ctx_with_roster(true, false));
        assert_ne!(text_of(&r), ADMIN_ONLY);
    }

    #[test]
    fn no_argument_reports_the_status_without_writing() {
        for (stored, expected) in [
            (serde_json::json!({}), "❌ Nonaktif"),
            (serde_json::json!({"notifCloseGroup": true}), "✅ Aktif"),
            (serde_json::json!({"notifCloseGroup": false}), "❌ Nonaktif"),
            (serde_json::json!({"notifCloseGroup": 1}), "❌ Nonaktif"),
        ] {
            let r = run(stored.clone(), None);
            assert!(
                text_of(&r).contains(&format!("> Status: {expected}\n")),
                "input {stored}: {}",
                text_of(&r)
            );
            assert!(r.mutations().is_empty(), "status tidak boleh menulis: {stored}");
        }
    }

    #[test]
    fn an_unknown_keyword_falls_through_to_the_status_branch() {
        let r = run(serde_json::json!({}), Some("ngapain"));
        assert!(text_of(&r).contains("> Status: ❌ Nonaktif"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_keyword_is_lowercased_before_matching() {
        let r = run(serde_json::json!({}), Some("ON"));
        assert_eq!(text_of(&r), "✅ *ɴᴏᴛɪꜰ ᴄʟᴏsᴇ ɢʀᴏᴜᴘ ᴅɪᴀᴋᴛɪꜰᴋᴀɴ*");
        assert_eq!(group_write(&r), &serde_json::json!({"notifCloseGroup": true}));
    }

    #[test]
    fn on_writes_only_the_own_field() {
        let r = run(serde_json::json!({"notifCloseGroup": false, "mute": true}), Some("on"));
        assert_eq!(
            group_write(&r).as_object().unwrap().len(),
            1,
            "field lain tinggal di store, jadi tidak ikut dikirim"
        );
        assert_eq!(group_write(&r)["notifCloseGroup"], serde_json::json!(true));
    }

    #[test]
    fn off_writes_false_and_says_disabled() {
        let r = run(serde_json::json!({"notifCloseGroup": true}), Some("off"));
        assert_eq!(text_of(&r), "❌ *ɴᴏᴛɪꜰ ᴄʟᴏsᴇ ɢʀᴏᴜᴘ ᴅɪɴᴏɴᴀᴋᴛɪꜰᴋᴀɴ*");
        assert_eq!(group_write(&r), &serde_json::json!({"notifCloseGroup": false}));
    }

    #[test]
    fn the_status_help_uses_the_live_prefix_and_command() {
        let r = run(serde_json::json!({}), None);
        assert!(
            text_of(&r).ends_with(
                "*Penggunaan:*\n`.notifclosegroup on` - Aktifkan\n`.notifclosegroup off` - Nonaktifkan"
            ),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).starts_with("🔒 *ɴᴏᴛɪꜰ ᴄʟᴏsᴇ ɢʀᴏᴜᴘ*"));
    }
}