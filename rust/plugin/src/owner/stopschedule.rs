//! `plugins/owner/stopschedule.js` — hentikan scheduler tertentu atau semua.
//!
//! `ponytail:` `stopSchedulerByName`, `stopSholatScheduler` (`src/lib/scheduler.js`,
//! `src/lib/sholat-scheduler.js`) dan `getDatabase()`/`db.setting` tidak tersedia
//! di `Ctx`; handler diakhiri `error_template` + `☢` (pola `swm.rs`). Teks
//! bantuan dan daftar scheduler di-port agar bisa diuji.
//! Saat scheduler + store tersedia, tambahkan `pub mod stopscheduler;` di
//! `src/owner/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// Daftar scheduler yang dikenal (`stopschedule.js:32-37`).
#[cfg(test)]
pub(crate) const JADWAL: [(&str, &str); 6] = [
    ("limitreset", "Daily Limit Reset"),
    ("groupschedule", "Group Schedule"),
    ("sewa", "Sewa Checker"),
    ("messages", "Scheduled Messages"),
    ("sholat", "Sholat Scheduler"),
    ("all", "Semua scheduler"),
];

/// Teks bantuan (`stopschedule.js:26-44`).
#[cfg(test)]
pub(crate) fn bantuan(_prefix: &str) -> String {
    let mut out = String::from(
        "🛑 *sᴛᴏᴘ sᴄʜᴇᴅᴜʟᴇʀ*\n\n\
         *Usage:*\n\
         `.stopschedule <nama>`\n\n\
         *Available schedulers:*\n",
    );
    for (nama, desc) in JADWAL {
        out.push_str(&format!("• `{nama}` - {desc}\n"));
    }
    out.push_str(
        "\n\
         *Example:*\n\
         `.stopschedule sholat`\n\
         `.stopschedule all`",
    );
    out
}

/// Apakah target adalah 'sholat' (`stopschedule.js:47`).
#[cfg(test)]
pub(crate) fn is_sholat(target: &str) -> bool {
    target == "sholat"
}

/// Apakah target adalah 'all' (`stopschedule.js:68`).
#[cfg(test)]
pub(crate) fn is_all(target: &str) -> bool {
    target == "all"
}

/// Teks scheduler dihentikan (`stopschedule.js:59-64` / `77-82`).
#[cfg(test)]
pub(crate) fn dihentikan(nama: &str) -> String {
    format!(
        "🛑 *sᴄʜᴇᴅᴜʟᴇʀ ᴅɪʜᴇɴᴛɪᴋᴀɴ*\n\n> Scheduler: *{nama}*\n> Status: ❌ Dihentikan\n\n_Gunakan `.startschedule {nama}` untuk mengaktifkan kembali_",
    )
}

#[plugin(
    name = "stopschedule",
    alias = ["stopscheduler", "schedstop", "pauseschedule"],
    category = "owner",
    description = "Menghentikan scheduler tertentu atau semua",
    usage = ".stopschedule <nama|all>",
    example = ".stopschedule sholat",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stopscheduler(ctx: Ctx) -> Reply {
    // `stopSchedulerByName` / `stopSholatScheduler` / `getDatabase` /
    // `db.setting` tidak tersedia di Ctx — diakhiri error_template seperti
    // swm.rs.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command)).with_reaction("☢")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_dan_alias() {
        let p = oktz_plugin_api::find("stopschedule").unwrap();
        assert_eq!(p.meta.name, "stopschedule");
        assert_eq!(p.meta.alias, ["stopscheduler", "schedstop", "pauseschedule"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 3);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
    }

    #[test]
    fn daftar_scheduler_sama_dengan_js() {
        assert_eq!(JADWAL.len(), 6);
        assert_eq!(JADWAL[0], ("limitreset", "Daily Limit Reset"));
        assert_eq!(JADWAL[4], ("sholat", "Sholat Scheduler"));
        assert_eq!(JADWAL[5], ("all", "Semua scheduler"));
    }

    #[test]
    fn bantuan_semua_scheduler_datang() {
        let out = bantuan(".");
        assert!(out.starts_with("🛑 *sᴛᴏᴘ sᴄʜᴇᴅᴜʟᴇʀ*\n\n*Usage:*\n"));
        assert!(out.contains("• `sholat` - Sholat Scheduler"));
        assert!(out.contains("• `all` - Semua scheduler"));
        assert!(out.contains(".stopschedule sholat"));
        assert!(out.contains(".stopschedule all"));
    }

    #[test]
    fn is_sholat_dan_is_all() {
        assert!(is_sholat("sholat"));
        assert!(!is_sholat("all"));
        assert!(is_all("all"));
        assert!(!is_all("sholat"));
    }

    #[test]
    fn dihentikan_bentuk_js() {
        assert!(
            dihentikan("sholat").contains("Scheduler: *sholat*")
        );
        assert!(dihentikan("sholat").contains("Status: ❌ Dihentikan"));
        assert!(dihentikan("sholat").contains(".startschedule sholat"));
    }

    #[test]
    fn handler_akhir_error_template() {
        let ctx = Ctx {
            prefix: ".".into(),
            command: "stopschedule".into(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            ..Ctx::default()
        };
        let r = stopscheduler(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "stopschedule")
        );
        assert!(
            matches!(
                r.without_mutations(),
                Reply::Reaction { emoji, .. } if emoji == "☢"
            )
        );
    }
}
