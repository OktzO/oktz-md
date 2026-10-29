//! `plugins/owner/ganti-promote.js` — ganti gambar promote.jpg.
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: memanggil `updateAssetUrl('promote', buffer, 'promote.jpeg')` dari
//! `../../src/lib/uploader.js`, yang bukan bagian dari `Ctx` API. Fungsi upload
//! aset belum ada di Rust API. Jangan port paksa.
//!
//! `ponytail:` kalau `updateAssetUrl` diekspose lewat `Ctx` nanti, port ulang
//! dari `plugins/owner/ganti-promote.js`.
//!
//! NOTE: baris `pub mod ganti_promote;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs dilarang disentuh karena sudah milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ganti-promote.jpg",
    alias = ["gantipromote", "setpromote"],
    category = "owner",
    description = "Ganti gambar promote.jpg (DIBLOKIR: updateAssetUrl belum tersedia di Rust)",
    usage = ".ganti-promote.jpg (reply/kirim gambar)",
    example = ".ganti-promote.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_promote(_ctx: Ctx) -> Reply {
    Reply::None
}
