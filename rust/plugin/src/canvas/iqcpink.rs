//! `plugins/canvas/iqcpink.js` — membuat Fake Quote iOS style versi pink.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port karena:
//!   1. `@napi-rs/canvas` (`createCanvas`, `loadImage`, `GlobalFonts`) —
//!      crate canvas tidak ada di dependensi `oktz-plugins`.
//!   2. Rendering teks dengan emoji Apple, wrapping, rotasi, dan bubble
//!      iOS — memerlukan canvas pixel-per-pixel yang sama persis dengan JS.
//!   3. `node:fs/promises` untuk menulis hasil ke `temp/` — `ctx.berkas()`
//!      tersedia, namun rendering gambar tidak.
//!   4. Apple emoji JSON dari GitHub — dapat di-fetch, namun tidak dapat
//!      dirender tanpa canvas.
//!
//! Berkas ini dibuat sebagai catatan; implementasi ditunda sampai crate
//! canvas tersedia.
//!
//! `ponytail:` saat crate canvas tersedia di `oktz-plugins`,
//! implementasikan ulang dan tambahkan `pub mod iqcpink;`
//! di `src/canvas/mod.rs` secara manual.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "iqcpink",
    alias = [],
    category = "canvas",
    description = "Membuat Fake Quote iOS style versi pink — BLOKIRAN",
    usage = ".iqcpink [text/reply]",
    cooldown = 5,
    energi = 2,
    is_group = false,
    is_private = false
)]
pub fn iqcpink(ctx: Ctx) -> Reply {
    let _ = ctx;
    Reply::text(
        "❌ *FAKE QUOTE iOS PINK — TIDAK TERSEDIA*\n\n\
         Port Rust tertunda karena butuh `@napi-rs/canvas` untuk render\n\
         gambar quote yang belum tersedia di `oktz-plugins`.\n\n\
         Contoh JS: `.iqcpink <teks kamu>`",
    )
}
