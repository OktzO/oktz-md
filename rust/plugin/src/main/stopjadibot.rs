//! `plugins/main/stopjadibot.js` — hentikan sesi jadibot via `jadibot-manager`.
//!
//! `ponytail:` `stopJadibot`, `isJadibotActive`, dan `getJadibotStatus` dari
//! `src/lib/jadibot-manager.js` tidak diekspos ke plugin di registry Rust.
//! Tidak ada state jadibot yang dapat dibaca atau diubah.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "stopjadibot",
    alias = ["berhentijadibot", "stopbot", "unjadibot"],
    category = "main",
    description = "Hentikan sesi jadibot kamu",
    usage = ".stopjadibot",
    example = ".stopjadibot",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn stopjadibot(ctx: Ctx) -> Reply {
    // `ponytail:` Semua operasi jadibot butuh jadibot-manager yang tidak
    // tersedia di registry Rust.
    Reply::text(crate::common::error_template(&ctx.prefix, &ctx.command))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: &str = "628111@s.whatsapp.net";

    #[test]
    fn selalu_error_tanpa_api_jadibot() {
        let ctx = Ctx {
            sender: ME.into(),
            chat: ME.into(),
            prefix: ".".into(),
            command: "stopjadibot".into(),
            ..Ctx::default()
        };
        let r = stopjadibot(ctx);
        assert_eq!(
            r.without_mutations().text_body().unwrap(),
            crate::common::error_template(".", "stopjadibot")
        );
    }
}
