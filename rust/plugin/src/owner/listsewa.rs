//! `plugins/owner/listsewa.js` — lihat daftar grup yang terdaftar sewa.
//!
//! `ponytail:` JS menyimpan data sewa di `db.db.data.sewa`, tabel top-level
//! yang bukan milik user maupun grup. `Mutation` hanya punya `SetUser` /
//! `SetGroup` / `SetSetting`, jadi tabel itu dipetakan ke key global `sewa` di
//! `settings` — persis yang dilakukan `group/checksewa.rs:18` untuk plugin
//! yang sama. Bentuk isinya sama dengan JS, hanya lokasinya yang berbeda.
//!
//! Semantik yang disalin dari JS:
//! - `db.db.data.sewa` yang falsy dibuat `{ enabled: false, groups: {} }`
//!   (`listsewa.js:43-46`); tabel yang sudah ada tidak pernah ditulis ulang.
//! - Urutan `Object.keys` — `serde_json` di workspace memakai
//!   `preserve_order`, jadi urutan key sama dengan urutan di file.
//! - Sort lifetime di belakang lalu `expiredAt` menaik (`:60-66`);
//!   `Array#sort` stabil, `sort_by` Rust juga.
//! - `timeHelper.fromTimestamp(addedAt, 'DD/MM/YYYY')` → `common::date_dmy`
//!   (zona Asia/Jakarta); `addedAt` falsy jadi `'-'` (`:80`).
//! - Ambang ⚠️ `diff <= 259200000` (`:37`), pembagian integer `Math.floor`
//!   (`:24-26`).
//! - `new Date(non-angka)` = Invalid Date → `formatToParts` melempar
//!   `RangeError` sebelum teks terkirim; `Reply::None` + mutasi init adalah
//!   padanan terdekat, sama seperti `group/checksewa.rs:185`.

use std::cmp::Ordering;

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

/// Key global tempat `db.db.data.sewa` dipetakan.
const SEWA_KEY: &str = "sewa";

const DAY_MS: f64 = 86_400_000.0;
const HOUR_MS: f64 = 3_600_000.0;
const MINUTE_MS: f64 = 60_000.0;
/// `diff <= 259200000` (`listsewa.js:37`) — tiga hari.
const ALMOST_MS: f64 = 259_200_000.0;

/// `db.db.data.sewa` — `Value::Null` kalau tabel belum pernah dibuat.
fn sewa(ctx: &Ctx) -> Value {
    ctx.setting(SEWA_KEY).cloned().unwrap_or(Value::Null)
}

/// `if (!db.db.data.sewa) { ...; db.db.write() }` (`listsewa.js:43-46`).
fn init_if_missing(ctx: &Ctx) -> Option<Mutation> {
    crate::common::js_falsy(ctx.setting(SEWA_KEY)).then(|| Mutation::SetSetting {
        key: SEWA_KEY.to_string(),
        value: serde_json::json!({ "enabled": false, "groups": {} }),
    })
}

/// `ToNumber(v)` JS — `NaN` untuk nilai yang tidak bisa jadi angka.
fn angka_js(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        // `Number(" 5 ")` = 5 — trim dulu, sama seperti JS.
        Some(Value::String(s)) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
        // `true`/`[]` punya nilai ToNumber di JS, tapi bukan bentuk data sewa.
        _ => f64::NAN,
    }
}

/// `data.expiredAt || 0` (`listsewa.js:65`) — semua nilai falsy jadi 0.
fn expired_at_urut(data: &Value) -> f64 {
    let v = data.get("expiredAt");
    if crate::common::js_truthy(v) {
        angka_js(v)
    } else {
        0.0
    }
}

/// `data.expiredAt` apa adanya, untuk `> Date.now()` dan `diff` (`:22,68,69`).
fn expired_raw(data: &Value) -> f64 {
    angka_js(data.get("expiredAt"))
}

/// `data.status === 'expired'` (`listsewa.js:20,33`).
fn status_expired(data: &Value) -> bool {
    data.get("status").and_then(Value::as_str) == Some("expired")
}

/// `data.isLifetime` sebagai kebenaran JS (`listsewa.js:21,34,63-64`).
fn lifetime(data: &Value) -> bool {
    crate::common::js_truthy(data.get("isLifetime"))
}

/// `formatCountdown(data)` (`listsewa.js:19-30`).
fn hitung_mundur(data: &Value, now: i64) -> String {
    if status_expired(data) {
        return "🚫 EXPIRED (left)".to_string();
    }
    if lifetime(data) {
        return "♾️ Permanent".to_string();
    }
    let diff = expired_raw(data) - now as f64;
    if diff <= 0.0 {
        return "❌ EXPIRED".to_string();
    }
    let days = (diff / DAY_MS).floor();
    let hours = ((diff % DAY_MS) / HOUR_MS).floor();
    let minutes = ((diff % HOUR_MS) / MINUTE_MS).floor();
    if days > 0.0 {
        return format!("{days}d {hours}h");
    }
    if hours > 0.0 {
        return format!("{hours}h {minutes}m");
    }
    // `NaN > 0` false di JS, jadi `expiredAt` yang bukan angka berakhir di
    // `"NaNm"` — Rust juga menulis `f64::NAN` sebagai "NaN".
    format!("{minutes}m")
}

/// `getStatusEmoji(data)` (`listsewa.js:32-39`).
fn emoji_status(data: &Value, now: i64) -> &'static str {
    if status_expired(data) {
        return "🚫";
    }
    if lifetime(data) {
        return "♾️";
    }
    let diff = expired_raw(data) - now as f64;
    if diff <= 0.0 {
        return "❌";
    }
    if diff <= ALMOST_MS {
        return "⚠️";
    }
    "✅"
}

/// `data.name || 'Unknown'` (`listsewa.js:82`).
fn nama_grup(data: &Value) -> String {
    match data.get("name") {
        Some(v) if crate::common::js_truthy_value(v) => match v.as_str() {
            Some(s) => s.to_string(),
            // `${v}` JS untuk angka/boolean: serde_json juga menulis apa adanya.
            None => v.to_string(),
        },
        _ => "Unknown".to_string(),
    }
}

/// `groupIds.sort(...)` (`listsewa.js:60-66`).
fn urutkan(groups: &serde_json::Map<String, Value>) -> Vec<(String, Value)> {
    let mut daftar: Vec<(String, Value)> = groups
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    daftar.sort_by(|(_, a), (_, b)| {
        match (lifetime(a), lifetime(b)) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            _ => {}
        }
        let d = expired_at_urut(a) - expired_at_urut(b);
        if d.is_nan() {
            // Spec `SortCompare`: comparator yang `NaN` dianggap `+0`.
            Ordering::Equal
        } else {
            d.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
        }
    });
    daftar
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

fn dengan_init(reply: Reply, init: Option<Mutation>) -> Reply {
    match init {
        Some(m) => reply.with_mutation(m),
        None => reply,
    }
}

#[plugin(
    name = "listsewa",
    alias = ["sewalist", "daftarsewa"],
    category = "owner",
    description = "Lihat daftar grup yang terdaftar sewa",
    usage = ".listsewa",
    example = ".listsewa",
    cooldown = 5,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false
)]
pub fn listsewa(ctx: Ctx) -> Reply {
    let init = init_if_missing(&ctx);
    let tabel = sewa(&ctx);
    // `db.db.data.sewa.groups || {}` (`listsewa.js:48`).
    let groups = tabel
        .get("groups")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let status_sistem = if crate::common::js_truthy(tabel.get("enabled")) {
        "✅ AKTIF"
    } else {
        "❌ NONAKTIF"
    };

    if groups.is_empty() {
        return dengan_init(
            Reply::text(format!(
                "📋 *DAFTAR SEWA*\n\nStatus: *{status_sistem}*\n\
                 Belum ada grup terdaftar\n\n\
                 Tambah dengan: *{}addsewa <link> <durasi>*",
                ctx.prefix
            )),
            init,
        );
    }

    let daftar = urutkan(&groups);
    let now = crate::common::now_ms();
    let aktif = daftar
        .iter()
        .filter(|(_, d)| lifetime(d) || expired_raw(d) > now as f64)
        .count();
    let nonaktif = daftar
        .iter()
        .filter(|(_, d)| !lifetime(d) && expired_raw(d) <= now as f64)
        .count();

    let mut text = format!(
        "📋 *DAFTAR SEWA*\n\nStatus sistem: *{status_sistem}*\n\
         Total: *{}* grup ({aktif} aktif, {nonaktif} expired)\n\n",
        groups.len()
    );

    for (i, (gid, data)) in daftar.iter().enumerate() {
        let added = if crate::common::js_truthy(data.get("addedAt")) {
            match epoch_ms(data.get("addedAt")) {
                Some(ms) => crate::common::date_dmy(ms),
                None => return dengan_init(Reply::None, init),
            }
        } else {
            "-".to_string()
        };
        text.push_str(&format!(
            "{} *{}. {}*\n   ID: {}\n   Sisa: {}\n   Ditambah: {added}\n\n",
            emoji_status(data, now),
            i + 1,
            nama_grup(data),
            crate::common::jid_number(gid),
            hitung_mundur(data, now),
        ));
    }

    text.push_str(&format!(
        "*AKSI:*\n• *{}renewsewa <id> <durasi>* — Perpanjang\n\
         • *{}delsewa <id>* — Hapus dari whitelist",
        ctx.prefix, ctx.prefix
    ));

    dengan_init(Reply::text(text), init)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::find;

    const CHAT: &str = "120363@g.us";
    /// 2023-11-15 05:13 WIB — sama dengan contoh di `checksewa.rs`.
    const WAKTU: i64 = 1_700_000_000_000;

    fn run(settings: Value) -> Reply {
        listsewa(Ctx {
            prefix: ".".into(),
            command: "listsewa".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: CHAT.into(),
            is_owner: true,
            settings,
            ..Default::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
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

    /// Grup aktif yang jauh dari kedaluwarsa (satu tahun dari test ini).
    fn tenant(extra: Value) -> Value {
        with_sewa(true, serde_json::json!({ (CHAT): extra }))
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = find("listsewa").unwrap();
        assert_eq!(p.meta.name, "listsewa");
        assert_eq!(p.meta.alias, ["sewalist", "daftarsewa"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Lihat daftar grup yang terdaftar sewa");
        assert_eq!(p.meta.usage, ".listsewa");
        assert_eq!(p.meta.example, ".listsewa");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
    }

    #[test]
    fn alias_menunjuk_ke_plugin_ini() {
        for a in ["sewalist", "daftarsewa"] {
            assert_eq!(find(a).map(|e| e.meta.name.as_str()), Some("listsewa"), "{a}");
        }
    }

    #[test]
    fn tabel_hilang_dibuat_dan_dilaporkan_nonaktif() {
        let r = run(Value::Null);
        assert_eq!(
            text_of(&r),
            "📋 *DAFTAR SEWA*\n\nStatus: *❌ NONAKTIF*\nBelum ada grup terdaftar\n\n\
             Tambah dengan: *.addsewa <link> <durasi>*"
        );
        assert_eq!(
            init_written(&r),
            Some(serde_json::json!({ "enabled": false, "groups": {} }))
        );
    }

    #[test]
    fn tanpa_grup_pakai_label_status_tanpa_angka_tidak_ada() {
        let r = run(with_sewa(true, serde_json::json!({})));
        for needle in ["Status: *✅ AKTIF*", "Belum ada grup terdaftar"] {
            assert!(text_of(&r).contains(needle), "{needle} tidak ada: {}", text_of(&r));
        }
        assert!(
            init_written(&r).is_none(),
            "tabel sudah ada -> jangan tulis ulang"
        );
        assert!(!text_of(&r).contains("Status sistem"));
        assert!(!text_of(&r).contains("*AKSI:*"));
    }

    #[test]
    fn daftar_grup_diformat_persis_seperti_js() {
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "628123@g.us": {
                    "name": "Grupku",
                    "addedAt": WAKTU,
                    // +1 jam: sisa tetap "400d" walau `now_ms()` di handler
                    // bergerak beberapa milidetik setelah JSON dibuat.
                    "expiredAt": crate::common::now_ms() + 400 * 86_400_000i64 + 3_600_000,
                }
            }),
        ));
        let t = text_of(&r);
        let sisa = t
            .lines()
            .find_map(|l| l.trim_start().strip_prefix("Sisa:").map(|_| l.trim_start()))
            .expect("baris Sisa");
        assert_eq!(
            t,
            format!(
                "📋 *DAFTAR SEWA*\n\nStatus sistem: *✅ AKTIF*\n\
                 Total: *1* grup (1 aktif, 0 expired)\n\n\
                 ✅ *1. Grupku*\n   ID: 628123\n   {sisa}\n   Ditambah: 15/11/2023\n\n\
                 *AKSI:*\n• *.renewsewa <id> <durasi>* — Perpanjang\n\
                 • *.delsewa <id>* — Hapus dari whitelist"
            )
        );
        assert!(sisa.starts_with("Sisa: 400d "), "{sisa}");
    }

    #[test]
    fn lifetime_selalu_di_urut_paling_belakang() {
        let jauh = crate::common::now_ms() + 999 * 86_400_000i64;
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "lifetime@g.us": { "name": "Selamanya", "isLifetime": true, "expiredAt": 0 },
                "cepat@g.us": { "name": "Cepat", "expiredAt": jauh },
                "lambat@g.us": { "name": "Lambat", "expiredAt": jauh + 86_400_000 },
            }),
        ));
        let t = text_of(&r);
        let pos = |n: usize, nama: &str| {
            t.find(&format!("*{n}. {nama}*"))
                .unwrap_or_else(|| panic!("{nama} tidak ada di {t}"))
        };
        assert!(pos(1, "Cepat") < pos(2, "Lambat"), "{t}");
        assert!(pos(2, "Lambat") < pos(3, "Selamanya"), "{t}");
        assert!(t.contains("♾️ *3. Selamanya*"), "{t}");
        assert!(t.contains("Total: *3* grup (3 aktif, 0 expired)"), "{t}");
    }

    #[test]
    fn urutan_dipertahankan_saat_expired_at_sama_atau_rusak() {
        // `Array#sort` stabil: key dengan `expiredAt` sama mengikuti urutan
        // `Object.keys` (di Rust urutan dokumen karena `preserve_order`).
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "b@g.us": { "name": "B", "expiredAt": WAKTU },
                "a@g.us": { "name": "A", "expiredAt": WAKTU },
                "c@g.us": { "name": "C", "expiredAt": "bukan angka" },
            }),
        ));
        let t = text_of(&r);
        let pos_b = t.find("*1. B*").expect("B");
        let pos_a = t.find("*2. A*").expect("A");
        let pos_c = t.find("*3. C*").expect("C");
        assert!(pos_b < pos_a && pos_a < pos_c, "{t}");
        // `(a.expiredAt || 0) - (b.expiredAt || 0)` = NaN -> urutan tetap.
        assert!(t.contains("NaNm"), "countdown JS untuk NaN: {t}");
        assert!(t.contains("✅ *3. C*"), "NaN tidak pernah <= 0: {t}");
    }

    #[test]
    fn hitung_aktif_dan_expired_mengikuti_dua_filter_js() {
        let now = crate::common::now_ms();
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "lifetime@g.us": { "isLifetime": true, "expiredAt": 0 },
                "aktif@g.us": { "expiredAt": now + 86_400_000i64 },
                "habis@g.us": { "expiredAt": 1 },
            }),
        ));
        assert!(
            text_of(&r).contains("Total: *3* grup (2 aktif, 1 expired)"),
            "{}",
            text_of(&r)
        );
    }

    #[test]
    fn grup_expired_dapat_teks_dan_emoji_expired() {
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "habis@g.us": { "name": "Habis", "expiredAt": 1 },
            }),
        ));
        let t = text_of(&r);
        assert!(t.contains("❌ *1. Habis*"), "{t}");
        assert!(t.contains("Sisa: ❌ EXPIRED"), "{t}");
        assert!(t.contains("Total: *1* grup (0 aktif, 1 expired)"), "{t}");
    }

    #[test]
    fn status_expired_menang_atas_semua_cabang_lain() {
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "habis@g.us": {
                    "name": "Habis",
                    "status": "expired",
                    "isLifetime": true,
                    "expiredAt": 0,
                },
            }),
        ));
        let t = text_of(&r);
        assert!(t.contains("🚫 *1. Habis*"), "{t}");
        assert!(t.contains("Sisa: 🚫 EXPIRED (left)"), "{t}");
        // `isLifetime` tetap ikut filter aktif JS (`listsewa.js:68`).
        assert!(t.contains("Total: *1* grup (1 aktif, 0 expired)"), "{t}");
    }

    #[test]
    fn nama_kosong_dan_added_at_falsy_dapat_fallback() {
        let r = run(with_sewa(
            true,
            serde_json::json!({
                "628999@g.us": { "name": "", "expiredAt": 1, "addedAt": 0 },
            }),
        ));
        let t = text_of(&r);
        assert!(t.contains("*1. Unknown*"), "{t}");
        assert!(t.contains("Ditambah: -"), "{t}");
    }

    #[test]
    fn tanggal_ditambah_pakai_zona_asia_jakarta() {
        let r = run(tenant(serde_json::json!({
            "name": "Grupku",
            "isLifetime": true,
            "addedAt": WAKTU,
        })));
        assert!(text_of(&r).contains("Ditambah: 15/11/2023"), "{}", text_of(&r));
    }

    #[test]
    fn added_at_yang_tidak_bisa_jadi_tanggal_tidak_membalas() {
        // JS: `new Date("bukan angka")` = Invalid Date -> `formatToParts`
        // melempar RangeError sebelum `m.reply` mana pun.
        let r = run(tenant(serde_json::json!({ "addedAt": "bukan angka" })));
        assert!(r.is_none(), "tidak ada teks yang boleh dikirim");
        assert!(r.text_body().is_none());
    }

    #[test]
    fn countdown_kasus_batas_sama_dengan_math_floor_js() {
        let now = WAKTU;
        let d = |at: Value| serde_json::json!({ "expiredAt": at });
        assert_eq!(
            hitung_mundur(&serde_json::json!({ "status": "expired" }), now),
            "🚫 EXPIRED (left)"
        );
        assert_eq!(
            hitung_mundur(&serde_json::json!({ "isLifetime": true }), now),
            "♾️ Permanent"
        );
        assert_eq!(hitung_mundur(&d(serde_json::json!(now)), now), "❌ EXPIRED");
        assert_eq!(hitung_mundur(&d(serde_json::json!(now - 1)), now), "❌ EXPIRED");
        assert_eq!(
            hitung_mundur(&d(serde_json::json!(now + 86_400_000i64)), now),
            "1d 0h"
        );
        assert_eq!(
            hitung_mundur(&d(serde_json::json!(now + 86_400_000i64 + 5 * 3_600_000i64)), now),
            "1d 5h"
        );
        assert_eq!(
            hitung_mundur(&d(serde_json::json!(now + 61_000i64)), now),
            "1m",
            "detik dipotong floor"
        );
        assert_eq!(
            hitung_mundur(&d(serde_json::json!(now + 3 * 3_600_000i64)), now),
            "3h 0m"
        );
        assert_eq!(
            hitung_mundur(&d(serde_json::json!(now + 59_000i64)), now),
            "0m"
        );
        // `expiredAt` hilang -> `NaN` -> `"NaNm"` plus emoji ✅ (`:30,35-38`).
        assert_eq!(hitung_mundur(&serde_json::json!({}), now), "NaNm");
    }

    #[test]
    fn emoji_status_kasus_batas_tiga_hari() {
        let now = WAKTU;
        let d = |at: i64| serde_json::json!({ "expiredAt": at });
        assert_eq!(emoji_status(&serde_json::json!({ "status": "expired" }), now), "🚫");
        assert_eq!(emoji_status(&serde_json::json!({ "isLifetime": true }), now), "♾️");
        assert_eq!(emoji_status(&d(now), now), "❌");
        assert_eq!(emoji_status(&d(now + 259_200_000), now), "⚠️", "batas inklusif");
        assert_eq!(emoji_status(&d(now + 259_200_001), now), "✅");
        // `NaN <= 259200000` false -> ✅, sama seperti JS.
        assert_eq!(emoji_status(&serde_json::json!({}), now), "✅");
    }
}
