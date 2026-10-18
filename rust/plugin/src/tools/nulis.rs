//! `plugins/tools/nulis.js` — generate tulisan tangan di kertas.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `@napi-rs/canvas` (`createCanvas`, `loadImage`,
//! `GlobalFonts.register` dengan font `Zahraaa` dari asset), `getAssetBuffer`
//! ("font" dan "kertas"), `timeHelper` (`formatDate "DD/MM/YYYY"`,
//! `formatFull "dddd"`), `saluranCtx()`, dan `sock.sendMedia(..., { type:
//! "image" }, contextInfo)`.
//!
//! `ponytail:` kalau API canvas dan aset tersedia di `Ctx` nanti, port ulang
//! dari `plugins/tools/nulis.js` (wrap teks lewat `measureText`, tanggal +
//! hari di kanan atas kertas, buffer JPEG).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "nulis",
    alias = ["tulis", "write"],
    category = "tools",
    description = "Generate tulisan tangan di kertas (DIBLOKIR: butuh canvas API + aset + saluranCtx)",
    usage = ".nulis <teks>",
    example = ".nulis Aku cinta kamu selamanya",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn nulis(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("nulis").unwrap();
        assert_eq!(p.meta.name, "nulis");
        assert_eq!(p.meta.alias, ["tulis", "write"]);
        assert_eq!(p.meta.category, "tools");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 1);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }
}
