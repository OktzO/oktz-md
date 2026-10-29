//! `plugins/canvas/starboy.js` — efek gambar Starboy dari foto.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh deteksi `mimetype` pada quoted/trigger (tidak lengkap di
//! `Ctx`), `q.download()`, `uploadImage` dari `src/lib/uploader.js`, `axios`
//! ke `https://api.cuki.biz.id/api/canvas/starboy?apikey=...&image=...`
//! (`responseType: 'arraybuffer'`), dan `sock.sendMessage(..., { image:
//! imageBuffer })`.
//!
//! `ponytail:` kalau media intake dan uploader tersedia di `Ctx` nanti, port
//! ulang dari `plugins/canvas/starboy.js` (reaksi `🕕` → `✅`/`☢`).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "starboy",
    alias = ["canvasstarboy", "efekstarboy"],
    category = "canvas",
    description = "Buat gambar efek Starboy dari foto (DIBLOKIR: butuh download media + uploader + sock)",
    usage = ".starboy <reply/kirim foto>",
    example = ".starboy",
    cooldown = 5,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn starboy(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("starboy").unwrap();
        assert_eq!(p.meta.name, "starboy");
        assert_eq!(p.meta.alias, ["canvasstarboy", "efekstarboy"]);
        assert_eq!(p.meta.category, "canvas");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }
}
