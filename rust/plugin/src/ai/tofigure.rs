//! `plugins/ai/tofigure.js` — ubah gambar ke style Figure/Action.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `uploadImage` dari `src/lib/uploader.js`,
//! `applyImageEffect` dari `src/lib/effect-api.js` (daftar provider dari
//! `config.effectApi` yang tidak ada di `oktz_core::config::Runtime`), dan
//! `sock.sendMedia(..., { type: "image" })`. Hanya satu efek `tofigura`.
//!
//! `ponytail:` kalau uploader dan daftar provider efek tersedia di `Ctx`
//! nanti, port ulang dari `plugins/ai/tofigure.js` (reaksi `🕕` → `✅`/`☢`,
//! upload gambar, terapkan efek, kirim hasil).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "tofigure",
    alias = ["figure", "figurestyle"],
    category = "ai",
    description = "Ubah gambar ke style Figure/Action (DIBLOKIR: butuh uploader + applyImageEffect + sock)",
    usage = ".tofigure (reply gambar)",
    example = ".tofigure",
    cooldown = 30,
    energi = 2,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn tofigure(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("tofigure").unwrap();
        assert_eq!(p.meta.name, "tofigure");
        assert_eq!(p.meta.alias, ["figure", "figurestyle"]);
        assert_eq!(p.meta.category, "ai");
        assert_eq!(p.meta.cooldown, 30);
        assert_eq!(p.meta.energi, 2);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }
}
