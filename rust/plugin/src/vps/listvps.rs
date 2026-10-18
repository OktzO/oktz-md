//! `plugins/vps/listvps.js` — list semua VPS DigitalOcean.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: membutuhkan `config.digitalocean.token` dan
//! `config.digitalocean.sellers` / `config.digitalocean.ownerPanels`, yang
//! tidak ada pada `oktz_core::config::Runtime` (hanya `api_keys`, `owner`,
//! `assets`, dst. — lihat `rust/crates/oktz-core/src/config.rs`). Tidak ada
//! jalur membaca kunci DigitalOcean dari `Ctx`, jadi tidak bisa meniru
//! perilaku JS apa adanya.
//!
//! `ponytail:` kalau `config.digitalocean` ditambahkan ke `Runtime` nanti,
//! port ulang dari `plugins/vps/listvps.js` (guard token + hasAccess, GET
//! `https://api.digitalocean.com/v2/droplets`, render daftar droplet).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "listvps",
    alias = [],
    category = "vps",
    description = "List semua VPS DigitalOcean (DIBLOKIR: config.digitalocean belum tersedia di Rust)",
    usage = ".listvps",
    example = ".listvps",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn listvps(_ctx: Ctx) -> Reply {
    Reply::None
}
