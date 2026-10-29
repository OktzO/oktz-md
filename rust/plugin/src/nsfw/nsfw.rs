//! `plugins/nsfw/nsfw.js` — koleksi gambar NSFW anime.
//!
//! Catatan: `pub mod nsfw;` perlu ditambah manual ke
//! `rust/crates/oktz-plugins/src/nsfw/mod.rs` (mod.rs milik plugin lain,
//! jangan disentuh).
//!
//! //! BLOKIRAN:
//! - `fs.readFileSync` pada `src/data/nsfw/<file>.json`: `ctx.berkas()` hanya
//!   melayani jalur di dalam akar aset; `src/data/nsfw` tidak dijamin berada
//!   di sana, dan daftar gambar dari JSON lokal tidak terbaca.
//! - `db.getGroup(m.chat)` / `db.setGroup(m.chat, groupData)` untuk toggle
//!   `groupData.nsfw`: `ctx.group_data` bisa dibaca, dan `Mutation::SetGroup`
//!   tersedia untuk tulis, tapi logika gabung baca-tulis per grup perlu
//!   koordinasi pipeline yang belum ada di handler tunggal.
//! - `m.isPremium` untuk gate premium: tidak ada di `Ctx` (hanya `is_owner`).
//! - `onigis` + `generateWAMessageFromContent` + `viewOnceMessage` +
//!   interactive message dengan tombol (Lanjut Lagi / Lihat Semua Kategori):
//!   `Reply::Interactive` hanya punya `body`, tanpa tombol/media header.
//! - `sock.relayMessage` + `contextInfo` quoted `m`: tidak ada di `Reply`.
//! - `axios` GET ke `api.waifu.pics/nsfw/<endpoint>`: HTTP tersedia, jadi
//!   bagian fetch gambar online bisa direplikasi; bagian di atas yang tidak.
