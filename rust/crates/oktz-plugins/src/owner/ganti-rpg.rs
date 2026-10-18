//! `plugins/owner/ganti-rpg.js` — ganti gambar rpg.jpg (thumbnail rpg).

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🖼️ *ɢᴀɴᴛɪ ʀᴘɢ.ᴊᴘɢ*\n\n> Kirim/reply gambar untuk mengganti\n> File: assets/image/rpg.jpeg\n\n`{prefix}ganti-rpg.jpg`",
        prefix = prefix
    )
}

#[plugin(
    name = "ganti-rpg.jpg",
    alias = ["gantirpg", "setrpg"],
    category = "owner",
    description = "Ganti gambar rpg.jpg (thumbnail rpg)",
    usage = ".ganti-rpg.jpg (reply/kirim gambar)",
    example = ".ganti-rpg.jpg",
    cooldown = 5,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_rpg(ctx: Ctx) -> Reply {
    // `ganti-rpg.js` butuh:
    //   1. `m.isImage` / `m.quoted` — cek media, sebagian ada di `Ctx`.
    //   2. `m.download()` / `m.quoted.download()` — byte gambar, tersedia via
    //      `ctx.media_fetch.download(...)`.
    //   3. `updateAssetUrl('rpg', buffer, 'rpg.jpeg')` — tulis file + rewrite
    //      `config.js`. I/O filesystem yang tidak ada di `Ctx`.
    //
    // `ponytail:` tambahkan akses tulis asset/file ke `Ctx`, lalu unduh byte
    // gambar dan simpan ke `assets/image/rpg.jpeg`.
    Reply::text(help(&ctx.prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_of() -> Ctx {
        Ctx {
            text: String::new(),
            args: Vec::new(),
            sender: "628111@s.whatsapp.net".into(),
            chat: "628111@s.whatsapp.net".into(),
            prefix: ".".into(),
            command: "ganti-rpg.jpg".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = ganti_rpg(ctx_of());
        assert!(r.text_body().unwrap().contains("ʀᴘɢ.ᴊᴘɢ"));
    }
}
