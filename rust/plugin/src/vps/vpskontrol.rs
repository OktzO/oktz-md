//! `plugins/vps/vpskontrol.js` — kontrol VPS (on/off/restart).
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
//! Catatan: JS memiliki 4 command dalam satu file (`turnon`, `turnoff`,
//! `restartvps`, `rebootvps`) yang membedakan emoji dan teks aksi berdasarkan
//! `m.command`. Di Rust satu `#[plugin]` = satu nama command, jadi hanya
//! `turnon` yang didaftarkan di sini; varian lain perlu file terpisah kalau
//! `config.digitalocean` sudah tersedia.
//!
//! `ponytail:` kalau `config.digitalocean` ditambahkan ke `Runtime` nanti,
//! port ulang dari `plugins/vps/vpskontrol.js` (guard token + hasAccess,
//! POST ke `https://api.digitalocean.com/v2/droplets/{id}/actions` dengan
//! body `{ type: ... }`, reaksi `✅`, dan render status aksi).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "turnon",
    alias = [],
    category = "vps",
    description = "Kontrol VPS (on/off/restart) (DIBLOKIR: config.digitalocean belum tersedia di Rust)",
    usage = ".turnon <id>",
    example = ".turnon 123456789",
    cooldown = 10,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = false,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn vpskontrol(_ctx: Ctx) -> Reply {
    Reply::None
}
