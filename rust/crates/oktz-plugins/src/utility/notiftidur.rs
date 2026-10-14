//! `plugins/utility/notiftidur.js` — pengingat tidur di
//! `settings.notifTidur[sender_chat]`.
//!
//! Bentuk entry-nya lebih tipis dari makan (`notif-scheduler.js:168-181` tidak
//! menulis `menu`), dan tampilannya tidak memakai label jam. Pembagian helper
//! ada di [`crate::utility::notifmakan`].

use oktz_plugin_api::{Ctx, Reply, plugin};
use serde_json::{Value, json};

use crate::common::{js_truthy, joined_strings};
use crate::utility::notifmakan::{
    entry_key, get_notif, jadwal_of, parse_jadwal, put_notif, status_of, store_of,
};

const SETTING_KEY: &str = "notifTidur";

/// `setNotifTidur` — `lastSent` dan `createdAt` lama dipertahankan.
fn set_notif_tidur(
    ctx: &Ctx,
    jadwal: &[String],
    store: serde_json::Map<String, Value>,
) -> oktz_plugin_api::Mutation {
    let key = entry_key(ctx);
    let prev = store.get(&key).cloned();
    let last_sent = prev
        .as_ref()
        .and_then(|p| p.get("lastSent"))
        .filter(|v| js_truthy(Some(v)))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let created_at = prev
        .as_ref()
        .and_then(|p| p.get("createdAt"))
        .filter(|v| js_truthy(Some(v)))
        .cloned()
        .unwrap_or_else(|| json!(crate::common::now_ms()));
    put_notif(
        ctx,
        SETTING_KEY,
        store,
        json!({
            "sender": ctx.sender,
            "chatJid": ctx.chat,
            "jadwal": jadwal,
            "enabled": true,
            "lastSent": last_sent,
            "createdAt": created_at,
        }),
    )
}

fn jadwal_bold(existing: Option<&Value>) -> String {
    match existing {
        Some(e) => {
            let body = joined_strings(jadwal_of(e))
                .iter()
                .map(|j| format!("*{j}* WIB"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("⏰ *Jadwal:* {body}")
        }
        None => String::new(),
    }
}

/// `existing.jadwal.map(j => `*${j}*`).join(', ')` — dipakai dua balasan
/// "sudah aktif"/"diaktifkan kembali".
fn jadwal_inline(existing: &Value) -> String {
    joined_strings(jadwal_of(existing))
        .iter()
        .map(|j| format!("*{j}*"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Balasan `on`/`edit`: judul baris jadwal berbeda antara keduanya.
fn jadwal_block(head: &str, heading: &str, jadwal: &[String], daily_note: bool) -> String {
    let mut out = format!("{head}\n\n⏰ *{heading}*\n");
    for j in jadwal {
        out.push_str(&format!("> 🕐 *{j}* WIB\n"));
    }
    if daily_note {
        out.push_str("\n> 💡 _Notifikasi akan dikirim ke chat ini setiap hari_");
    }
    out
}

fn info(ctx: &Ctx) -> String {
    let existing = get_notif(ctx, SETTING_KEY);
    format!(
        "🌙 *PENGINGAT TIDUR*\n\n\
         📌 *Status:* {}\n\
         {}\n\n\
         *📋 Cara Pakai:*\n\
         > `{}notiftidur on 22.00`\n\
         > `{}notiftidur on 22.00,23.30`\n\
         > `{}notiftidur edit 23.00`\n\
         > `{}notiftidur off`\n\
         \n> 💡 _Jam bisa pakai titik atau titik dua (22.00 / 22:00)_\n\
         > 💡 _Bisa multiple jam, pisahkan pakai koma_",
        status_of(existing.as_ref()),
        jadwal_bold(existing.as_ref()),
        ctx.prefix,
        ctx.prefix,
        ctx.prefix,
        ctx.prefix
    )
}


#[plugin(
    name = "notiftidur",
    alias = ["jadwaltidur", "tidurreminder", "sleepreminder"],
    category = "group",
    description = "Atur pengingat waktu tidur otomatis",
    usage = ".notiftidur on <jam1,jam2,...> / off / edit <jam1,jam2,...>",
    example = ".notiftidur on 22.00",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn notiftidur(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let existing = get_notif(&ctx, SETTING_KEY);

    if !matches!(sub.as_deref(), Some("on" | "off" | "edit")) {
        return Reply::text(info(&ctx));
    }

    if sub.as_deref() == Some("off") {
        let Some(mut entry) = existing else {
            return Reply::text("❌ *Belum ada pengingat tidur* yang aktif di chat ini");
        };
        entry["enabled"] = json!(false);
        return Reply::text(format!(
            "✅ *Pengingat tidur dinonaktifkan* 🔕\n\n\
             > Ketik `{}notiftidur on` untuk mengaktifkan kembali",
            ctx.prefix
        ))
        .with_mutation(put_notif(&ctx, SETTING_KEY, store_of(&ctx, SETTING_KEY), entry));
    }

    let bare = ctx.args.len() == 1;
    if sub.as_deref() == Some("on") && bare {
        if let Some(entry) = existing.clone().filter(|e| js_truthy(e.get("enabled"))) {
            return Reply::text(format!(
                "⚠️ *Pengingat tidur sudah aktif!*\n\n⏰ Jadwal: {} WIB\n\n\
                 > Gunakan `{}notiftidur edit` untuk mengubah jadwal",
                jadwal_inline(&entry),
                ctx.prefix
            ));
        }
        if let Some(mut entry) = existing {
            entry["enabled"] = json!(true);
            let text = format!(
                "✅ *Pengingat tidur diaktifkan kembali!* 🔔\n\n⏰ Jadwal: {} WIB",
                jadwal_inline(&entry)
            );
            return Reply::text(text)
                .with_mutation(put_notif(&ctx, SETTING_KEY, store_of(&ctx, SETTING_KEY), entry));
        }
    }

    if sub.as_deref() == Some("edit") && existing.is_none() {
        return Reply::text(format!(
            "❌ *Belum ada pengingat tidur!*\n\n> Aktifkan dulu: `{}notiftidur on 22.00`",
            ctx.prefix
        ));
    }

    let Some(input) = ctx.args.get(1).map(String::as_str) else {
        return Reply::text(format!(
            "❌ *Masukkan jadwal tidur!*\n\n> Contoh: `{}notiftidur on 22.00`",
            ctx.prefix
        ));
    };

    let jadwal = parse_jadwal(input);
    if jadwal.is_empty() {
        return Reply::text(
            "❌ *Format jam salah!*\n\n> Gunakan format *HH.MM* atau *HH:MM*\n\
             > Contoh: `22.00` atau `23.30`"
                .to_string(),
        );
    }

    let is_edit = sub.as_deref() == Some("edit");
    Reply::text(jadwal_block(
        if is_edit {
            "✅ *Jadwal tidur diperbarui!* ✏️"
        } else {
            "✅ *Pengingat tidur aktif!* 🔔"
        },
        if is_edit { "Jadwal baru:" } else { "Jadwal:" },
        &jadwal,
        !is_edit,
    ))
    .with_mutation(set_notif_tidur(&ctx, &jadwal, store_of(&ctx, SETTING_KEY)))
}

#[cfg(test)]
mod tests {
    use super::*;

    use oktz_plugin_api::Mutation;
    use serde_json::json;

    const SENDER: &str = "628111@s.whatsapp.net";
    const CHAT: &str = "120363@g.us";
    const KEY: &str = "628111@s.whatsapp.net_120363@g.us";

    fn run(settings: Value, args: &[&str]) -> Reply {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "notiftidur".into(),
            sender: SENDER.into(),
            chat: CHAT.into(),
            is_group: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            settings,
            ..Default::default()
        };
        (oktz_plugin_api::find("notiftidur").unwrap().handler)(ctx)
    }

    fn text_of(r: &Reply) -> &str {
        r.without_mutations().text_body().expect("harus ada teks")
    }

    fn setting_of(r: &Reply) -> Value {
        assert_eq!(r.mutations().len(), 1);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, value } => {
                assert_eq!(key, SETTING_KEY);
                value.clone()
            }
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }

    fn existing(jadwal: &[&str], enabled: bool) -> Value {
        json!({ SETTING_KEY: { KEY: {
            "sender": SENDER, "chatJid": CHAT, "jadwal": jadwal,
            "enabled": enabled, "lastSent": {"22:00_2026-01-01": true}, "createdAt": 1
        }}})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("notiftidur").unwrap();
        assert_eq!(p.meta.name, "notiftidur");
        assert_eq!(p.meta.alias, ["jadwaltidur", "tidurreminder", "sleepreminder"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Atur pengingat waktu tidur otomatis");
        assert_eq!(p.meta.usage, ".notiftidur on <jam1,jam2,...> / off / edit <jam1,jam2,...>");
        assert_eq!(p.meta.example, ".notiftidur on 22.00");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private, "keduanya false di JS");
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["jadwaltidur", "tidurreminder", "sleepreminder"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "notiftidur");
        }
    }

    #[test]
    fn without_a_recognised_keyword_the_plugin_shows_the_info_panel() {
        for args in [vec![], vec!["status"], vec!["nyanyi"]] {
            let r = run(json!({}), &args);
            let t = text_of(&r);
            assert!(t.starts_with("🌙 *PENGINGAT TIDUR*"), "args {args:?}: {t}");
            assert!(t.contains("📌 *Status:* ⚪ Belum diatur"), "args {args:?}: {t}");
            assert!(r.mutations().is_empty(), "args {args:?} hanya membaca");
        }
    }

    #[test]
    fn the_info_panel_shows_the_schedule_with_the_same_statuses_as_notifmakan() {
        let r = run(existing(&["22:00", "23:30"], true), &[]);
        let t = text_of(&r);
        assert!(t.contains("📌 *Status:* ✅ Aktif"), "{t}");
        assert!(t.contains("⏰ *Jadwal:* *22:00* WIB, *23:30* WIB"), "{t}");

        let r = run(existing(&["22:00"], false), &[]);
        let t = text_of(&r);
        assert!(t.contains("📌 *Status:* ❌ Nonaktif"), "{t}");
    }

    #[test]
    fn on_stores_the_schedule_without_a_menu_field() {
        let r = run(json!({}), &["on", "22.00,23.30"]);
        let entry = &setting_of(&r)[KEY];
        assert_eq!(entry["jadwal"], json!(["22:00", "23:30"]));
        assert_eq!(entry["enabled"], json!(true));
        assert!(
            entry.get("menu").is_none(),
            "setNotifTidur tidak menulis menu: {entry}"
        );
        let t = text_of(&r);
        assert!(t.starts_with("✅ *Pengingat tidur aktif!* 🔔\n\n⏰ *Jadwal:*"), "{t}");
        assert!(t.contains("> 🕐 *22:00* WIB\n> 🕐 *23:30* WIB\n"), "{t}");
        assert!(t.ends_with("> 💡 _Notifikasi akan dikirim ke chat ini setiap hari_"), "{t}");
    }

    #[test]
    fn an_existing_disabled_entry_is_reactivated_without_a_schedule() {
        let r = run(existing(&["22:00"], false), &["on"]);
        assert_eq!(
            text_of(&r),
            "✅ *Pengingat tidur diaktifkan kembali!* 🔔\n\n⏰ Jadwal: *22:00* WIB"
        );
        let entry = &setting_of(&r)[KEY];
        assert_eq!(entry["enabled"], json!(true));
        assert_eq!(entry["jadwal"], json!(["22:00"]));
    }

    #[test]
    fn an_already_active_entry_points_at_edit() {
        let r = run(existing(&["22:00", "23:30"], true), &["on"]);
        assert_eq!(
            text_of(&r),
            "⚠️ *Pengingat tidur sudah aktif!*\n\n⏰ Jadwal: *22:00*, *23:30* WIB\n\n\
             > Gunakan `.notiftidur edit` untuk mengubah jadwal"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn off_keeps_the_schedule_and_the_last_sent_bookkeeping() {
        let r = run(existing(&["22:00"], true), &["off"]);
        let entry = &setting_of(&r)[KEY];
        assert_eq!(entry["enabled"], json!(false));
        assert_eq!(entry["jadwal"], json!(["22:00"]));
        assert_eq!(entry["lastSent"], json!({"22:00_2026-01-01": true}));
        assert!(text_of(&r).contains("✅ *Pengingat tidur dinonaktifkan* 🔕"), "{}", text_of(&r));
    }

    #[test]
    fn off_without_an_entry_is_refused() {
        let r = run(json!({}), &["off"]);
        assert_eq!(text_of(&r), "❌ *Belum ada pengingat tidur* yang aktif di chat ini");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn edit_replaces_the_schedule_and_keeps_the_old_ones_metadata() {
        let r = run(existing(&["22:00"], true), &["edit", "23.00"]);
        let entry = &setting_of(&r)[KEY];
        assert_eq!(entry["jadwal"], json!(["23:00"]));
        assert_eq!(entry["lastSent"], json!({"22:00_2026-01-01": true}));
        assert_eq!(entry["createdAt"], json!(1));
        let t = text_of(&r);
        assert!(t.starts_with("✅ *Jadwal tidur diperbarui!* ✏️\n\n⏰ *Jadwal baru:*"), "{t}");
        assert!(!t.contains("setiap hari"), "catatan harian hanya untuk `on`: {t}");
    }

    #[test]
    fn edit_without_an_existing_entry_points_at_on() {
        let r = run(json!({}), &["edit", "23.00"]);
        assert_eq!(
            text_of(&r),
            "❌ *Belum ada pengingat tidur!*\n\n> Aktifkan dulu: `.notiftidur on 22.00`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_missing_or_malformed_schedule_is_refused() {
        for (settings, args) in [
            (json!({}), vec!["on"]),
            (json!({}), vec!["on", "abc"]),
            (json!({}), vec!["on", "24.00"]),
            (existing(&["22:00"], false), vec!["edit"]),
            (existing(&["22:00"], false), vec!["edit", "22:99"]),
        ] {
            let r = run(settings.clone(), &args);
            let expected = if args.len() == 1 {
                "❌ *Masukkan jadwal tidur!*\n\n> Contoh: `.notiftidur on 22.00`"
            } else {
                "❌ *Format jam salah!*\n\n> Gunakan format *HH.MM* atau *HH:MM*\n> Contoh: `22.00` atau `23.30`"
            };
            assert_eq!(text_of(&r), expected, "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn the_two_reminders_live_in_different_setting_keys() {
        let r = run(json!({}), &["on", "22.00"]);
        match &r.mutations()[0] {
            Mutation::SetSetting { key, .. } => assert_eq!(key, "notifTidur"),
            other => panic!("harus SetSetting, dapat {other:?}"),
        }
    }
}