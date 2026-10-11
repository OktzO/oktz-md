//! Plugin batch pertama — port 1:1 dari `plugins/*.js` versi JS yang terbukti jalan.
//!
//! Cuma class A (logika murni, tanpa HTTP/media) sampai sekarang: 10 plugin
//! `cek*`. Kalau plugin butuh I/O atau WA fitur khusus, jangan dipaksa ke sini —
//! class B/C/D butuh API sendiri (lihat `docs/rust-rewrite/PLUGIN-CLASSIFICATION.md`).

pub mod cek;
