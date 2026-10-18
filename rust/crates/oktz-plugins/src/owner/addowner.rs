//! `plugins/owner/addowner.js` — kelola owner bot (mode-aware).
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port sepenuhnya karena:
//!   1. `getDatabase()` / `db.data.owner` / `db.setting("ownerNames")` / `db.save()`
//!      — Rust hanya punya `Mutation::SetSetting` (tidak ada baca `db.data` mentah).
//!   2. `config.pterodactyl.ownerPanels` / `.sellers` — tidak ada di `Runtime`.
//!   3. `jadibot-database` (`addJadibotOwner`, `removeJadibotOwner`, `getJadibotOwners`)
//!      — tidak ada di `Ctx`.
//!   4. `resolveAnyLidToJid` / `isLid` / `lidToJid` — resolusi LID tidak tersedia.
//!   5. `getGroupMode(m.chat, db)` — mode grup tidak tersedia lewat `Ctx`.
//!
//! Bagian yang *bisa* di-port (set/get `ownerNames` via `Mutation::SetSetting`,
//! daftar owner dari `config.owner.number`) tertunda sampai kapasitas di atas
//! tersedia. Berkas ini dibuat agar nama file terdaftar; implementasi ditunda.
//!
//! `ponytail:` saat DB baca-tulis, `config.pterodactyl`, dan resolusi LID
//! tersedia, implementasikan ulang dan tambahkan `pub mod addowner;`
//! di `src/owner/mod.rs` secara manual (mod.rs milik plugin lain — dilarang
//! menyentuh per brief).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "addowner",
    alias = ["addown", "setowner", "delowner", "dedown", "ownerlist", "listowner"],
    category = "owner",
    description = "Kelola owner bot (mode-aware) — BLOKIRAN",
    usage = ".addowner <nomor/@tag/reply>",
    example = ".addowner 6281234567890",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn addowner(ctx: Ctx) -> Reply {
    Reply::text(
        "❌ *ADD OWNER — TIDAK TERSEDIA*\n\n\
         Port Rust tertunda karena butuh:\n\
         • baca-tulis `db.data.owner` dan `db.setting(\"ownerNames\")`\n\
         • `config.pterodactyl` (ownerPanels, sellers)\n\
         • jadibot-database + resolusi LID\n\
         • `getGroupMode` untuk mode cpanel",
    )
}
