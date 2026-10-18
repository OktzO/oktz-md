//! `plugins/owner/ptvch.js` — kirim video sebagai PTV ke channel.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: butuh `m.download()` / `m.quoted.download()` (media intake yang
//! tidak ada di `Ctx`), dan `sock.sendMessage(channelId, { video, mimetype,
//! gifPlayback: true, ptv: true })` — tidak ada variant `Reply` untuk video
//! PTV dan tidak ada `download()` di `Ctx`. Channel ID dari
//! `config.saluran?.id` dengan fallback hardcode.
//!
//! `ponytail:` kalau `download_media` dan variant video PTV tersedia di
//! `Ctx`/`Reply` nanti, port ulang dari `plugins/owner/ptvch.js` (reaksi
//! `🕕` lalu `✅`, pesan konfirmasi ke pemilik).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ptvch",
    alias = ["ptvchanel", "ptvstory"],
    category = "owner",
    description = "Kirim video sebagai PTV ke channel (DIBLOKIR: butuh download media + kirim video PTV via sock)",
    usage = ".ptvch (reply video)",
    example = ".ptvch",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ptvch(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("ptvch").unwrap();
        assert_eq!(p.meta.name, "ptvch");
        assert_eq!(p.meta.alias, ["ptvchanel", "ptvstory"]);
        assert_eq!(p.meta.category, "owner");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(p.meta.is_owner);
        assert!(!p.meta.is_group && !p.meta.is_private);
    }
}
