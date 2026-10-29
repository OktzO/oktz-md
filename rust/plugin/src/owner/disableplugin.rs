//! `plugins/owner/disableplugin.js` — menonaktifkan plugin tertentu.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port karena:
//!   1. Plugin Rust tidak punya kolom `isEnabled` yang mengontrol runtime
//!      (beda dengan JS yang menimpa `isEnabled: false` di berkas sumber).
//!   2. `ctx.berkas()` berakar ke folder aset, bukan `plugins/`; tidak dapat
//!      membaca/mengubah berkas plugin JS secara dinamis.
//!   3. Tidak ada mekanisme `import()` dinamis untuk memeriksa
//!      `plugin.config` dari berkas lain.
//!
//! Berkas ini dibuat sebagai catatan; implementasi ditunda sampai ada
//! mekanisme enable/disable plugin di level registry.
//!
//! `ponytail:` saat registry mendukung `is_enabled` runtime dan `berkas()`
//! dapat mengakses folder plugin, implementasikan ulang dan tambahkan
//! `pub mod disableplugin;` di `src/owner/mod.rs` secara manual.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "disableplugin",
    alias = ["dplugin", "plugindisable", "offplugin"],
    category = "owner",
    description = "Menonaktifkan plugin tertentu — BLOKIRAN",
    usage = ".disableplugin <nama_plugin>",
    example = ".disableplugin sticker",
    cooldown = 3,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true
)]
pub fn disableplugin(ctx: Ctx) -> Reply {
    Reply::text(
        "❌ *DISABLE PLUGIN — TIDAK TERSEDIA*\n\n\
         Plugin Rust tidak punya kolom `isEnabled` runtime dan tidak dapat\n\
         mengubah berkas plugin lain secara dinamis. Fitur ini tertunda.\n\n\
         Contoh JS: `.disableplugin sticker`",
    )
}
