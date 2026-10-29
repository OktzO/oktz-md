//! `plugins/owner/cekschedule.js` — melihat status semua scheduler bot.
//!
//! `ponytail:` `getFullSchedulerStatus()`, `getTodaySchedule()`,
//! `extractPrayerTimes()`, dan `db.setting('autoSholatKota')` berasal dari
//! `src/lib/scheduler.js` dan `src/lib/sholat-api.js`, yang bukan bagian dari
//! `Ctx` API. Hanya `db.setting('autoSholat')` (lewat `ctx.setting`) yang
//! tersedia. Sisa tidak dapat di-port saat ini.
//!
//! Catatan: baris `pub mod cekschedule;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "cekschedule",
    alias = ["cekscheduler", "schedulerstatus", "schedstatus"],
    category = "owner",
    description = "Melihat status semua scheduler bot",
    usage = ".cekschedule",
    example = ".cekschedule",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cekschedule(ctx: Ctx) -> Reply {
    let sholat_enabled = ctx.setting_bool("autoSholat", false);

    let mut text = String::from("📊 *sᴄʜᴇᴅᴜʟᴇʀ sᴛᴀᴛᴜs*\n\n");
    text.push_str("⚠️ Status scheduler detail tidak tersedia di port Rust\n");
    text.push_str("   (scheduler.js belum diekspose lewat Ctx API)\n\n");

    let sholat_icon = if sholat_enabled { "✅" } else { "❌" };
    text.push_str(&format!("{} *Sholat Scheduler*\n", sholat_icon));
    text.push_str("   └ Key: `sholat`\n");
    text.push_str("   └ Notifikasi waktu sholat (real-time)\n\n");

    if sholat_enabled {
        text.push_str("   └ Lokasi: tidak diketahui (autoSholatKota belum tersedia)\n");
        text.push_str("   └ Next: tidak tersedia\n\n");
    }

    text.push_str("━━━━━━━━━━━━━━━━━━━\n");
    if sholat_enabled {
        text.push_str("✅ Aktif: 1\n❌ Nonaktif: 0\n\n");
    } else {
        text.push_str("✅ Aktif: 0\n❌ Nonaktif: 1\n\n");
    }
    text.push_str("> Gunakan `.stopschedule <key>` untuk stop\n");
    text.push_str("> Gunakan `.startschedule <key>` untuk start");

    Reply::text(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(settings: Value) -> Reply {
        let ctx = Ctx {
            command: "cekschedule".into(),
            settings,
            ..Default::default()
        };
        cekschedule(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("cekschedule").unwrap();
        assert_eq!(p.meta.name, "cekschedule");
        assert_eq!(p.meta.alias, ["cekscheduler", "schedulerstatus", "schedstatus"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn sholat_tidak_aktif_tampilkan_status_nonaktif() {
        let r = run(serde_json::json!({}));
        let t = body(&r);
        assert!(t.contains("❌ *Sholat Scheduler*"), "{t}");
        assert!(t.contains("✅ Aktif: 0"), "{t}");
        assert!(t.contains("❌ Nonaktif: 1"), "{t}");
    }

    #[test]
    fn sholat_aktif_tampilkan_status_aktif() {
        let r = run(serde_json::json!({ "autoSholat": true }));
        let t = body(&r);
        assert!(t.contains("✅ *Sholat Scheduler*"), "{t}");
        assert!(t.contains("✅ Aktif: 1"), "{t}");
        assert!(t.contains("❌ Nonaktif: 0"), "{t}");
    }

    #[test]
    fn nada_diubah_karena_baca_saja() {
        let ctx = Ctx {
            command: "cekschedule".into(),
            settings: serde_json::json!({ "autoSholat": true }),
            ..Default::default()
        };
        let r = cekschedule(ctx);
        assert!(r.mutations().is_empty());
    }
}
