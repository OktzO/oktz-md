//! `plugins/owner/ganti-winner.js` — ganti gambar winner.jpg (thumbnail game winner).
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: memanggil `updateAssetUrl('winner', buffer, 'winner.jpeg')` dari
//! `../../src/lib/uploader.js`, yang bukan bagian dari `Ctx` API. Fungsi upload
//! aset belum ada di Rust API. Jangan port paksa.
//!
//! `ponytail:` kalau `updateAssetUrl` diekspose lewat `Ctx` nanti, port ulang
//! dari `plugins/owner/ganti-winner.js`.
//!
//! NOTE: baris `pub mod ganti_winner;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs dilarang disentuh karena sudah milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ganti-winner.jpg",
    alias = ["gantiwinner", "setwinner"],
    category = "owner",
    description = "Ganti gambar winner.jpg (thumbnail game winner) (DIBLOKIR: updateAssetUrl belum tersedia di Rust)",
    usage = ".ganti-winner.jpg (reply/kirim gambar)",
    example = ".ganti-winner.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_winner(_ctx: Ctx) -> Reply {
    Reply::None
}
