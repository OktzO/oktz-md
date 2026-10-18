//! `plugins/cek/cekfemboy.js` — cek seberapa femboy kamu.
//!
//! `//! BLOKIRAN:` plugin ini tidak dapat di-port karena:
//!   1. `cekfemboy` scraper (`../../src/scraper/lufemboy.js`) tidak ada di
//!      crate `oktz_scraper`. Brief melarang menyalin logika scraper ke plugin.
//!   2. `queueFFmpeg` untuk convert GIF ke MP4 — ffmpeg tersedia, namun
//!      memerlukan buffer GIF dari scraper yang tidak ada.
//!   3. `sock.sendMedia(..., { type: 'video', gifPlayback: true })` —
//!      `Reply::video_gif` tersedia, namun butuh byte video dari konversi.
//!
//! Bagian hasil teks (`result.hasil`) bisa di-port jika scraper tersedia.
//! Berkas ini dibuat sebagai catatan.
//!
//! `ponytail:` saat scraper `lufemboy` tersedia di `oktz_scraper`,
//! implementasikan ulang dan tambahkan `pub mod cekfemboy;`
//! di `src/cek/mod.rs` secara manual (mod.rs milik plugin lain).

use oktz_plugin_api::{Ctx, Reply, plugin};

#[plugin(
    name = "cekfemboy",
    alias = ["femboy"],
    category = "cek",
    description = "Cek seberapa femboy kamu — BLOKIRAN",
    usage = ".cekfemboy <nama>",
    example = ".cekfemboy Budi",
    cooldown = 5,
    energi = 0,
    is_group = false,
    is_private = false
)]
pub fn cekfemboy(ctx: Ctx) -> Reply {
    let nama = if ctx.text.trim().is_empty() {
        "Kamu".to_string()
    } else {
        ctx.text.trim().to_string()
    };

    Reply::text(
        format!(
            "❌ *CEK FEMBOY — TIDAK TERSEDIA*\n\n\
             Scraper `lufemboy` belum tersedia di `oktz_scraper`.\n\
             Port Rust tertunda sampai scraper bisa dipanggil.\n\n\
             Contoh JS: `.cekfemboy {nama}`"
        )
    )
}
