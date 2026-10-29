//! `plugins/tools/deploy.js` — deploy HTML atau file ZIP ke Vercel.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: membaca `config.vercel.token` dari `config.js`, tetapi field
//! `vercel` tidak ada pada `oktz_core::config::Runtime` (hanya `api_keys`,
//! `owner`, `assets`, dst. — lihat `rust/crates/oktz-core/src/config.rs`).
//! Tidak ada jalur membaca kunci Vercel dari `Ctx`, jadi tidak bisa meniru
//! perilaku JS apa adanya. Selain itu, `AdmZip` (unzip buffer ZIP) dan
//! `m.quoted.download()` untuk file ZIP/HTML butuh kapasitas yang belum ada.
//!
//! `ponytail:` kalau `config.vercel` ditambahkan ke `Runtime` nanti, port ulang
//! dari `plugins/tools/deploy.js` (guard token, reaksi `🕕`, ekstrak ZIP/HTML,
//! POST ke `https://api.vercel.com/v13/deployments`, GET domains, fallback
//! ke `${name}.vercel.app`).
//!
//! NOTE: baris `pub mod deploy;` perlu ditambah ke `tools/mod.rs`
//! secara manual (mod.rs dilarang disentuh karena sudah milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "deploy",
    alias = ["vercel"],
    category = "owner",
    description = "Deploy HTML atau file ZIP ke Vercel (DIBLOKIR: config.vercel belum tersedia di Rust)",
    usage = ".deploy <namawebsite>",
    example = ".deploy mysite",
    cooldown = 60,
    energi = 1,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn deploy(_ctx: Ctx) -> Reply {
    Reply::None
}
