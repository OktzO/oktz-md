//! `plugins/owner/stopdandeletejadibot.js` — stop dan hapus session jadibot user.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🗑️ *sᴛᴏᴘ & ᴅᴇʟᴇᴛᴇ ᴊᴀᴅɪʙᴏᴛ*\n\n> Pilih target dengan mention atau reply:\n\n> Contow: `{prefix}stopdandeletejadibot @628xxx`",
        prefix = prefix
    )
}

#[plugin(
    name = "stopdandeletejadibot",
    alias = ["deletejadibot", "removejadibot", "hapusjadibot"],
    category = "owner",
    description = "Stop dan hapus session jadibot user secara permanen",
    usage = ".stopdandeletejadibot @user",
    example = ".stopdandeletejadibot @628xxx",
    cooldown = 10,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stop_dan_delete_jadibot(ctx: Ctx) -> Reply {
    // `stopdandeletejadibot.js` butuh:
    //   1. `stopJadibot(target, true)` / `getAllJadibotSessions()` dari
    //      `src/lib/jadibot-manager.js` — manajemen session jadibot (WebSocket,
    //      auth folder, Turso) yang tidak ada di crate Rust.
    //   2. `m.quoted.sender`, `m.mentionedJid`, `m.text` — sebagian ada di
    //      `Ctx`, tapi operasi stop/hapus session tidak tersedia.
    //
    // `ponytail:` sediakan antarmuka stop/hapus session jadibot, lalu
    // replikasikan daftar sesi + pilihan target via mention/reply.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of(text: &str) -> Ctx {
        Ctx {
            text: text.to_string(),
            args: text.split_whitespace().map(str::to_string).collect(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "stopdandeletejadibot".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = stop_dan_delete_jadibot(ctx_of(""));
        assert!(r.text_body().unwrap().contains("sᴛᴏᴘ & ᴅᴇʟᴇᴛᴇ ᴊᴀᴅɪʙᴏᴛ"));
    }
}
