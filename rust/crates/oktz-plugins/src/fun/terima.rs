//! `plugins/fun/terima.js` — menerima tembakan (pasangan).
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `db.getUser`/`db.setUser` untuk JID lain (bukan hanya
//! `user_data` milik pengirim yang tersedia di `Ctx`), state global
//! `tembakSessions`, `m.botNumber`, `saluranCtx()` (injek `mentionedJid`),
//! dan `sock.sendMessage(..., { contextInfo })` dengan dua mention.
//!
//! `ponytail:` kalau store grup dan `sock` tersedia di `Ctx` nanti, port
//! ulang dari `plugins/fun/terima.js` (guard target tembakan, tulis
//! `pasangan`/`jadiPacar`/`terimaCount` ke dua user, hapus session, reaksi
//! `💕`).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "terima",
    alias = ["accept", "yes"],
    category = "fun",
    description = "Menerima tembakan dari seseorang (DIBLOKIR: butuh db.getUser/setUser + state global + sock)",
    usage = ".terima @tag",
    example = ".terima @628xxx",
    cooldown = 5,
    energi = 0,
    is_group = true,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn terima(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("terima").unwrap();
        assert_eq!(p.meta.name, "terima");
        assert_eq!(p.meta.alias, ["accept", "yes"]);
        assert_eq!(p.meta.category, "fun");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_group);
        assert!(!p.meta.is_private && !p.meta.is_owner);
    }
}
