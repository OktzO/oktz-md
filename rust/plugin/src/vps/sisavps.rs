//! `plugins/vps/sisavps.js` — cek sisa kuota VPS DigitalOcean.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: sama dengan `cekvps.js` — `config.digitalocean.token`,
//! `sellers`, dan `ownerPanels` tidak ada pada
//! `oktz_core::config::Runtime`, dan `Promise.all` dua endpoint
//! (`/v2/account` + `/v2/droplets`) butuh `sock`-less HTTP yang belum
//! tersedia untuk layout respons akun DigitalOcean dalam satu `Reply`.
//!
//! `ponytail:` kalau `config.digitalocean` ditambahkan ke `Runtime` nanti,
//! port ulang dari `plugins/vps/sisavps.js`.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "sisavps",
    alias = ["sisadroplet", "vpsquota"],
    category = "vps",
    description = "Cek sisa kuota VPS DigitalOcean (DIBLOKIR: config.digitalocean belum tersedia di Rust)",
    usage = ".sisavps",
    example = ".sisavps",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn sisavps(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("sisavps").unwrap();
        assert_eq!(p.meta.name, "sisavps");
        assert_eq!(p.meta.alias, ["sisadroplet", "vpsquota"]);
        assert_eq!(p.meta.category, "vps");
        assert_eq!(p.meta.cooldown, 10);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }
}
