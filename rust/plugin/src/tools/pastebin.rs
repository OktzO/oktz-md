//! `plugins/tools/pastebin.js` — upload teks ke Pastebin.
//!
//! BLOKIR (gelombang-5 §Catatan tambahan): plugin butuh `sock` untuk
//! `sendToolsPreview` dari `src/lib/context.js` (I/O WhatsApp, bukan
//! `Reply`), dan baca `process.env.PASTEBIN_DEV_KEY` yang tidak ada di
//! `Runtime`. `Ctx` sengaja tidak punya `sock`. Jangan port paksa. File ini
//! dibuat agar "semua file ada"; implementasi ditunda sampai API soket dan
//! kunci pastebin tersedia.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - teks = `m.args.join(' ')`, kalau quoted.text → gunakan quoted.
//!   - kalau kosong → kartu pakai.
//!   - kalau `PASTEBIN_DEV_KEY` kosong → pesan belum diset.
//!   - POST form ke https://pastebin.com/api/api_post.php
//!     (api_dev_key, api_option=paste, api_paste_code, api_paste_name,
//!     api_paste_private=1), timeout 15 detik.
//!   - kalau body startsWith "Bad API request" → gagal.
//!   - `sendToolsPreview(sock, m.chat, responseText, "Pastebin Upload",
//!     api_paste_name, { quoted: m })`.
//!   - error → `te(m.prefix, m.command, m.pushName)`.
//!
//! `ponytail:` saat soket dan `PASTEBIN_DEV_KEY` (lewat `Ctx::config()` atau
//! env) tersedia, tambahkan `pub mod pastebin;` di `src/tools/mod.rs` secara
//! manual (mod.rs milik plugin lain — dilarang menyentuh per brief).
