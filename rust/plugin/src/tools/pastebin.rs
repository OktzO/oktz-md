//! `plugins/tools/pastebin.js` — upload teks ke Pastebin.
//!
//! Catatan pendaftaran: `pub mod pastebin;` perlu ditambah manual ke
//! `rust/plugin/src/tools/mod.rs` lewat `scripts/pasang-modul.mjs` (mod.rs
//! milik orchestrator — jangan disentuh).
//!
//! BLOKIRAN — tidak di-port paksa:
//! - `process.env.PASTEBIN_DEV_KEY` (`pastebin.js:38`) tidak bisa dibaca
//!   plugin: `Ctx` tidak punya akses env, dan `config::Runtime`
//!   (`rust/lib/oktz-core/config.rs:225-235`) tidak punya field ini — ini juga
//!   yang tercatat di `docs/rust-rewrite/BLOCKED.md` §27 ("Akses
//!   env/`process.env`"). Tanpa kunci, cabang yang bisa dicapai cuma pesan
//!   "API Key belum diset", bukan upload sungguhan.
//! - `sendToolsPreview(sock, m.chat, responseText, ...)` (`pastebin.js:82-89`)
//!   memanggil `sock.sendPreview` (`src/lib/context.js:135-149`) dengan
//!   `contextInfo` newsletter dari `_saluranCtx()` (`src/lib/context.js:50-62`);
//!   `Reply` tidak punya varian preview maupun jalur `contextInfo`
//!   (`BLOCKED.md` §2/§15). Balasan teks polos akan kehilangan preview yang
//!   justru jadi inti `sendToolsPreview`.
//! - `api_paste_name` memakai `m.pushName` (`pastebin.js:48`) yang tidak ada
//!   di `Ctx` (`BLOCKED.md` §18) plus `toLocaleDateString("id-ID")`.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - teks = `m.args.join(' ')`, kalau quoted.text → gunakan quoted.
//!   - kalau kosong → kartu pakai (`pastebin.js:28-35`).
//!   - kalau `PASTEBIN_DEV_KEY` kosong → pesan belum diset (`pastebin.js:39-46`).
//!   - POST form ke https://pastebin.com/api/api_post.php
//!     (api_dev_key, api_option=paste, api_paste_code, api_paste_name,
//!     api_paste_private=1), timeout 15 detik (`pastebin.js:50-66`).
//!   - kalau body startsWith "Bad API request" → `❌ *ɢᴀɢᴀʟ*` (`pastebin.js:70-72`).
//!   - sukses → `sendToolsPreview` dengan kartu `✅ *ᴘᴀsᴛᴇʙɪɴ ʙᴇʀʜᴀsɪʟ*`
//!     (`pastebin.js:74-89`).
//!   - error → `te(m.prefix, m.command, m.pushName)` (`pastebin.js:91`).
//!
//! `ponytail:` jalur POST-nya sendiri sudah bisa direplikasi sekarang lewat
//! `HttpCall::post(url).form(body)` (`registry.rs:1202-1247`); yang menahan port
//! adalah kunci dev + preview di atas. Begitu kunci bisa dibaca dari
//! `Ctx::config()`/env dan `Reply` punya varian preview, implementasi bisa
//! ditulis tanpa menyentuh berkas ini lagi.
