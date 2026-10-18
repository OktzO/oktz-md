//! `plugins/game/minecraft.js` — Minecraft - Mining & Crafting Game.
//!
//! `NOTE (mod.rs):` `game/mod.rs` sudah memuat 8 modul lain (bukan milik
//! tugas ini), jadi berkas ini sengaja tidak disentuh. Tambahkan baris
//! berikut secara manual ke `game/mod.rs` saat orchestrator menyetujui:
//!   pub mod minecraft;

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🧱 *MINECRAFT GAME*\n_⛏️ Sistem pertambangan & 🛠️ crafting terlengkap_\n\n`{prefix}mct <command>`",
        prefix = prefix
    )
}

#[plugin(
    name = "mct",
    alias = ["minecraft"],
    category = "game",
    description = "Minecraft - Mining & Crafting Game",
    usage = ".mct <command>",
    example = ".mct help",
    cooldown = 3,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn minecraft(ctx: Ctx) -> Reply {
    // `minecraft.js` (1017 baris) butuh:
    //   1. `getDatabase()` + seluruh operasi store user (`getOrCreateMCUser`,
    //      `db.setGroup`, `db.markDirty`, `db.getGroup`) — store tidak ada di
    //      `Ctx`.
    //   2. `src/lib/minecraft.js` + `src/lib/minecraft-data.js` — logika game
    //      penuh (tambang, craft, combat, gacha, jackpot, prestige, dll.) yang
    //      tidak ada di crate Rust.
    //   3. `getAssetBuffer('minecraft')` — baca thumbnail dari disk, tidak ada
    //      di `Ctx`.
    //   4. `sock.sendPreview` — kirim preview link, I/O WA yang belum ada di
    //      `Ctx` class A.
    //
    // `ponytail:` sediakan akses store user/grup + asset buffer + kirim
    // preview ke `Ctx`, lalu port seluruh logika game dari `minecraft.js` dan
    // `minecraft-data.js` ke fungsi-fungsi di berkas ini (atau modul pembantu
    // yang di-copy, karena dilarang menyentuh berkas lain).
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(sub: &str) -> Ctx {
        let text = format!(".mct {sub}");
        Ctx {
            text,
            args: if sub.is_empty() {
                Vec::new()
            } else {
                sub.split_whitespace().map(str::to_string).collect()
            },
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "mct".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = minecraft(ctx_of("help"));
        assert!(r.text_body().unwrap().contains("MINECRAFT GAME"));
    }

    #[test]
    fn alias_minecraft_sama_handler() {
        let r = minecraft(ctx_of(""));
        assert!(r.text_body().unwrap().contains("MINECRAFT"));
    }
}
