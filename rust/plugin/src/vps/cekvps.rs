//! `plugins/vps/cekvps.js` — cek detail VPS DigitalOcean.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: `config.digitalocean.token` dan
//! `config.digitalocean.sellers` / `config.digitalocean.ownerPanels` tidak
//! ada pada `oktz_core::config::Runtime` (hanya `api_keys`, `owner`,
//! `assets`, dst. — lihat `rust/crates/oktz-core/src/config.rs`). Tidak ada
//! jalur membaca kunci DigitalOcean dari `Ctx`, jadi guard `hasAccess` dan
//! `GET https://api.digitalocean.com/v2/droplets/{id}` tidak bisa dijalankan.
//!
//! `ponytail:` kalau `config.digitalocean` ditambahkan ke `Runtime` nanti,
//! port ulang dari `plugins/vps/cekvps.js` (guard token + hasAccess, render
//! detail droplet dengan IP v4/v6, status, spec, region, OS, dan created_at).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "cekvps",
    alias = ["cekdroplet", "vpsstatus", "infovps"],
    category = "vps",
    description = "Cek detail VPS DigitalOcean (DIBLOKIR: config.digitalocean belum tersedia di Rust)",
    usage = ".cekvps <id>",
    example = ".cekvps 123456789",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn cekvps(_ctx: Ctx) -> Reply {
    Reply::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_with_the_js_plugin_config() {
        let p = oktz_plugin_api::find("cekvps").unwrap();
        assert_eq!(p.meta.name, "cekvps");
        assert_eq!(
            p.meta.alias,
            ["cekdroplet", "vpsstatus", "infovps"]
        );
        assert_eq!(p.meta.category, "vps");
        assert_eq!(p.meta.cooldown, 5);
        assert_eq!(p.meta.energi, 0);
        assert!(!p.meta.is_owner && !p.meta.is_group && !p.meta.is_private);
    }
}
