//! `plugins/canvas/fakeml.js` — membuat fake ML profile card.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port karena:
//!   1. `uploadTo0x0(buffer, { filename, contentType })` — unggah berkas ke
//!      layanan pihak ketiga (0x0.st) yang tidak ada di `Ctx`.
//!   2. `sock.profilePictureUrl(sender, "image")` — tersedia di `Sock`,
//!      namun hasilnya butuh di-unggah ulang via `uploadTo0x0`.
//!   3. API eksternal `api.nexray.web.id/maker/fakelobyml` dapat dipanggil
//!      via `ctx.http`, namun butuh `avatar` URL hasil unggahan.
//!
//! Tanpa `uploadTo0x0`, alur lengkap tidak dapat direplikasi.
//! Berkas ini dibuat sebagai catatan.
//!
//! `ponytail:` saat mekanisme unggah berkas ke URL eksternal tersedia,
//! implementasikan ulang dan tambahkan `pub mod fakeml;` di `src/canvas/mod.rs`
//! secara manual.

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "fakeml",
    alias = ["mlbbfake", "mlcard", "mlfake"],
    category = "canvas",
    description = "Membuat fake ML profile card — BLOKIRAN",
    usage = ".fakeml <nama> (reply/kirim foto)",
    example = ".fakeml Misaki",
    cooldown = 10,
    energi = 1,
    is_group = false,
    is_private = false
)]
pub fn fakeml(ctx: Ctx) -> Reply {
    let _ = ctx;
    Reply::text(
        "❌ *FAKE ML — TIDAK TERSEDIA*\n\n\
         Port Rust tertunda karena butuh `uploadTo0x0` (unggah berkas ke\n\
         layanan eksternal) yang belum tersedia di `Ctx`.\n\n\
         Contoh JS: `.fakeml <nama>` (reply/kirim foto)",
    )
}
