//! `plugins/main/leaderboard.js` — lihat leaderboard global (koin, exp, energi).

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🏆 *Lᴇᴀᴅᴇʀʙᴏᴀʀᴅ*\n\n> Lihat leaderboard global (koin, exp, energi)\n\n`{prefix}leaderboard`",
        prefix = prefix
    )
}

#[plugin(
    name = "leaderboard",
    alias = ["lb", "top", "leaderboard", "ranking", "rank", "topglobal",
             "topbalance", "topbal", "topkoin", "topcoin", "topmoney",
             "toplimit", "topexp", "topxp", "toplevel",
             "topenergi", "topenergy"],
    category = "main",
    description = "Lihat leaderboard global (koin, exp, energi)",
    usage = ".leaderboard",
    example = ".topkoin",
    cooldown = 10,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn leaderboard(ctx: Ctx) -> Reply {
    // `leaderboard.js` butuh:
    //   1. `getDatabase()` + `db.data.users` / `db.getAllUsers()` — data user
    //      tersimpan di store, tidak ada di `Ctx`.
    //   2. `getAssetBuffer('foto')` — baca file asset dari disk, tidak ada di
    //      `Ctx`.
    //   3. `sock.sendButton` / `sock.sendMedia` — kirim pesan interaktif +
    //      media, I/O WA yang belum ada di `Ctx` class A.
    //
    // Semua perubahan state (db.markDirty) juga butuh akses store.
    //
    // `ponytail:` sediakan akses user data + asset buffer lewat `Ctx` (atau
    // lewat `Mutation::SetSetting` jika router menyediakannya), lalu render
    // teks ranking sesuai format JS.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(cmd: &str) -> Ctx {
        Ctx {
            text: String::new(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: cmd.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = leaderboard(ctx_of("leaderboard"));
        let t = r.text_body().unwrap();
        assert!(t.contains("🏆 *Lᴇᴀᴅᴇʀʙᴏᴀʀᴅ*"));
        assert!(t.contains("`.leaderboard`"));
    }

    #[test]
    fn alias_topkoin_masih_pakai_handler_sama() {
        let r = leaderboard(ctx_of("topkoin"));
        assert!(r.text_body().unwrap().contains("Lᴇᴀᴅᴇʀʙᴏᴀʀᴅ"));
    }
}
