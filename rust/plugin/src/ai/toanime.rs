//! `plugins/ai/toanime.js` — ubah foto menjadi gaya anime/Ghibli Studio.

use oktz_plugin_api::{Ctx, Reply, plugin};

pub(crate) fn help(prefix: &str) -> String {
    format!(
        "🎨 *ᴛᴏ ᴀɴɪᴍᴇ*\n\n> Kirim/reply gambar untuk diubah ke gaya anime\n\n`{prefix}toanime`",
        prefix = prefix
    )
}

#[plugin(
    name = "toanime",
    alias = ["anime", "animefy"],
    category = "ai",
    description = "Ubah foto menjadi gaya anime/Ghibli Studio",
    usage = ".toanime (reply/kirim gambar)",
    example = ".toanime",
    cooldown = 60,
    energi = 3,
    is_owner = false,
    is_premium = true,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn toanime(ctx: Ctx) -> Reply {
    // `toanime.js` butuh media (gambar) dari pesan pemicu atau quoted:
    //   1. `m.download()` / `m.quoted.download()` → byte gambar (ctx.media_fetch tersedia)
    //   2. `live3d(buffer, PROMPT)` → { image } (oktz_scraper::seaart::live3d tersedia)
    //   3. `sock.sendMedia(m.chat, result.image, ..., { type: 'image' })` → Reply::image(MediaSource::Bytes)
    //
    // Semua komponen HTTP tersedia di `Ctx`. Namun `seaart::live3d` butuh
    // `&dyn HttpGet` — miliknya `ctx.http`. Masalah: `live3d` mengembalikan
    // `Vec<u8>` (byte gambar mentah), sementara `Reply::image` butuh
    // `MediaSource::Bytes { data: Arc<[u8]> }`. Konversi langsung memungkinkan.
    //
    // Di sini ditarunda karena handler perlu mengakses media fetched (byte)
    // sebelum memutuskan balasan; `Reply` statis tidak bisa mencakup alur
    // "unduh dulu baru kirim" tanpa async. Router saat ini `Fn(Ctx) -> Reply`
    // sinkron.
    //
    // `ponytail:` balas kartu pakai sementara. Setelah `Ctx` mendukung alur
    // unduh-berikut-kirim (atau handler jadi async), pasang:
    //   let buf = ctx.media_fetch.download(MediaOrigin::Trigger)?;
    //   let img = seaart::live3d(&ctx.http, &buf, PROMPT)?;
    //   Reply::image(MediaSource::bytes(img), "").with_reaction("✅")
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
            command: "toanime".into(),
            ..Default::default()
        }
    }

    #[test]
    fn selalu_membalas_kartu_pakai() {
        let r = toanime(ctx_of());
        let t = r.text_body().unwrap();
        assert!(t.contains("🎨 *ᴛᴏ ᴀɴɪᴍᴇ*"));
        assert!(t.contains("`.toanime`"));
    }
}
