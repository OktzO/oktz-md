//! `plugins/owner/ganti-games.js` — ganti gambar games.jpg (thumbnail games).
//!
//! ## BLOKIR — tidak di-port.
//!
//! Alasan: memanggil `updateAssetUrl('games', buffer, 'games.jpeg')` dari
//! `../../src/lib/uploader.js`, yang bukan bagian dari `Ctx` API. `m.download()`
//! / `m.quoted.download()` tersedia via `MediaFetch`, tetapi fungsi upload aset
//! belum ada di Rust API. Jangan port paksa.
//!
//! `ponytail:` kalau `updateAssetUrl` diekspose lewat `Ctx` nanti, port ulang
//! dari `plugins/owner/ganti-games.js` (cek `m.isImage` / `m.quoted.type`,
//! unduh buffer, upload, kirim URL baru).
//!
//! NOTE: baris `pub mod ganti_games;` perlu ditambah ke `owner/mod.rs`
//! secara manual (mod.rs dilarang disentuh karena sudah milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "ganti-games.jpg",
    alias = ["gantigames", "setgames"],
    category = "owner",
    description = "Ganti gambar games.jpg (thumbnail games) (DIBLOKIR: updateAssetUrl belum tersedia di Rust)",
    usage = ".ganti-games.jpg (reply/kirim gambar)",
    example = ".ganti-games.jpg",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false,
    is_owner = true,
    is_premium = false,
    is_admin = false,
    is_bot_admin = false
)]
pub fn ganti_games(_ctx: Ctx) -> Reply {
    Reply::None
}
