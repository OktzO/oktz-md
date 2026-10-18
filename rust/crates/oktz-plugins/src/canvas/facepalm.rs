//! `plugins/canvas/facepalm.js` — bikin gambar facepalm dari foto.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `downloadMediaMessage` + `loadImage` (media intake dan
//! decoding gambar yang tidak ada di `Ctx`), `createCanvas`/`canvas.getContext`
//! (`@napi-rs/canvas`, render overlay 632x357), `axios` untuk ambil overlay
//! dari GitHub (`responseType: "arraybuffer"`), dan
//! `sock.sendMessage(..., { image: buffer, caption: "🤦‍♂️ *Facepalm*" })`.
//
//! `ponytail:` kalau API canvas dan `download_media` tersedia di `Ctx` nanti,
//! port ulang dari `plugins/canvas/facepalm.js` (guard tipe imageMessage,
//! reaksi `🕕`, gambar avatar di (199,112) 235x235, overlay di (0,0),
//! encode PNG).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "facepalm",
    alias = [],
    category = "canvas",
    description = "Bikin gambar facepalm dari fotomu (DIBLOKIR: butuh canvas API + download media + sock)",
    usage = ".facepalm (reply/kirim foto)",
    example = ".facepalm",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn facepalm(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("facepalm").unwrap();
        assert_eq!(p.meta.name, "facepalm");
        assert!(p.meta.alias.is_empty());
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }
}
