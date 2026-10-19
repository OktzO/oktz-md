//! `plugins/sticker/smeme-animated.js` — stiker meme animasi.
//!
//! Catatan pendaftaran: `pub mod smeme_animated;` perlu ditambah manual ke
//! `rust/plugin/src/sticker/mod.rs` lewat `scripts/pasang-modul.mjs` (mod.rs
//! milik orchestrator — jangan disentuh).
//!
//! BLOKIRAN — tidak di-port paksa:
//! - `uploadImage(buffer)` (`src/lib/uploader.js:14-40`, dipanggil
//!   `smeme-animated.js:48`) mencoba 9 host keyless berurutan
//!   (`src/lib/upload-providers.js`, timeout 20 detik per host — `uploader.js:3`)
//!   dan melempar kalau semuanya gagal (`uploader.js:39`). Tidak ada padanannya
//!   di API plugin: `HttpBody::Multipart` (`rust/lib/oktz-plugin-api/registry.rs:1038-1060`)
//!   bisa membentuk request multipart, tapi tiap host punya bentuk/token
//!   sendiri; tidak ada satu endpoint yang bisa dipakai apa adanya. Ini juga
//!   yang tercatat di `docs/rust-rewrite/BLOCKED.md` §13.
//! - `api.neoxr.eu/api/memegenvid?image=<imageUrl>...` (`smeme-animated.js:50`)
//!   butuh `imageUrl` hasil upload di atas, jadi HTTP-nya tersedia tapi
//!   parameternya tidak bisa diisi.
//! - `sock.sendImageAsSticker(m.chat, buffer, m, { packname, author })`
//!   (`smeme-animated.js:58-61`, fallback `:66-71`) tidak ada di `Reply`;
//!   `Reply::Sticker` ada (`registry.rs:300-304`), tapi metadata
//!   `packname`/`author` (`config.sticker.packname`/`config.sticker.author`)
//!   tidak punya kanal sama sekali — tidak ada penyebutan `packname` di jalur
//!   kirim `oktz-wa`/`oktz-core` selain struct config (`config.rs:57-60`).
//! - Reaksi `🕕` lalu `✅`/`☢` (`smeme-animated.js:37,73,75`): satu `Reply`
//!   membawa satu reaksi.
//!
//! Yang TIDAK memblokir: `downloadMediaMessage(targetMsg, 'buffer', ...)`
//! (`smeme-animated.js:41-46`) setara `ctx.download_media()` /
//! `ctx.download_quoted_media()` (`registry.rs:794-801`), dan deteksi
//! `isImage`/`isQuotedImage` (`smeme-animated.js:30-35`) setara
//! `ctx.is_media_kind(...)` / `ctx.quoted()`. Jadi begitu kapabilitas upload
//! ada, port lengkap bisa ditulis: split `'|'` atas/bawah, download buffer,
//! upload, GET Neoxr, lalu `Reply::sticker(MediaSource::Bytes)`.
