//! `plugins/utility/notifmakan.js` — pengaturan pengingat makan di
//! `settings.notifMakan[sender_chat]`.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::{Value};

pub(crate) const SETTING_KEY: &str = "notifMakan";

/// `parseJadwal` (`notif-scheduler.js:205-217`) — `^(\d{1,2})[.:](\d{2})$`,
/// hasil dinormalkan ke `HH:MM`, didedupe, lalu diurutkan.
pub(crate) fn parse_jadwal(input: &str) -> Vec<String> {
    let mut out: Vec<String> = input
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .filter_map(|t| {
            let (h, m) = t.split_once(['.', ':'])?;
            if h.is_empty() || h.len() > 2 || m.len() != 2 {
                return None;
            }
            if !h.bytes().all(|b| b.is_ascii_digit()) || !m.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let h: u32 = h.parse().ok()?;
            let m: u32 = m.parse().ok()?;
            (h <= 23 && m <= 59).then(|| format!("{h:02}:{m:02}"))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// `entry.jadwal` — array apa adanya; bentuk lain (`undefined`, objek, teks)
/// menjadi kosong, persis `existing.jadwal.map(...)` yang akan melempar kalau
/// bukan array — di sini dibaca sebagai tidak ada jadwal.
pub(crate) fn jadwal_of(entry: &Value) -> &[Value] {
    entry
        .get("jadwal")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// `getMealLabel` — dipakai juga oleh scheduler yang mengirim pengingat.
pub(crate) fn meal_label(jam: &str) -> &'static str {
    let hour: i64 = jam.split(':').next().unwrap_or("0").parse().unwrap_or(0);
    match hour {
        4..=9 => "pagi",
        10..=14 => "siang",
        15..=17 => "sore",
        _ => "malam",
    }
}

/// `getNotifKey(sender, chatJid)`.
pub(crate) fn entry_key(ctx: &Ctx) -> String {
    format!("{}_{}", ctx.sender, ctx.chat)
}

/// `db.setting(key) || {}` — seluruh peta, untuk ditulis ulang utuh.
pub(crate) fn store_of(ctx: &Ctx, key: &str) -> serde_json::Map<String, Value> {
    ctx.setting(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// `data[key] || null`.
pub(crate) fn get_notif(ctx: &Ctx, key: &str) -> Option<Value> {
    store_of(ctx, key)
        .get(&entry_key(ctx))
        .cloned()
        .filter(crate::common::js_truthy_value)
}

/// `db.setting(key, data)` — mutasinya seluruh peta, bukan satu entri.
pub(crate) fn put_notif(ctx: &Ctx, key: &str, mut store: serde_json::Map<String, Value>, entry: Value) -> Mutation {
    store.insert(entry_key(ctx), entry);
    Mutation::SetSetting {
        key: key.into(),
        value: Value::Object(store),
    }
}

/// `setNotifMakan` (`notif-scheduler.js:150-166`) — `lastSent`/`createdAt`
/// lama dipertahankan.
pub(crate) fn set_notif_makan(
    ctx: &Ctx,
    jadwal: &[String],
    menu: &str,
    store: serde_json::Map<String, Value>,
) -> Mutation {
    let key = entry_key(ctx);
    let prev = store.get(&key).cloned();
    let last_sent = prev
        .as_ref()
        .and_then(|p| p.get("lastSent"))
        .filter(|v| crate::common::js_truthy_value(v))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let created_at = prev
        .as_ref()
        .and_then(|p| p.get("createdAt"))
        .filter(|v| crate::common::js_truthy_value(v))
        .cloned()
        .unwrap_or_else(|| serde_json::json!(crate::common::now_ms()));
    put_notif(
        ctx,
        SETTING_KEY,
        store,
        serde_json::json!({
            "sender": ctx.sender,
            "chatJid": ctx.chat,
            "jadwal": jadwal,
            "menu": menu,
            "enabled": true,
            "lastSent": last_sent,
            "createdAt": created_at,
        }),
    )
}

/// `existing ? (existing.enabled ? '✅ Aktif' : '❌ Nonaktif') : '⚪ Belum diatur'`
pub(crate) fn status_of(existing: Option<&Value>) -> &'static str {
    match existing {
        None => "⚪ Belum diatur",
        Some(e) => {
            if crate::common::js_truthy(e.get("enabled")) {
                "✅ Aktif"
            } else {
                "❌ Nonaktif"
            }
        }
    }
}

fn jadwal_line(existing: Option<&Value>) -> String {
    match existing {
        Some(e) => {
            let list = crate::common::joined_strings(jadwal_of(e));
            let body = list
                .iter()
                .map(|j| format!("*{j}* WIB"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("⏰ *Jadwal:* {body}")
        }
        None => String::new(),
    }
}

fn info(ctx: &Ctx) -> String {
    let existing = get_notif(ctx, SETTING_KEY);
    let mut out = String::from("🍽️ *PENGINGAT MAKAN*\n\n");
    out.push_str(&format!("📌 *Status:* {}\n", status_of(existing.as_ref())));
    out.push_str(&jadwal_line(existing.as_ref()));
    if let Some(menu) = existing
        .as_ref()
        .and_then(|e| e.get("menu"))
        .filter(|v| crate::common::js_truthy_value(v))
        .and_then(Value::as_str)
    {
        out.push_str(&format!("\n🍴 *Menu:* _{menu}_"));
    }
    out.push_str(&format!(
        "\n\n*📋 Cara Pakai:*\n\
         > `{}notifmakan on 07.00,12.00,19.00`\n\
         > `{}notifmakan on 07.00,12.00 Nasi Goreng`\n\
         > `{}notifmakan edit 08.00,13.00`\n\
         > `{}notifmakan off`\n\
         \n> 💡 _Jam bisa pakai titik atau titik dua (07.00 / 07:00)_\n\
         > 💡 _Bisa multiple jam, pisahkan pakai koma_",
        ctx.prefix,
        ctx.prefix,
        ctx.prefix,
        ctx.prefix
    ));
    out
}

fn jadwal_reply(head: &str, heading: &str, jadwal: &[String], menu: &str) -> String {
    let mut out = format!("{head}\n\n⏰ *{heading}*\n");
    for j in jadwal {
        out.push_str(&format!("> 🕐 *{j}* WIB _({})_\n", meal_label(j)));
    }
    if !menu.is_empty() {
        out.push_str(&format!("\n🍴 *Menu:* _{menu}_"));
    }
    out
}

#[plugin(
    name = "notifmakan",
    alias = ["jadwalmakan", "makanreminder"],
    category = "group",
    description = "Atur pengingat waktu makan otomatis",
    usage = ".notifmakan on <jam1,jam2,...> [menu] / off / edit <jam1,jam2,...> [menu]",
    example = ".notifmakan on 07.00,12.00,19.00 Nasi Padang",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn notifmakan(ctx: Ctx) -> Reply {
    let sub = ctx.args.first().map(|a| a.to_lowercase());
    let existing = get_notif(&ctx, SETTING_KEY);

    if !matches!(sub.as_deref(), Some("on" | "off" | "edit")) {
        return Reply::text(info(&ctx));
    }

    let bare = ctx.args.len() == 1;

    if sub.as_deref() == Some("off") {
        let Some(mut entry) = existing else {
            return Reply::text("❌ *Belum ada pengingat makan* yang aktif di chat ini");
        };
        entry["enabled"] = serde_json::json!(false);
        return Reply::text(format!(
            "✅ *Pengingat makan dinonaktifkan* 🔕\n\n\
             > Ketik `{}notifmakan on` untuk mengaktifkan kembali",
            ctx.prefix
        ))
        .with_mutation(put_notif(&ctx, SETTING_KEY, store_of(&ctx, SETTING_KEY), entry));
    }

    let jadwal_input = ctx.args.get(1).map(String::as_str);

    // `on` tanpa jadwal pada entri yang sudah ada = toggle, bukan reset.
    if sub.as_deref() == Some("on") && bare {
        if let Some(entry) = existing.clone().filter(|e| crate::common::js_truthy(e.get("enabled"))) {
            let list = crate::common::joined_strings(jadwal_of(&entry));
            return Reply::text(format!(
                "⚠️ *Pengingat makan sudah aktif!*\n\n⏰ Jadwal: {} WIB\n\n\
                 > Gunakan `{}notifmakan edit` untuk mengubah jadwal",
                list.iter().map(|j| format!("*{j}*")).collect::<Vec<_>>().join(", "),
                ctx.prefix
            ));
        }
        if let Some(mut entry) = existing {
            entry["enabled"] = serde_json::json!(true);
            let list = crate::common::joined_strings(jadwal_of(&entry));
            return Reply::text(format!(
                "✅ *Pengingat makan diaktifkan kembali!* 🔔\n\n⏰ Jadwal: {} WIB",
                list.iter().map(|j| format!("*{j}*")).collect::<Vec<_>>().join(", ")
            ))
            .with_mutation(put_notif(&ctx, SETTING_KEY, store_of(&ctx, SETTING_KEY), entry));
        }
    }

    if sub.as_deref() == Some("edit") && existing.is_none() {
        return Reply::text(format!(
            "❌ *Belum ada pengingat makan!*\n\n> Aktifkan dulu: `{}notifmakan on 07.00,12.00,19.00`",
            ctx.prefix
        ));
    }

    let Some(input) = jadwal_input else {
        return Reply::text(format!(
            "❌ *Masukkan jadwal makan!*\n\n> Contoh: `{}notifmakan on 07.00,12.00,19.00`",
            ctx.prefix
        ));
    };

    let jadwal = parse_jadwal(input);
    if jadwal.is_empty() {
        return Reply::text(
            "❌ *Format jam salah!*\n\n> Gunakan format *HH.MM* atau *HH:MM*\n\
             > Contoh: `07.00,12.30,19.00`"
                .to_string(),
        );
    }

    let typed_menu = ctx.args[2.min(ctx.args.len())..].join(" ").trim().to_string();
    let menu = if sub.as_deref() == Some("edit") && typed_menu.is_empty() {
        existing
            .as_ref()
            .and_then(|e| e.get("menu"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    } else {
        typed_menu
    };

    let is_edit = sub.as_deref() == Some("edit");
    let mut reply = jadwal_reply(
        if is_edit {
            "✅ *Jadwal makan diperbarui!* ✏️"
        } else {
            "✅ *Pengingat makan aktif!* 🔔"
        },
        if is_edit { "Jadwal baru:" } else { "Jadwal:" },
        &jadwal,
        &menu,
    );
    if !is_edit {
        reply.push_str("\n\n> 💡 _Notifikasi akan dikirim ke chat ini setiap hari_");
    }

    Reply::text(reply)
        .with_mutation(set_notif_makan(&ctx, &jadwal, &menu, store_of(&ctx, SETTING_KEY)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SENDER: &str = "628111@s.whatsapp.net";
    const CHAT: &str = "120363@g.us";
    const KEY: &str = "628111@s.whatsapp.net_120363@g.us";

    fn ctx(settings: Value, args: &[&str]) -> Ctx {
        Ctx {
            prefix: ".".into(),
            command: "notifmakan".into(),
            sender: SENDER.into(),
            chat: CHAT.into(),
            is_group: true,
            args: args.iter().map(|s| s.to_string()).collect(),
            settings,
            ..Default::default()
        }
    }

    fn run(settings: Value, args: &[&str]) -> Reply {
        (oktz_plugin_api::find("notifmakan").unwrap().handler)(ctx(settings, args))
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

    /// Nilai `Mutation::SetSetting` = peta entri yang menjadi isi setting
    /// `notifMakan`, jadi entri dibaca langsung dari sana.
    fn entry_of(written: &Value) -> Value {
        written[KEY].clone()
    }

    fn existing(jadwal: &[&str], menu: &str, enabled: bool) -> Value {
        serde_json::json!({ SETTING_KEY: { KEY: {
            "sender": SENDER, "chatJid": CHAT, "jadwal": jadwal,
            "menu": menu, "enabled": enabled, "lastSent": {"07:00_2026-01-01": true},
            "createdAt": 1
        }}})
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("notifmakan").unwrap();
        assert_eq!(p.meta.name, "notifmakan");
        assert_eq!(p.meta.alias, ["jadwalmakan", "makanreminder"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Atur pengingat waktu makan otomatis");
        assert_eq!(
            p.meta.usage,
            ".notifmakan on <jam1,jam2,...> [menu] / off / edit <jam1,jam2,...> [menu]"
        );
        assert_eq!(p.meta.example, ".notifmakan on 07.00,12.00,19.00 Nasi Padang");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_group && !p.meta.is_private, "keduanya false di JS");
        assert!(!p.meta.is_admin && !p.meta.is_bot_admin && !p.meta.is_owner);
    }

    #[test]
    fn every_alias_resolves_to_this_plugin() {
        for a in ["jadwalmakan", "makanreminder"] {
            assert_eq!(oktz_plugin_api::find(a).unwrap().meta.name, "notifmakan");
        }
    }

    #[test]
    fn without_a_recognised_keyword_the_plugin_shows_the_info_panel() {
        for args in [vec![], vec!["status"], vec!["ON!"]] {
            let r = run(serde_json::json!({}), &args);
            let t = text_of(&r);
            assert!(t.starts_with("🍽️ *PENGINGAT MAKAN*"), "args {args:?}: {t}");
            assert!(t.contains("📌 *Status:* ⚪ Belum diatur"), "args {args:?}: {t}");
            assert!(r.mutations().is_empty(), "args {args:?} hanya membaca");
        }
    }

    #[test]
    fn the_info_panel_lists_schedule_menu_and_status() {
        let r = run(existing(&["07:00", "19:00"], "Nasi Padang", true), &[]);
        let t = text_of(&r);
        assert!(t.contains("📌 *Status:* ✅ Aktif"), "{t}");
        assert!(t.contains("⏰ *Jadwal:* *07:00* WIB, *19:00* WIB"), "{t}");
        assert!(t.contains("🍴 *Menu:* _Nasi Padang_"), "{t}");

        let r = run(existing(&["07:00"], "", false), &[]);
        let t = text_of(&r);
        assert!(t.contains("📌 *Status:* ❌ Nonaktif"), "{t}");
        assert!(!t.contains("🍴 *Menu:*"), "menu kosong tidak boleh tampil: {t}");
    }

    #[test]
    fn on_stores_the_schedule_with_its_meal_labels() {
        let r = run(serde_json::json!({}), &["on", "07.00,12.00,19.00"]);
        let entry = entry_of(&setting_of(&r));
        assert_eq!(entry["jadwal"], serde_json::json!(["07:00", "12:00", "19:00"]));
        assert_eq!(entry["menu"], serde_json::json!(""));
        assert_eq!(entry["enabled"], serde_json::json!(true));
        assert_eq!(entry["sender"], serde_json::json!(SENDER));
        assert_eq!(entry["chatJid"], serde_json::json!(CHAT));
        assert_eq!(entry["lastSent"], serde_json::json!({}), "entri baru tidak punya lastSent");
        assert!(entry["createdAt"].as_i64().unwrap() > 0, "createdAt diisi");

        let t = text_of(&r);
        assert!(t.starts_with("✅ *Pengingat makan aktif!* 🔔\n\n⏰ *Jadwal:*"), "{t}");
        assert!(t.contains("> 🕐 *07:00* WIB _(pagi)_"), "{t}");
        assert!(t.contains("> 🕐 *12:00* WIB _(siang)_"), "{t}");
        assert!(t.contains("> 🕐 *19:00* WIB _(malam)_"), "{t}");
        assert!(t.ends_with("> 💡 _Notifikasi akan dikirim ke chat ini setiap hari_"), "{t}");
    }

    #[test]
    fn on_keeps_the_joined_menu_after_the_schedule() {
        let r = run(serde_json::json!({}), &["on", "07.00,12.00", "Nasi", "Goreng"]);
        assert_eq!(entry_of(&setting_of(&r))["menu"], serde_json::json!("Nasi Goreng"));
        assert!(text_of(&r).contains("🍴 *Menu:* _Nasi Goreng_"), "{}", text_of(&r));
    }

    #[test]
    fn an_existing_disabled_entry_is_reactivated_without_a_schedule() {
        let r = run(existing(&["07:00", "19:00"], "", false), &["on"]);
        assert_eq!(
            text_of(&r),
            "✅ *Pengingat makan diaktifkan kembali!* 🔔\n\n⏰ Jadwal: *07:00*, *19:00* WIB"
        );
        let entry = entry_of(&setting_of(&r));
        assert_eq!(entry["enabled"], serde_json::json!(true));
        assert_eq!(entry["jadwal"], serde_json::json!(["07:00", "19:00"]), "jadwal tidak berubah");
    }

    #[test]
    fn an_already_active_entry_tells_the_user_to_use_edit() {
        let r = run(existing(&["07:00"], "Gado", true), &["on"]);
        assert_eq!(
            text_of(&r),
            "⚠️ *Pengingat makan sudah aktif!*\n\n⏰ Jadwal: *07:00* WIB\n\n\
             > Gunakan `.notifmakan edit` untuk mengubah jadwal"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn off_disables_but_keeps_the_schedule_and_menu() {
        let r = run(existing(&["07:00"], "Gado", true), &["off"]);
        let entry = entry_of(&setting_of(&r));
        assert_eq!(entry["enabled"], serde_json::json!(false));
        assert_eq!(entry["jadwal"], serde_json::json!(["07:00"]));
        assert_eq!(entry["menu"], serde_json::json!("Gado"));
        assert_eq!(entry["lastSent"], serde_json::json!({"07:00_2026-01-01": true}), "lastSent tidak hilang");
        assert!(text_of(&r).contains("✅ *Pengingat makan dinonaktifkan* 🔕"), "{}", text_of(&r));
    }

    #[test]
    fn off_without_an_entry_is_refused() {
        for s in [serde_json::json!({}), serde_json::json!({ SETTING_KEY: {} })] {
            let r = run(s.clone(), &["off"]);
            assert_eq!(text_of(&r), "❌ *Belum ada pengingat makan* yang aktif di chat ini", "input {s}");
            assert!(r.mutations().is_empty(), "input {s}");
        }
    }

    #[test]
    fn edit_replaces_the_schedule_and_keeps_the_old_menu() {
        let r = run(existing(&["07:00"], "Gado", true), &["edit", "08.00,20.00"]);
        let entry = entry_of(&setting_of(&r));
        assert_eq!(entry["jadwal"], serde_json::json!(["08:00", "20:00"]));
        assert_eq!(entry["menu"], serde_json::json!("Gado"), "menu lama dipakai kalau tidak ada yang baru");
        assert_eq!(entry["createdAt"], serde_json::json!(1), "createdAt lama dipertahankan");
        assert_eq!(entry["lastSent"], serde_json::json!({"07:00_2026-01-01": true}), "lastSent lama dipertahankan");
        let t = text_of(&r);
        assert!(t.starts_with("✅ *Jadwal makan diperbarui!* ✏️\n\n⏰ *Jadwal baru:*"), "{t}");
        assert!(!t.contains("setiap hari"), "catatan harian hanya untuk `on`: {t}");
    }

    #[test]
    fn edit_with_a_new_menu_overrides_the_old_one() {
        let r = run(existing(&["07:00"], "Gado", true), &["edit", "08.00", "Soto"]);
        assert_eq!(entry_of(&setting_of(&r))["menu"], serde_json::json!("Soto"));
    }

    #[test]
    fn edit_without_an_existing_entry_points_at_on() {
        let r = run(serde_json::json!({}), &["edit", "08.00"]);
        assert_eq!(
            text_of(&r),
            "❌ *Belum ada pengingat makan!*\n\n> Aktifkan dulu: `.notifmakan on 07.00,12.00,19.00`"
        );
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn a_missing_or_malformed_schedule_is_refused_by_the_same_branch() {
        // `on` tanpa jadwal memakai store kosong supaya tidak tertahan oleh
        // cabang "sudah aktif"; `edit` butuh entri tapi tidak boleh enabled.
        let on_missing = serde_json::json!({});
        let edit_base = existing(&["07:00"], "", false);
        for (settings, args) in [
            (on_missing.clone(), vec!["on"]),
            (on_missing.clone(), vec!["on", "abc"]),
            (on_missing.clone(), vec!["on", "25:00"]),
            (on_missing.clone(), vec!["on", "7"]),
            (on_missing.clone(), vec!["on", "07:5"]),
            (on_missing.clone(), vec!["on", ",,,"]),
            (edit_base.clone(), vec!["edit"]),
            (edit_base.clone(), vec!["edit", "24.00"]),
            (edit_base.clone(), vec!["edit", "abc"]),
        ] {
            let r = run(settings.clone(), &args);
            let expected = if args.len() == 1 {
                "❌ *Masukkan jadwal makan!*\n\n> Contoh: `.notifmakan on 07.00,12.00,19.00`"
            } else {
                "❌ *Format jam salah!*\n\n> Gunakan format *HH.MM* atau *HH:MM*\n> Contoh: `07.00,12.30,19.00`"
            };
            assert_eq!(text_of(&r), expected, "args {args:?}");
            assert!(r.mutations().is_empty(), "args {args:?}");
        }
    }

    #[test]
    fn the_written_setting_keeps_other_chats_entries() {
        let s = serde_json::json!({ SETTING_KEY: {
            "628222@s.whatsapp.net_120363@g.us": {"jadwal": ["09:00"], "enabled": true},
            KEY: {"jadwal": ["07:00"], "enabled": true, "menu": ""}
        }});
        let __balasan = run(s.clone(), &["on", "10.00"]);
        let written = setting_of(&__balasan);
        let map = written.as_object().unwrap();
        assert_eq!(map.len(), 2, "entri milik chat lain tidak boleh hilang");
        assert_eq!(map["628222@s.whatsapp.net_120363@g.us"]["jadwal"], serde_json::json!(["09:00"]));
        assert_eq!(map[KEY]["jadwal"], serde_json::json!(["10:00"]));
    }

    #[test]
    fn parse_jadwal_accepts_both_separators_and_normalizes() {
        assert_eq!(parse_jadwal("07.00,12:00"), vec!["07:00", "12:00"]);
        assert_eq!(parse_jadwal(" 07.00 , 12.00 "), vec!["07:00", "12:00"]);
        assert_eq!(parse_jadwal("19.00,07.00"), vec!["07:00", "19:00"], "diurutkan");
        assert_eq!(parse_jadwal("07.00,07.00"), vec!["07:00"], "didedupe");
        assert_eq!(parse_jadwal("23:59"), vec!["23:59"]);
    }

    #[test]
    fn parse_jadwal_drops_out_of_range_and_malformed_times() {
        for bad in ["24.00", "07.60", "7", "07", "abc", ".00", "07.000"] {
            assert!(parse_jadwal(bad).is_empty(), "input {bad} harus ditolak");
        }
        assert_eq!(parse_jadwal("24.00,07.00"), vec!["07:00"], "yang valid tetap lewat");
    }

    #[test]
    fn meal_labels_follow_the_js_bands() {
        for (jam, label) in [
            ("04:00", "pagi"),
            ("09:59", "pagi"),
            ("10:00", "siang"),
            ("14:59", "siang"),
            ("15:00", "sore"),
            ("17:59", "sore"),
            ("18:00", "malam"),
            ("03:59", "malam"),
            ("23:59", "malam"),
        ] {
            assert_eq!(meal_label(jam), label, "jam {jam}");
        }
    }
}