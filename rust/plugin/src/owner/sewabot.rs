//! `plugins/owner/sewabot.js` — toggle dan kelola sistem sewa bot.
//!
//! `ponytail:` `getDatabase()` / `db.db.data.sewa`, `pendingConfirmations`
//! (state global), `sock.groupFetchAllParticipating()`, `sock.sendText`,
//! `sock.groupLeave()`, dan `saluranCtx()` tidak ada di `Ctx` API. Semua
//! cabang (status/on/off/confirm/leave/cancel) tidak dapat dijalankan. Yang
//! dipindah: teks penjelasan ketersediaan.
//!
//! Catatan: baris `pub mod sewabot;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs sudah milik plugin lain, dilarang disentuh).

use oktz_plugin_api::{Ctx, Reply, plugin};

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
    let _ = ctx;
    Reply::text(
        "⚠️ *Sistem Sewa Bot tidak tersedia di port Rust*\n\n\
         > `getDatabase()` / `db.db.data.sewa`, state konfirmasi pending,\n\
         > `sock.groupFetchAllParticipating()`, `sock.groupLeave()`, dan\n\
         > `saluranCtx()` belum tersedia di `Ctx` API.\n\
         > Cabang `on`/`off`/`leave`/`confirm` tidak dapat dijalankan.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> Reply {
        let ctx = Ctx {
            command: "sewabot".into(),
            ..Default::default()
        };
        sewabot(ctx)
    }

    fn body(r: &Reply) -> String {
        r.text_body().expect("harus ada teks").to_string()
    }

    #[test]
    fn registers_with_js_plugin_config() {
        let p = oktz_plugin_api::find("sewabot").unwrap();
        assert_eq!(p.meta.name, "sewabot");
        assert_eq!(p.meta.alias, ["sewa"]);
        assert_eq!(p.meta.category, "owner");
        assert!(p.meta.is_owner);
    }

    #[test]
    fn tampilkan_pesan_keterbatasan() {
        let r = run();
        assert!(body(&r).contains("Sistem Sewa Bot tidak tersedia"));
        assert!(r.mutations().is_empty());
    }
}
