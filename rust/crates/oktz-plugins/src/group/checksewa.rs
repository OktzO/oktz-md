//! `plugins/group/checksewa.js` — sisa waktu sewa bot di grup ini.
//!
//! `ponytail:` JS menyimpan data sewa di `db.db.data.sewa`, yaitu tabel
//! top-level yang bukan milik user maupun grup. `Mutation` hanya punya
//! `SetUser`/`SetGroup`/`SetSetting`, jadi tabel itu dipetakan ke key global
//! `sewa` di `settings` — persis yang dilakukan `clan/clancreate.rs` untuk
//! `db.db.data.clans` dan `rpg/guild.rs` untuk `db.db.data.guilds`. Bentuk
//! isinya sama dengan JS, hanya lokasinya yang berbeda.
//!
//! `formatCountdown` memakai `Math.floor`, jadi pecahan milidetik dipangkas ke
//! bawah — sama dengan pembagian integer yang dipakai di sini.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};

use serde_json::{Value};

/// Key global tempat `db.db.data.sewa` dipetakan.
const SEWA_KEY: &str = "sewa";

const DAY_MS: f64 = 86_400_000.0;
const HOUR_MS: f64 = 3_600_000.0;
const MINUTE_MS: f64 = 60_000.0;
/// `diff <= 259200000` (`checksewa.js:77`) — tiga hari.
const ALMOST_MS: f64 = 259_200_000.0;

/// `db.db.data.sewa` — `Value::Null` kalau tabel belum pernah dibuat.
fn sewa(ctx: &Ctx) -> Value {
    ctx.setting(SEWA_KEY).cloned().unwrap_or(Value::Null)
}

/// `if (!db.db.data.sewa) { db.db.data.sewa = {...}; db.db.write() }`.
fn init_if_missing(ctx: &Ctx) -> Option<Mutation> {
    sewa(ctx)
        .is_null()
        .then(|| Mutation::SetSetting {
            key: SEWA_KEY.to_string(),
            value: serde_json::json!({ "enabled": false, "groups": {} }),
        })
}

/// `new Date(x)` -> epoch ms. `None` berarti `Invalid Date` di JS, dan
/// `Intl.DateTimeFormat.formatToParts` melempar `RangeError` atasnya.
fn epoch_ms(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    if let Some(n) = v.as_f64() {
        return Some(n as i64);
    }
    v.as_str()?.trim().parse::<f64>().ok().map(|n| n as i64)
}

/// `formatCountdown(expiredAt)` (`checksewa.js:20-31`) — teks + flag expired.
fn format_countdown(expired_at: i64, now: i64) -> (String, bool) {
    let diff = (expired_at - now) as f64;
    if diff <= 0.0 {
        return ("EXPIRED".to_string(), true);
    }
    let days = (diff / DAY_MS).floor();
    let hours = ((diff % DAY_MS) / HOUR_MS).floor();
    let minutes = ((diff % HOUR_MS) / MINUTE_MS).floor();
    let mut text = String::new();
    if days > 0.0 {
        text.push_str(&format!("{days} hari "));
    }
    if hours > 0.0 {
        text.push_str(&format!("{hours} jam "));
    }
    if minutes > 0.0 && days == 0.0 {
        text.push_str(&format!("{minutes} menit"));
    }
    (text.trim().to_string(), false)
}

/// `fromTimestamp(ts, 'D MMMM YYYY')`.
fn tanggal(ms: i64) -> String {
    crate::common::date_long(ms)
}

/// `fromTimestamp(ts, 'D MMMM YYYY HH:mm')`.
fn tanggal_jam(ms: i64) -> String {
    format!("{} {}", crate::common::date_long(ms), crate::common::time_hm(ms))
}

fn lifetime_text(group_name: &str, added: &str) -> String {
    format!(
        "♾️ *STATUS SEWA*\n\n\
         Grup: *{group_name}*\n\
         Status: *Permanent* ♾️\n\
         Terdaftar sejak: *{added}*\n\n\
         Bot akan aktif selamanya di grup ini."
    )
}

fn expired_text(group_name: &str, expired: &str) -> String {
    format!(
        "❌ *SEWA EXPIRED*\n\n\
         Grup: *{group_name}*\n\
         Berakhir: *{expired}*\n\n\
         Hubungi owner bot untuk perpanjang sewa."
    )
}

fn active_text(
    group_name: &str,
    countdown: &str,
    expired: &str,
    added: &str,
    almost: bool,
) -> String {
    let mut text = format!(
        "⏱️ *STATUS SEWA*\n\n\
         Grup: *{group_name}*\n\
         Sisa waktu: *{countdown}*\n\
         Berakhir: *{expired}*\n\
         Terdaftar sejak: *{added}*"
    );
    if almost {
        text.push_str("\n\n⚠️ Sewa hampir habis! Hubungi owner bot untuk perpanjang.");
    }
    text
}

fn with_init(reply: Reply, init: Option<Mutation>) -> Reply {
    match init {
        Some(m) => reply.with_mutation(m),
        None => reply,
    }
}

#[plugin(
    name = "checksewa",
    alias = ["ceksewa", "sisasewa"],
    category = "group",
    description = "Cek sisa waktu sewa bot di grup ini",
    usage = ".checksewa",
    example = ".checksewa",
    cooldown = 10,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = true,
    is_private = false,
    is_admin = false
)]
pub fn checksewa(ctx: Ctx) -> Reply {
    let init = init_if_missing(&ctx);
    let table = sewa(&ctx);

    if !crate::common::js_truthy(table.get("enabled")) {
        let reply =
            Reply::text("ℹ️ Sistem sewa tidak aktif\n\nBot ini bisa digunakan di semua grup.");
        return with_init(reply, init);
    }

    let entry = table.get("groups").and_then(|g| g.get(&ctx.chat));
    if !crate::common::js_truthy(entry) {
        let reply = Reply::text(
            "❌ Grup ini tidak terdaftar dalam sistem sewa\n\nHubungi owner bot untuk info sewa.",
        );
        return with_init(reply, init);
    }
    let entry = entry.cloned().unwrap_or(Value::Null);

    let group_name = match entry.get("name") {
        Some(name) if crate::common::js_truthy_value(name) => name.as_str().unwrap_or("").to_string(),
        _ => crate::common::jid_number(&ctx.chat).to_string(),
    };
    let added = match entry.get("addedAt") {
        Some(v) if crate::common::js_truthy_value(v) => tanggal(epoch_ms(Some(v)).unwrap_or(0)),
        _ => "-".to_string(),
    };

    if crate::common::js_truthy(entry.get("isLifetime")) {
        let reply = Reply::text(lifetime_text(&group_name, &added)).with_reaction("♾️");
        return with_init(reply, init);
    }

    // `timeHelper.fromTimestamp` atas `Invalid Date` melempar `RangeError`, jadi
    // JS tidak pernah sampai ke balasan mana pun untuk tenant tanpa
    // `expiredAt`. `Reply::None` + diam adalah padanan terdekat; teks fallback
    // "Sepertinya ada kendala, coba hubungi owner" milik `handler.js`, bukan
    // milik plugin ini.
    let Some(expired_at) = epoch_ms(entry.get("expiredAt")) else {
        return with_init(Reply::None, init);
    };

    let now = crate::common::now_ms();
    let (countdown, is_expired) = format_countdown(expired_at, now);
    let expired_str = tanggal_jam(expired_at);

    if is_expired {
        return with_init(
            Reply::text(expired_text(&group_name, &expired_str)),
            init,
        );
    }

    let almost = ((expired_at - now) as f64) <= ALMOST_MS;
    let emoji = if almost { "⚠️" } else { "⏱️" };
    let reply = Reply::text(active_text(&group_name, &countdown, &expired_str, &added, almost))
        .with_reaction(emoji);
    with_init(reply, init)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const CHAT: &str = "120363@g.us";

    fn run(chat: &str, settings: Value) -> Reply {
        (find("checksewa").unwrap().handler)(Ctx {
            prefix: ".".into(),
            command: "checksewa".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: chat.into(),
            is_group: true,
            settings,
            ..Default::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations().text_body().unwrap_or_default().to_string()
    }

    fn init_written(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == SEWA_KEY => Some(value.clone()),
            _ => None,
        })
    }

    fn with_sewa(enabled: bool, groups: Value) -> Value {
        serde_json::json!({ "sewa": { "enabled": enabled, "groups": groups } })
    }

    fn tenant(extra: Value) -> Value {
        with_sewa(true, serde_json::json!({ CHAT: extra }))
    }

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = find("checksewa").unwrap();
        assert_eq!(p.meta.name, "checksewa");
        assert_eq!(p.meta.alias, ["ceksewa", "sisasewa"]);
        assert_eq!(p.meta.category, "group");
        assert_eq!(p.meta.description, "Cek sisa waktu sewa bot di grup ini");
        assert_eq!(p.meta.usage, ".checksewa");
        assert_eq!(p.meta.example, ".checksewa");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_owner && !p.meta.is_premium);
        assert!(!p.meta.is_admin, "isAdmin:false di JS");
        assert!(!p.meta.is_bot_admin, "tidak ada di JS");
    }

    #[test]
    fn every_alias_resolves_back_to_this_plugin() {
        for a in ["ceksewa", "sisasewa"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("checksewa"), "{a}");
        }
    }

    #[test]
    fn a_missing_table_is_created_once_and_reported_as_inactive() {
        let r = run(CHAT, Value::Null);
        assert_eq!(
            text_of(&r),
            "ℹ️ Sistem sewa tidak aktif\n\nBot ini bisa digunakan di semua grup."
        );
        assert_eq!(init_written(&r), Some(serde_json::json!({ "enabled": false, "groups": {} })));
    }

    #[test]
    fn an_existing_table_is_never_rewritten() {
        let r = run(CHAT, with_sewa(false, serde_json::json!({})));
        assert!(init_written(&r).is_none(), "tabel sudah ada -> jangan tulis ulang");
    }

    #[test]
    fn an_unknown_group_is_told_it_is_not_registered() {
        let r = run(CHAT, with_sewa(true, serde_json::json!({ "other@g.us": {} })));
        assert_eq!(
            text_of(&r),
            "❌ Grup ini tidak terdaftar dalam sistem sewa\n\nHubungi owner bot untuk info sewa."
        );
    }

    #[test]
    fn a_lifetime_tenant_reacts_with_the_infinity_sign() {
        let r = run(
            CHAT,
            tenant(serde_json::json!({ "name": "Grupku", "isLifetime": true, "addedAt": 1_700_000_000_000i64 })),
        );
        let t = text_of(&r);
        assert!(t.starts_with("♾️ *STATUS SEWA*"), "{t}");
        assert!(t.contains("Grup: *Grupku*"), "{t}");
        assert!(t.contains("Status: *Permanent* ♾️"), "{t}");
        assert!(t.contains("Terdaftar sejak: *15 November 2023*"), "{t}");
        assert!(t.ends_with("Bot akan aktif selamanya di grup ini."), "{t}");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "♾️"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn a_missing_added_at_falls_back_to_a_dash() {
        let r = run(CHAT, tenant(serde_json::json!({ "isLifetime": true })));
        assert!(text_of(&r).contains("Terdaftar sejak: *-*"), "{}", text_of(&r));
    }

    #[test]
    fn an_expired_tenant_gets_the_expired_card() {
        let r = run(CHAT, tenant(serde_json::json!({ "expiredAt": 1_700_000_000_000i64 })));
        let t = text_of(&r);
        assert!(t.starts_with("❌ *SEWA EXPIRED*"), "{t}");
        assert!(t.contains("Berakhir: *15 November 2023 05:13*"), "{t}");
        assert!(r.mutations().is_empty());
    }

    #[test]
    fn the_countdown_drops_units_that_are_zero_like_the_js_template() {
        let now = 1_700_000_000_000i64;
        let hour = 3_600_000i64;
        let minute = 60_000i64;
        let day = 86_400_000i64;
        // `minutes` hanya ikut kalau `days === 0` (`checksewa.js:29`).
        assert_eq!(format_countdown(now + 24 * hour + 30 * minute, now).0, "1 hari");
        assert_eq!(format_countdown(now + day + 5 * hour + 30 * minute, now).0, "1 hari 5 jam");
        assert_eq!(format_countdown(now + 3 * hour, now).0, "3 jam");
        assert_eq!(format_countdown(now + 90 * minute, now).0, "1 jam 30 menit");
        assert_eq!(format_countdown(now, now), ("EXPIRED".to_string(), true));
        assert_eq!(format_countdown(now - 1, now), ("EXPIRED".to_string(), true));
    }

    #[test]
    fn a_tenant_without_expiry_never_reaches_a_reply() {
        // JS: `fromTimestamp(undefined)` melempar RangeError sebelum balasan
        // mana pun terkirim.
        let r = run(CHAT, tenant(serde_json::json!({ "name": "Grupku" })));
        assert!(r.is_none(), "tidak ada teks yang boleh dikirim");
        assert!(r.text_body().is_none());
    }

    #[test]
    fn a_tenant_inside_three_days_gets_the_amber_reaction_and_the_warning() {
        let soon = crate::common::now_ms() + 2 * 86_400_000i64;
        let r = run(CHAT, tenant(serde_json::json!({ "expiredAt": soon })));
        let t = text_of(&r);
        assert!(t.starts_with("⏱️ *STATUS SEWA*"), "{t}");
        assert!(t.contains("Sisa waktu: *2 hari*"), "{t}");
        assert!(
            t.ends_with("\n\n⚠️ Sewa hampir habis! Hubungi owner bot untuk perpanjang."),
            "{t}"
        );
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "⚠️"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn a_tenant_far_from_expiring_uses_the_clock_reaction() {
        let far = crate::common::now_ms() + 400 * 86_400_000i64;
        let r = run(CHAT, tenant(serde_json::json!({ "expiredAt": far })));
        let t = text_of(&r);
        assert!(t.contains("Sisa waktu: *400 hari*"), "{t}");
        assert!(!t.contains("Sewa hampir habis"), "{t}");
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => assert_eq!(emoji, "⏱️"),
            other => panic!("harus Reaction, dapat {other:?}"),
        }
    }

    #[test]
    fn the_group_name_falls_back_to_the_chat_number() {
        let r = run(CHAT, tenant(serde_json::json!({ "expiredAt": 1i64 })));
        assert!(text_of(&r).contains("Grup: *120363*"), "{}", text_of(&r));
    }
}