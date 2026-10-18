//! `plugins/owner/pinchat.js` — pin/unpin chat.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📌 *ᴘɪɴ ᴄʜᴀᴛ*\n\n> `.pinchat 628xxx` — Pin chat\n> `.pinchat` (di private chat) — Pin chat ini\n> `.pinchat buka 628xxx` — Unpin chat\n\n`{prefix}pinchat <nomor/reply> atau .pinchat buka <nomor>`",
        prefix = prefix
    )
}

#[plugin(
    name = "pinchat",
    alias = [],
    category = "owner",
    description = "Pin/unpin chat",
    usage = ".pinchat <nomor/reply> atau .pinchat buka <nomor>",
    example = ".pinchat 628xxx",
    cooldown = 3,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn pinchat(ctx: Ctx) -> Reply {
    // `pinchat.js` butuh `sock.chatModify({ pin }, targetJid)` — operasi WA
    // langsung yang belum ada di `Ctx` class A. Juga butuh `m.mentionedJid`,
    // `m.quoted.sender/participant`, `m.args` — sebagian ada, tapi kiriman
    // ke WA tidak tersedia.
    //
    // `ponytail:` sediakan operasi chatModify lewat `Ctx` (atau router),
    // lalu replikasikan logika targetJid (mention → quoted → args → chat).
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
            command: "pinchat".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = pinchat(ctx_of(""));
        assert!(r.text_body().unwrap().contains("ᴘɪɴ ᴄʜᴀᴛ"));
    }

    #[test]
    fn argumen_buka_tetap_kartu_pakai_belum_ada_wa_io() {
        let r = pinchat(ctx_of("buka 628123"));
        assert!(r.text_body().unwrap().contains("ᴘɪɴ ᴄʜᴀᴛ"));
    }
}
