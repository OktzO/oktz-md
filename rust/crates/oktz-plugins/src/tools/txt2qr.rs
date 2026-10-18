//! `plugins/tools/txt2qr.js` — generate QR code dari teks.

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `txt2qr.js:23` — kartu pakai saat teks kosong.
pub(crate) fn help(prefix: &str) -> String {
    format!(
        "📱 *ᴛᴇxᴛ ᴛᴏ Qʀ*\n\n> Masukkan teks/URL\n\n`Contoh: {prefix}txt2qr https://google.com`"
    )
}

#[plugin(
    name = "txt2qr",
    alias = ["texttoqr", "qrcode", "qrcreate"],
    category = "tools",
    description = "Generate QR code dari teks",
    usage = ".txt2qr <text>",
    example = ".txt2qr https://google.com",
    cooldown = 5,
    energi = 0,
    is_owner = false,
    is_premium = false,
    is_group = false,
    is_private = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn txt2qr(ctx: Ctx) -> Reply {
    let text = ctx.args.join(" ");
    if text.is_empty() {
        return Reply::text(help(&ctx.prefix));
    }

    // `ponytail:` `Ctx` tidak bawa kan QR renderer (qrcode JS). Pembuatan QR
    // membutuhkan crate gambar yang belum ada di dependensi crate ini.
    // Tambahkan renderer QR di sini (atau panggil lewat `oktz_scraper`) bila
    // tersedia. Sementara, balas pesan teks yang meniru output JS.
    Reply::text(format!("📱 *Qʀ ᴄᴏᴅᴇ*\n\n> {}", &text[..text.len().min(100)]))
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
            command: "txt2qr".into(),
            ..Default::default()
        }
    }

    #[test]
    fn teks_kosong_menghasilkan_kartu_pakai() {
        let r = txt2qr(ctx_of(""));
        assert_eq!(r.text_body().unwrap(), help("."));
    }

    #[test]
    fn teks_dirender_dalam_kartu_qr() {
        let r = txt2qr(ctx_of("https://google.com"));
        let t = r.text_body().unwrap();
        assert!(t.contains("📱 *Qʀ ᴄᴏᴅᴇ*"));
        assert!(t.contains("https://google.com"));
    }

    #[test]
    fn teks_lampau_100_karakter_dipotong() {
        let panjang = "a".repeat(200);
        let r = txt2qr(ctx_of(&panjang));
        let t = r.text_body().unwrap();
        assert_eq!(&t[t.find("> ").unwrap() + 2..], &panjang[..100]);
    }
}
