//! `plugins/main/jadibot.js` — jadikan nomor jadi bot (Pairing Code / QR).
//!
//! BLOKIR (gelombang-5 §Catatan tambahan): plugin butuh `sock` dan
//! `src/lib/jadibot-manager.js` (`startJadibot`, `isJadibotActive`) yang
//! merupakan entry point kedua / I/O WhatsApp. `Ctx` sengaja tidak punya
//! `sock`, dan `answer_handler` belum dipakai router untuk ini. Jangan port
//! paksa. File ini dibuat agar "semua file ada"; implementasi ditunda sampai
//! API soket/entry point kedua tersedia.
//!
//! Logika JS (cerminan, untuk port nanti):
//!   - guard `sender`; kalau `isJadibotActive(sender)` → balas sudah aktif +
//!     arahkan `.stopjadibot`.
//!   - argumen `qr` → mode QR, selainnya pairing code; balas menyiapkan.
//!   - `startJadibot(sock, m, sender, !useQR)`; tangkap error → balas gagal.
//!
//! `ponytail:` saat soket/entry point kedua tersedia, tambahkan
//! `pub mod jadibot;` di `src/main/mod.rs` secara manual (mod.rs milik
//! plugin lain — dilarang menyentuh per brief).
