//! `plugins/owner/ganti-asset.js` — ganti asset secara interaktif (image/video/audio/font/other).

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🖼️ *ɢᴀɴᴛɪ ᴀssᴇᴛ*\n\n> Silakan reply media (gambar/video/audio/document) dengan pesan `{prefix}ganti-asset`",
        prefix = prefix
    )
}

#[plugin(
    name = "ganti-asset",
    alias = ["gantiasset", "setasset"],
    category = "owner",
    description = "All-in-one tools untuk ganti asset secara interaktif",
    usage = ".ganti-asset (reply media)",
    example = ".ganti-asset",
    cooldown = 5,
    energi = 0,
    is_owner = true,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_asset(ctx: Ctx) -> Reply {
    // `ganti-asset.js` butuh:
    //   1. `config.assets` — daftar kunci asset, ada di `ctx.config().assets`.
    //   2. `m.download()` / `m.quoted.download()` — byte media, tersedia via
    //      `ctx.media_fetch.download(...)`.
    //   3. `updateAssetUrl(key, buffer, filename)` — tulis file ke disk +
    //      rewrite `config.js` (regex replace) + `fs.writeFileSync`. I/O
    //      filesystem yang tidak ada di `Ctx`.
    //   4. Sesi interaktif per chat (`global.gantiAssetSessions`) + timeout
    //      120 detik, dan `gantiAssetAnswerHandler` untuk balasan nomor.
    //
    // `Ctx` tidak puny mekanisme sesi interaktif yang bertahan antar pesan,
    // dan tidak bisa menulis `config.js`. Handler ini tidak bisa di-port
    // sepenuhnya tanpa menambah kemampuan tersebut.
    //
    // `ponytail:` sediakan akses tulis config + filesystem, dan daftarkan
    // `answer_handler` untuk nomor balasan asset.
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
            command: "ganti-asset".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = ganti_asset(ctx_of());
        assert!(r.text_body().unwrap().contains("ɢᴀɴᴛɪ ᴀssᴇᴛ"));
    }
}
