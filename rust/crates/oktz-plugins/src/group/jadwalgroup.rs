//! `plugins/group/jadwalgroup.js` — `groups[chat].scheduleOpen`/`scheduleClose`.
//!
//! Jadwal di sini hanya disimpan; yang benar-benar membuka/menutup grup
//! dijalankan `handler.js` dari cron, bukan dari plugin ini.
//!
//! `ponytail:` `delete group.scheduleOpen` jadi `scheduleOpen = null`, karena
//! `Mutation::SetGroup` merge (`{...existing, ...data}`) dan tidak bisa
//! menghapus key. Pembaca di JS semuanya `group.scheduleOpen || null`, jadi
//! `null` dan key yang hilang tidak bisa dibedakan.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

use crate::common::pad2;

pub(crate) const BAD_ACTION: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
> Action harus `open` atau `close`!\n\n\
> *Contoh:*\n\
> `.jadwalgroup open 06:00`\n\
> `.jadwalgroup close 22:00`";

pub(crate) const BAD_HAPUS: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
> Gunakan: `.jadwalgroup hapus open`\n\
> atau: `.jadwalgroup hapus close`";

pub(crate) fn missing_time(action: &str) -> String {
    format!(
        "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
         > Waktu harus diisi!\n\n\
         > *Format:* `HH:MM` (24 jam)\n\
         > *Contoh:* `.jadwalgroup {action} 08:00`"
    )
}

pub(crate) const BAD_FORMAT: &str = "⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*\n\n\
> Format waktu tidak valid!\n\n\
> *Format:* `HH:MM` (24 jam)\n\
> *Contoh:* `06:00`, `22:30`, `08:15`";

/// `> Jadwal *buka grup*` / `*tutup grup*` — hanya kata itu yang berbeda.
pub(crate) fn cleared(kind: &str) -> String {
    format!("✅ *ʙᴇʀʜᴀsɪʟ*\n\n> Jadwal *{kind}* otomatis telah dihapus.")
}

/// `parseTime` (`jadwalgroup.js:22-33`) — `^(\d{1,2}):(\d{2})$` setelah semua
/// spasi dibuang. `None` kalau tidak cocok atau di luar rentang jam/menit.
pub(crate) fn parse_time(raw: &str) -> Option<(u32, u32)> {
    let cleaned: String = raw.trim().chars().filter(|c| !c.is_whitespace()).collect();
    let (h, m) = cleaned.split_once(':')?;
    if h.is_empty() || h.len() > 2 || m.len() != 2 {
        return None;
    }
    if !h.bytes().all(|b| b.is_ascii_digit()) || !m.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hours: u32 = h.parse().ok()?;
    let minutes: u32 = m.parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some((hours, minutes))
}

/// `group[field] || null` lalu di-`${}`-kan di templat status.
fn shown(ctx: &Ctx, field: &str) -> String {
    match ctx.group_field(field) {
        Some(v) if crate::common::js_truthy_value(v) => match v.as_str() {
            Some(s) => s.to_string(),
            None => v.to_string(),
        },
        _ => "Tidak aktif".to_string(),
    }
}

fn status(ctx: &Ctx) -> String {
    format!(
        "⏰ *ᴊᴀᴅᴡᴀʟ ɢʀᴜᴘ*\n\n\
         「 📋 *sᴛᴀᴛᴜs* 」\n\
         🔓 ᴏᴘᴇɴ: *{}*\n\
         🔒 ᴄʟᴏsᴇ: *{}*\n\n\
         *Cara Penggunaan:*\n\
         `.jadwalgroup open 06:00`\n\
         `.jadwalgroup close 22:00`\n\
         `.jadwalgroup hapus open`\n\
         `.jadwalgroup hapus close`",
        shown(ctx, "scheduleOpen"),
        shown(ctx, "scheduleClose")
    )
}

/// `time` dari `m.args`: `06 : 00` digabung, sisanya `args.slice(1).join('')`.
/// `None` kalau tidak ada argumen kedua.
fn time_arg(args: &[String]) -> Option<String> {
    if args.len() >= 4 && args[2] == ":" {
        return Some(format!("{}:{}", args[1], args[3]));
    }
    if args.len() >= 2 {
        return Some(args[1..].concat());
    }
    None
}

#[plugin(
    name = "jadwalgroup",
    alias = ["schedulegroup", "jdwlgrup", "autoopenclose"],
    category = "group",
    description = "Jadwal buka/tutup grup otomatis",
    usage = ".jadwalgroup <open/close> <HH:MM>",
    example = ".jadwalgroup open 06:00",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_admin = true,
    is_bot_admin = true
)]
pub fn jadwalgroup(ctx: Ctx) -> Reply {
    let action = ctx.args.first().map(|a| a.to_lowercase());
    let time = time_arg(&ctx.args);

    let Some(action) = action else {
        return Reply::text(status(&ctx));
    };

    if matches!(action.as_str(), "hapus" | "delete" | "remove") {
        let Some(kind) = ctx.args.get(1).map(|a| a.to_lowercase()) else {
            return Reply::text(BAD_HAPUS);
        };
        let field = match kind.as_str() {
            "open" => ("scheduleOpen", "buka grup"),
            "close" => ("scheduleClose", "tutup grup"),
            _ => return Reply::text(BAD_HAPUS),
        };
        return Reply::text(cleared(field.1)).with_mutation(Mutation::SetGroup {
            jid: ctx.chat.clone(),
            data: serde_json::json!({ field.0: Value::Null }),
        });
    }

    if action != "open" && action != "close" {
        return Reply::text(BAD_ACTION);
    }

    let Some(time) = time.filter(|t| !t.is_empty()) else {
        return Reply::text(missing_time(&action));
    };

    let Some((hours, minutes)) = parse_time(&time) else {
        return Reply::text(BAD_FORMAT);
    };

    let formatted = format!("{}:{}", pad2(hours), pad2(minutes));
    let (field, action_text, emoji, verb) = if action == "open" {
        ("scheduleOpen", "BUKA", "🔓", "dibuka")
    } else {
        ("scheduleClose", "TUTUP", "🔒", "ditutup")
    };

    Reply::text(format!(
        "✅ *ᴊᴀᴅᴡᴀʟ ᴅɪsɪᴍᴘᴀɴ*\n\n\
         ╭┈┈⬡「 ⏰ *sᴇᴛᴛɪɴɢ* 」\n\
         ┃ ㊗ {emoji} ᴀᴋsɪ: *{action_text}*\n\
         ┃ ㊗ ⏱️ ᴡᴏᴋᴛᴜ: *{formatted} WIB*\n\
         ┃ ㊗ 📡 sᴛᴀᴛᴜs: *🟢 Aktif*\n\
         ╰┈┈⬡\n\n\
         > _Grup akan otomatis {verb}_\n\
         > _setiap hari pada jam *{formatted}* WIB._"
    ))
    .with_mutation(Mutation::SetGroup {
        jid: ctx.chat.clone(),
        data: serde_json::json!({ field: formatted }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    fn run(group: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "jadwalgroup".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "120363@g.us".into(),
            is_group: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            group_data: group,
            ..Default::default()
        };
        (oktz_plugin_api::find("jadwalgroup").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn group_write(r: &Reply) -> &Value {
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
        let p = oktz_plugin_api::find("jadwalgroup").unwrap();
        assert_eq!(p.meta.name, "jadwalgroup");
        assert_eq!(p.meta.alias, ["schedulegroup", "jdwlgrup", "autoopenclose"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Jadwal buka/tutup grup otomatis");
        assert_eq!(p.meta.usage, ".jadwalgroup <open/close> <HH:MM>");
        assert_eq!(p.meta.example, ".jadwalgroup open 06:00");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && p.meta.is_admin && p.meta.is_bot_admin && !p.meta.is_private);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["schedulegroup", "jdwlgrup", "autoopenclose"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "jadwalgroup");
        }
    }

    #[test]
    fn no_argument_shows_both_schedules_as_inactive() {
        let r = run(json!({}), &[]);
        assert_eq!(
            text_of(&r),
            "⏰ *ᴊᴀᴅᴡᴀʟ ɢʀᴜᴘ*\n\n\
             「 📋 *sᴛᴀᴛᴜs* 」\n\
             🔓 ᴏᴘᴇɴ: *Tidak aktif*\n\
             🔒 ᴄʟᴏsᴇ: *Tidak aktif*\n\n\
             *Cara Penggunaan:*\n\
             `.jadwalgroup open 06:00`\n\
             `.jadwalgroup close 22:00`\n\
             `.jadwalgroup hapus open`\n\
             `.jadwalgroup hapus close`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_status_line_shows_stored_times() {
        let r = run(
            json!({"scheduleOpen": "06:00", "scheduleClose": "22:30"}),
            &[],
        );
        assert!(text_of(&r).contains("🔓 ᴏᴘᴇɴ: *06:00*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("🔒 ᴄʟᴏsᴇ: *22:30*"), "{}", text_of(&r));
    }

    #[test]
    fn setting_open_pads_the_hour_and_writes_schedule_open() {
        let r = run(json!({}), &["open", "6:00"]);
        assert_eq!(group_write(&r), &json!({"scheduleOpen": "06:00"}));
        assert!(
            text_of(&r).contains("┃ ㊗ ⏱️ ᴡᴏᴋᴛᴜ: *06:00 WIB*"),
            "{}",
            text_of(&r)
        );
        assert!(text_of(&r).contains("> _Grup akan otomatis dibuka_"));
        assert!(text_of(&r).contains("*🟢 Aktif*"));
    }

    #[test]
    fn setting_close_uses_the_close_wording() {
        let r = run(json!({}), &["CLOSE", "22:30"]);
        assert_eq!(group_write(&r), &json!({"scheduleClose": "22:30"}));
        assert!(text_of(&r).contains("ᴀᴋsɪ: *TUTUP*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> _Grup akan otomatis ditutup_"));
    }

    #[test]
    fn a_time_split_into_three_tokens_is_joined_again() {
        // `.jadwalgroup open 6 : 00` — `args[2] === ':'`.
        let r = run(json!({}), &["open", "6", ":", "00"]);
        assert_eq!(group_write(&r), &json!({"scheduleOpen": "06:00"}));
    }

    #[test]
    fn an_unknown_action_is_rejected() {
        let r = run(json!({}), &["jadwal", "06:00"]);
        assert_eq!(text_of(&r), BAD_ACTION);
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn hapus_clears_the_named_schedule_only() {
        let r = run(
            json!({"scheduleOpen": "06:00", "scheduleClose": "22:00"}),
            &["hapus", "open"],
        );
        assert_eq!(group_write(&r), &json!({"scheduleOpen": null}));
        assert!(text_of(&r).contains("*buka grup*"), "{}", text_of(&r));
    }

    #[test]
    fn hapus_close_names_the_close_schedule() {
        let r = run(json!({"scheduleClose": "22:00"}), &["delete", "close"]);
        assert_eq!(group_write(&r), &json!({"scheduleClose": null}));
        assert!(text_of(&r).contains("*tutup grup*"), "{}", text_of(&r));
    }

    #[test]
    fn hapus_with_a_wrong_or_missing_kind_is_rejected() {
        // `args[1]?.toLowerCase()` — "OPEN" tetap sah, "JADWAL" tidak.
        for args in [&["hapus"][..], &["remove", "jadwal"][..]] {
            let r = run(json!({}), args);
            assert_eq!(text_of(&r), BAD_HAPUS, "args {args:?}");
            assert!(r.mutations().is_empty());
        }
        let r = run(json!({"scheduleOpen": "06:00"}), &["remove", "OPEN"]);
        assert_eq!(group_write(&r), &json!({"scheduleOpen": null}), "huruf besar di-lowercase dulu");
    }

    #[test]
    fn a_missing_time_is_reported_with_the_action_name() {
        let r = run(json!({}), &["open"]);
        assert!(text_of(&r).starts_with("⚠️ *ᴠᴀʟɪᴅᴀsɪ ɢᴀɢᴀʟ*"), "{}", text_of(&r));
        assert!(text_of(&r).contains("> Waktu harus diisi!"), "{}", text_of(&r));
        assert!(text_of(&r).contains("`.jadwalgroup open 08:00`"), "{}", text_of(&r));
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn malformed_times_are_rejected_by_the_same_branch() {
        for t in ["25:00", "6:000", "abc", "06", "06:0", "", "-1:00", "06:60"] {
            let r = run(json!({}), &["open", t]);
            let expected = if t.is_empty() {
                missing_time("open")
            } else {
                BAD_FORMAT.to_string()
            };
            assert_eq!(text_of(&r), expected, "input {t:?}");
            assert!(r.mutations().is_empty(), "input {t:?}");
        }
    }

    #[test]
    fn parse_time_matches_the_js_regex() {
        assert_eq!(parse_time("06:00"), Some((6, 0)));
        assert_eq!(parse_time(" 6 : 30 "), Some((6, 30)));
        assert_eq!(parse_time("23:59"), Some((23, 59)));
        assert_eq!(parse_time("24:00"), None, "jam 24 di luar rentang");
        assert_eq!(parse_time("6:0"), None, "menit harus dua digit");
        assert_eq!(parse_time("060:00"), None, "jam maksimal dua digit");
        assert_eq!(parse_time("06:00:00"), None);
    }
}