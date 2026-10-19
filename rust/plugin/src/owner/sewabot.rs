//! `plugins/owner/sewabot.js` — toggle dan kelola sistem sewa bot.
//!
//! `ponytail:` JS menyimpan data sewa di `db.db.data.sewa`, tabel top-level
//! yang bukan milik user maupun grup. `Mutation` hanya punya `SetUser` /
//! `SetGroup` / `SetSetting`, jadi tabel itu dipetakan ke key global `sewa` di
//! `settings` — persis `group/checksewa.rs:18` dan `owner/listsewa.rs`.
//!
//! `ponytail:` cabang `confirm` dan `leave` di JS menyapu semua grup
//! (`sock.groupFetchAllParticipating`, `sock.sendText` per grup, lalu
//! `sock.groupLeave`). Pengiriman ke JID lain tidak punya padanan: `Reply`
//! hanya sampai ke `caller.chat` (`pipeline.rs:3058`). Karena itu jalur `try`
//! diperlakukan seperti gagal di JS dan handler mengirim jalur `catch`-nya:
//! `confirm` → reaksi `✅` + `te(...)` (`sewabot.js:142-145`), `leave` →
//! reaksi `☢` + `te(...)` (`:199-203`). Cabang `confirm` tetap menulis
//! `enabled: true` karena JS menulis SEBELUM `try` (`:98-99`).
//! `ponytail:` reaksi pembuka JS (`🕕`, `:100` dan `:150`) hilang — `Reply`
//! hanya mengirim reaksi sebagai pembungkus level atas (`pipeline.rs:2999`),
//! tidak ada dua reaksi berurutan.
//!
//! `pendingConfirmations` (`:19`) dipetakan ke `ctx.sessions` key
//! `sewabot:pending:<sender>`. TTL 60000 ms yang di JS memakai `setTimeout`
//! (`:77-80`) diperiksa saat dibaca: entri kedaluwarsa dibuang, jadi `confirm`
//! dan `cancel` melihat kondisi "sudah dihapus timer" yang sama.
//!
//! Catatan: `mod.rs` kategori `owner` dimiliki plugin lain (tidak boleh
//! disentuh), jadi baris `pub mod sewabot;` perlu ditambahkan manual.

use oktz_plugin_api::{Ctx, Mutation, Reply, plugin};
use serde_json::Value;

/// Key global tempat `db.db.data.sewa` dipetakan.
const SEWA_KEY: &str = "sewa";

/// `pendingConfirmations` keyed `m.sender` (`sewabot.js:19,63`).
const PENDING_PREFIX: &str = "sewabot:pending:";
/// `Date.now() - pending.timestamp < 60000` (`sewabot.js:67,80`).
const PENDING_TTL_MS: i64 = 60_000;
/// `pending.type === "sewabot_on"` (`sewabot.js:66,78,92,207`).
const PENDING_TYPE: &str = "sewabot_on";

/// `if (!db.db.data.sewa) { db.db.data.sewa = {...}; db.db.write() }`
/// (`sewabot.js:23-26`).
fn init_if_missing(ctx: &Ctx) -> Option<Mutation> {
    crate::common::js_falsy(ctx.setting(SEWA_KEY)).then(|| Mutation::SetSetting {
        key: SEWA_KEY.to_string(),
        value: serde_json::json!({ "enabled": false, "groups": {} }),
    })
}

/// Tabel `sewa` sebagaimana terlihat cabang-cabang di bawah: JS menginisialisasi
/// lebih dulu (`:23-26`), jadi tabel yang belum ada dibaca sebagai bentuk init.
fn tabel_setelah_init(ctx: &Ctx) -> Value {
    match ctx.setting(SEWA_KEY) {
        Some(v) if crate::common::js_truthy_value(v) => v.clone(),
        _ => serde_json::json!({ "enabled": false, "groups": {} }),
    }
}

/// `db.db.data.sewa.enabled` + `Object.keys(db.db.data.sewa.groups || {})`
/// (`sewabot.js:27-28`).
fn aktif_dan_grup(tabel: &Value) -> (bool, Vec<String>) {
    let aktif = crate::common::js_truthy(tabel.get("enabled"));
    let grup = tabel
        .get("groups")
        .and_then(Value::as_object)
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    (aktif, grup)
}

/// `db.db.data.sewa.enabled = X; db.db.write()` (`sewabot.js:55-56,98-99`) —
/// key lain tabel (termasuk `groups`) tidak disentuh.
fn set_enabled(tabel: &Value, enabled: bool) -> Mutation {
    let mut baru = tabel.as_object().cloned().unwrap_or_default();
    baru.insert("enabled".to_string(), Value::Bool(enabled));
    Mutation::SetSetting {
        key: SEWA_KEY.to_string(),
        value: Value::Object(baru),
    }
}

fn pending_key(ctx: &Ctx) -> String {
    format!("{PENDING_PREFIX}{}", ctx.sender)
}

fn pending_sewabot_on(v: &Value) -> bool {
    v.get("type").and_then(Value::as_str) == Some(PENDING_TYPE)
}

/// `pendingConfirmations.get(m.sender)` + efek `setTimeout` 60000 ms
/// (`sewabot.js:77-80`). `Ctx` tidak punya timer latar, jadi entri yang sudah
/// lewat 60 detik dibuang saat dibaca — sama seperti callback JS yang
/// menghapusnya tepat setelah 60 detik.
fn pending(ctx: &Ctx) -> Option<Value> {
    let key = pending_key(ctx);
    let v = ctx.session_get(&key)?;
    let ts = v.get("timestamp").and_then(Value::as_i64).unwrap_or(0);
    if pending_sewabot_on(&v) && crate::common::now_ms() - ts >= PENDING_TTL_MS {
        ctx.session_take(&key);
        return None;
    }
    Some(v)
}

/// `pendingConfirmations.set(m.sender, { type: "sewabot_on", timestamp: Date.now() })`
/// (`sewabot.js:73-76`).
fn set_pending(ctx: &Ctx) {
    ctx.session_set(
        &pending_key(ctx),
        serde_json::json!({ "type": PENDING_TYPE, "timestamp": crate::common::now_ms() }),
    );
}

/// `te(m.prefix, m.command, m.pushName)` — `src/lib/error.js`.
fn te(ctx: &Ctx) -> String {
    crate::common::error_template(&ctx.prefix, &ctx.command)
}

/// Panduan + status (`sewabot.js:30-52`).
fn status_text(prefix: &str, aktif: bool, jumlah: usize) -> String {
    format!(
        "🔧 *SISTEM SEWA BOT*\n\n\
         Status: *{}*\n\
         Grup terdaftar: *{jumlah}*\n\n\
         *PERINTAH TERSEDIA:*\n\
         • *{prefix}sewabot on* — Aktifkan sistem sewa\n\
         • *{prefix}sewabot off* — Nonaktifkan sistem sewa\n\
         • *{prefix}sewabot leave* — Keluar dari semua grup non-whitelist\n\n\
         *KELOLA SEWA:*\n\
         • *{prefix}addsewa <link> <durasi>* — Tambah grup + auto join\n\
         • *{prefix}delsewa <link/id>* — Hapus grup dari whitelist\n\
         • *{prefix}renewsewa <link/id> <durasi>* — Perpanjang sewa\n\
         • *{prefix}listsewa* — Lihat semua grup terdaftar\n\
         • *{prefix}checksewa* — Cek sisa sewa (di grup)\n\n\
         *FORMAT DURASI:*\n\
         30i (menit) • 12h (jam) • 7d (hari) • 1m (bulan) • 1y (tahun) • lifetime\n\n\
         *CARA KERJA:*\n\
         1. Tambahkan grup dengan *{prefix}addsewa*\n\
         2. Bot otomatis join jika pakai link\n\
         3. Aktifkan dengan *{prefix}sewabot on*\n\
         4. Bot akan keluar dari semua grup yang tidak terdaftar\n\
         5. Sewa expired → bot otomatis keluar dari grup",
        if aktif { "✅ AKTIF" } else { "❌ NONAKTIF" }
    )
}

fn dengan_init(reply: Reply, init: Option<Mutation>) -> Reply {
    match init {
        Some(m) => reply.with_mutation(m),
        None => reply,
    }
}

#[plugin(
    name = "sewabot",
    alias = ["sewa"],
    category = "owner",
    description = "Toggle dan kelola sistem sewa bot",
    usage = ".sewabot <on/off/leave/status>",
    example = ".sewabot on",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn sewabot(ctx: Ctx) -> Reply {
    let init = init_if_missing(&ctx);
    let tabel = tabel_setelah_init(&ctx);
    let (aktif, grup) = aktif_dan_grup(&tabel);
    // `const args = m.text?.trim()?.toLowerCase()` (`:22`) — SELURUH teks,
    // bukan token pertama, jadi `sewabot on extra` jatuh ke cabang tidak valid.
    let args = crate::common::text_keyword(&ctx);

    if args.is_empty() || args == "status" {
        return dengan_init(
            Reply::text(status_text(&ctx.prefix, aktif, grup.len())),
            init,
        );
    }

    if args == "off" {
        return Reply::text("✅ Sistem sewa dinonaktifkan\n\nBot tidak akan meninggalkan grup manapun.")
            .with_reaction("✅")
            .with_mutation(set_enabled(&tabel, false));
    }

    if args == "on" {
        if pending(&ctx).is_some_and(|p| pending_sewabot_on(&p)) {
            return dengan_init(
                Reply::text(format!(
                    "🕕 Sudah ada permintaan pending\n\n\
                     Ketik *{}sewabot confirm* untuk lanjut\n\
                     Ketik *{}sewabot cancel* untuk batal",
                    ctx.prefix, ctx.prefix
                )),
                init,
            );
        }
        set_pending(&ctx);
        return dengan_init(
            Reply::text(format!(
                "⚠️ *KONFIRMASI AKTIVASI SEWA*\n\n\
                 Jika diaktifkan:\n\
                 • ✅ {} grup ter-whitelist tetap aman\n\
                 • ❌ Semua grup lain akan ditinggalkan!\n\n\
                 Ketik *{}sewabot confirm* untuk lanjut\n\
                 Ketik *{}sewabot cancel* untuk batal\n\n\
                 💡 Pastikan sudah whitelist grup penting dengan:\n\
                 *{}addsewa <link grup> <durasi>*",
                grup.len(),
                ctx.prefix,
                ctx.prefix,
                ctx.prefix
            )),
            init,
        );
    }

    if args == "confirm" || args == "yes" || args == "y" {
        if !pending(&ctx).is_some_and(|p| pending_sewabot_on(&p)) {
            return dengan_init(
                Reply::text(format!(
                    "❌ Tidak ada permintaan pending\nKetik *{}sewabot on* dulu",
                    ctx.prefix
                )),
                init,
            );
        }
        ctx.session_take(&pending_key(&ctx));
        // ponytail: sapu grup (`sendText` + `groupLeave` per grup non-whitelist)
        // butuh pengiriman ke JID lain — `Reply` berhenti di `caller.chat`
        // (`pipeline.rs:3058`). Jalur `try` dianggap gagal, jadi hasilnya
        // `catch` JS: reaksi ✅ + `te(...)` (`sewabot.js:142-145`). Tambahkan
        // begitu pipeline punya API kirim/keluar grup.
        return Reply::text(te(&ctx))
            .with_reaction("✅")
            .with_mutation(set_enabled(&tabel, true));
    }

    if args == "leave" {
        if !aktif {
            return dengan_init(
                Reply::text(format!(
                    "❌ Aktifkan sewabot dulu dengan *{}sewabot on*",
                    ctx.prefix
                )),
                init,
            );
        }
        // ponytail: sama seperti `confirm` — sapu grup tidak bisa dari `Reply`;
        // hasilnya `catch` JS: reaksi ☢ + `te(...)` (`sewabot.js:199-203`).
        return dengan_init(Reply::text(te(&ctx)).with_reaction("☢"), init);
    }

    if args == "cancel" || args == "no" || args == "n" {
        if !pending(&ctx).is_some_and(|p| pending_sewabot_on(&p)) {
            return dengan_init(Reply::text("❌ Tidak ada permintaan pending"), init);
        }
        ctx.session_take(&pending_key(&ctx));
        return dengan_init(
            Reply::text(format!(
                "❌ Aktivasi dibatalkan\nWhitelist grup dulu dengan *{}addsewa*",
                ctx.prefix
            ))
            .with_reaction("❌"),
            init,
        );
    }

    dengan_init(
        Reply::text(format!(
            "❌ Perintah tidak valid\n\n\
             Ketik *{}sewabot* untuk melihat panduan lengkap",
            ctx.prefix
        )),
        init,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use oktz_plugin_api::{SessionStore, find, new_session_store};

    const ME: &str = "628111@s.whatsapp.net";
    const CHAT: &str = "628111@s.whatsapp.net";

    fn run(teks: &str, settings: Value, sessions: &SessionStore) -> Reply {
        sewabot(Ctx {
            text: teks.into(),
            args: teks.split_whitespace().map(str::to_string).collect(),
            sender: ME.into(),
            chat: CHAT.into(),
            prefix: ".".into(),
            command: "sewabot".into(),
            settings,
            sessions: sessions.clone(),
            ..Ctx::default()
        })
    }

    fn text_of(r: &Reply) -> String {
        r.without_mutations()
            .text_body()
            .unwrap_or_default()
            .to_string()
    }

    fn init_tersimpan(r: &Reply) -> Option<Value> {
        r.mutations().iter().find_map(|m| match m {
            Mutation::SetSetting { key, value } if key == SEWA_KEY => Some(value.clone()),
            _ => None,
        })
    }

    fn reaksi(r: &Reply) -> Option<&str> {
        match r.without_mutations() {
            Reply::Reaction { emoji, .. } => Some(emoji),
            _ => None,
        }
    }

    fn dengan_sewa(enabled: bool, groups: Value) -> Value {
        serde_json::json!({ "sewa": { "enabled": enabled, "groups": groups } })
    }

    fn pending_key_test() -> String {
        format!("{PENDING_PREFIX}{ME}")
    }

    fn seed_pending(sessions: &SessionStore, timestamp: i64) {
        sessions.lock().unwrap().insert(
            pending_key_test(),
            serde_json::json!({ "type": PENDING_TYPE, "timestamp": timestamp }),
        );
    }

    fn pending_tersimpan(sessions: &SessionStore) -> Option<Value> {
        sessions.lock().unwrap().get(&pending_key_test()).cloned()
    }

    fn baru() -> SessionStore {
        new_session_store()
    }

    #[test]
    fn terdaftar_dengan_meta_js() {
        let p = find("sewabot").unwrap();
        assert_eq!(p.meta.name, "sewabot");
        assert_eq!(p.meta.alias, ["sewa"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.description, "Toggle dan kelola sistem sewa bot");
        assert_eq!(p.meta.usage, ".sewabot <on/off/leave/status>");
        assert_eq!(p.meta.example, ".sewabot on");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
        assert!(!p.meta.is_admin, "isAdmin tidak ada di JS");
    }

    #[test]
    fn alias_menunjuk_ke_plugin_ini() {
        assert_eq!(find("sewa").map(|e| e.meta.name.as_str()), Some("sewabot"));
    }

    #[test]
    fn status_tanpa_tabel_membuat_tabel_dan_melaporkan_nonaktif() {
        let s = baru();
        let r = run("", Value::Null, &s);
        let t = text_of(&r);
        assert!(t.contains("🔧 *SISTEM SEWA BOT*"), "{t}");
        assert!(t.contains("Status: *❌ NONAKTIF*"), "{t}");
        assert!(t.contains("Grup terdaftar: *0*"), "{t}");
        assert_eq!(
            init_tersimpan(&r),
            Some(serde_json::json!({ "enabled": false, "groups": {} }))
        );
    }

    #[test]
    fn status_menghitung_grup_dan_status_aktif() {
        let s = baru();
        let r = run(
            "status",
            dengan_sewa(true, serde_json::json!({ "a@g.us": {}, "b@g.us": {} })),
            &s,
        );
        let t = text_of(&r);
        assert!(t.contains("Status: *✅ AKTIF*"), "{t}");
        assert!(t.contains("Grup terdaftar: *2*"), "{t}");
        assert!(init_tersimpan(&r).is_none(), "tabel sudah ada");
        assert!(pending_tersimpan(&s).is_none(), "status tidak menyentuh pending");
    }

    #[test]
    fn panduan_memakai_prefix_dari_pesan() {
        let s = baru();
        let mut ctx = Ctx {
            text: String::new(),
            sender: ME.into(),
            chat: CHAT.into(),
            prefix: "!".into(),
            command: "sewabot".into(),
            ..Ctx::default()
        };
        ctx.sessions = s.clone();
        let r = sewabot(ctx);
        let t = text_of(&r);
        for baris in [
            "• *!sewabot on* — Aktifkan sistem sewa",
            "• *!addsewa <link> <durasi>* — Tambah grup + auto join",
            "• *!checksewa* — Cek sisa sewa (di grup)",
            "30i (menit) • 12h (jam) • 7d (hari) • 1m (bulan) • 1y (tahun) • lifetime",
            "5. Sewa expired → bot otomatis keluar dari grup",
        ] {
            assert!(t.contains(baris), "hilang: {baris}\n{t}");
        }
    }

    #[test]
    fn off_menonaktifkan_dan_menjaga_grup() {
        let s = baru();
        let r = run(
            "off",
            dengan_sewa(true, serde_json::json!({ "a@g.us": {}, "b@g.us": {} })),
            &s,
        );
        assert_eq!(
            text_of(&r),
            "✅ Sistem sewa dinonaktifkan\n\nBot tidak akan meninggalkan grup manapun."
        );
        assert_eq!(reaksi(&r), Some("✅"));
        let m = init_tersimpan(&r).expect("off menulis tabel");
        assert_eq!(m["enabled"], serde_json::json!(false));
        assert_eq!(m["groups"].as_object().unwrap().len(), 2, "{m}");
    }

    #[test]
    fn on_pertama_membuat_pending_dan_menyebut_jumlah_whitelist() {
        let s = baru();
        let r = run(
            "on",
            dengan_sewa(false, serde_json::json!({ "a@g.us": {}, "b@g.us": {} })),
            &s,
        );
        let t = text_of(&r);
        assert!(t.contains("⚠️ *KONFIRMASI AKTIVASI SEWA*"), "{t}");
        assert!(t.contains("• ✅ 2 grup ter-whitelist tetap aman"), "{t}");
        assert!(t.contains("Ketik *.sewabot confirm* untuk lanjut"), "{t}");
        assert!(t.contains("*.addsewa <link grup> <durasi>*"), "{t}");
        let p = pending_tersimpan(&s).expect("pending harus tersimpan");
        assert_eq!(p["type"], serde_json::json!(PENDING_TYPE));
        assert_eq!(init_tersimpan(&r), None, "tabel sudah ada");
    }

    #[test]
    fn on_kedua_saat_pending_masih_segar_tidak_menimpa() {
        let s = baru();
        let segar = crate::common::now_ms() - 5_000;
        seed_pending(&s, segar);
        let r = run("on", Value::Null, &s);
        assert_eq!(
            text_of(&r),
            "🕕 Sudah ada permintaan pending\n\n\
             Ketik *.sewabot confirm* untuk lanjut\n\
             Ketik *.sewabot cancel* untuk batal"
        );
        let p = pending_tersimpan(&s).unwrap();
        assert_eq!(p["timestamp"], serde_json::json!(segar), "JS tidak menimpa pending");
    }

    #[test]
    fn on_setelah_lewat_60_detik_menganggap_pending_sudah_dibuang() {
        let s = baru();
        let basi = crate::common::now_ms() - PENDING_TTL_MS - 1;
        seed_pending(&s, basi);
        let r = run("on", Value::Null, &s);
        assert!(text_of(&r).contains("⚠️ *KONFIRMASI AKTIVASI SEWA*"), "{}", text_of(&r));
        let p = pending_tersimpan(&s).unwrap();
        assert!(
            p["timestamp"].as_i64().unwrap() > basi,
            "entri basi diganti yang baru: {p}"
        );
    }

    #[test]
    fn confirm_tanpa_pending_memberi_petunjuk_on() {
        let s = baru();
        let r = run("confirm", Value::Null, &s);
        assert_eq!(
            text_of(&r),
            "❌ Tidak ada permintaan pending\nKetik *.sewabot on* dulu"
        );
        assert!(init_tersimpan(&r).is_some(), "init tetap jalan (`:23-26`)");
    }

    #[test]
    fn confirm_dengan_pending_segar_mengaktifkan_lalu_jatuh_ke_catch_js() {
        let s = baru();
        seed_pending(&s, crate::common::now_ms());
        let r = run(
            "confirm",
            dengan_sewa(false, serde_json::json!({ "a@g.us": {} })),
            &s,
        );
        assert_eq!(text_of(&r), crate::common::error_template(".", "sewabot"));
        assert_eq!(reaksi(&r), Some("✅"));
        let m = init_tersimpan(&r).expect("confirm menulis tabel");
        assert_eq!(m["enabled"], serde_json::json!(true), "{m}");
        assert_eq!(m["groups"]["a@g.us"], serde_json::json!({}), "{m}");
        assert!(pending_tersimpan(&s).is_none(), "pending dihapus (`:97`)");
    }

    #[test]
    fn confirm_menerima_alias_yes_dan_y() {
        for kata in ["yes", "y"] {
            let s = baru();
            seed_pending(&s, crate::common::now_ms());
            let r = run(kata, Value::Null, &s);
            assert_eq!(reaksi(&r), Some("✅"), "{kata}");
            assert!(pending_tersimpan(&s).is_none(), "{kata}");
        }
    }

    #[test]
    fn confirm_setelah_pending_basi_tidak_ada_permintaan() {
        let s = baru();
        seed_pending(&s, crate::common::now_ms() - PENDING_TTL_MS - 1);
        let r = run("confirm", Value::Null, &s);
        assert!(
            text_of(&r).contains("❌ Tidak ada permintaan pending"),
            "{}",
            text_of(&r)
        );
        assert!(pending_tersimpan(&s).is_none(), "entri basi dibuang");
        assert_eq!(reaksi(&r), None, "tidak ada reaksi di jalur gagal");
    }

    #[test]
    fn cancel_tanpa_pending_dan_dengan_pending() {
        let s = baru();
        let r = run("cancel", Value::Null, &s);
        assert_eq!(text_of(&r), "❌ Tidak ada permintaan pending");

        let s = baru();
        seed_pending(&s, crate::common::now_ms());
        let r = run("cancel", Value::Null, &s);
        assert_eq!(
            text_of(&r),
            "❌ Aktivasi dibatalkan\nWhitelist grup dulu dengan *.addsewa*"
        );
        assert_eq!(reaksi(&r), Some("❌"));
        assert!(pending_tersimpan(&s).is_none());
    }

    #[test]
    fn leave_saat_nonaktif_menyuruh_aktifkan_dulu() {
        let s = baru();
        let r = run("leave", dengan_sewa(false, serde_json::json!({})), &s);
        assert_eq!(
            text_of(&r),
            "❌ Aktifkan sewabot dulu dengan *.sewabot on*"
        );
        assert_eq!(reaksi(&r), None, "guard JS tidak bereaksi");
    }

    #[test]
    fn leave_saat_aktif_jatuh_ke_catch_js() {
        let s = baru();
        let r = run(
            "leave",
            dengan_sewa(true, serde_json::json!({ "a@g.us": {} })),
            &s,
        );
        assert_eq!(text_of(&r), crate::common::error_template(".", "sewabot"));
        assert_eq!(reaksi(&r), Some("☢"));
        assert!(r.mutations().is_empty(), "leave tidak menulis store");
    }

    #[test]
    fn perintah_tidak_valid_menunjuk_panduan() {
        let s = baru();
        for teks in ["hapus", "on extra", "on extra panjang", "confirme"] {
            let r = run(teks, Value::Null, &s);
            assert_eq!(
                text_of(&r),
                "❌ Perintah tidak valid\n\nKetik *.sewabot* untuk melihat panduan lengkap",
                "{teks:?}"
            );
        }
    }

    #[test]
    fn teks_di_trim_dan_lowercase_sebelum_cocok() {
        let s = baru();
        let r = run("  STATUS  ", Value::Null, &s);
        assert!(text_of(&r).contains("🔧 *SISTEM SEWA BOT*"), "{}", text_of(&r));

        let s = baru();
        let r = run("  OFF ", Value::Null, &s);
        assert_eq!(reaksi(&r), Some("✅"));
        assert_eq!(
            text_of(&r),
            "✅ Sistem sewa dinonaktifkan\n\nBot tidak akan meninggalkan grup manapun."
        );
    }
}
