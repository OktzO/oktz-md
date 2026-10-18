//! `plugins/ai/tohijab.js` — tambahkan hijab ke gambar.

use oktz_plugin_api::{Ctx, MediaSource, Reply, plugin};

#[plugin(
    name = "tohijab",
    alias = ["hijab", "hijabstyle", "addhijab"],
    category = "ai",
    description = "Tambahkan hijab ke gambar",
    usage = ".tohijab (reply gambar)",
    example = ".tohijab",
    cooldown = 30,
    energi = 2,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tohijab(ctx: Ctx) -> Reply {
    // `tohijab.js` butuh media (gambar) dari pesan pemicu atau quoted, lalu
    // `uploadImage(buffer, 'image.jpg')` dan `applyImageEffect('tohijab', url)`.
    // Kedua butuh I/O (download media, upload ke penyimpanan, panggil API efek)
    // yang belum ada di `Ctx` class A. Fitur ini tidak bisa di-port sepenuhnya
    // tanpa menambah field/media-fetch ke `Ctx`.
    //
    // `ponytail:` tambahkan `Ctx::download_media()` (sudah ada) untuk mengambil
    // byte gambar, lalu gunakan `oktz_scraper` atau efek API untuk menghasilkan
    // gambar hijab. Saat ini balas pesan penjelasan saja.
    Reply::text(format!(
        "🧕 *ʜɪᴊᴀʙ sᴛʏʟᴇ*\n\n> Kirim/reply gambar\n\n`{prefix}tohijab`",
        prefix = ctx.prefix
    ))
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
            command: "tohijab".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = tohijab(ctx_of());
        let t = r.text_body().unwrap();
        assert!(t.contains("🧕 *ʜɪᴊᴀʙ sᴛʏʟᴇ*"));
        assert!(t.contains("`.tohijab`"));
    }
}
