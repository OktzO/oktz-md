//! `plugins/game/ulartangga.js` — ular tangga bersama player lain dengan visual board.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port karena:
//!   1. `drawBoard` dari `../../src/lib/game-ulartangga.js` — render board
//!      kustom yang memerlukan canvas/gambar, tidak ada di `Ctx`.
//!   2. `sock.sendPreview(...)` — tidak ada di trait `Sock` atau `Reply`.
//!   3. `global.ulartanggaGames` — state game global antar pesan tidak tersedia
//!      lewat `Ctx` (hanya `sessions` per-kunci).
//!   4. `config.isOwner?.(m.sender)` — pemeriksaan owner custom tidak tersedia.
//!   5. `db.updateKoin`, `db.updateEnergi`, `db.getUser`, `db.setUser` —
//!      hanya `Mutation::SetUser`/`SetSetting` yang tersedia.
//!   6. Answer handler `"kocok"` — tersedia via `#[answer_handler]`, namun
//!      butuh state game yang sama.
//!
//! Berkas ini dibuat sebagai catatan; implementasi ditunda sampai kapasitas
//! di atas tersedia.
//!
//! `ponytail:` saat state game + canvas board tersedia, implementasikan ulang
//! dan tambahkan `pub mod ulartangga;` di `src/game/mod.rs` secara manual.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ulartangga",
    alias = ["ut", "snakeladder", "sl"],
    category = "game",
    description = "Main ular tangga bersama player lain dengan visual board — BLOKIRAN",
    usage = ".ulartangga <create|join|start|info|exit|delete>",
    example = ".ulartangga create",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false
)]
pub fn ulartangga(ctx: Ctx) -> Reply {
    let _ = ctx;
    Reply::text(
        "❌ *ULAR TANGGA — TIDAK TERSEDIA*\n\n\
         Port Rust tertunda karena butuh:\n\
         • render board kustom (`drawBoard`)\n\
         • `sock.sendPreview`\n\
         • state game global antar pesan\n\
         • DB koin/energi/user\n\n\
         Contoh JS: `.ulartangga create`",
    )
}
