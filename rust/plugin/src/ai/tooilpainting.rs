//! `plugins/ai/tooilpainting.js` — gaya lukisan minyak (oil painting) via
//! `src/scraper/seaart.js` (`live3d`).
//!
//! ## BLOKIR — tidak di-port (parsial).
//!
//! Alasan: butuh `m.download()` / `m.quoted.download()` (media intake yang
//! tidak ada di `Ctx`) untuk bentuk buffer input `live3d`, dan
//! `sock.sendMedia(..., { type: "image" })` untuk hasil byte gambar. Scraper
//! `oktz_scraper::seaart::live3d` **sudah ada** dan siap dipakai; yang belum
//! tersedia adalah ambient-nya di `Ctx`.
//!
//! `ponytail:` kalau `download_media` tersedia di `Ctx` dan variant kirim
//! gambar byte tersedia di `Reply` nanti, port ulang dari
//! `plugins/ai/tooilpainting.js` (PROMPT hardcode, reaksi `🕕` → `✅`/`☢`).

use oktz_plugin_api::{Ctx, Reply, plugin};

/// `tooilpainting.js:19-22` — prompt hardcode.
#[cfg(test)]
pub(crate) const PROMPT: &str = "Transform this image into a classical oil painting style. \
Apply thick brushstrokes, rich colors, and the texture of traditional oil paint on canvas. \
Keep the original composition but make it look like a masterpiece painting \
with visible brushwork, artistic color blending, and that timeless gallery-quality aesthetic.";

#[plugin(
    name = "tooilpainting",
    alias = ["oilpainting", "tooil", "oil"],
    category = "ai",
    description = "Ubah foto menjadi gaya lukisan minyak (oil painting) (DIBLOKIR: butuh download media + kirim gambar via sock)",
    usage = ".tooilpainting (reply/kirim gambar)",
    example = ".tooilpainting",
    cooldown = 60,
    energi = 3,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = true,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tooilpainting(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tooilpainting").unwrap();
        assert_eq!(p.meta.name, "tooilpainting");
        assert_eq!(p.meta.alias, ["oilpainting", "tooil", "oil"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 60);
        assert_eq!(p.meta.energi, 3);
        assert!(p.meta.is_premium);
        assert!(!p.meta.is_group && !p.meta.is_private && !p.meta.is_owner);
    }

    #[test]
    fn prompt_sama_dengan_js() {
        assert!(!PROMPT.is_empty());
        assert!(PROMPT.contains("oil painting"));
    }
}
